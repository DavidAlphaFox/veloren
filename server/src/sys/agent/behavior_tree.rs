use common::{
    comp::{
        agent::{
            AgentEvent, Target, TimerAction, DEFAULT_INTERACTION_TIME, TRADE_INTERACTION_TIME,
        },
        compass::{Direction, Distance},
        dialogue::{MoodContext, MoodState, Subject},
        invite::{InviteKind, InviteResponse},
        Agent, Alignment, BehaviorState, Body, BuffKind, ControlAction, ControlEvent, Controller,
        InputKind, InventoryEvent, UnresolvedChatMsg, UtteranceKind,
    },
    event::{Emitter, ServerEvent},
    path::TraversalConfig,
    rtsim::{Memory, MemoryItem, RtSimEvent},
    trade::{TradeAction, TradePhase, TradeResult},
};
use rand::{prelude::ThreadRng, thread_rng, Rng};
use specs::saveload::{Marker, MarkerAllocator};
use vek::Vec2;

use crate::rtsim::entity::PersonalityTrait;

use super::{
    consts::{
        DAMAGE_MEMORY_DURATION, FLEE_DURATION, HEALING_ITEM_THRESHOLD, MAX_FLEE_DIST,
        MAX_FOLLOW_DIST, NPC_PICKUP_RANGE, RETARGETING_THRESHOLD_SECONDS,
    },
    data::{AgentData, ReadData, TargetData},
    util::{get_entity_by_id, is_dead, is_dead_or_invulnerable, is_invulnerable, stop_pursuing},
};

/// Struct containing essential data for running a behavior tree
struct BehaviorData<'a, 'b, 'c> {
    agent: &'a mut Agent,
    agent_data: AgentData<'a>,
    read_data: &'a ReadData<'a>,
    event_emitter: &'a mut Emitter<'c, ServerEvent>,
    controller: &'a mut Controller,
    rng: &'b mut ThreadRng,
}

/// Behavior function
/// Determines if the current situation can be handled and act accordingly
/// Returns true if an action has been taken, stopping the tree execution
type BehaviorFn = fn(&mut BehaviorData) -> bool;

/// ~~list~~ ""tree"" of behavior functions
/// This struct will allow you to run through multiple behavior function until
/// one finally handles an event
pub struct BehaviorTree {
    tree: Vec<BehaviorFn>,
}

impl BehaviorTree {
    pub fn root() -> Self {
        Self {
            tree: vec![
                react_on_dangerous_fall,
                react_if_on_fire,
                target_if_attacked,
                do_target_tree_if_target,
                do_idle_tree,
            ],
        }
    }

    pub fn target() -> Self {
        Self {
            tree: vec![
                untarget_if_dead,
                do_hostile_tree_if_hostile,
                do_pet_tree_if_owned,
                do_pickup_loot,
                untarget,
                do_idle_tree,
            ],
        }
    }

    pub fn pet() -> Self {
        Self {
            tree: vec![follow_if_far_away, attack_if_owner_hurt, do_idle_tree],
        }
    }

    pub fn interaction() -> Self {
        Self {
            tree: vec![
                increment_timer_deltatime,
                handle_inbox_talk,
                handle_inbox_trade_invite,
                handle_inbox_trade_accepted,
                handle_inbox_finished_trade,
                handle_inbox_update_pending_trade,
            ],
        }
    }

    pub fn hostile() -> Self {
        Self {
            tree: vec![heal_self_if_hurt, hurt_utterance, do_combat],
        }
    }

    pub fn idle() -> Self {
        Self {
            tree: vec![
                set_owner_if_no_target,
                process_inbox_sound_and_hurt,
                process_inbox_interaction,
                handle_timer,
            ],
        }
    }

    /// Run the behavior tree until an event has been handled
    pub fn run<'a, 'b>(
        &self,
        agent: &'a mut Agent,
        agent_data: AgentData<'a>,
        read_data: &'a ReadData,
        event_emitter: &'a mut Emitter<'b, ServerEvent>,
        controller: &'a mut Controller,
    ) -> bool {
        let mut behavior_data = BehaviorData {
            agent,
            agent_data,
            read_data,
            event_emitter,
            controller,
            rng: &mut thread_rng(),
        };

        self.run_with_behavior_data(&mut behavior_data)
    }

