use super::arithmetic::{checked_trunc_f64_to_i64, CombatArithmeticError};

const PERCENT_SCALE: f32 = 0.01_f32;
const BASE_CRITICAL_FACTOR: f32 = 1.75_f32;
const GEAR_CRITICAL_CAP: f32 = 1.8_f32;
const RIFT_NPC_SCALE: f32 = 0.0001_f32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalOpaqueJobPercentInputs {
    pub base_percent: f32,
    pub per_value_percent: f32,
    pub decoded_value: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalOutgoingBaseInputs {
    pub calc_damage: i64,
    pub calc_attack_speed: f32,
    pub use_attack_speed_base: bool,
    pub job_trait_5_enabled: bool,
    pub job_trait_5_runtime_gate: f32,
    pub job_trait_5_opaque_base_percent: f32,
    pub job_trait_5_opaque_per_skill_percent: f32,
    pub job_trait_5_skill_value: i32,
    pub outgoing_reduction_rate_0x7a0: f32,
    pub gear_property_need_move_speed_rate: f32,
    pub dragon_protection_gate: bool,
    pub dragon_protection_fairy_attack_rate: f32,
    pub in_meze_state: bool,
    pub riding_pet_gear_property_11_percent: i32,
    pub later_job_percent: Option<OriginalOpaqueJobPercentInputs>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OriginalOutgoingBaseResult {
    pub initial_d8: f64,
    pub job_trait_5_percent: f64,
    pub reduced_d8: f64,
    pub early_percent: f32,
    pub d10_before_later_job_multiplier: f64,
    pub d10: f64,
}

/// Replays the recovered D8/D10 tree at the start of `HunterCtrl.getDamage`.
/// Opaque trait operands remain named by their native role until their content
/// definitions and lookup key are proven.
pub(crate) fn original_outgoing_damage_base(
    input: OriginalOutgoingBaseInputs,
) -> Result<OriginalOutgoingBaseResult, CombatArithmeticError> {
    let initial_d8 = if input.use_attack_speed_base {
        if !input.calc_attack_speed.is_finite() {
            return Err(CombatArithmeticError::NonFinite);
        }
        if input.calc_attack_speed == 0.0 {
            return Err(CombatArithmeticError::UnsupportedDomain);
        }
        (input.calc_damage as f32 / input.calc_attack_speed) as f64
    } else {
        input.calc_damage as f64
    };

    let job_trait_5_percent = if input.use_attack_speed_base
        && input.job_trait_5_runtime_gate > 1.0
        && input.job_trait_5_enabled
    {
        let percent_f32 = input.job_trait_5_opaque_base_percent
            + input.job_trait_5_opaque_per_skill_percent * input.job_trait_5_skill_value as f32;
        percent_f32 as f64 * 0.01_f64
    } else {
        0.0
    };
    let mut d8 = initial_d8 + initial_d8 * job_trait_5_percent;

    if input.outgoing_reduction_rate_0x7a0 != 0.0 {
        d8 *= 1.0_f64 - input.outgoing_reduction_rate_0x7a0 as f64;
    }

    let mut early_percent = input.gear_property_need_move_speed_rate;
    if input.dragon_protection_gate {
        early_percent += input.dragon_protection_fairy_attack_rate;
    }
    if !input.in_meze_state && input.riding_pet_gear_property_11_percent > 0 {
        early_percent += input.riding_pet_gear_property_11_percent as f32 * PERCENT_SCALE;
    }

    let d10_before_later_job_multiplier = d8 * (1.0_f64 + early_percent as f64);
    let d10 = input
        .later_job_percent
        .map(|percent| {
            let expression =
                percent.base_percent + percent.per_value_percent * percent.decoded_value as f32;
            let factor = 1.0_f32 + expression * PERCENT_SCALE;
            d10_before_later_job_multiplier * factor as f64
        })
        .unwrap_or(d10_before_later_job_multiplier);

    if !d10.is_finite() {
        return Err(CombatArithmeticError::NonFinite);
    }

    Ok(OriginalOutgoingBaseResult {
        initial_d8,
        job_trait_5_percent,
        reduced_d8: d8,
        early_percent,
        d10_before_later_job_multiplier,
        d10,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalCriticalDamageInputs {
    pub user_critical_damage_percent: f32,
    pub collection_critical_damage_percent: f32,
    pub relic_collection_critical_damage_percent: f32,
    pub village_pet_critical_damage_rate: f32,
    pub riding_pet_critical_damage_rate: f32,
    pub sylph_bonus_enabled: bool,
    pub sylph_critical_damage_rate: f32,
    pub heroic_job_trait_critical_damage_rate: f32,
    pub opaque_hunter_rate_0x7fc: f32,
    pub opaque_hunter_rate_0x810: f32,
    pub opaque_hunter_rate_0x854: f32,
    pub gear_property_43: Option<[i32; 2]>,
    pub gear_property_59: Option<[i32; 2]>,
    pub target_race_is_one: bool,
    pub gear_property_14_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OriginalCriticalDamageResult {
    pub base_and_direct_adds: f32,
    pub gear_temporary: f32,
    pub factor: f32,
}

fn add_if_positive(accumulator: &mut f32, value: f32) {
    if value > 0.0_f32 {
        *accumulator += value;
    }
}

/// Replays `HunterCtrl.getCriticalDamage` without assigning product names to
/// the three still-obfuscated Hunter fields or GearProperty rows.
pub(crate) fn original_critical_damage_factor(
    input: OriginalCriticalDamageInputs,
) -> OriginalCriticalDamageResult {
    let mut critical = BASE_CRITICAL_FACTOR;
    add_if_positive(
        &mut critical,
        input.user_critical_damage_percent * PERCENT_SCALE,
    );
    add_if_positive(
        &mut critical,
        input.collection_critical_damage_percent * PERCENT_SCALE,
    );
    add_if_positive(
        &mut critical,
        input.relic_collection_critical_damage_percent * PERCENT_SCALE,
    );
    add_if_positive(&mut critical, input.village_pet_critical_damage_rate);
    add_if_positive(&mut critical, input.riding_pet_critical_damage_rate);
    if input.sylph_bonus_enabled {
        add_if_positive(&mut critical, input.sylph_critical_damage_rate);
    }
    add_if_positive(&mut critical, input.heroic_job_trait_critical_damage_rate);
    add_if_positive(&mut critical, input.opaque_hunter_rate_0x7fc);
    add_if_positive(&mut critical, input.opaque_hunter_rate_0x810);
    add_if_positive(&mut critical, input.opaque_hunter_rate_0x854);

    let mut gear_temporary = 0.0_f32;
    if let Some([first, second]) = input.gear_property_43 {
        if first > 0 || second >= 1 {
            gear_temporary += first.wrapping_sub(second) as f32 * PERCENT_SCALE;
        }
    }
    if input.target_race_is_one {
        if let Some([first, second]) = input.gear_property_59 {
            if first >= 1 {
                gear_temporary += first.wrapping_sub(second) as f32 * PERCENT_SCALE;
            }
        }
    }
    if input.gear_property_14_enabled && gear_temporary > GEAR_CRITICAL_CAP {
        gear_temporary = GEAR_CRITICAL_CAP;
    }

    OriginalCriticalDamageResult {
        base_and_direct_adds: critical,
        gear_temporary,
        factor: critical + gear_temporary,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OriginalOutgoingDamageTailInputs {
    pub accumulated_base: f64,
    pub additive_rate_1: f32,
    pub additive_rate_2: f32,
    pub additive_rate_3: f64,
    pub slayer_and_rift_factor: f64,
    pub critical_factor: f32,
    pub stack_factor: f32,
    pub gear_set_factor: f32,
    pub job_target_factor: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OriginalOutgoingDamageTailResult {
    pub after_additive_rate_1: f64,
    pub after_additive_rate_2: f64,
    pub after_additive_rate_3: f64,
    pub before_truncation: f64,
    pub damage: i64,
}

/// Exact final SSA chain from `HunterCtrl.getDamage`. Semantic producer names
/// stay generic until every target, skill and mode caller is proven.
pub(crate) fn original_outgoing_damage_tail(
    input: OriginalOutgoingDamageTailInputs,
) -> Result<OriginalOutgoingDamageTailResult, CombatArithmeticError> {
    let after_additive_rate_1 = input.accumulated_base * (1.0_f64 + input.additive_rate_1 as f64);
    let after_additive_rate_2 = after_additive_rate_1 * (1.0_f64 + input.additive_rate_2 as f64);
    let after_additive_rate_3 = after_additive_rate_2 * (1.0_f64 + input.additive_rate_3);
    let before_truncation = after_additive_rate_3
        * input.slayer_and_rift_factor
        * input.critical_factor as f64
        * input.stack_factor as f64
        * input.gear_set_factor as f64
        * input.job_target_factor as f64;
    let damage = checked_trunc_f64_to_i64(before_truncation)?;

    Ok(OriginalOutgoingDamageTailResult {
        after_additive_rate_1,
        after_additive_rate_2,
        after_additive_rate_3,
        before_truncation,
        damage,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalSlayerNamedInputs {
    pub target_race: i32,
    pub riding_pet_slayer_damage_rate: f32,
    pub gear_property: Option<[i32; 2]>,
    pub collection_primate_damage_rate: f32,
    pub relic_primate_damage_rate: f32,
    pub collection_undead_damage_rate: f32,
    pub relic_undead_damage_rate: f32,
    pub collection_evil_damage_rate: f32,
    pub relic_evil_damage_rate: f32,
    pub collection_animal_damage_rate: f32,
    pub relic_animal_damage_rate: f32,
    pub collection_boss_damage_rate: f32,
    pub relic_boss_damage_rate: f32,
}

/// Replays the named common/race segment of `getSlayerDamage`. The original
/// helper has additional opaque job-trait/UserData terms kept outside this
/// function until their writers and ordering semantics are resolved.
pub(crate) fn original_slayer_named_segment(input: OriginalSlayerNamedInputs) -> f32 {
    let mut result = 0.0_f32;
    add_if_positive(&mut result, input.riding_pet_slayer_damage_rate);

    let (collection, relic, supported_race) = match input.target_race {
        1 => (
            input.collection_primate_damage_rate,
            input.relic_primate_damage_rate,
            true,
        ),
        2 => (
            input.collection_undead_damage_rate,
            input.relic_undead_damage_rate,
            true,
        ),
        3 => (
            input.collection_evil_damage_rate,
            input.relic_evil_damage_rate,
            true,
        ),
        4 => (
            input.collection_animal_damage_rate,
            input.relic_animal_damage_rate,
            true,
        ),
        5 => (
            input.collection_boss_damage_rate,
            input.relic_boss_damage_rate,
            true,
        ),
        _ => (0.0, 0.0, false),
    };
    if supported_race {
        if let Some([first, second]) = input.gear_property {
            result += first.wrapping_sub(second) as f32 * PERCENT_SCALE;
        }
    }
    add_if_positive(&mut result, collection);
    add_if_positive(&mut result, relic);
    result
}

/// Exact arithmetic/gates of `getRiftNpcBuffDamage` once the unresolved
/// dictionary lookup has produced its nested integer.
pub(crate) fn original_rift_npc_buff_damage(
    input_matches_supported_static_id: bool,
    nested_value: Option<i32>,
) -> f32 {
    if !input_matches_supported_static_id {
        return 0.0;
    }
    nested_value
        .map(|value| value as f32 * RIFT_NPC_SCALE)
        .unwrap_or(0.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalOutgoingAdditiveRates {
    pub s12_initial_percent_rate: f32,
    pub punishment_token_gate: bool,
    pub punishment_token_rate: f32,
    pub s12_positive_rate_0x8c4: f32,
    pub job_trait_67_gate: bool,
    pub s12_positive_rate_0x9dc: f32,
    pub s13_static_gate: bool,
    pub s13_static_percent: i32,
    pub s13_positive_rate_0x560: f32,
    pub s13_positive_rate_0x580: f32,
    pub s13_positive_rate_0x520: f32,
    pub d14_positive_percent_0x7c0: f32,
    pub d14_positive_percent_0x824: f32,
    pub d14_positive_rate_0x65c: f32,
}

/// Exact arithmetic for the recovered S12/S13/D14 producers. Inputs retain
/// native offsets where their gameplay-facing writers are still unresolved.
pub(crate) fn original_outgoing_additive_rates(
    input: OriginalOutgoingAdditiveRates,
) -> (f32, f32, f64) {
    let mut s12 = input.s12_initial_percent_rate;
    if input.punishment_token_gate {
        s12 += input.punishment_token_rate;
    }
    add_if_positive(&mut s12, input.s12_positive_rate_0x8c4);
    if input.job_trait_67_gate {
        add_if_positive(&mut s12, input.s12_positive_rate_0x9dc);
    }

    let mut s13 = 0.0_f32;
    if input.s13_static_gate {
        s13 += input.s13_static_percent as f32 * PERCENT_SCALE;
    }
    add_if_positive(&mut s13, input.s13_positive_rate_0x560);
    add_if_positive(&mut s13, input.s13_positive_rate_0x580);
    add_if_positive(&mut s13, input.s13_positive_rate_0x520);

    let mut d14 = 0.0_f64;
    if input.d14_positive_percent_0x7c0 > 0.0 {
        d14 += (input.d14_positive_percent_0x7c0 * PERCENT_SCALE) as f64;
    }
    if input.d14_positive_percent_0x824 > 0.0 {
        d14 += (input.d14_positive_percent_0x824 * PERCENT_SCALE) as f64;
    }
    if input.d14_positive_rate_0x65c > 0.0 {
        d14 += input.d14_positive_rate_0x65c as f64;
    }
    (s12, s13, d14)
}

pub(crate) fn original_outgoing_stack_factor(
    gear_set_count: i32,
    current_value: i64,
    maximum_value: i64,
    threshold_percent: i32,
    bonus_percent: i32,
) -> f32 {
    if gear_set_count < 2 || current_value < 1 || maximum_value < 1 {
        return 1.0;
    }
    let ratio = current_value as f64 / maximum_value as f64;
    let threshold = (threshold_percent as f32 * PERCENT_SCALE) as f64;
    if ratio < threshold {
        1.0_f32 + bonus_percent as f32 * PERCENT_SCALE
    } else {
        1.0
    }
}

pub(crate) fn original_outgoing_s8_factor(
    gear_set_2_enabled: bool,
    gear_set_2_percent: i32,
    gear_property_67_enabled: bool,
    gear_property_67_percent: i32,
    job_trait_21_percent_expression: Option<f32>,
) -> f32 {
    let mut factor = 1.0_f32;
    if gear_set_2_enabled {
        factor += gear_set_2_percent as f32 * PERCENT_SCALE;
    }
    if gear_property_67_enabled {
        factor += gear_property_67_percent as f32 * PERCENT_SCALE;
    }
    if let Some(percent_expression) = job_trait_21_percent_expression {
        factor += percent_expression * PERCENT_SCALE;
    }
    factor
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalJobTargetDamageRates {
    pub collection_berserker: f32,
    pub relic_berserker: f32,
    pub collection_paladin: f32,
    pub relic_paladin: f32,
    pub collection_ranger: f32,
    pub relic_ranger: f32,
    pub collection_sorcerer: f32,
    pub relic_sorcerer: f32,
    pub collection_dark_knight: f32,
    pub relic_dark_knight: f32,
}

pub(crate) fn original_job_target_damage_factor(
    hunter_job: i32,
    input: OriginalJobTargetDamageRates,
) -> f32 {
    let (collection, relic) = match hunter_job {
        0 => (input.collection_berserker, input.relic_berserker),
        1 => (input.collection_paladin, input.relic_paladin),
        2 => (input.collection_ranger, input.relic_ranger),
        3 => (input.collection_sorcerer, input.relic_sorcerer),
        4 => (input.collection_dark_knight, input.relic_dark_knight),
        _ => return 1.0,
    };
    let mut factor = 1.0_f32;
    add_if_positive(&mut factor, collection);
    add_if_positive(&mut factor, relic);
    factor
}

#[cfg(test)]
mod tests {
    use super::{
        original_critical_damage_factor, original_outgoing_damage_base,
        original_outgoing_damage_tail, OriginalCriticalDamageInputs,
        OriginalOpaqueJobPercentInputs, OriginalOutgoingBaseInputs,
        OriginalOutgoingDamageTailInputs,
    };
    use crate::simulation::combat_core::arithmetic::CombatArithmeticError;

    #[test]
    fn outgoing_base_preserves_float32_trait_and_early_percent_stages() {
        let result = original_outgoing_damage_base(OriginalOutgoingBaseInputs {
            calc_damage: 1_001,
            calc_attack_speed: 2.0,
            use_attack_speed_base: true,
            job_trait_5_enabled: true,
            job_trait_5_runtime_gate: 1.5,
            job_trait_5_opaque_base_percent: 10.0,
            job_trait_5_opaque_per_skill_percent: 2.5,
            job_trait_5_skill_value: 4,
            outgoing_reduction_rate_0x7a0: 0.1,
            gear_property_need_move_speed_rate: 0.05,
            dragon_protection_gate: true,
            dragon_protection_fairy_attack_rate: 0.02,
            riding_pet_gear_property_11_percent: 3,
            later_job_percent: Some(OriginalOpaqueJobPercentInputs {
                base_percent: 5.0,
                per_value_percent: 10.0,
                decoded_value: 2,
            }),
            ..OriginalOutgoingBaseInputs::default()
        })
        .unwrap();

        assert_eq!(result.initial_d8, 500.5_f64);
        assert_eq!(result.job_trait_5_percent, 0.2_f64);
        assert_eq!(result.early_percent.to_bits(), 0.1_f32.to_bits());
        assert!(result.d10 > result.d10_before_later_job_multiplier);
    }

    #[test]
    fn outgoing_base_rejects_an_unproven_zero_attack_speed_domain() {
        let result = original_outgoing_damage_base(OriginalOutgoingBaseInputs {
            calc_damage: 100,
            use_attack_speed_base: true,
            calc_attack_speed: 0.0,
            ..OriginalOutgoingBaseInputs::default()
        });
        assert_eq!(result, Err(CombatArithmeticError::UnsupportedDomain));
    }

    #[test]
    fn critical_damage_preserves_positive_gates_scaling_and_sylph_gate() {
        let result = original_critical_damage_factor(OriginalCriticalDamageInputs {
            user_critical_damage_percent: 10.0,
            collection_critical_damage_percent: 20.0,
            relic_collection_critical_damage_percent: -50.0,
            village_pet_critical_damage_rate: 0.1,
            riding_pet_critical_damage_rate: 0.2,
            sylph_bonus_enabled: false,
            sylph_critical_damage_rate: 0.4,
            heroic_job_trait_critical_damage_rate: 0.3,
            ..OriginalCriticalDamageInputs::default()
        });

        assert_eq!(result.base_and_direct_adds.to_bits(), 0x4029_9999);
        assert_eq!(result.gear_temporary, 0.0);
        assert_eq!(result.factor.to_bits(), 0x4029_9999);
    }

    #[test]
    fn gear_temporary_uses_ordered_rows_target_gate_and_cap() {
        let uncapped = original_critical_damage_factor(OriginalCriticalDamageInputs {
            gear_property_43: Some([50, 10]),
            gear_property_59: Some([100, 20]),
            target_race_is_one: true,
            ..OriginalCriticalDamageInputs::default()
        });
        assert_eq!(uncapped.gear_temporary.to_bits(), 0x3f99_9999);

        let capped = original_critical_damage_factor(OriginalCriticalDamageInputs {
            gear_property_43: Some([200, 0]),
            gear_property_59: Some([100, 0]),
            target_race_is_one: true,
            gear_property_14_enabled: true,
            ..OriginalCriticalDamageInputs::default()
        });
        assert_eq!(capped.gear_temporary.to_bits(), 1.8_f32.to_bits());

        let target_gated = original_critical_damage_factor(OriginalCriticalDamageInputs {
            gear_property_59: Some([100, 0]),
            target_race_is_one: false,
            ..OriginalCriticalDamageInputs::default()
        });
        assert_eq!(target_gated.gear_temporary, 0.0);
    }

    #[test]
    fn outgoing_tail_preserves_native_widening_order_and_truncation() {
        let result = original_outgoing_damage_tail(OriginalOutgoingDamageTailInputs {
            accumulated_base: 100.0,
            additive_rate_1: 0.1,
            additive_rate_2: 0.2,
            additive_rate_3: 0.3,
            slayer_and_rift_factor: 1.1,
            critical_factor: 1.75,
            stack_factor: 1.2,
            gear_set_factor: 0.9,
            job_target_factor: 1.05,
        })
        .unwrap();

        assert_eq!(result.damage, 374);
        assert!(result.before_truncation > 374.0);
        assert!(result.before_truncation < 375.0);
    }

    #[test]
    fn outgoing_tail_fails_closed_outside_the_proven_conversion_domain() {
        let result = original_outgoing_damage_tail(OriginalOutgoingDamageTailInputs {
            accumulated_base: f64::INFINITY,
            additive_rate_1: 0.0,
            additive_rate_2: 0.0,
            additive_rate_3: 0.0,
            slayer_and_rift_factor: 1.0,
            critical_factor: 1.0,
            stack_factor: 1.0,
            gear_set_factor: 1.0,
            job_target_factor: 1.0,
        });
        assert_eq!(result, Err(CombatArithmeticError::NonFinite));
    }

    #[test]
    fn slayer_named_segment_selects_only_the_target_race_columns() {
        let result = super::original_slayer_named_segment(super::OriginalSlayerNamedInputs {
            target_race: 3,
            riding_pet_slayer_damage_rate: 0.1,
            gear_property: Some([40, 10]),
            collection_evil_damage_rate: 0.2,
            relic_evil_damage_rate: 0.3,
            collection_boss_damage_rate: 9.0,
            relic_boss_damage_rate: 9.0,
            ..super::OriginalSlayerNamedInputs::default()
        });
        assert_eq!(result.to_bits(), 0x3f66_6666);

        let unsupported = super::original_slayer_named_segment(super::OriginalSlayerNamedInputs {
            target_race: 99,
            gear_property: Some([100, 0]),
            ..super::OriginalSlayerNamedInputs::default()
        });
        assert_eq!(unsupported, 0.0);
    }

    #[test]
    fn rift_npc_helper_requires_the_static_id_and_nested_value() {
        assert_eq!(super::original_rift_npc_buff_damage(false, Some(500)), 0.0);
        assert_eq!(super::original_rift_npc_buff_damage(true, None), 0.0);
        assert_eq!(
            super::original_rift_npc_buff_damage(true, Some(500)).to_bits(),
            0x3d4c_cccc
        );
    }

    #[test]
    fn additive_rate_producers_keep_float32_and_float64_boundaries() {
        let (s12, s13, d14) =
            super::original_outgoing_additive_rates(super::OriginalOutgoingAdditiveRates {
                s12_initial_percent_rate: 0.1,
                punishment_token_gate: true,
                punishment_token_rate: -0.05,
                s12_positive_rate_0x8c4: 0.2,
                job_trait_67_gate: true,
                s12_positive_rate_0x9dc: 0.3,
                s13_static_gate: true,
                s13_static_percent: 10,
                s13_positive_rate_0x560: 0.2,
                s13_positive_rate_0x580: -1.0,
                s13_positive_rate_0x520: 0.3,
                d14_positive_percent_0x7c0: 10.0,
                d14_positive_percent_0x824: 20.0,
                d14_positive_rate_0x65c: 0.4,
            });
        assert_eq!(s12.to_bits(), 0x3f0c_cccd);
        assert_eq!(s13.to_bits(), 0x3f19_999a);
        assert!(d14 > 0.699_999_9 && d14 < 0.700_000_1);
    }

    #[test]
    fn stack_s8_and_job_factors_replay_the_recovered_gates() {
        assert_eq!(
            super::original_outgoing_stack_factor(2, 20, 100, 30, 50),
            1.5
        );
        assert_eq!(
            super::original_outgoing_stack_factor(2, 30, 100, 30, 50),
            1.0
        );
        assert_eq!(
            super::original_outgoing_s8_factor(true, 10, true, 20, Some(30.0)).to_bits(),
            0x3fcc_cccc
        );
        let job = super::original_job_target_damage_factor(
            2,
            super::OriginalJobTargetDamageRates {
                collection_ranger: 0.2,
                relic_ranger: 0.3,
                collection_paladin: 9.0,
                ..super::OriginalJobTargetDamageRates::default()
            },
        );
        assert_eq!(job, 1.5);
    }
}
