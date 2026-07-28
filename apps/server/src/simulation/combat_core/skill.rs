use super::arithmetic::{checked_trunc_f32_to_i64, CombatArithmeticError};

const PERCENT_SCALE: f32 = 0.01_f32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OriginalBlizzardDamageInputs {
    pub base_damage: i64,
    pub coefficient_percent: f32,
    pub modifier_aggregate: f32,
}

/// Exact coefficient segment recovered from `HunterCtrl.GDBMICDJBOK`. This is
/// specific to the Blizzard builder and is not generalized to other skills.
pub(crate) fn original_blizzard_damage(
    input: OriginalBlizzardDamageInputs,
) -> Result<i64, CombatArithmeticError> {
    let value = input.base_damage as f32
        * input.coefficient_percent
        * (1.0_f32 + input.modifier_aggregate)
        * PERCENT_SCALE;
    checked_trunc_f32_to_i64(value)
}

/// Shared arithmetic proven for four plain-Single and two decoded
/// ObscuredFloat caller bodies. Routing/action semantics remain outside core.
pub(crate) fn original_plain_percent_skill_damage(
    base_damage: i64,
    coefficient_percent: f32,
) -> Result<i64, CombatArithmeticError> {
    checked_trunc_f32_to_i64(base_damage as f32 * coefficient_percent * PERCENT_SCALE)
}

/// Exact family used by callers that decode an internal `ObscuredInt`, scale
/// the percentage first in float32, round-trip it through `ObscuredFloat`, and
/// only then multiply by the float32 base damage.
pub(crate) fn original_internal_percent_skill_damage(
    base_damage: i64,
    decoded_coefficient_percent: i32,
) -> Result<i64, CombatArithmeticError> {
    let percent = decoded_coefficient_percent as f32 * PERCENT_SCALE;
    checked_trunc_f32_to_i64(base_damage as f32 * percent)
}

/// Exact affine coefficient family used by two captured caller bodies.
pub(crate) fn original_affine_percent_skill_damage(
    base_damage: i64,
    base_percent: f32,
    coefficient_percent: f32,
    internal_multiplier: f32,
) -> Result<i64, CombatArithmeticError> {
    let combined_percent = base_percent + coefficient_percent * internal_multiplier;
    checked_trunc_f32_to_i64(base_damage as f32 * combined_percent * PERCENT_SCALE)
}

pub(crate) fn original_poison_aura_damage(
    base_damage: i64,
    power_percent: i32,
    modifier_a: f32,
    modifier_b: f32,
    target_modifier: Option<f32>,
) -> Result<i64, CombatArithmeticError> {
    let modifier_sum = modifier_a + modifier_b;
    let percent = power_percent as f32 * PERCENT_SCALE;
    let damage = checked_trunc_f32_to_i64(modifier_sum * base_damage as f32 * percent)?;
    target_modifier
        .map(|modifier| checked_trunc_f32_to_i64(damage as f32 * modifier))
        .unwrap_or(Ok(damage))
}

pub(crate) fn original_curse_aura_damage(
    base_damage: i64,
    power_percent: f32,
    modifier_a: f32,
    modifier_b: f32,
    target_modifier: Option<f32>,
) -> Result<i64, CombatArithmeticError> {
    let scaled_base = checked_trunc_f32_to_i64(base_damage as f32 * power_percent * PERCENT_SCALE)?;
    let damage = checked_trunc_f32_to_i64(scaled_base as f32 * (modifier_a + modifier_b))?;
    target_modifier
        .map(|modifier| checked_trunc_f32_to_i64(damage as f32 * modifier))
        .unwrap_or(Ok(damage))
}

pub(crate) fn original_optional_integer_scaled_skill_damage(
    base_damage: i64,
    integer_scale: Option<i32>,
    parameter: f32,
    coefficient: f32,
) -> Result<i64, CombatArithmeticError> {
    let scaled_base = integer_scale
        .map(|scale| base_damage.wrapping_mul(scale as i64))
        .unwrap_or(base_damage);
    checked_trunc_f32_to_i64(scaled_base as f32 * parameter * coefficient * PERCENT_SCALE)
}

pub(crate) fn original_sniping_family_damage(
    base_damage: i64,
    parameter: f32,
    dynamic_coefficient: f32,
) -> Result<i64, CombatArithmeticError> {
    let effective_parameter = parameter * (1.0_f32 + dynamic_coefficient);
    checked_trunc_f32_to_i64(base_damage as f32 * effective_parameter * PERCENT_SCALE)
}

pub(crate) fn original_thunder_dragon_fury_damage(
    base_damage: i64,
    base_power: i32,
    selected_power: i32,
    selected_property_value: i32,
) -> Result<i64, CombatArithmeticError> {
    let combined_power = base_power
        .wrapping_add(selected_power)
        .wrapping_add(selected_power.wrapping_mul(selected_property_value));
    let integer_product = base_damage.wrapping_mul(combined_power as i64);
    checked_trunc_f32_to_i64(integer_product as f32 * PERCENT_SCALE)
}

#[cfg(test)]
mod tests {
    use super::{original_blizzard_damage, OriginalBlizzardDamageInputs};
    use crate::simulation::combat_core::arithmetic::CombatArithmeticError;

    #[test]
    fn blizzard_keeps_float32_order_and_truncates_once_at_the_end() {
        let result = original_blizzard_damage(OriginalBlizzardDamageInputs {
            base_damage: 1_001,
            coefficient_percent: 125.0,
            modifier_aggregate: 0.2,
        });
        assert_eq!(result, Ok(1_501));
    }

    #[test]
    fn blizzard_fails_closed_when_the_proven_conversion_domain_is_exceeded() {
        let result = original_blizzard_damage(OriginalBlizzardDamageInputs {
            base_damage: i64::MAX,
            coefficient_percent: f32::INFINITY,
            modifier_aggregate: 0.0,
        });
        assert_eq!(result, Err(CombatArithmeticError::NonFinite));
    }

    #[test]
    fn plain_and_affine_families_keep_their_distinct_coefficient_order() {
        assert_eq!(
            super::original_plain_percent_skill_damage(1_001, 125.0),
            Ok(1_251)
        );
        assert_eq!(
            super::original_affine_percent_skill_damage(1_001, 100.0, 25.0, 2.0),
            Ok(1_501)
        );
    }

    #[test]
    fn internal_integer_family_scales_before_multiplying_base_damage() {
        assert_eq!(super::original_internal_percent_skill_damage(5, 20), Ok(0));
        assert_eq!(super::original_plain_percent_skill_damage(5, 20.0), Ok(1));
    }

    #[test]
    fn poison_and_curse_aura_keep_different_intermediate_truncations() {
        assert_eq!(
            super::original_poison_aura_damage(5, 20, 0.5, 0.5, None),
            Ok(0)
        );
        assert_eq!(
            super::original_curse_aura_damage(5, 20.0, 0.5, 0.5, None),
            Ok(1)
        );
        assert_eq!(
            super::original_curse_aura_damage(101, 20.0, 0.5, 0.5, Some(1.5)),
            Ok(30)
        );
    }

    #[test]
    fn constant_data_families_preserve_integer_and_float32_order() {
        assert_eq!(
            super::original_optional_integer_scaled_skill_damage(100, Some(2), 1.5, 20.0),
            Ok(60)
        );
        assert_eq!(
            super::original_sniping_family_damage(1_000, 2.0, 0.5),
            Ok(30)
        );
        assert_eq!(
            super::original_thunder_dragon_fury_damage(100, 10, 5, 2),
            Ok(25)
        );
    }
}