    fn run_with_behavior_data(&self, bdata: &mut BehaviorData) -> bool {
        for behavior_fn in self.tree.iter() {
            if behavior_fn(bdata) {
                return true;
            }
        }
        false
    }
}

/// If falling velocity is critical, throw everything
/// and save yourself!
///
/// If can fly - fly.
/// If have glider - glide.
/// Else, rest in peace.
fn react_on_dangerous_fall(bdata: &mut BehaviorData) -> bool {
    // Falling damage starts from 30.0 as of time of writing
    // But keep in mind our 25 m/s gravity
    let is_falling_dangerous = bdata.agent_data.vel.0.z < -20.0;

    if is_falling_dangerous && bdata.agent_data.traversal_config.can_fly {
        bdata.agent_data.fly_upward(bdata.controller);
        return true;
    } else if is_falling_dangerous && bdata.agent_data.glider_equipped {
        bdata.agent_data.glider_fall(bdata.controller);
        return true;
    }
    false
}

/// If on fire and able, stop, drop, and roll
fn react_if_on_fire(bdata: &mut BehaviorData) -> bool {
    let is_on_fire = bdata
        .read_data
        .buffs
        .get(*bdata.agent_data.entity)
        .map_or(false, |b| b.kinds.contains_key(&BuffKind::Burning));

    if is_on_fire
        && bdata.agent_data.body.map_or(false, |b| b.is_humanoid())
        && bdata.agent_data.physics_state.on_ground.is_some()
        && bdata
            .rng
            .gen_bool((2.0 * bdata.read_data.dt.0).clamp(0.0, 1.0) as f64)
    {
        bdata.controller.inputs.move_dir = bdata
            .agent_data
            .ori
            .look_vec()
            .xy()
            .try_normalized()
            .unwrap_or_else(Vec2::zero);
        bdata.controller.push_basic_input(InputKind::Roll);
        return true;
    }
    false
}

/// Target an entity that's attacking us if the attack was recent and we have
/// a health component
fn target_if_attacked(bdata: &mut BehaviorData) -> bool {
    match bdata.agent_data.health {
        Some(health)
            if bdata.read_data.time.0 - health.last_change.time.0 < DAMAGE_MEMORY_DURATION =>
        {
            if let Some(by) = health.last_change.damage_by() {
                if let Some(attacker) = bdata
                    .read_data
                    .uid_allocator
                    .retrieve_entity_internal(by.uid().0)
                {
                    // If target is dead or invulnerable (for now, this only
                    // means safezone), untarget them and idle.
                    if is_dead_or_invulnerable(attacker, bdata.read_data) {
                        bdata.agent.target = None;
                    } else {
                        if bdata.agent.target.is_none() {
                            bdata
                                .controller
                                .push_event(ControlEvent::Utterance(UtteranceKind::Angry));
                        }

                        // Determine whether the new target should be a priority
                        // over the old one (i.e: because it's either close or
                        // because they attacked us).
                        if bdata.agent.target.map_or(true, |target| {
                            bdata.agent_data.is_more_dangerous_than_target(
                                attacker,
                                target,
                                bdata.read_data,
                            )
                        }) {
                            bdata.agent.target = Some(Target {
                                target: attacker,
                                hostile: true,
                                selected_at: bdata.read_data.time.0,
                                aggro_on: true,
                            });
                        }

                        // Remember this attack if we're an RtSim entity
                        if let Some(attacker_stats) = bdata
                            .agent_data
                            .rtsim_entity
                            .and(bdata.read_data.stats.get(attacker))
                        {
                            bdata
                                .agent
                                .add_fight_to_memory(&attacker_stats.name, bdata.read_data.time.0);
                        }
                    }
                }
            }
        },
        _ => {},
    }
    false
}

fn do_target_tree_if_target(bdata: &mut BehaviorData) -> bool {
    if bdata.agent.target.is_some() {
        BehaviorTree::target().run_with_behavior_data(bdata);
        return true;
    }
    false
}

fn do_idle_tree(bdata: &mut BehaviorData) -> bool {
    BehaviorTree::idle().run_with_behavior_data(bdata);
    true
}

