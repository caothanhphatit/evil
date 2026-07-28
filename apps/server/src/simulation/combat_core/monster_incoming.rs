use super::arithmetic::{checked_trunc_f32_to_i64, CombatArithmeticError};

const POSITIVE_SUBNORMAL_THRESHOLD: f32 = f32::from_bits(1);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalMonsterDamageInputs {
    pub incoming_damage: i64,
    pub hunter_feel: f32,
    pub hunter_now_feel: f32,
    pub rand_damage_multiplier: f32,
    pub direct_bonus_a: f32,
    pub direct_bonus_b: f32,
    pub pre_armor_bonus_rate: f32,
    pub ignore_armor: bool,
    pub armor_reduction_rate: f32,
    pub monster_armor: i64,
    pub monster_now_hp: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OriginalMonsterDamageResult {
    pub feel_scalar_bits: u32,
    pub randomized_damage: i64,
    pub damage_after_direct_bonus: i64,
    pub pre_armor_bonus_damage: i64,
    pub effective_armor: i64,
    pub final_damage: i64,
    pub monster_now_hp: i64,
}

fn original_feel_scalar(feel: f32, now_feel: f32) -> f32 {
    if now_feel >= feel * 0.8_f32 {
        1.2_f32
    } else if now_feel >= feel * 0.6_f32 {
        1.1_f32
    } else if now_feel >= feel * 0.4_f32 {
        1.0_f32
    } else if now_feel >= feel * 0.2_f32 {
        0.9_f32
    } else {
        0.8_f32
    }
}

pub(crate) fn original_monster_damage(
    input: OriginalMonsterDamageInputs,
) -> Result<OriginalMonsterDamageResult, CombatArithmeticError> {
    let feel_scalar = original_feel_scalar(input.hunter_feel, input.hunter_now_feel);
    let randomized_damage = checked_trunc_f32_to_i64(
        input.incoming_damage as f32 * feel_scalar * input.rand_damage_multiplier,
    )?;

    let direct_bonus = input.direct_bonus_a + input.direct_bonus_b;
    let damage_after_direct_bonus = if direct_bonus > POSITIVE_SUBNORMAL_THRESHOLD {
        checked_trunc_f32_to_i64(randomized_damage as f32 * (1.0_f32 + direct_bonus))?
    } else {
        randomized_damage
    };

    let pre_armor_bonus_damage =
        checked_trunc_f32_to_i64(damage_after_direct_bonus as f32 * input.pre_armor_bonus_rate)?;
    let pre_armor_damage = damage_after_direct_bonus.wrapping_add(pre_armor_bonus_damage);

    let effective_armor = if input.ignore_armor {
        0
    } else {
        let removed_armor =
            checked_trunc_f32_to_i64(input.monster_armor as f32 * input.armor_reduction_rate)?;
        input.monster_armor.wrapping_sub(removed_armor).max(0)
    };
    let post_armor = pre_armor_damage.wrapping_sub(effective_armor);
    let final_damage = if post_armor <= 0 { 1 } else { post_armor };

    Ok(OriginalMonsterDamageResult {
        feel_scalar_bits: feel_scalar.to_bits(),
        randomized_damage,
        damage_after_direct_bonus,
        pre_armor_bonus_damage,
        effective_armor,
        final_damage,
        monster_now_hp: input.monster_now_hp.wrapping_sub(final_damage),
    })
}

#[cfg(test)]
mod tests {
    use super::{original_monster_damage, CombatArithmeticError, OriginalMonsterDamageInputs};

    #[test]
    fn applies_native_order_before_armor_and_hp() {
        let result = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 100,
            hunter_feel: 100.0,
            hunter_now_feel: 80.0,
            rand_damage_multiplier: 0.91,
            direct_bonus_a: 0.1,
            direct_bonus_b: 0.2,
            pre_armor_bonus_rate: 0.25,
            armor_reduction_rate: 0.2,
            monster_armor: 50,
            monster_now_hp: 500,
            ..OriginalMonsterDamageInputs::default()
        })
        .unwrap();

        assert_eq!(result.feel_scalar_bits, 1.2_f32.to_bits());
        assert_eq!(result.randomized_damage, 109);
        assert_eq!(result.damage_after_direct_bonus, 141);
        assert_eq!(result.pre_armor_bonus_damage, 35);
        assert_eq!(result.effective_armor, 40);
        assert_eq!(result.final_damage, 136);
        assert_eq!(result.monster_now_hp, 364);
    }

    #[test]
    fn minimum_damage_and_ignore_armor_branches_are_explicit() {
        let minimum = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 10,
            hunter_feel: 100.0,
            hunter_now_feel: 0.0,
            rand_damage_multiplier: 1.0,
            monster_armor: 100,
            monster_now_hp: 20,
            ..OriginalMonsterDamageInputs::default()
        })
        .unwrap();
        assert_eq!(minimum.final_damage, 1);
        assert_eq!(minimum.monster_now_hp, 19);

        let ignored = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 10,
            hunter_feel: 100.0,
            hunter_now_feel: 100.0,
            rand_damage_multiplier: 1.0,
            ignore_armor: true,
            monster_armor: 100,
            monster_now_hp: 20,
            ..OriginalMonsterDamageInputs::default()
        })
        .unwrap();
        assert_eq!(ignored.effective_armor, 0);
        assert_eq!(ignored.final_damage, 12);
        assert_eq!(ignored.monster_now_hp, 8);
    }

    #[test]
    fn direct_bonus_gate_uses_the_native_subnormal_comparison() {
        let no_bonus = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 100,
            hunter_feel: 1.0,
            hunter_now_feel: 1.0,
            rand_damage_multiplier: 1.0,
            direct_bonus_a: f32::from_bits(1),
            ..OriginalMonsterDamageInputs::default()
        })
        .unwrap();
        assert_eq!(no_bonus.damage_after_direct_bonus, 120);
    }

    #[test]
    fn monster_overkill_keeps_the_recovered_wrapping_hp_mutation() {
        let result = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 10,
            hunter_feel: 100.0,
            hunter_now_feel: 100.0,
            rand_damage_multiplier: 1.0,
            ignore_armor: true,
            monster_now_hp: 5,
            ..OriginalMonsterDamageInputs::default()
        })
        .unwrap();

        assert_eq!(result.final_damage, 12);
        assert_eq!(result.monster_now_hp, -7);
    }

    #[test]
    fn armor_reduction_is_truncated_before_wrapping_subtraction_and_flooring() {
        let result = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 10,
            hunter_feel: 100.0,
            hunter_now_feel: 100.0,
            rand_damage_multiplier: 1.0,
            monster_armor: 101,
            armor_reduction_rate: 0.2,
            monster_now_hp: 100,
            ..OriginalMonsterDamageInputs::default()
        })
        .unwrap();

        assert_eq!(result.effective_armor, 81);
        assert_eq!(result.final_damage, 1);
    }

    #[test]
    fn non_finite_values_fail_only_when_the_native_branch_converts_them() {
        let converted = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 10,
            hunter_feel: 100.0,
            hunter_now_feel: 100.0,
            rand_damage_multiplier: f32::NAN,
            ..OriginalMonsterDamageInputs::default()
        });
        assert_eq!(converted, Err(CombatArithmeticError::NonFinite));

        let bypassed = original_monster_damage(OriginalMonsterDamageInputs {
            incoming_damage: 10,
            hunter_feel: 100.0,
            hunter_now_feel: 100.0,
            rand_damage_multiplier: 1.0,
            ignore_armor: true,
            armor_reduction_rate: f32::NAN,
            monster_now_hp: 20,
            ..OriginalMonsterDamageInputs::default()
        })
        .unwrap();
        assert_eq!(bypassed.effective_armor, 0);
    }
}
