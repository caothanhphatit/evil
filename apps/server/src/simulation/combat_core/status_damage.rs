use super::arithmetic::{
    checked_trunc_f32_to_i64, checked_trunc_f64_to_i64, CombatArithmeticError,
};

const PERCENT_SCALE: f32 = 0.01_f32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OriginalStatusDamageInputs {
    pub hunter_damage: i64,
    pub weapon_speed: f32,
    pub hunter_level: i32,
    pub hunter_revive: i32,
    pub gear_damage: f32,
    pub personal_damage: f32,
    pub option_damage: f32,
    pub rank_damage: f32,
    pub building_attack_up: f32,
    pub pet_attack_up: i32,
    pub pet_attack_up_2: i32,
    pub pet_attack_up_3: i32,
    pub pet_hp_attack_up: i32,
    pub guild_attack_up: f32,
    pub gup_property_4: f32,
    pub heroic_job_trait_attack_up: f32,
    pub gear_damage_upgrade: f32,
    pub costume_attack_up: f32,
    pub seal_attack_up: f32,
    pub collection_attack_up: f32,
    pub riding_pet_attack_up: f32,
    pub relic_collection_attack_up: f32,
    pub fairy_index: i32,
    pub poly_index: i32,
    pub torment_attack_up: f32,
    pub guild_rank_buff_attack: f32,
    pub damage_potion_value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OriginalStatusDamageResult {
    pub calc_level: f32,
    pub calc_revive: i32,
    pub fairy_attack_up: f32,
    pub damage_before_potion: i64,
    pub calc_damage: i64,
}

pub(crate) fn original_status_calc_level(hunter_level: i32) -> f32 {
    1.0_f32 + hunter_level as f32 * 0.003_f32
}

pub(crate) fn original_status_calc_revive(hunter_revive: i32) -> i32 {
    if hunter_revive < 1 {
        1
    } else {
        hunter_revive.wrapping_mul(3)
    }
}

fn original_fairy_attack_up(fairy_index: i32) -> f32 {
    match fairy_index {
        78 | 418 | 599 | 600 => 0.02_f32,
        360 => 0.04_f32,
        748 | 773 => 0.06_f32,
        _ => 0.0_f32,
    }
}

pub(crate) fn original_status_calc_damage(
    input: OriginalStatusDamageInputs,
) -> Result<OriginalStatusDamageResult, CombatArithmeticError> {
    let calc_level = original_status_calc_level(input.hunter_level);
    let calc_revive = original_status_calc_revive(input.hunter_revive);

    // The native producer performs this base chain in float32, then widens it.
    let base = input.hunter_damage as f32 * input.weapon_speed * calc_revive as f32 * calc_level;
    let mut damage = base as f64 + input.gear_damage as f64;

    let percent_sum = input.personal_damage
        + input.option_damage
        + input.rank_damage
        + input.building_attack_up
        + input.pet_attack_up as f32
        + input.pet_attack_up_2 as f32
        + input.pet_attack_up_3 as f32
        + input.pet_hp_attack_up as f32
        + input.guild_attack_up
        + input.gup_property_4;
    let primary_multiplier = 1.0_f32
        + percent_sum * PERCENT_SCALE
        + input.heroic_job_trait_attack_up
        + input.gear_damage_upgrade;
    damage *= primary_multiplier as f64;

    let collection_multiplier = 1.0_f32
        + input.costume_attack_up
        + input.seal_attack_up
        + input.collection_attack_up
        + input.riding_pet_attack_up
        + input.relic_collection_attack_up;
    damage *= collection_multiplier as f64;

    let fairy_attack_up = original_fairy_attack_up(input.fairy_index);
    if fairy_attack_up != 0.0 {
        damage *= (1.0_f32 + fairy_attack_up) as f64;
    }
    if input.poly_index == 49 {
        damage *= 1.299_999_952_316_284_2_f64;
    }

    let torment_guild_multiplier = 1.0_f32 + input.torment_attack_up + input.guild_rank_buff_attack;
    damage *= torment_guild_multiplier as f64;

    let damage_before_potion = checked_trunc_f64_to_i64(damage)?;
    let mut calc_damage = if input.damage_potion_value > 0.0 {
        let potion_bonus =
            checked_trunc_f32_to_i64(damage_before_potion as f32 * input.damage_potion_value)?;
        damage_before_potion.wrapping_add(potion_bonus)
    } else {
        damage_before_potion
    };
    if calc_damage < 0 {
        calc_damage = 0;
    }

    Ok(OriginalStatusDamageResult {
        calc_level,
        calc_revive,
        fairy_attack_up,
        damage_before_potion,
        calc_damage,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        original_status_calc_damage, original_status_calc_level, original_status_calc_revive,
        OriginalStatusDamageInputs,
    };

    #[test]
    fn level_and_revive_match_native_boundaries() {
        assert_eq!(original_status_calc_level(0), 1.0);
        assert_eq!(original_status_calc_level(10), 1.03_f32);
        assert_eq!(original_status_calc_revive(-1), 1);
        assert_eq!(original_status_calc_revive(0), 1);
        assert_eq!(original_status_calc_revive(1), 3);
        assert_eq!(original_status_calc_revive(5), 15);
    }

    #[test]
    fn calc_damage_preserves_layer_order_and_potion_truncation() {
        let result = original_status_calc_damage(OriginalStatusDamageInputs {
            hunter_damage: 100,
            weapon_speed: 1.0,
            hunter_level: 10,
            hunter_revive: 0,
            gear_damage: 10.0,
            personal_damage: 10.0,
            fairy_index: 78,
            poly_index: 49,
            damage_potion_value: 0.1,
            ..OriginalStatusDamageInputs::default()
        })
        .unwrap();

        assert_eq!(result.calc_level, 1.03_f32);
        assert_eq!(result.calc_revive, 1);
        assert_eq!(result.fairy_attack_up, 0.02_f32);
        assert_eq!(result.damage_before_potion, 164);
        assert_eq!(result.calc_damage, 180);
    }

    #[test]
    fn base_chain_rounds_in_float32_before_widening_to_float64() {
        let result = original_status_calc_damage(OriginalStatusDamageInputs {
            hunter_damage: 16_000_000,
            weapon_speed: 1.01,
            hunter_level: 37,
            hunter_revive: 1,
            ..OriginalStatusDamageInputs::default()
        })
        .unwrap();

        assert_eq!(result.calc_level.to_bits(), 1.111_000_1_f32.to_bits());
        assert_eq!(result.damage_before_potion, 53_861_284);
    }

    #[test]
    fn unsupported_fairy_ids_do_not_receive_a_silent_bonus() {
        let result = original_status_calc_damage(OriginalStatusDamageInputs {
            hunter_damage: 100,
            weapon_speed: 1.0,
            fairy_index: 999,
            ..OriginalStatusDamageInputs::default()
        })
        .unwrap();
        assert_eq!(result.fairy_attack_up, 0.0);
        assert_eq!(result.calc_damage, 100);
    }

    #[test]
    fn unsupported_non_finite_conversion_fails_closed() {
        let result = original_status_calc_damage(OriginalStatusDamageInputs {
            hunter_damage: 100,
            weapon_speed: f32::NAN,
            ..OriginalStatusDamageInputs::default()
        });
        assert_eq!(result, Err(super::CombatArithmeticError::NonFinite));
    }

    #[test]
    fn potion_addition_keeps_the_native_wrapping_then_negative_clamp_order() {
        let result = original_status_calc_damage(OriginalStatusDamageInputs {
            hunter_damage: 1_i64 << 62,
            weapon_speed: 1.0,
            damage_potion_value: 1.0,
            ..OriginalStatusDamageInputs::default()
        })
        .unwrap();

        assert_eq!(result.damage_before_potion, 1_i64 << 62);
        assert_eq!(result.calc_damage, 0);
    }
}