/// If target is dead, forget them
fn untarget_if_dead(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if let Some(tgt_health) = bdata.read_data.healths.get(target) {
            // If target is dead, forget them
            if tgt_health.is_dead {
                if let Some(tgt_stats) = bdata
                    .agent_data
                    .rtsim_entity
                    .and(bdata.read_data.stats.get(target))
                {
                    bdata.agent.forget_enemy(&tgt_stats.name);
                }
                bdata.agent.target = None;
                return true;
            }
        }
    }
    false
}

/// If target is hostile, hostile tree
fn do_hostile_tree_if_hostile(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { hostile, .. }) = bdata.agent.target {
        if hostile {
            BehaviorTree::hostile().run_with_behavior_data(bdata);
            return true;
        }
    }
    false
}

/// if owned, act as pet to them
fn do_pet_tree_if_owned(bdata: &mut BehaviorData) -> bool {
    if let (Some(Target { target, .. }), Some(Alignment::Owned(uid))) =
        (bdata.agent.target, bdata.agent_data.alignment)
    {
        if bdata.read_data.uids.get(target) == Some(uid) {
            BehaviorTree::pet().run_with_behavior_data(bdata);
        } else {
            bdata.agent.target = None;
            BehaviorTree::idle().run_with_behavior_data(bdata);
        }
        return true;
    }
    false
}

fn do_pickup_loot(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if matches!(bdata.read_data.bodies.get(target), Some(Body::ItemDrop(_))) {
            if let Some(tgt_pos) = bdata.read_data.positions.get(target) {
                let dist_sqrd = bdata.agent_data.pos.0.distance_squared(tgt_pos.0);
                if dist_sqrd < NPC_PICKUP_RANGE.powi(2) {
                    if let Some(uid) = bdata.read_data.uids.get(target) {
                        bdata
                            .controller
                            .push_event(ControlEvent::InventoryEvent(InventoryEvent::Pickup(*uid)));
                    }
                } else if let Some((bearing, speed)) = bdata.agent.chaser.chase(
                    &*bdata.read_data.terrain,
                    bdata.agent_data.pos.0,
                    bdata.agent_data.vel.0,
                    tgt_pos.0,
                    TraversalConfig {
                        min_tgt_dist: NPC_PICKUP_RANGE - 1.0,
                        ..bdata.agent_data.traversal_config
                    },
                ) {
                    bdata.controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                            * speed.min(0.2 + (dist_sqrd - (NPC_PICKUP_RANGE - 1.5).powi(2)) / 8.0);
                    bdata.agent_data.jump_if(bearing.z > 1.5, bdata.controller);
                    bdata.controller.inputs.move_z = bearing.z;
                }
            }
            return true;
        }
    }
    false
}

fn untarget(bdata: &mut BehaviorData) -> bool {
    bdata.agent.target = None;
    false
}

// If too far away, then follow
fn follow_if_far_away(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if let Some(tgt_pos) = bdata.read_data.positions.get(target) {
            let dist_sqrd = bdata.agent_data.pos.0.distance_squared(tgt_pos.0);

            if dist_sqrd > (MAX_FOLLOW_DIST).powi(2) {
                bdata.agent_data.follow(
                    bdata.agent,
                    bdata.controller,
                    &bdata.read_data.terrain,
                    tgt_pos,
                );
                return true;
            }
        }
    }
    false
}

/// Attack target's attacker (if there is one)
/// Target is the owner in this case
fn attack_if_owner_hurt(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if bdata.read_data.positions.get(target).is_some() {
            let owner_recently_attacked =
                if let Some(target_health) = bdata.read_data.healths.get(target) {
                    bdata.read_data.time.0 - target_health.last_change.time.0 < 5.0
                        && target_health.last_change.amount < 0.0
                } else {
                    false
                };

            if owner_recently_attacked {
                bdata.agent_data.attack_target_attacker(
                    bdata.agent,
                    bdata.read_data,
                    bdata.controller,
                    bdata.rng,
                );
                return true;
            }
        }
    }
    false
}

/// Set owner if no target
fn set_owner_if_no_target(bdata: &mut BehaviorData) -> bool {
    let small_chance = bdata.rng.gen_bool(0.1);

    if bdata.agent.target.is_none() && small_chance {
        if let Some(Alignment::Owned(owner)) = bdata.agent_data.alignment {
            if let Some(owner) = get_entity_by_id(owner.id(), bdata.read_data) {
                bdata.agent.target = Some(Target::new(owner, false, bdata.read_data.time.0, false));
            }
        }
    }
    false
}

