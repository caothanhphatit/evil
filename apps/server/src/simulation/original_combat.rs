/// Exact 30-entry multiplier stream returned by the original client's
/// `GameManager.RandDamage()`. Values use hundredths to keep server math integer.
pub const ORIGINAL_DAMAGE_MULTIPLIER_HUNDREDTHS: [u16; 30] = [
    91, 100, 110, 92, 91, 110, 103, 106, 113, 95, 100, 92, 106, 98, 113, 95, 110, 110, 92, 105,
    103, 99, 110, 110, 90, 90, 110, 90, 100, 110,
];

/// Native-confirmed critical-damage base. `10_000` represents `1.0x`.
#[allow(dead_code)] // Evidence-backed reference; not connected until input semantics are resolved.
pub const ORIGINAL_BASE_CRITICAL_DAMAGE_BPS: u32 = 17_500;

/// `ConstantData.DEFALUT_DAMAGE_DECREASE_VALUE`, decoded with the original
/// ACTk byte-unshuffle and XOR path.
pub const ORIGINAL_DEFAULT_DAMAGE_DECREASE_FACTOR: f32 = 0.75_f32;

/// Replays the exact three-slot float stack from `EvilCtrl.GetReduceAttackValue`.
///
/// The slot writers receive integer percent values and multiply each by the
/// original float32 `0.01` constant. Callers must not attach gameplay names to
/// these slots until the remaining effect identifiers are recovered.
#[allow(dead_code)]
pub fn original_monster_attack_reduction(
    first_percent: i32,
    second_percent: i32,
    third_percent: i32,
) -> f32 {
    let percent_scale = 0.01_f32;
    (1.0 - first_percent as f32 * percent_scale)
        * (1.0 - second_percent as f32 * percent_scale)
        * (1.0 - third_percent as f32 * percent_scale)
}

/// Native-confirmed Hunter attack animation duration for an already-composed
/// speed factor. The writers that produce the factor remain unresolved.
#[allow(dead_code)]
pub fn original_hunter_attack_animation_time(composite_speed: f32) -> f32 {
    if composite_speed > 1.0 {
        0.333_f32 / composite_speed
    } else {
        0.7_f32
    }
}

/// Native-confirmed Evil attack delay for an already-composed factor. The
/// factor's obfuscated writer chain remains evidence-only.
#[allow(dead_code)]
pub fn original_monster_attack_delay(attack_factor: f32) -> f32 {
    0.08_f32 * attack_factor.max(1.0)
}

/// Exact `StatusData.COJNMPDBOOO` AttackSpeed aggregation.
#[allow(dead_code)]
pub fn original_hunter_attack_speed(
    weapon_speed: f32,
    personal_attack_speed: f32,
    option_attack_speed: f32,
    rank_attack_speed: f32,
    guild_attack_speed_up: f32,
    gup_property_7: f32,
    riding_pet_attack_speed_up: f32,
) -> f32 {
    weapon_speed
        * (1.0_f32
            + 0.01_f32
                * (personal_attack_speed + option_attack_speed + rank_attack_speed
                    - guild_attack_speed_up
                    - gup_property_7
                    - riding_pet_attack_speed_up))
}

/// Exact denominator branch and `CalcAttackSpeed` floor from the recovered
/// StatusData producer.
#[allow(dead_code)]
pub fn original_hunter_calc_attack_speed(
    attack_speed: f32,
    personal_attack_speed: f32,
    quicken: f32,
    fury_value: f32,
    speed_potion_value: f32,
) -> f32 {
    let denominator = if fury_value > 1.0_f32 {
        quicken + fury_value + speed_potion_value
    } else {
        quicken + speed_potion_value + personal_attack_speed
    };
    (attack_speed / denominator).max(0.25_f32)
}

/// Exact base returned by `StatusData.LCENGICKKGP`: float32 division first,
/// then widening into the outgoing `ObscuredDouble` chain.
#[allow(dead_code)]
pub fn original_hunter_base_outgoing_damage(
    calculated_damage: i64,
    calculated_attack_speed: f32,
) -> f64 {
    (calculated_damage as f32 / calculated_attack_speed) as f64
}

/// Exact final conversion at the end of `HunterCtrl.getDamage`.
#[allow(dead_code)]
pub fn original_hunter_finalize_outgoing_damage(value: f64) -> i64 {
    value as i64
}

