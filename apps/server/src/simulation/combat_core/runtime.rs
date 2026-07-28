use super::{
    arithmetic::CombatArithmeticError,
    critical::{original_critical_roll_succeeds, original_critical_threshold},
    hit_resolution::resolve_original_hunter_dodge,
    hunter_incoming::{
        original_hunter_apply_first_shield_then_hp, original_hunter_armor_scratch,
        original_hunter_forwarded_damage,
    },
    monster_incoming::{original_monster_damage, OriginalMonsterDamageInputs},
    outgoing::{
        original_critical_damage_factor, original_outgoing_damage_base,
        original_outgoing_damage_tail, OriginalCriticalDamageInputs, OriginalOutgoingBaseInputs,
        OriginalOutgoingDamageTailInputs,
    },
};
use crate::simulation::original_combat::ORIGINAL_DEFAULT_DAMAGE_DECREASE_FACTOR;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OriginalHitPresentation {
    Normal,
    Critical,
    Miss,
    Evade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OriginalHunterAttackResult {
    pub presentation: OriginalHitPresentation,
    pub critical_threshold: i32,
    pub outgoing_damage: i64,
    pub final_damage: i64,
    pub target_hp: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalHunterAttackInputs {
    /// Operational projection of the already-aggregated `StatusData.CalcDamage`.
    pub calculated_damage: i64,
    pub calculated_critical_percent: i32,
    pub critical_roll_zero_to_ninety_nine: i32,
    pub conditional_critical_bonus_enabled: bool,
    pub conditional_critical_bonus_percent: i32,
    pub target_armor: i64,
    pub target_hp: i64,
    pub hunter_feel: f32,
    pub hunter_now_feel: f32,
    pub rand_damage_multiplier: f32,
}

/// Connects the fully recovered neutral ordinary-attack spine. Optional
/// trait/gear/skill contributors stay at their native neutral values until
/// their writers and caller bindings are proven.
pub(crate) fn resolve_original_neutral_hunter_attack(
    input: OriginalHunterAttackInputs,
) -> Result<OriginalHunterAttackResult, CombatArithmeticError> {
    let critical_threshold = original_critical_threshold(
        input.calculated_critical_percent,
        input.conditional_critical_bonus_enabled,
        input.conditional_critical_bonus_percent,
    );
    let critical = original_critical_roll_succeeds(
        critical_threshold,
        input.critical_roll_zero_to_ninety_nine,
    );
    let critical_factor = if critical {
        original_critical_damage_factor(OriginalCriticalDamageInputs::default()).factor
    } else {
        1.0
    };
    let base = original_outgoing_damage_base(OriginalOutgoingBaseInputs {
        calc_damage: input.calculated_damage,
        ..OriginalOutgoingBaseInputs::default()
    })?;
    let outgoing = original_outgoing_damage_tail(OriginalOutgoingDamageTailInputs {
        accumulated_base: base.d10,
        slayer_and_rift_factor: 1.0,
        critical_factor,
        stack_factor: 1.0,
        gear_set_factor: 1.0,
        job_target_factor: 1.0,
        additive_rate_1: 0.0,
        additive_rate_2: 0.0,
        additive_rate_3: 0.0,
    })?;
    let target = original_monster_damage(OriginalMonsterDamageInputs {
        incoming_damage: outgoing.damage,
        hunter_feel: input.hunter_feel,
        hunter_now_feel: input.hunter_now_feel,
        rand_damage_multiplier: input.rand_damage_multiplier,
        monster_armor: input.target_armor,
        monster_now_hp: input.target_hp,
        ..OriginalMonsterDamageInputs::default()
    })?;

    Ok(OriginalHunterAttackResult {
        presentation: if critical {
            OriginalHitPresentation::Critical
        } else {
            OriginalHitPresentation::Normal
        },
        critical_threshold,
        outgoing_damage: outgoing.damage,
        final_damage: target.final_damage,
        target_hp: target.monster_now_hp.max(0),
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalMonsterAttackInputs {
    pub incoming_damage: i64,
    pub rand_damage_multiplier: f32,
    pub attacker_effect_54_value: i32,
    pub effect_54_roll_zero_to_ninety_nine: i32,
    pub hunter_armor: i64,
    pub hunter_feel: f32,
    pub hunter_now_feel: f32,
    pub hunter_shield: i64,
    pub hunter_hp: i64,
    pub hunter_calc_dodge: i32,
    pub hunter_dodge_primary_roll_zero_to_ninety_nine: i32,
    pub hunter_riding_pet_dodge: i32,
    pub hunter_riding_pet_roll_zero_to_nine_ninety_nine: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OriginalMonsterAttackResult {
    pub presentation: OriginalHitPresentation,
    pub final_damage: i64,
    pub hunter_shield: i64,
    pub hunter_hp: i64,
}

pub(crate) fn resolve_original_neutral_monster_attack(
    input: OriginalMonsterAttackInputs,
) -> Result<OriginalMonsterAttackResult, CombatArithmeticError> {
    if super::hit_resolution::original_effect_54_aborts_damage(
        input.attacker_effect_54_value,
        input.effect_54_roll_zero_to_ninety_nine,
    ) {
        return Ok(OriginalMonsterAttackResult {
            presentation: OriginalHitPresentation::Miss,
            final_damage: 0,
            hunter_shield: input.hunter_shield,
            hunter_hp: input.hunter_hp,
        });
    }

    let dodge = resolve_original_hunter_dodge(super::hit_resolution::OriginalDodgeInputs {
        is_meze_state: false,
        calc_dodge: input.hunter_calc_dodge,
        effect_type_5_bonus: 0,
        riding_pet_dodge: input.hunter_riding_pet_dodge,
        primary_roll_zero_to_ninety_nine: input.hunter_dodge_primary_roll_zero_to_ninety_nine,
        riding_pet_roll_zero_to_nine_ninety_nine: Some(
            input.hunter_riding_pet_roll_zero_to_nine_ninety_nine,
        ),
    })
    .map_err(|_| CombatArithmeticError::UnsupportedDomain)?;
    if matches!(
        dodge,
        super::hit_resolution::OriginalDodgeResolution::PrimaryEvade
            | super::hit_resolution::OriginalDodgeResolution::RidingPetEvade
    ) {
        return Ok(OriginalMonsterAttackResult {
            presentation: OriginalHitPresentation::Evade,
            final_damage: 0,
            hunter_shield: input.hunter_shield,
            hunter_hp: input.hunter_hp,
        });
    }

    let randomized = (input.incoming_damage as f32 * input.rand_damage_multiplier) as i64;
    let armor = original_hunter_armor_scratch(
        input.hunter_armor,
        input.hunter_feel,
        input.hunter_now_feel,
    )?;
    let forwarded = original_hunter_forwarded_damage(
        randomized,
        armor,
        ORIGINAL_DEFAULT_DAMAGE_DECREASE_FACTOR,
    )?;
    let applied =
        original_hunter_apply_first_shield_then_hp(input.hunter_shield, input.hunter_hp, forwarded);
    Ok(OriginalMonsterAttackResult {
        presentation: OriginalHitPresentation::Normal,
        final_damage: applied.hp_damage,
        hunter_shield: applied.current_shield,
        hunter_hp: applied.now_hp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_hunter_attack_replays_critical_variance_feel_armor_and_hp() {
        let result = resolve_original_neutral_hunter_attack(OriginalHunterAttackInputs {
            calculated_damage: 100,
            calculated_critical_percent: 20,
            critical_roll_zero_to_ninety_nine: 19,
            target_armor: 50,
            target_hp: 500,
            hunter_feel: 100.0,
            hunter_now_feel: 80.0,
            rand_damage_multiplier: 0.91,
            ..OriginalHunterAttackInputs::default()
        })
        .unwrap();

        assert_eq!(result.presentation, OriginalHitPresentation::Critical);
        assert_eq!(result.critical_threshold, 20);
        assert_eq!(result.outgoing_damage, 175);
        assert_eq!(result.final_damage, 141);
        assert_eq!(result.target_hp, 359);
    }

    #[test]
    fn ordinary_monster_attack_replays_random_armor_default_factor_and_hp() {
        let result = resolve_original_neutral_monster_attack(OriginalMonsterAttackInputs {
            incoming_damage: 100,
            rand_damage_multiplier: 0.91,
            hunter_armor: 30,
            hunter_feel: 100.0,
            hunter_now_feel: 40.0,
            hunter_hp: 100,
            ..OriginalMonsterAttackInputs::default()
        })
        .unwrap();

        assert_eq!(result.presentation, OriginalHitPresentation::Normal);
        assert_eq!(result.final_damage, 45);
        assert_eq!(result.hunter_hp, 55);
    }

    #[test]
    fn effect_54_miss_aborts_before_armor_and_hp() {
        let result = resolve_original_neutral_monster_attack(OriginalMonsterAttackInputs {
            incoming_damage: 100,
            rand_damage_multiplier: 1.0,
            attacker_effect_54_value: 25,
            effect_54_roll_zero_to_ninety_nine: 24,
            hunter_armor: 30,
            hunter_feel: 100.0,
            hunter_now_feel: 100.0,
            hunter_hp: 100,
            ..OriginalMonsterAttackInputs::default()
        })
        .unwrap();

        assert_eq!(result.presentation, OriginalHitPresentation::Miss);
        assert_eq!(result.final_damage, 0);
        assert_eq!(result.hunter_hp, 100);
    }

    #[test]
    fn total_calc_dodge_evades_before_armor_shield_and_hp() {
        let result = resolve_original_neutral_monster_attack(OriginalMonsterAttackInputs {
            incoming_damage: 100,
            rand_damage_multiplier: 1.0,
            hunter_calc_dodge: 25,
            hunter_dodge_primary_roll_zero_to_ninety_nine: 24,
            hunter_armor: 30,
            hunter_shield: 20,
            hunter_hp: 100,
            ..OriginalMonsterAttackInputs::default()
        })
        .unwrap();

        assert_eq!(result.presentation, OriginalHitPresentation::Evade);
        assert_eq!(result.final_damage, 0);
        assert_eq!(result.hunter_shield, 20);
        assert_eq!(result.hunter_hp, 100);
    }
}