/// Interact if incoming messages
fn process_inbox_sound_and_hurt(bdata: &mut BehaviorData) -> bool {
    if !bdata.agent.inbox.is_empty() {
        if matches!(
            bdata.agent.inbox.front(),
            Some(AgentEvent::ServerSound(_)) | Some(AgentEvent::Hurt)
        ) {
            let sound = bdata.agent.inbox.pop_front();
            match sound {
                Some(AgentEvent::ServerSound(sound)) => {
                    bdata.agent.sounds_heard.push(sound);
                },
                Some(AgentEvent::Hurt) => {
                    // Hurt utterances at random upon receiving damage
                    if bdata.rng.gen::<f32>() < 0.4 {
                        bdata.controller.push_utterance(UtteranceKind::Hurt);
                    }
                },
                //Note: this should be unreachable
                Some(_) | None => {},
            }
        } else {
            bdata.agent.action_state.timer = 0.1;
        }
    }
    false
}

/// If we receive a new interaction, start the interaction timer
fn process_inbox_interaction(bdata: &mut BehaviorData) -> bool {
    if bdata.agent.allowed_to_speak() && BehaviorTree::interaction().run_with_behavior_data(bdata) {
        bdata
            .agent
            .timer
            .start(bdata.read_data.time.0, TimerAction::Interact);
    }
    false
}

fn handle_timer(bdata: &mut BehaviorData) -> bool {
    let timeout = if bdata.agent.behavior.is(BehaviorState::TRADING) {
        TRADE_INTERACTION_TIME
    } else {
        DEFAULT_INTERACTION_TIME
    };

    match bdata.agent.timer.timeout_elapsed(
        bdata.read_data.time.0,
        TimerAction::Interact,
        timeout as f64,
    ) {
        None => {
            // Look toward the interacting entity for a while
            if let Some(Target { target, .. }) = &bdata.agent.target {
                bdata
                    .agent_data
                    .look_toward(bdata.controller, bdata.read_data, *target);
                bdata.controller.push_action(ControlAction::Talk);
            }
        },
        Some(just_ended) => {
            if just_ended {
                bdata.agent.target = None;
                bdata.controller.push_action(ControlAction::Stand);
            }

            if bdata.rng.gen::<f32>() < 0.1 {
                bdata
                    .agent_data
                    .choose_target(bdata.agent, bdata.controller, bdata.read_data);
            } else {
                bdata.agent_data.handle_sounds_heard(
                    bdata.agent,
                    bdata.controller,
                    bdata.read_data,
                    bdata.rng,
                );
            }
        },
    }
    false
}

fn heal_self_if_hurt(bdata: &mut BehaviorData) -> bool {
    if bdata.agent_data.damage < HEALING_ITEM_THRESHOLD
        && bdata
            .agent_data
            .heal_self(bdata.agent, bdata.controller, false)
    {
        bdata.agent.action_state.timer = 0.01;
        return true;
    }
    false
}

fn hurt_utterance(bdata: &mut BehaviorData) -> bool {
    if let Some(AgentEvent::Hurt) = bdata.agent.inbox.pop_front() {
        // Hurt utterances at random upon receiving damage
        if bdata.rng.gen::<f32>() < 0.4 {
            bdata.controller.push_utterance(UtteranceKind::Hurt);
        }
    }
    false
}

