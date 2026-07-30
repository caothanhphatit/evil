use super::{
    DurableHunterRuntimeInventory, DurableHunterState, Postgres, RepositoryError, Transaction, Uuid,
};

pub(super) async fn save_hunter_runtime_in(
    transaction: &mut Transaction<'_, Postgres>,
    player_token: Uuid,
    hunter: &DurableHunterState,
) -> Result<(), RepositoryError> {
    let hunter_id = i64::from(hunter.hunter_id);
    let runtime = &hunter.runtime;
    let status = runtime.status.as_ref();
    sqlx::query(
        r#"UPDATE player_hunter
           SET source_dictionary_key = $3, source_index = $4, source_job = $5,
               source_sub_job = $6, source_third_job = $7, source_fourth_job = $8,
               source_personality = $9, source_grade_rank_up = $10, source_dark_soul = $11,
               source_used_dark_soul = $12, source_used_job_trait = $13,
               source_hp = $14, source_now_hp = $15, source_feel = $16,
               source_now_feel = $17, source_hungry = $18, source_now_hungry = $19,
               source_tire = $20, source_now_tire = $21, source_damage = $22,
               source_armor = $23, source_critical = $24, source_attack_speed = $25,
               source_dodge = $26
           WHERE player_token = $1 AND hunter_id = $2"#,
    )
    .bind(player_token)
    .bind(hunter_id)
    .bind(&runtime.source_dictionary_key)
    .bind(runtime.source_index)
    .bind(runtime.source_job)
    .bind(runtime.source_sub_job)
    .bind(runtime.source_third_job)
    .bind(runtime.source_fourth_job)
    .bind(runtime.source_personality)
    .bind(runtime.source_grade_rank_up)
    .bind(runtime.source_dark_soul)
    .bind(runtime.source_used_dark_soul)
    .bind(runtime.source_used_job_trait)
    .bind(status.map(|value| value.hp))
    .bind(status.map(|value| value.now_hp))
    .bind(status.map(|value| value.feel))
    .bind(status.map(|value| value.now_feel))
    .bind(status.map(|value| value.hungry))
    .bind(status.map(|value| value.now_hungry))
    .bind(status.map(|value| value.tire))
    .bind(status.map(|value| value.now_tire))
    .bind(status.map(|value| value.damage))
    .bind(status.map(|value| value.armor))
    .bind(status.map(|value| value.critical))
    .bind(status.map(|value| value.attack_speed))
    .bind(status.map(|value| value.dodge))
    .execute(&mut **transaction)
    .await?;

    for statement in [
        "DELETE FROM player_hunter_runtime_section WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_runtime_appearance WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_runtime_skill WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_runtime_item WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_runtime_gear WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_runtime_consumable WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_runtime_growth WHERE player_token = $1 AND hunter_id = $2",
        "DELETE FROM player_hunter_runtime_riding_pet WHERE player_token = $1 AND hunter_id = $2",
    ] {
        sqlx::query(statement)
            .bind(player_token)
            .bind(hunter_id)
            .execute(&mut **transaction)
            .await?;
    }

    for (section, captured) in [
        ("status", runtime.status.is_some()),
        ("skills", runtime.skills.is_some()),
        ("inventory", runtime.inventory.is_some()),
        ("growth", runtime.growth.is_some()),
        ("riding_pet", runtime.riding_pet.is_some()),
    ] {
        if captured {
            sqlx::query(
                "INSERT INTO player_hunter_runtime_section (player_token, hunter_id, section, value_captured) VALUES ($1, $2, $3, TRUE)",
            )
            .bind(player_token)
            .bind(hunter_id)
            .bind(section)
            .execute(&mut **transaction)
            .await?;
        }
    }

    if let Some(appearance) = runtime.appearance.as_ref() {
        sqlx::query(
            r#"INSERT INTO player_hunter_runtime_appearance
               (player_token, hunter_id, body_index, costume_index, costume_hidden,
                fairy_index, fairy_hidden, weapon_costume_index, weapon_costume_hidden,
                wing_costume_index, wing_costume_hidden, seal_costume_index,
                seal_costume_hidden, ramble_pet_index, ramble_pet_hidden,
                hat_hidden, costume_hat_hidden)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)"#,
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(appearance.body_index)
        .bind(appearance.costume_index)
        .bind(appearance.costume_hidden)
        .bind(appearance.fairy_index)
        .bind(appearance.fairy_hidden)
        .bind(appearance.weapon_costume_index)
        .bind(appearance.weapon_costume_hidden)
        .bind(appearance.wing_costume_index)
        .bind(appearance.wing_costume_hidden)
        .bind(appearance.seal_costume_index)
        .bind(appearance.seal_costume_hidden)
        .bind(appearance.ramble_pet_index)
        .bind(appearance.ramble_pet_hidden)
        .bind(appearance.hat_hidden)
        .bind(appearance.costume_hat_hidden)
        .execute(&mut **transaction)
        .await?;
    }

    for skill in runtime.skills.iter().flatten() {
        sqlx::query(
            r#"INSERT INTO player_hunter_runtime_skill
               (player_token, hunter_id, dictionary_key, source_index, skill_index, cool_time, skill_level)
               VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(&skill.dictionary_key)
        .bind(skill.source_index)
        .bind(skill.skill_index)
        .bind(skill.cool_time)
        .bind(skill.level)
        .execute(&mut **transaction)
        .await?;
    }

    if let Some(inventory) = runtime.inventory.as_ref() {
        save_hunter_runtime_inventory(transaction, player_token, hunter_id, inventory).await?;
    }
    for growth in runtime.growth.iter().flatten() {
        sqlx::query(
            "INSERT INTO player_hunter_runtime_growth (player_token, hunter_id, source_order, property_level) VALUES ($1,$2,$3,$4)",
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(growth.source_order)
        .bind(growth.property_level)
        .execute(&mut **transaction)
        .await?;
    }
    if let Some(pet) = runtime.riding_pet.as_ref() {
        sqlx::query(
            r#"INSERT INTO player_hunter_runtime_riding_pet
               (player_token, hunter_id, pasture_index, source_index, master_index, rating,
                skill_index, trait_index, trait_level, use_soul, use_growth_stone, locked)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)"#,
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(pet.pasture_index)
        .bind(pet.source_index)
        .bind(&pet.master_index)
        .bind(pet.rating)
        .bind(pet.skill_index)
        .bind(pet.trait_index)
        .bind(pet.trait_level)
        .bind(pet.use_soul)
        .bind(pet.use_growth_stone)
        .bind(pet.locked)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

pub(super) async fn save_hunter_runtime_inventory(
    transaction: &mut Transaction<'_, Postgres>,
    player_token: Uuid,
    hunter_id: i64,
    inventory: &DurableHunterRuntimeInventory,
) -> Result<(), RepositoryError> {
    for item in &inventory.items {
        sqlx::query(
            r#"INSERT INTO player_hunter_runtime_item
               (player_token, hunter_id, dictionary_key, new_check, source_index,
                item_count, reservation, infinity_check)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"#,
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(&item.dictionary_key)
        .bind(item.new_check)
        .bind(item.source_index)
        .bind(item.count)
        .bind(item.reservation)
        .bind(item.infinity_check)
        .execute(&mut **transaction)
        .await?;
    }
    for gear in &inventory.gear {
        sqlx::query(
            r#"INSERT INTO player_hunter_runtime_gear
               (player_token, hunter_id, dictionary_key, source_index, gear_index,
                inventory_index, quality, new_check, gear_level, rating, gear_group,
                plus_type, plus_value, minus_type, minus_value, additional_plus_type,
                additional_plus_value, additional_minus_type, additional_minus_value,
                buy_gold, buy_date, buy_date_value, quality_count, option_count,
                lock_count, potential, runes_index, runes_value, skill_runes_index,
                skill_runes_value, delete_count, unidentified_option_count)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                       $17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32)"#,
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(&gear.dictionary_key)
        .bind(gear.source_index)
        .bind(gear.gear_index)
        .bind(gear.inventory_index)
        .bind(gear.quality)
        .bind(gear.new_check)
        .bind(gear.level)
        .bind(gear.rating)
        .bind(gear.group)
        .bind(&gear.plus_type)
        .bind(&gear.plus_value)
        .bind(&gear.minus_type)
        .bind(&gear.minus_value)
        .bind(&gear.additional_plus_type)
        .bind(&gear.additional_plus_value)
        .bind(&gear.additional_minus_type)
        .bind(&gear.additional_minus_value)
        .bind(gear.buy_gold)
        .bind(&gear.buy_date)
        .bind(gear.buy_date_value)
        .bind(gear.quality_count)
        .bind(gear.option_count)
        .bind(gear.lock_count)
        .bind(gear.potential)
        .bind(gear.runes_index)
        .bind(gear.runes_value)
        .bind(gear.skill_runes_index)
        .bind(gear.skill_runes_value)
        .bind(gear.delete_count)
        .bind(gear.unidentified_option_count)
        .execute(&mut **transaction)
        .await?;
    }
    for consumable in &inventory.consumables {
        sqlx::query(
            r#"INSERT INTO player_hunter_runtime_consumable
               (player_token, hunter_id, dictionary_key, total_count, nested_values_resolved)
               VALUES ($1,$2,$3,$4,FALSE)"#,
        )
        .bind(player_token)
        .bind(hunter_id)
        .bind(&consumable.dictionary_key)
        .bind(consumable.total_count)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}
