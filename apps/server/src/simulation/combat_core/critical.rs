/// Exact signed 32-bit threshold arithmetic recovered from HunterCtrl.getDamage.
pub(crate) fn original_critical_threshold(
    calculated_percent: i32,
    conditional_bonus_enabled: bool,
    conditional_bonus_percent: i32,
) -> i32 {
    let bonus = if conditional_bonus_enabled {
        conditional_bonus_percent
    } else {
        0
    };
    calculated_percent.wrapping_add(bonus).min(100)
}

/// The caller supplies Unity's integer roll from Random.Range(0, 100).
pub(crate) fn original_critical_roll_succeeds(threshold: i32, roll: i32) -> bool {
    roll < threshold
}

#[cfg(test)]
mod tests {
    use super::{original_critical_roll_succeeds, original_critical_threshold};

    #[test]
    fn threshold_caps_at_one_hundred_and_roll_is_exclusive() {
        assert_eq!(original_critical_threshold(20, true, 10), 30);
        assert!(original_critical_roll_succeeds(30, 29));
        assert!(!original_critical_roll_succeeds(30, 30));
        assert_eq!(original_critical_threshold(95, true, 20), 100);
        assert!(original_critical_roll_succeeds(100, 99));
    }

    #[test]
    fn threshold_addition_keeps_native_i32_wrapping_before_the_cap() {
        assert_eq!(original_critical_threshold(i32::MAX, true, 1), i32::MIN);
        assert!(!original_critical_roll_succeeds(i32::MIN, 0));
        assert_eq!(original_critical_threshold(-5, false, 99), -5);
    }
}