fn do_combat(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        event_emitter,
        controller,
        rng,
    } = bdata;

    if let Some(Target {
        target,
        selected_at,
        aggro_on,
        ..
    }) = &mut agent.target
    {
        let target = *target;
        let selected_at = *selected_at;
        if let Some(tgt_pos) = read_data.positions.get(target) {
            let dist_sqrd = agent_data.pos.0.distance_squared(tgt_pos.0);
            let origin_dist_sqrd = match agent.patrol_origin {
                Some(pos) => pos.distance_squared(agent_data.pos.0),
                None => 1.0,
            };

            let own_health_fraction = match agent_data.health {
                Some(val) => val.fraction(),
                None => 1.0,
            };
            let target_health_fraction = match read_data.healths.get(target) {
                Some(val) => val.fraction(),
                None => 1.0,
            };

            let in_aggro_range = agent
                .psyche
                .aggro_dist
                .map_or(true, |ad| dist_sqrd < ad.powi(2));

            if in_aggro_range {
                *aggro_on = true;
            }
            let aggro_on = *aggro_on;

            if agent_data.below_flee_health(agent) {
                let has_opportunity_to_flee = agent.action_state.timer < FLEE_DURATION;
                let within_flee_distance = dist_sqrd < MAX_FLEE_DIST.powi(2);

                // FIXME: Using action state timer to see if allowed to speak is a hack.
                if agent.action_state.timer == 0.0 {
                    agent_data.cry_out(agent, event_emitter, read_data);
                    agent.action_state.timer = 0.01;
                } else if within_flee_distance && has_opportunity_to_flee {
                    agent_data.flee(agent, controller, tgt_pos, &read_data.terrain);
                    agent.action_state.timer += read_data.dt.0;
                } else {
                    agent.action_state.timer = 0.0;
                    agent.target = None;
                    agent_data.idle(agent, controller, read_data, rng);
                }
            } else if is_dead(target, read_data) {
                agent_data.exclaim_relief_about_enemy_dead(agent, event_emitter);
                agent.target = None;
                agent_data.idle(agent, controller, read_data, rng);
            } else if is_invulnerable(target, read_data)
                || stop_pursuing(
                    dist_sqrd,
                    origin_dist_sqrd,
                    own_health_fraction,
                    target_health_fraction,
                    read_data.time.0 - selected_at,
                    &agent.psyche,
                )
            {
                agent.target = None;
                agent_data.idle(agent, controller, read_data, rng);
            } else {
                let is_time_to_retarget =
                    read_data.time.0 - selected_at > RETARGETING_THRESHOLD_SECONDS;

                if !in_aggro_range && is_time_to_retarget {
                    agent_data.choose_target(agent, controller, read_data);
                }

                if aggro_on {
                    let target_data = TargetData::new(
                        tgt_pos,
                        read_data.bodies.get(target),
                        read_data.scales.get(target),
                    );
                    let tgt_name = read_data.stats.get(target).map(|stats| stats.name.clone());

                    tgt_name.map(|tgt_name| agent.add_fight_to_memory(&tgt_name, read_data.time.0));
                    agent_data.attack(agent, controller, &target_data, read_data, rng);
                } else {
                    agent_data.menacing(agent, controller, target, read_data, event_emitter, rng);
                }
            }
        }
    }
    false
}

fn increment_timer_deltatime(bdata: &mut BehaviorData) -> bool {
    bdata.agent.action_state.timer += bdata.read_data.dt.0;
    false
}