/// Exact `FixedUpdate` countdown for `HunterCtrl.mAttackDelay`.
#[allow(dead_code)]
pub fn original_hunter_attack_delay_countdown(delay: f32, delta_time: f32) -> f32 {
    (delay - delta_time).max(0.0_f32)
}

/// Native-confirmed critical branch from `HunterCtrl.getDamage`. The caller
/// supplies Unity's already-generated integer roll in `[0, 100)`.
#[allow(dead_code)]
pub fn original_hunter_critical_hit(
    calculated_critical_percent: i32,
    conditional_bonus_enabled: bool,
    conditional_bonus_percent: i32,
    roll_zero_to_ninety_nine: i32,
) -> bool {
    let threshold = super::combat_core::critical::original_critical_threshold(
        calculated_critical_percent,
        conditional_bonus_enabled,
        conditional_bonus_percent,
    );
    super::combat_core::critical::original_critical_roll_succeeds(
        threshold,
        roll_zero_to_ninety_nine,
    )
}

/// Exact initialization of the native incoming-damage accumulator before its
/// still-unresolved skill, gear, trait and mode modifiers.
#[allow(dead_code)]
pub fn original_hunter_randomized_incoming_damage(
    incoming_damage: i64,
    multiplier_hundredths: u16,
) -> i64 {
    (incoming_damage as f32 * (multiplier_hundredths as f32 * 0.01_f32)) as i64
}

/// Exact five-band armor factor selected from `HunterData.nowFeel/feel`.
/// Keeping the native float32 division is significant at equality boundaries.
#[allow(dead_code)]
pub fn original_hunter_feel_armor_factor(feel: f32, now_feel: f32) -> f32 {
    let ratio = now_feel / feel;
    if ratio >= 0.8_f32 {
        1.2_f32
    } else if ratio >= 0.6_f32 {
        1.1_f32
    } else if ratio >= 0.4_f32 {
        1.0_f32
    } else if ratio >= 0.2_f32 {
        0.9_f32
    } else {
        0.8_f32
    }
}

/// Exact first armor conversion before the later unresolved modifier at
/// `HunterCtrl+0x7A0` is applied.
#[allow(dead_code)]
pub fn original_hunter_feel_armor_scratch(calc_armor: i64, feel: f32, now_feel: f32) -> i64 {
    (calc_armor as f32 * original_hunter_feel_armor_factor(feel, now_feel)) as i64
}

/// Exact common tail after the unresolved pre-armor damage modifiers.
#[allow(dead_code)]
pub fn original_hunter_damage_tail(
    accumulator: i64,
    armor_scratch: i64,
    selected_final_factor: f32,
) -> i64 {
    let post_armor = accumulator.wrapping_sub(armor_scratch);
    if post_armor <= 0 {
        1
    } else {
        (post_armor as f32 * selected_final_factor) as i64
    }
}

/// Exact normal tail using `ConstantData.DEFALUT_DAMAGE_DECREASE_VALUE`.
#[allow(dead_code)]
pub fn original_hunter_damage_tail_with_default_factor(
    accumulator: i64,
    armor_scratch: i64,
) -> i64 {
    original_hunter_damage_tail(
        accumulator,
        armor_scratch,
        ORIGINAL_DEFAULT_DAMAGE_DECREASE_FACTOR,
    )
}

