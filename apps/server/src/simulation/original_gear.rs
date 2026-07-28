const ORIGINAL_SEAL_ATTACK_IDS: [i32; 15] = [
    157, 158, 159, 160, 161, 202, 255, 320, 359, 476, 489, 531, 706, 758, 822,
];

fn original_gear_quality_multiplier(quality: i32) -> f64 {
    match quality {
        0 => 0.8,
        1 => 0.9,
        3 => 1.1,
        4 => 1.2,
        _ => 1.0,
    }
}

/// Exact neutral arithmetic shared by `GetGearArmor` and `GetGearAcc`.
/// Catalog row semantics for the three numeric inputs remain unresolved.
#[allow(dead_code)]
pub fn original_gear_armor_or_accuracy(
    base: f64,
    rating_percent: f64,
    level_percent: f64,
    quality: i32,
) -> i64 {
    let value = base
        * (rating_percent / 100.0)
        * (1.0 + level_percent / 100.0)
        * original_gear_quality_multiplier(quality);
    value.round_ties_even() as i64
}

/// Exact step schedule used by `GameManager.GetFirstPercent`.
///
/// The native caller supplies the catalog array. As in the original method,
/// missing entries for a referenced step are invalid rather than defaulted.
#[allow(dead_code)]
pub fn original_get_first_percent(first_percent_values: &[i32], limit: i32) -> i32 {
    if limit < 0 {
        return 0;
    }

    let mut total: i32 = 0;
    for step in 0..=limit {
        let index = match step {
            1..=20 => Some(((step - 1) / 5) as usize),
            21..=25 => Some((step - 17) as usize),
            _ => None,
        };
        if let Some(index) = index {
            total = total.wrapping_add(first_percent_values[index]);
        }
    }
    total
}

/// Exact structural arithmetic recovered from `GameManager.GetGearDamage`.
/// The caller-provided level adjustment and broader stat aggregation remain
/// unresolved, so this reference is deliberately disconnected from live combat.
#[allow(dead_code)]
pub fn original_gear_damage(
    first_value: f64,
    rating_values: &[f64],
    rating: usize,
    first_percent_values: &[i32],
    level_plus_adjustment: i32,
    quality: i32,
    second_value: f64,
) -> i64 {
    assert!(
        !rating_values.is_empty(),
        "the original catalog rating array must not be empty"
    );
    let rating_percent = rating_values[rating.min(rating_values.len() - 1)];
    let level_percent =
        original_get_first_percent(first_percent_values, level_plus_adjustment) as f64;
    let value = first_value
        * (rating_percent / 100.0)
        * (1.0 + level_percent / 100.0)
        * original_gear_quality_multiplier(quality)
        * (second_value / 100.0);
    value.round_ties_even() as i64
}

/// Exact selector/result boundary for `GameManager.GetSealAttackUp`.
#[allow(dead_code)]
pub fn original_seal_attack_up(seal_id: i32, selected_row_first_value: i32) -> f32 {
    if ORIGINAL_SEAL_ATTACK_IDS.contains(&seal_id) {
        selected_row_first_value as f32 * 0.01_f32
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        original_gear_armor_or_accuracy, original_gear_damage, original_get_first_percent,
        original_seal_attack_up,
    };

    #[test]
    fn armor_and_accuracy_pipeline_replays_quality_and_percentage_vectors() {
        assert_eq!(original_gear_armor_or_accuracy(100.0, 100.0, 0.0, 2), 100);
        assert_eq!(original_gear_armor_or_accuracy(100.0, 80.0, 25.0, 0), 80);
        assert_eq!(original_gear_armor_or_accuracy(100.0, 110.0, 10.0, 4), 145);
    }

    #[test]
    fn gear_rounding_uses_midpoint_ties_to_even() {
        assert_eq!(original_gear_armor_or_accuracy(10.5, 100.0, 0.0, 2), 10);
        assert_eq!(original_gear_armor_or_accuracy(11.5, 100.0, 0.0, 2), 12);
    }

    #[test]
    fn first_percent_replays_the_original_step_schedule() {
        let values = [2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(original_get_first_percent(&values, -1), 0);
        assert_eq!(original_get_first_percent(&values, 0), 0);
        assert_eq!(original_get_first_percent(&values, 5), 10);
        assert_eq!(original_get_first_percent(&values, 12), 33);
        assert_eq!(original_get_first_percent(&values, 25), 110);
        assert_eq!(original_get_first_percent(&values, 30), 110);
    }

    #[test]
    fn gear_damage_replays_recovered_golden_vectors() {
        assert_eq!(
            original_gear_damage(100.0, &[100.0], 0, &[0; 9], 0, 2, 100.0),
            100
        );
        assert_eq!(
            original_gear_damage(120.0, &[80.0], 0, &[5, 0, 0, 0, 0, 0, 0, 0, 0], 5, 4, 150.0,),
            216
        );
    }

    #[test]
    fn gear_damage_clamps_rating_to_the_last_catalog_entry() {
        assert_eq!(
            original_gear_damage(100.0, &[50.0, 80.0], 99, &[0; 9], 0, 2, 100.0),
            80
        );
    }

    #[test]
    fn seal_attack_selector_rejects_unknown_ids() {
        assert!((original_seal_attack_up(157, 25) - 0.25).abs() < f32::EPSILON);
        assert!((original_seal_attack_up(161, 125) - 1.25).abs() < f32::EPSILON);
        assert_eq!(original_seal_attack_up(156, 25), 0.0);
    }
}