/// Handles Talk event if the front of the agent's inbox contains one
fn handle_inbox_talk(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        event_emitter,
        controller,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::Talk(_, _))) {
        return false;
    }

    if let Some(AgentEvent::Talk(by, subject)) = agent.inbox.pop_front() {
        if agent.allowed_to_speak() {
            if let Some(target) = get_entity_by_id(by.id(), read_data) {
                agent.target = Some(Target::new(target, false, read_data.time.0, false));

                if agent_data.look_toward(controller, read_data, target) {
                    controller.push_action(ControlAction::Stand);
                    controller.push_action(ControlAction::Talk);
                    controller.push_utterance(UtteranceKind::Greeting);

                    match subject {
                        Subject::Regular => {
                            if let (Some((_travel_to, destination_name)), Some(rtsim_entity)) =
                                (&agent.rtsim_controller.travel_to, &agent_data.rtsim_entity)
                            {
                                let personality = &rtsim_entity.brain.personality;
                                let standard_response_msg = || -> String {
                                    if personality
                                        .personality_traits
                                        .contains(PersonalityTrait::Extroverted)
                                    {
                                        format!(
                                            "I'm heading to {}! Want to come along?",
                                            destination_name
                                        )
                                    } else if personality
                                        .personality_traits
                                        .contains(PersonalityTrait::Disagreeable)
                                    {
                                        "Hrm.".to_string()
                                    } else {
                                        "Hello!".to_string()
                                    }
                                };
                                let msg = if let Some(tgt_stats) = read_data.stats.get(target) {
                                    agent.rtsim_controller.events.push(RtSimEvent::AddMemory(
                                        Memory {
                                            item: MemoryItem::CharacterInteraction {
                                                name: tgt_stats.name.clone(),
                                            },
                                            time_to_forget: read_data.time.0 + 600.0,
                                        },
                                    ));
                                    if rtsim_entity.brain.remembers_character(&tgt_stats.name) {
                                        if personality
                                            .personality_traits
                                            .contains(PersonalityTrait::Extroverted)
                                        {
                                            format!(
                                                "Greetings fair {}! It has been far too long \
                                                 since last I saw you. I'm going to {} right now.",
                                                &tgt_stats.name, destination_name
                                            )
                                        } else if personality
                                            .personality_traits
                                            .contains(PersonalityTrait::Disagreeable)
                                        {
                                            "Oh. It's you again.".to_string()
                                        } else {
                                            format!(
                                                "Hi again {}! Unfortunately I'm in a hurry right \
                                                 now. See you!",
                                                &tgt_stats.name
                                            )
                                        }
                                    } else {
                                        standard_response_msg()
                                    }
                                } else {
                                    standard_response_msg()
                                };
                                agent_data.chat_npc(msg, event_emitter);
                            } else if agent.behavior.can_trade() {
                                if !agent.behavior.is(BehaviorState::TRADING) {
                                    controller.push_initiate_invite(by, InviteKind::Trade);
                                    agent_data.chat_npc(
                                        "npc.speech.merchant_advertisement",
                                        event_emitter,
                                    );
                                } else {
                                    let default_msg = "npc.speech.merchant_busy";
                                    let msg = agent_data.rtsim_entity.map_or(default_msg, |e| {
                                        if e.brain
                                            .personality
                                            .personality_traits
                                            .contains(PersonalityTrait::Disagreeable)
                                        {
                                            "npc.speech.merchant_busy_rude"
                                        } else {
                                            default_msg
                                        }
                                    });
                                    agent_data.chat_npc(msg, event_emitter);
                                }
                            } else {
                                let mut rng = thread_rng();
                                if let Some(extreme_trait) = agent_data
                                    .rtsim_entity
                                    .and_then(|e| e.brain.personality.random_chat_trait(&mut rng))
                                {
                                    let msg = match extreme_trait {
                                        PersonalityTrait::Open => "npc.speech.villager_open",
                                        PersonalityTrait::Adventurous => {
                                            "npc.speech.villager_adventurous"
                                        },
                                        PersonalityTrait::Closed => "npc.speech.villager_closed",
                                        PersonalityTrait::Conscientious => {
                                            "npc.speech.villager_conscientious"
                                        },
                                        PersonalityTrait::Busybody => {
                                            "npc.speech.villager_busybody"
                                        },
                                        PersonalityTrait::Unconscientious => {
                                            "npc.speech.villager_unconscientious"
                                        },
                                        PersonalityTrait::Extroverted => {
                                            "npc.speech.villager_extroverted"
                                        },
                                        PersonalityTrait::Introverted => {
                                            "npc.speech.villager_introverted"
                                        },
                                        PersonalityTrait::Agreeable => {
                                            "npc.speech.villager_agreeable"
                                        },
                                        PersonalityTrait::Sociable => {
                                            "npc.speech.villager_sociable"
                                        },
                                        PersonalityTrait::Disagreeable => {
                                            "npc.speech.villager_disagreeable"
                                        },
                                        PersonalityTrait::Neurotic => {
                                            "npc.speech.villager_neurotic"
                                        },
                                        PersonalityTrait::Seeker => "npc.speech.villager_seeker",
                                        PersonalityTrait::SadLoner => {
                                            "npc.speech.villager_sad_loner"
                                        },
                                        PersonalityTrait::Worried => "npc.speech.villager_worried",
                                        PersonalityTrait::Stable => "npc.speech.villager_stable",
                                    };
                                    agent_data.chat_npc(msg, event_emitter);
                                } else {
                                    agent_data.chat_npc("npc.speech.villager", event_emitter);
                                }
                            }
                        },
                        Subject::Trade => {
                            if agent.behavior.can_trade() {
                                if !agent.behavior.is(BehaviorState::TRADING) {
                                    controller.push_initiate_invite(by, InviteKind::Trade);
                                    agent_data.chat_npc(
                                        "npc.speech.merchant_advertisement",
                                        event_emitter,
                                    );
                                } else {
                                    agent_data.chat_npc("npc.speech.merchant_busy", event_emitter);
                                }
                            } else {
                                // TODO: maybe make some travellers willing to trade with
                                // simpler goods like potions
                                agent_data
                                    .chat_npc("npc.speech.villager_decline_trade", event_emitter);
                            }
                        },
                        Subject::Mood => {
                            if let Some(rtsim_entity) = agent_data.rtsim_entity {
                                if !rtsim_entity.brain.remembers_mood() {
                                    // TODO: the following code will need a rework to
                                    // implement more mood contexts
                                    // This require that town NPCs becomes rtsim_entities to
                                    // work fully.
                                    match rand::random::<u32>() % 3 {
                                        0 => agent.rtsim_controller.events.push(
                                            RtSimEvent::SetMood(Memory {
                                                item: MemoryItem::Mood {
                                                    state: MoodState::Good(
                                                        MoodContext::GoodWeather,
                                                    ),
                                                },
                                                time_to_forget: read_data.time.0 + 21200.0,
                                            }),
                                        ),
                                        1 => agent.rtsim_controller.events.push(
                                            RtSimEvent::SetMood(Memory {
                                                item: MemoryItem::Mood {
                                                    state: MoodState::Neutral(
                                                        MoodContext::EverydayLife,
                                                    ),
                                                },
                                                time_to_forget: read_data.time.0 + 21200.0,
                                            }),
                                        ),
                                        2 => agent.rtsim_controller.events.push(
                                            RtSimEvent::SetMood(Memory {
                                                item: MemoryItem::Mood {
                                                    state: MoodState::Bad(MoodContext::GoodWeather),
                                                },
                                                time_to_forget: read_data.time.0 + 86400.0,
                                            }),
                                        ),
                                        _ => {}, // will never happen
                                    }
                                }
                                if let Some(memory) = rtsim_entity.brain.get_mood() {
                                    let msg = match &memory.item {
                                        MemoryItem::Mood { state } => state.describe(),
                                        _ => "".to_string(),
                                    };
                                    agent_data.chat_npc(msg, event_emitter);
                                }
                            }
                        },
                        Subject::Location(location) => {
                            if let Some(tgt_pos) = read_data.positions.get(target) {
                                let raw_dir = location.origin.as_::<f32>() - tgt_pos.0.xy();
                                let dist = Distance::from_dir(raw_dir).name();
                                let dir = Direction::from_dir(raw_dir).name();

                                let msg = format!(
                                    "{} ? I think it's {} {} from here!",
                                    location.name, dist, dir
                                );
                                agent_data.chat_npc(msg, event_emitter);
                            }
                        },
                        Subject::Person(person) => {
                            if let Some(src_pos) = read_data.positions.get(target) {
                                let msg = if let Some(person_pos) = person.origin {
                                    let distance =
                                        Distance::from_dir(person_pos.xy() - src_pos.0.xy());
                                    match distance {
                                        Distance::NextTo | Distance::Near => {
                                            format!(
                                                "{} ? I think he's {} {} from here!",
                                                person.name(),
                                                distance.name(),
                                                Direction::from_dir(
                                                    person_pos.xy() - src_pos.0.xy(),
                                                )
                                                .name()
                                            )
                                        },
                                        _ => {
                                            format!(
                                                "{} ? I think he's gone visiting another town. \
                                                 Come back later!",
                                                person.name()
                                            )
                                        },
                                    }
                                } else {
                                    format!(
                                        "{} ? Sorry, I don't know where you can find him.",
                                        person.name()
                                    )
                                };
                                agent_data.chat_npc(msg, event_emitter);
                            }
                        },
                        Subject::Work => {},
                    }
                }
            }
        }
    }
    true
}

