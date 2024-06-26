use super::{
    super::{vek::*, Animation},
    hammer_start, CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::{AbilityInfo, StageSection};
use core::f32::consts::{PI, TAU};

pub struct SelfBuffAnimation;
impl Animation for SelfBuffAnimation {
    type Dependency<'a> = (Option<&'a str>, Option<StageSection>, Option<AbilityInfo>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_self_buff\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_self_buff")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, _ability_info): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);

        match ability_id {
            Some("common.abilities.sword.heavy_fortitude") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.foot_l.position += Vec3::new(move1 * 1.0, move1 * 2.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.3);
                next.belt.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_x(move1 * 0.4 + move2 * 0.6);
                next.control.orientation.rotate_z(move1 * 0.4);

                next.foot_r.position += Vec3::new(move2 * -1.0, move2 * -2.0, 0.0);
                next.control.position += Vec3::new(move2 * 5.0, move2 * 7.0, move2 * 5.0);
                next.chest.position += Vec3::new(0.0, 0.0, move2 * -1.0);
                next.shorts.orientation.rotate_x(move2 * 0.2);
                next.shorts.position += Vec3::new(0.0, move2 * 1.0, 0.0);
            },
            Some("common.abilities.sword.defensive_stalwart_sword") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move2 * -0.5);

                next.foot_r.position += Vec3::new(move1 * 1.0, move1 * -2.0, 0.0);
                next.foot_r.orientation.rotate_z(move1 * -0.9);
                next.chest.orientation = Quaternion::rotation_z(move1 * -0.5);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.3);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_x(move1 * 0.4);
                next.control.orientation.rotate_z(move1 * 0.5);
                next.control.position += Vec3::new(0.0, 0.0, move1 * 4.0);

                next.control.position += Vec3::new(move2 * 8.0, 0.0, move2 * -1.0);
                next.control.orientation.rotate_x(move2 * -0.6);
                next.chest.position += Vec3::new(0.0, 0.0, move2 * -2.0);
                next.belt.position += Vec3::new(0.0, 0.0, move2 * 1.0);
                next.shorts.position += Vec3::new(0.0, 0.0, move2 * 1.0);
                next.shorts.orientation.rotate_x(move2 * 0.2);
                next.control.orientation.rotate_z(move2 * 0.4);
            },
            Some("common.abilities.sword.agile_dancing_edge") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.head.orientation = Quaternion::rotation_x(move1 * 0.3);
                next.head.position += Vec3::new(0.0, 0.0, move1 * -1.0);
                next.control.position += Vec3::new(move1 * 8.0, move1 * 5.0, 0.0);

                next.head.orientation.rotate_x(move2 * 0.2);
                next.head.position += Vec3::new(0.0, 0.0, move2 * -1.0);
                next.control.position += Vec3::new(0.0, move2 * -2.0, move2 * 12.0);
                next.control.orientation.rotate_x(move2 * 1.1);
            },
            Some("common.abilities.sword.cleaving_blade_fever") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.foot_l.position += Vec3::new(move1 * 1.0, move1 * 2.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.3);
                next.belt.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_x(move1 * 0.4 + move2 * 0.6);
                next.control.orientation.rotate_z(move1 * 0.4);

                next.foot_r.position += Vec3::new(move2 * -1.0, move2 * -2.0, 0.0);
                next.control.position += Vec3::new(move2 * 5.0, move2 * 7.0, move2 * 5.0);
                next.chest.position += Vec3::new(0.0, 0.0, move2 * -1.0);
                next.shorts.orientation.rotate_x(move2 * 0.2);
                next.shorts.position += Vec3::new(0.0, move2 * 1.0, 0.0);
            },
            Some("common.abilities.axe.berserk") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                    * Quaternion::rotation_y(s_a.ac.4)
                    * Quaternion::rotation_z(s_a.ac.5);

                next.control.orientation.rotate_z(move1 * -2.0);
                next.control.orientation.rotate_x(move1 * 3.5);
                next.control.position += Vec3::new(move1 * 14.0, move1 * -6.0, move1 * 15.0);

                next.head.orientation.rotate_x(move2 * 0.6);
                next.chest.orientation.rotate_x(move2 * 0.4);
            },
            Some("common.abilities.axe.savage_sense") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                    * Quaternion::rotation_y(s_a.ac.4)
                    * Quaternion::rotation_z(s_a.ac.5);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.6);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.3);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.foot_r.position += Vec3::new(0.0, move1 * 4.0, move1 * 4.0);
                next.foot_r.orientation.rotate_x(move1 * 1.2);

                next.foot_r.position += Vec3::new(0.0, move2 * 4.0, move2 * -4.0);
                next.foot_r.orientation.rotate_x(move2 * -1.2);
            },
            Some("common.abilities.axe.adrenaline_rush") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                    * Quaternion::rotation_y(s_a.ac.4)
                    * Quaternion::rotation_z(s_a.ac.5 - move1 * PI);

                next.control.orientation.rotate_z(move1 * -1.8);
                next.control.orientation.rotate_y(move1 * 1.5);
                next.control.position += Vec3::new(move1 * 11.0, 0.0, 0.0);

                next.control.orientation.rotate_y(move2 * 0.7);
                next.control.orientation.rotate_z(move2 * 1.6);
                next.control.position += Vec3::new(move2 * -8.0, 0.0, move2 * -3.0);
            },
            Some("common.abilities.axe.bloodfeast") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                    * Quaternion::rotation_y(s_a.ac.4)
                    * Quaternion::rotation_z(s_a.ac.5);

                next.control.orientation.rotate_z(move1 * -3.4);
                next.control.orientation.rotate_x(move1 * 1.1);
                next.control.position += Vec3::new(move1 * 14.0, move1 * -3.0, 0.0);

                next.control.orientation.rotate_x(move2 * 1.7);
                next.control.orientation.rotate_z(move2 * -1.3);
                next.control.orientation.rotate_y(move2 * 0.8);
            },
            Some("common.abilities.axe.furor") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                    * Quaternion::rotation_y(s_a.ac.4)
                    * Quaternion::rotation_z(s_a.ac.5 - move1 * PI);

                next.control.orientation.rotate_x(move1 * -1.0);
                next.control.position += Vec3::new(move1 * 3.0, move1 * -2.0, move1 * 14.0);
                next.control.orientation.rotate_z(move1 * 1.5);

                next.control.orientation.rotate_y(move2 * -1.0);
                next.control.orientation.rotate_z(move2 * -1.6);
                next.control.orientation.rotate_y(move2 * 0.7);
                next.control.orientation.rotate_x(move2 * -0.5);
                next.control.position += Vec3::new(move2 * 9.0, move2 * -3.0, move2 * -14.0);
            },
            Some("common.abilities.axe.sunder") => {
                let (move1_raw, move2_raw, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1_raw * pullback;
                let move2 = move2_raw * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                    * Quaternion::rotation_y(s_a.ac.4)
                    * Quaternion::rotation_z(s_a.ac.5);

                next.control.orientation.rotate_z(move1 * -1.5);
                next.control.position += Vec3::new(move1 * 12.0, 0.0, move1 * 5.0);
                next.control.orientation.rotate_y(move1 * 0.5);
                next.main.position += Vec3::new(0.0, move1 * 10.0, 0.0);
                next.main.orientation.rotate_z(move1_raw * TAU);
                next.second.position += Vec3::new(0.0, move1 * 10.0, 0.0);
                next.second.orientation.rotate_z(move1_raw * -TAU);

                next.main.orientation.rotate_z(move2_raw * TAU);
                next.main.position += Vec3::new(0.0, move2 * -10.0, 0.0);
                next.second.orientation.rotate_z(move2_raw * -TAU);
                next.second.position += Vec3::new(0.0, move2 * -10.0, 0.0);
                next.control.position += Vec3::new(0.0, 0.0, move2 * -5.0);
            },
            Some("common.abilities.axe.defiance") => {
                let (move1, tension, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, (anim_time * 20.0).sin(), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                    * Quaternion::rotation_y(s_a.ac.4)
                    * Quaternion::rotation_z(s_a.ac.5);

                next.control.orientation.rotate_z(move1 * -1.6);
                next.control.orientation.rotate_x(move1 * 1.7);
                next.control.position += Vec3::new(move1 * 12.0, move1 * -10.0, move1 * 18.0);
                next.head.orientation.rotate_x(move1 * 0.6);
                next.head.position += Vec3::new(0.0, 0.0, move1 * -3.0);
                next.control.orientation.rotate_z(move1 * 0.4);

                next.head.orientation.rotate_x(tension * 0.3);
                next.control.position += Vec3::new(0.0, 0.0, tension * 2.0);
            },
            Some("common.abilities.hammer.tenacity") => {
                hammer_start(&mut next, s_a);
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.control.orientation.rotate_x(move1 * 0.6);
                next.control.orientation.rotate_y(move1 * 0.9);
                next.control.orientation.rotate_x(move1 * -0.6);
                next.chest.orientation.rotate_x(move1 * 0.4);
                next.control.position += Vec3::new(0.0, 4.0, 3.0) * move1;

                next.control.position += Vec3::new(
                    (move2 * 50.0).sin(),
                    (move2 * 67.0).sin(),
                    (move2 * 83.0).sin(),
                );
            },
            _ => {},
        }

        next
    }
}