/// Exact default `HitDamageProcess` HP mutation. The original also has an
/// unresolved auxiliary-pool branch which is intentionally not represented.
#[allow(dead_code)]
pub fn original_hunter_default_hp_after_damage(now_hp: i64, forwarded_damage: i64) -> i64 {
    now_hp.wrapping_sub(forwarded_damage).max(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalShieldDamageResult {
    pub current_shield: i64,
    pub hp_damage: i64,
    pub now_hp: i64,
}

/// Exact routing for the first `ShieldData` yielded by the original Hunter's
/// shield dictionary. Multi-entry dictionary ownership/order remains unresolved.
#[allow(dead_code)]
pub fn original_hunter_apply_shield_and_hp(
    current_shield: i64,
    now_hp: i64,
    forwarded_damage: i64,
) -> OriginalShieldDamageResult {
    let (current_shield, hp_damage) = if current_shield < forwarded_damage {
        (0, forwarded_damage.wrapping_sub(current_shield))
    } else {
        (current_shield.wrapping_sub(forwarded_damage), 0)
    };
    OriginalShieldDamageResult {
        current_shield,
        hp_damage,
        now_hp: original_hunter_default_hp_after_damage(now_hp, hp_damage),
    }
}

/// Exact attacker-owned effect-54 abort gate before `HunterCtrl.Damaged`.
/// Native evidence does not establish a public gameplay name for the effect.
#[allow(dead_code)]
pub fn original_effect_54_aborts_attack(value: i32, roll_zero_to_ninety_nine: i32) -> bool {
    super::combat_core::hit_resolution::original_effect_54_aborts_damage(
        value,
        roll_zero_to_ninety_nine,
    )
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OriginalDamageMultiplierStream {
    next_index: usize,
}

impl OriginalDamageMultiplierStream {
    pub fn next_hundredths(&mut self) -> u16 {
        let value = ORIGINAL_DAMAGE_MULTIPLIER_HUNDREDTHS[self.next_index];
        self.next_index = (self.next_index + 1) % ORIGINAL_DAMAGE_MULTIPLIER_HUNDREDTHS.len();
        value
    }

    pub fn next_index(&self) -> usize {
        self.next_index
    }
}

#[cfg(test)]
mod tests {
    use super::{
        original_effect_54_aborts_attack, original_hunter_apply_shield_and_hp,
        original_hunter_attack_animation_time, original_hunter_attack_delay_countdown,
        original_hunter_attack_speed, original_hunter_base_outgoing_damage,
        original_hunter_calc_attack_speed, original_hunter_critical_hit,
        original_hunter_damage_tail, original_hunter_damage_tail_with_default_factor,
        original_hunter_default_hp_after_damage, original_hunter_feel_armor_factor,
        original_hunter_feel_armor_scratch, original_hunter_finalize_outgoing_damage,
        original_hunter_randomized_incoming_damage, original_monster_attack_delay,
        original_monster_attack_reduction, OriginalDamageMultiplierStream,
        OriginalShieldDamageResult, ORIGINAL_BASE_CRITICAL_DAMAGE_BPS,
        ORIGINAL_DAMAGE_MULTIPLIER_HUNDREDTHS, ORIGINAL_DEFAULT_DAMAGE_DECREASE_FACTOR,
    };

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn rand_damage_stream_replays_the_captured_original_sequence() {
        let mut stream = OriginalDamageMultiplierStream::default();

        let first_cycle: Vec<u16> = (0..ORIGINAL_DAMAGE_MULTIPLIER_HUNDREDTHS.len())
            .map(|_| stream.next_hundredths())
            .collect();

        assert_eq!(first_cycle, ORIGINAL_DAMAGE_MULTIPLIER_HUNDREDTHS);
        assert_eq!(stream.next_index(), 0);
        assert_eq!(stream.next_hundredths(), 91);
        assert_eq!(stream.next_index(), 1);
    }

    #[test]
    fn critical_damage_base_matches_the_native_constant() {
        assert_eq!(ORIGINAL_BASE_CRITICAL_DAMAGE_BPS, 17_500);
        assert_close(ORIGINAL_DEFAULT_DAMAGE_DECREASE_FACTOR, 0.75);
    }

    #[test]
    fn monster_attack_reduction_multiplies_all_three_percent_slots() {
        assert_close(original_monster_attack_reduction(20, 10, 5), 0.684);
        assert_close(original_monster_attack_reduction(0, 0, 0), 1.0);
    }

    #[test]
    fn attack_cadence_replays_the_native_branch_boundaries() {
        assert_close(original_hunter_attack_animation_time(1.0), 0.7);
        assert_close(original_hunter_attack_animation_time(2.0), 0.1665);
        assert_close(original_monster_attack_delay(0.5), 0.08);
        assert_close(original_monster_attack_delay(2.0), 0.16);
        assert_close(original_hunter_attack_delay_countdown(0.1, 0.04), 0.06);
        assert_close(original_hunter_attack_delay_countdown(0.02, 0.04), 0.0);
    }

    #[test]
    fn hunter_attack_speed_producer_replays_recovered_vectors() {
        assert_close(
            original_hunter_attack_speed(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            1.0,
        );
        assert_close(
            original_hunter_attack_speed(0.8, 10.0, 20.0, 5.0, 3.0, 2.0, 1.0),
            1.032,
        );
        assert_close(
            original_hunter_calc_attack_speed(0.5, 0.0, 1.0, 1.0, 0.0),
            0.5,
        );
        assert_close(
            original_hunter_calc_attack_speed(0.5, 4.0, 1.0, 2.0, 1.0),
            0.25,
        );
    }

    #[test]
    fn outgoing_base_uses_float32_division_and_final_truncation() {
        assert_eq!(original_hunter_base_outgoing_damage(100, 0.5), 200.0);
        assert_eq!(
            original_hunter_base_outgoing_damage(1, 3.0),
            (1.0_f32 / 3.0_f32) as f64
        );
        assert_eq!(original_hunter_finalize_outgoing_damage(99.999), 99);
        assert_eq!(original_hunter_finalize_outgoing_damage(-99.999), -99);
    }

    #[test]
    fn critical_roll_uses_an_exclusive_capped_threshold() {
        assert!(original_hunter_critical_hit(20, true, 10, 29));
        assert!(!original_hunter_critical_hit(20, true, 10, 30));
        assert!(original_hunter_critical_hit(95, true, 20, 99));
        assert!(!original_hunter_critical_hit(-5, false, 0, 0));
        assert!(!original_hunter_critical_hit(i32::MAX, true, 1, 0));
    }

    #[test]
    fn hunter_damage_tail_replays_armor_and_final_factor_vectors() {
        assert_eq!(original_hunter_damage_tail(100, 30, 0.5), 35);
        assert_eq!(original_hunter_damage_tail(30, 30, 0.5), 1);
        assert_eq!(original_hunter_damage_tail(20, 30, 2.0), 1);
        assert_eq!(original_hunter_damage_tail_with_default_factor(100, 20), 60);
    }

    #[test]
    fn incoming_damage_accumulator_uses_float32_and_truncates_toward_zero() {
        assert_eq!(original_hunter_randomized_incoming_damage(100, 91), 91);
        assert_eq!(original_hunter_randomized_incoming_damage(35, 91), 31);
        assert_eq!(original_hunter_randomized_incoming_damage(-35, 91), -31);
    }

    #[test]
    fn feel_armor_selector_keeps_equalities_in_the_higher_band() {
        assert_close(original_hunter_feel_armor_factor(100.0, 100.0), 1.2);
        assert_close(original_hunter_feel_armor_factor(100.0, 80.0), 1.2);
        assert_close(original_hunter_feel_armor_factor(100.0, 79.999), 1.1);
        assert_close(original_hunter_feel_armor_factor(100.0, 60.0), 1.1);
        assert_close(original_hunter_feel_armor_factor(100.0, 40.0), 1.0);
        assert_close(original_hunter_feel_armor_factor(100.0, 20.0), 0.9);
        assert_close(original_hunter_feel_armor_factor(100.0, 19.999), 0.8);
        assert_eq!(original_hunter_feel_armor_scratch(101, 100.0, 80.0), 121);
    }

    #[test]
    fn shield_absorbs_before_the_remaining_damage_reaches_hp() {
        assert_eq!(
            original_hunter_apply_shield_and_hp(30, 100, 50),
            OriginalShieldDamageResult {
                current_shield: 0,
                hp_damage: 20,
                now_hp: 80,
            }
        );
        assert_eq!(
            original_hunter_apply_shield_and_hp(50, 100, 30),
            OriginalShieldDamageResult {
                current_shield: 20,
                hp_damage: 0,
                now_hp: 100,
            }
        );
        assert_eq!(
            original_hunter_apply_shield_and_hp(30, 100, 30),
            OriginalShieldDamageResult {
                current_shield: 0,
                hp_damage: 0,
                now_hp: 100,
            }
        );
    }

    #[test]
    fn default_hunter_hp_mutation_floors_at_zero() {
        assert_eq!(original_hunter_default_hp_after_damage(100, 35), 65);
        assert_eq!(original_hunter_default_hp_after_damage(20, 35), 0);
        assert_eq!(original_hunter_default_hp_after_damage(1, 1), 0);
    }

    #[test]
    fn effect_54_uses_an_enabled_exclusive_threshold() {
        assert!(!original_effect_54_aborts_attack(0, 0));
        assert!(original_effect_54_aborts_attack(25, 24));
        assert!(!original_effect_54_aborts_attack(25, 25));
        assert!(original_effect_54_aborts_attack(100, 99));
    }
}