fn handle_inbox_trade_invite(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        event_emitter,
        controller,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::TradeInvite(_))) {
        return false;
    }

    if let Some(AgentEvent::TradeInvite(with)) = agent.inbox.pop_front() {
        if agent.behavior.can_trade() {
            if !agent.behavior.is(BehaviorState::TRADING) {
                // stand still and looking towards the trading player
                controller.push_action(ControlAction::Stand);
                controller.push_action(ControlAction::Talk);
                if let Some(target) = get_entity_by_id(with.id(), read_data) {
                    agent.target = Some(Target::new(target, false, read_data.time.0, false));
                }
                controller.push_invite_response(InviteResponse::Accept);
                agent.behavior.unset(BehaviorState::TRADING_ISSUER);
                agent.behavior.set(BehaviorState::TRADING);
            } else {
                controller.push_invite_response(InviteResponse::Decline);
                agent_data.chat_npc_if_allowed_to_speak(
                    "npc.speech.merchant_busy",
                    agent,
                    event_emitter,
                );
            }
        } else {
            // TODO: Provide a hint where to find the closest merchant?
            controller.push_invite_response(InviteResponse::Decline);
            agent_data.chat_npc_if_allowed_to_speak(
                "npc.speech.villager_decline_trade",
                agent,
                event_emitter,
            );
        }
    }
    true
}

