use super::arithmetic::{checked_trunc_f32_to_i64, CombatArithmeticError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HunterShieldAndHpResult {
    pub current_shield: i64,
    pub hp_damage: i64,
    pub now_hp: i64,
}

pub(crate) fn original_hunter_feel_armor_factor(
    feel: f32,
    now_feel: f32,
) -> Result<f32, CombatArithmeticError> {
    if !feel.is_finite() || !now_feel.is_finite() {
        return Err(CombatArithmeticError::NonFinite);
    }
    if feel == 0.0 {
        return Err(CombatArithmeticError::UnsupportedDomain);
    }
    let ratio = now_feel / feel;
    Ok(if ratio >= 0.8_f32 {
        1.2_f32
    } else if ratio >= 0.6_f32 {
        1.1_f32
    } else if ratio >= 0.4_f32 {
        1.0_f32
    } else if ratio >= 0.2_f32 {
        0.9_f32
    } else {
        0.8_f32
    })
}

pub(crate) fn original_hunter_armor_scratch(
    calc_armor: i64,
    feel: f32,
    now_feel: f32,
) -> Result<i64, CombatArithmeticError> {
    let factor = original_hunter_feel_armor_factor(feel, now_feel)?;
    checked_trunc_f32_to_i64(calc_armor as f32 * factor)
}

pub(crate) fn original_hunter_forwarded_damage(
    accumulator: i64,
    armor_scratch: i64,
    selected_final_factor: f32,
) -> Result<i64, CombatArithmeticError> {
    let post_armor = accumulator.wrapping_sub(armor_scratch);
    if post_armor <= 0 {
        Ok(1)
    } else {
        checked_trunc_f32_to_i64(post_armor as f32 * selected_final_factor)
    }
}

pub(crate) fn original_hunter_hp_after_damage(now_hp: i64, forwarded_damage: i64) -> i64 {
    now_hp.wrapping_sub(forwarded_damage).max(0)
}

pub(crate) fn original_hunter_apply_first_shield_then_hp(
    current_shield: i64,
    now_hp: i64,
    forwarded_damage: i64,
) -> HunterShieldAndHpResult {
    let (current_shield, hp_damage) = if current_shield < forwarded_damage {
        (0, forwarded_damage.wrapping_sub(current_shield))
    } else {
        (current_shield.wrapping_sub(forwarded_damage), 0)
    };
    HunterShieldAndHpResult {
        current_shield,
        hp_damage,
        now_hp: original_hunter_hp_after_damage(now_hp, hp_damage),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feel_bands_keep_native_equalities_and_reject_unproven_zero_denominator() {
        assert_eq!(original_hunter_feel_armor_factor(100.0, 80.0), Ok(1.2));
        assert_eq!(original_hunter_feel_armor_factor(100.0, 60.0), Ok(1.1));
        assert_eq!(original_hunter_feel_armor_factor(100.0, 40.0), Ok(1.0));
        assert_eq!(original_hunter_feel_armor_factor(100.0, 20.0), Ok(0.9));
        assert_eq!(original_hunter_feel_armor_factor(100.0, 19.999), Ok(0.8));
        assert_eq!(
            original_hunter_feel_armor_factor(0.0, 0.0),
            Err(CombatArithmeticError::UnsupportedDomain)
        );
    }

    #[test]
    fn armor_and_forwarded_damage_preserve_each_truncation_boundary() {
        assert_eq!(original_hunter_armor_scratch(101, 100.0, 80.0), Ok(121));
        assert_eq!(original_hunter_forwarded_damage(100, 30, 0.5), Ok(35));
        assert_eq!(original_hunter_forwarded_damage(30, 30, 0.5), Ok(1));
        assert_eq!(original_hunter_forwarded_damage(20, 30, 2.0), Ok(1));
    }

    #[test]
    fn shield_routes_before_the_clamped_hp_mutation() {
        assert_eq!(
            original_hunter_apply_first_shield_then_hp(30, 10, 50),
            HunterShieldAndHpResult {
                current_shield: 0,
                hp_damage: 20,
                now_hp: 0,
            }
        );
        assert_eq!(
            original_hunter_apply_first_shield_then_hp(50, 10, 30),
            HunterShieldAndHpResult {
                current_shield: 20,
                hp_damage: 0,
                now_hp: 10,
            }
        );
    }

    #[test]
    fn hunter_hp_uses_wrapping_subtraction_before_flooring() {
        assert_eq!(original_hunter_hp_after_damage(20, 35), 0);
        assert_eq!(original_hunter_hp_after_damage(i64::MIN, 1), i64::MAX);
    }
}
