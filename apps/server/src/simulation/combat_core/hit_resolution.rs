use std::collections::BTreeMap;

/// Exact attacker-owned effect-54 gate in the direct Evil-to-Hunter path.
/// A successful roll returns before `HunterCtrl.Damaged`; evidence does not
/// support naming this effect as accuracy, dodge, blind, evasion, or miss.
pub(crate) fn original_effect_54_aborts_damage(
    attacker_effect_value: i32,
    roll_zero_to_ninety_nine: i32,
) -> bool {
    attacker_effect_value >= 1 && roll_zero_to_ninety_nine < attacker_effect_value
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OriginalDodgeResolution {
    MezeBypass,
    PrimaryEvade,
    RidingPetEvade,
    Hit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OriginalDodgeInputError {
    PrimaryRollOutOfRange,
    MissingRidingPetRoll,
    RidingPetRollOutOfRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OriginalDodgeInputs {
    pub is_meze_state: bool,
    pub calc_dodge: i32,
    pub effect_type_5_bonus: i32,
    pub riding_pet_dodge: i32,
    pub primary_roll_zero_to_ninety_nine: i32,
    pub riding_pet_roll_zero_to_nine_ninety_nine: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OriginalCalcDodgeInputs {
    pub hunter_dodge: i32,
    pub option_dodge: i32,
    pub personal_dodge: f32,
    pub rank_dodge: f32,
    pub gup_property_8: f32,
}

/// Replays `StatusData.CEOBAMNDIIL`'s common producer. The integer base sum
/// wraps before it is widened to float32, matching the native instruction order.
pub(crate) fn original_calc_dodge(
    input: OriginalCalcDodgeInputs,
) -> Result<i32, super::arithmetic::CombatArithmeticError> {
    let integer_base = input.hunter_dodge.wrapping_add(input.option_dodge) as f32;
    let raw = integer_base + input.personal_dodge + input.rank_dodge + input.gup_property_8;
    if !raw.is_finite() {
        return Err(super::arithmetic::CombatArithmeticError::NonFinite);
    }
    let clamped = raw.max(0.0);
    if clamped > i32::MAX as f32 {
        return Err(super::arithmetic::CombatArithmeticError::OutOfRange);
    }
    Ok(clamped.round_ties_even() as i32)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct OriginalEvasionCalculator {
    hunter_dodge: i32,
    option_dodge: i32,
    additive_sources: BTreeMap<String, f32>,
}

impl OriginalEvasionCalculator {
    pub(crate) fn set_hunter_dodge(&mut self, value: i32) {
        self.hunter_dodge = value;
    }

    pub(crate) fn set_option_dodge(&mut self, value: i32) {
        self.option_dodge = value;
    }

    pub(crate) fn set_additive_source(&mut self, source_id: impl Into<String>, value: f32) {
        self.additive_sources.insert(source_id.into(), value);
    }

    pub(crate) fn remove_additive_source(&mut self, source_id: &str) {
        self.additive_sources.remove(source_id);
    }

    pub(crate) fn calc_dodge(&self) -> Result<i32, super::arithmetic::CombatArithmeticError> {
        let mut total = self.hunter_dodge.wrapping_add(self.option_dodge) as f32;
        const ORIGINAL_SOURCE_ORDER: [&str; 3] = ["personal_dodge", "rank_dodge", "gup_property_8"];
        for source_id in ORIGINAL_SOURCE_ORDER {
            total += self.additive_sources.get(source_id).copied().unwrap_or(0.0);
        }
        for (source_id, value) in &self.additive_sources {
            if !ORIGINAL_SOURCE_ORDER.contains(&source_id.as_str()) {
                total += *value;
            }
        }
        if !total.is_finite() {
            return Err(super::arithmetic::CombatArithmeticError::NonFinite);
        }
        let clamped = total.max(0.0);
        if clamped > i32::MAX as f32 {
            return Err(super::arithmetic::CombatArithmeticError::OutOfRange);
        }
        Ok(clamped.round_ties_even() as i32)
    }
}

/// Replays `HunterCtrl.DGPHLIIAEFL` without owning an RNG. The optional pet
/// roll makes the native short-circuit consumption order explicit.
pub(crate) fn resolve_original_hunter_dodge(
    input: OriginalDodgeInputs,
) -> Result<OriginalDodgeResolution, OriginalDodgeInputError> {
    if input.is_meze_state {
        return Ok(OriginalDodgeResolution::MezeBypass);
    }
    if !(0..100).contains(&input.primary_roll_zero_to_ninety_nine) {
        return Err(OriginalDodgeInputError::PrimaryRollOutOfRange);
    }

    let threshold = input.calc_dodge.wrapping_add(input.effect_type_5_bonus);
    if input.primary_roll_zero_to_ninety_nine < threshold {
        return Ok(OriginalDodgeResolution::PrimaryEvade);
    }

    let pet_roll = input
        .riding_pet_roll_zero_to_nine_ninety_nine
        .ok_or(OriginalDodgeInputError::MissingRidingPetRoll)?;
    if !(0..1000).contains(&pet_roll) {
        return Err(OriginalDodgeInputError::RidingPetRollOutOfRange);
    }
    if pet_roll < input.riding_pet_dodge {
        Ok(OriginalDodgeResolution::RidingPetEvade)
    } else {
        Ok(OriginalDodgeResolution::Hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_54_is_enabled_at_one_and_uses_an_exclusive_threshold() {
        assert!(!original_effect_54_aborts_damage(0, 0));
        assert!(original_effect_54_aborts_damage(1, 0));
        assert!(original_effect_54_aborts_damage(25, 24));
        assert!(!original_effect_54_aborts_damage(25, 25));
        assert!(original_effect_54_aborts_damage(100, 99));
    }

    fn dodge_input(
        calc_dodge: i32,
        bonus: i32,
        primary_roll: i32,
        pet_dodge: i32,
        pet_roll: Option<i32>,
    ) -> OriginalDodgeInputs {
        OriginalDodgeInputs {
            is_meze_state: false,
            calc_dodge,
            effect_type_5_bonus: bonus,
            riding_pet_dodge: pet_dodge,
            primary_roll_zero_to_ninety_nine: primary_roll,
            riding_pet_roll_zero_to_nine_ninety_nine: pet_roll,
        }
    }

    #[test]
    fn meze_bypasses_dodge_without_requiring_any_legal_roll() {
        let mut input = dodge_input(100, 0, -1, 0, None);
        input.is_meze_state = true;
        assert_eq!(
            resolve_original_hunter_dodge(input),
            Ok(OriginalDodgeResolution::MezeBypass)
        );
    }

    #[test]
    fn primary_threshold_is_exclusive_and_success_skips_pet_roll() {
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(25, 0, 24, 0, None)),
            Ok(OriginalDodgeResolution::PrimaryEvade)
        );
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(25, 0, 25, 0, None)),
            Err(OriginalDodgeInputError::MissingRidingPetRoll)
        );
    }

    #[test]
    fn pet_fallback_is_exclusive() {
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(0, 0, 0, 10, Some(9))),
            Ok(OriginalDodgeResolution::RidingPetEvade)
        );
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(0, 0, 0, 10, Some(10))),
            Ok(OriginalDodgeResolution::Hit)
        );
    }

    #[test]
    fn native_signed_threshold_has_no_explicit_clamp() {
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(-1, 0, 0, 0, Some(0))),
            Ok(OriginalDodgeResolution::Hit)
        );
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(101, 0, 99, 0, None)),
            Ok(OriginalDodgeResolution::PrimaryEvade)
        );
    }

    #[test]
    fn threshold_addition_wraps_as_native_i32() {
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(i32::MAX, 1, 0, 0, Some(0))),
            Ok(OriginalDodgeResolution::Hit)
        );
        assert_eq!(
            resolve_original_hunter_dodge(dodge_input(i32::MIN, -1, 99, 0, None)),
            Ok(OriginalDodgeResolution::PrimaryEvade)
        );
    }

    #[test]
    fn calc_dodge_replays_integer_sum_float_layers_clamp_and_bankers_rounding() {
        assert_eq!(
            original_calc_dodge(OriginalCalcDodgeInputs {
                hunter_dodge: 2,
                option_dodge: 1,
                personal_dodge: 0.5,
                rank_dodge: 0.0,
                gup_property_8: 0.0,
            }),
            Ok(4)
        );
        assert_eq!(
            original_calc_dodge(OriginalCalcDodgeInputs {
                hunter_dodge: -5,
                option_dodge: 0,
                personal_dodge: 0.0,
                rank_dodge: 0.0,
                gup_property_8: 0.0,
            }),
            Ok(0)
        );
    }

    #[test]
    fn evasion_calculator_adds_removes_and_defaults_missing_sources_to_zero() {
        let mut calculator = OriginalEvasionCalculator::default();
        calculator.set_hunter_dodge(2);
        calculator.set_option_dodge(1);
        calculator.set_additive_source("personal_dodge", 0.5);
        calculator.set_additive_source("rank_dodge", 1.0);
        calculator.set_additive_source("gup_property_8", 0.0);
        assert_eq!(calculator.calc_dodge(), Ok(4));

        calculator.remove_additive_source("rank_dodge");
        assert_eq!(calculator.calc_dodge(), Ok(4));
        calculator.remove_additive_source("personal_dodge");
        assert_eq!(calculator.calc_dodge(), Ok(3));
    }
}