fn handle_inbox_trade_accepted(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent, read_data, ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::TradeAccepted(_))) {
        return false;
    }

    if let Some(AgentEvent::TradeAccepted(with)) = agent.inbox.pop_front() {
        if !agent.behavior.is(BehaviorState::TRADING) {
            if let Some(target) = get_entity_by_id(with.id(), read_data) {
                agent.target = Some(Target::new(target, false, read_data.time.0, false));
            }
            agent.behavior.set(BehaviorState::TRADING);
            agent.behavior.set(BehaviorState::TRADING_ISSUER);
        }
    }
    true
}

fn handle_inbox_finished_trade(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        event_emitter,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::FinishedTrade(_))) {
        return false;
    }

    if let Some(AgentEvent::FinishedTrade(result)) = agent.inbox.pop_front() {
        if agent.behavior.is(BehaviorState::TRADING) {
            match result {
                TradeResult::Completed => {
                    agent_data.chat_npc("npc.speech.merchant_trade_successful", event_emitter);
                },
                _ => {
                    agent_data.chat_npc("npc.speech.merchant_trade_declined", event_emitter);
                },
            }
            agent.behavior.unset(BehaviorState::TRADING);
        }
    }
    true
}

fn handle_inbox_update_pending_trade(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        event_emitter,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::UpdatePendingTrade(_))) {
        return false;
    }

    if let Some(AgentEvent::UpdatePendingTrade(boxval)) = agent.inbox.pop_front() {
        let (tradeid, pending, prices, inventories) = *boxval;
        if agent.behavior.is(BehaviorState::TRADING) {
            let who: usize = if agent.behavior.is(BehaviorState::TRADING_ISSUER) {
                0
            } else {
                1
            };
            let balance0: f32 = prices.balance(&pending.offers, &inventories, 1 - who, true);
            let balance1: f32 = prices.balance(&pending.offers, &inventories, who, false);
            if balance0 >= balance1 {
                // If the trade is favourable to us, only send an accept message if we're
                // not already accepting (since otherwise, spam-clicking the accept button
                // results in lagging and moving to the review phase of an unfavorable trade
                // (although since the phase is included in the message, this shouldn't
                // result in fully accepting an unfavourable trade))
                if !pending.accept_flags[who] && !pending.is_empty_trade() {
                    event_emitter.emit(ServerEvent::ProcessTradeAction(
                        *agent_data.entity,
                        tradeid,
                        TradeAction::Accept(pending.phase),
                    ));
                    tracing::trace!(?tradeid, ?balance0, ?balance1, "Accept Pending Trade");
                }
            } else {
                if balance1 > 0.0 {
                    let msg = format!(
                        "That only covers {:.0}% of my costs!",
                        (balance0 / balance1 * 100.0).floor()
                    );
                    if let Some(tgt_data) = &agent.target {
                        // If talking with someone in particular, "tell" it only to them
                        if let Some(with) = read_data.uids.get(tgt_data.target) {
                            event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc_tell(
                                *agent_data.uid,
                                *with,
                                msg,
                            )));
                        } else {
                            event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc_say(
                                *agent_data.uid,
                                msg,
                            )));
                        }
                    } else {
                        event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc_say(
                            *agent_data.uid,
                            msg,
                        )));
                    }
                }
                if pending.phase != TradePhase::Mutate {
                    // we got into the review phase but without balanced goods, decline
                    agent.behavior.unset(BehaviorState::TRADING);
                    event_emitter.emit(ServerEvent::ProcessTradeAction(
                        *agent_data.entity,
                        tradeid,
                        TradeAction::Decline,
                    ));
                }
            }
        }
    }
    true
}
