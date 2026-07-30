use super::{
    DurableHunterRuntimeAppearance, DurableHunterRuntimeConsumable, DurableHunterRuntimeGear,
    DurableHunterRuntimeGrowth, DurableHunterRuntimeInventory, DurableHunterRuntimeItem,
    DurableHunterRuntimeRidingPet, DurableHunterRuntimeSkill, DurableHunterRuntimeState,
    DurableHunterRuntimeStatus, HashMap, PgRow, Postgres, RepositoryError, Row, Transaction, Uuid,
};

pub(super) fn runtime_status_from_row(
    row: &PgRow,
) -> Result<Option<DurableHunterRuntimeStatus>, RepositoryError> {
    let values = (
        row.try_get::<Option<i64>, _>("source_hp")?,
        row.try_get::<Option<i64>, _>("source_now_hp")?,
        row.try_get::<Option<f32>, _>("source_feel")?,
        row.try_get::<Option<f32>, _>("source_now_feel")?,
        row.try_get::<Option<f32>, _>("source_hungry")?,
        row.try_get::<Option<f32>, _>("source_now_hungry")?,
        row.try_get::<Option<f32>, _>("source_tire")?,
        row.try_get::<Option<f32>, _>("source_now_tire")?,
        row.try_get::<Option<i64>, _>("source_damage")?,
        row.try_get::<Option<i64>, _>("source_armor")?,
        row.try_get::<Option<i32>, _>("source_critical")?,
        row.try_get::<Option<f32>, _>("source_attack_speed")?,
        row.try_get::<Option<i32>, _>("source_dodge")?,
    );
    match values {
        (None, None, None, None, None, None, None, None, None, None, None, None, None) => Ok(None),
        (
            Some(hp),
            Some(now_hp),
            Some(feel),
            Some(now_feel),
            Some(hungry),
            Some(now_hungry),
            Some(tire),
            Some(now_tire),
            Some(damage),
            Some(armor),
            Some(critical),
            Some(attack_speed),
            Some(dodge),
        ) => Ok(Some(DurableHunterRuntimeStatus {
            hp,
            now_hp,
            feel,
            now_feel,
            hungry,
            now_hungry,
            tire,
            now_tire,
            damage,
            armor,
            critical,
            attack_speed,
            dodge,
        })),
        _ => Err(RepositoryError::InvalidOperation),
    }
}

pub(super) async fn load_hunter_runtime_in(
    transaction: &mut Transaction<'_, Postgres>,
    player_token: Uuid,
) -> Result<HashMap<u32, DurableHunterRuntimeState>, RepositoryError> {
    let mut runtime = HashMap::<u32, DurableHunterRuntimeState>::new();
    let section_rows = sqlx::query(
        "SELECT hunter_id, section, value_captured FROM player_hunter_runtime_section WHERE player_token = $1",
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in section_rows {
        if !row.try_get::<bool, _>("value_captured")? {
            continue;
        }
        let hunter_id = runtime_hunter_id(&row)?;
        let state = runtime.entry(hunter_id).or_default();
        match row.try_get::<String, _>("section")?.as_str() {
            "skills" => state.skills = Some(Vec::new()),
            "inventory" => state.inventory = Some(DurableHunterRuntimeInventory::default()),
            "growth" => state.growth = Some(Vec::new()),
            "riding_pet" | "status" => {}
            _ => return Err(RepositoryError::InvalidOperation),
        }
    }

    let appearance_rows = sqlx::query(
        r#"SELECT hunter_id, body_index, costume_index, costume_hidden, fairy_index,
                  fairy_hidden, weapon_costume_index, weapon_costume_hidden,
                  wing_costume_index, wing_costume_hidden, seal_costume_index,
                  seal_costume_hidden, ramble_pet_index, ramble_pet_hidden,
                  hat_hidden, costume_hat_hidden
           FROM player_hunter_runtime_appearance WHERE player_token = $1"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in appearance_rows {
        let hunter_id = runtime_hunter_id(&row)?;
        runtime.entry(hunter_id).or_default().appearance = Some(DurableHunterRuntimeAppearance {
            body_index: row.try_get("body_index")?,
            costume_index: row.try_get("costume_index")?,
            costume_hidden: row.try_get("costume_hidden")?,
            fairy_index: row.try_get("fairy_index")?,
            fairy_hidden: row.try_get("fairy_hidden")?,
            weapon_costume_index: row.try_get("weapon_costume_index")?,
            weapon_costume_hidden: row.try_get("weapon_costume_hidden")?,
            wing_costume_index: row.try_get("wing_costume_index")?,
            wing_costume_hidden: row.try_get("wing_costume_hidden")?,
            seal_costume_index: row.try_get("seal_costume_index")?,
            seal_costume_hidden: row.try_get("seal_costume_hidden")?,
            ramble_pet_index: row.try_get("ramble_pet_index")?,
            ramble_pet_hidden: row.try_get("ramble_pet_hidden")?,
            hat_hidden: row.try_get("hat_hidden")?,
            costume_hat_hidden: row.try_get("costume_hat_hidden")?,
        });
    }

    let skill_rows = sqlx::query(
        r#"SELECT hunter_id, dictionary_key, source_index, skill_index, cool_time, skill_level
           FROM player_hunter_runtime_skill WHERE player_token = $1
           ORDER BY hunter_id, dictionary_key"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in skill_rows {
        let hunter_id = runtime_hunter_id(&row)?;
        runtime
            .entry(hunter_id)
            .or_default()
            .skills
            .get_or_insert_with(Vec::new)
            .push(DurableHunterRuntimeSkill {
                dictionary_key: row.try_get("dictionary_key")?,
                source_index: row.try_get("source_index")?,
                skill_index: row.try_get("skill_index")?,
                cool_time: row.try_get("cool_time")?,
                level: row.try_get("skill_level")?,
            });
    }

    let item_rows = sqlx::query(
        r#"SELECT hunter_id, dictionary_key, new_check, source_index, item_count, reservation, infinity_check
           FROM player_hunter_runtime_item WHERE player_token = $1
           ORDER BY hunter_id, dictionary_key"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in item_rows {
        let hunter_id = runtime_hunter_id(&row)?;
        runtime_inventory(&mut runtime, hunter_id)
            .items
            .push(DurableHunterRuntimeItem {
                dictionary_key: row.try_get("dictionary_key")?,
                new_check: row.try_get("new_check")?,
                source_index: row.try_get("source_index")?,
                count: row.try_get("item_count")?,
                reservation: row.try_get("reservation")?,
                infinity_check: row.try_get("infinity_check")?,
            });
    }

    let gear_rows = sqlx::query(
        r#"SELECT hunter_id, dictionary_key, source_index, gear_index, inventory_index,
                  quality, new_check, gear_level, rating, gear_group, plus_type, plus_value,
                  minus_type, minus_value, additional_plus_type, additional_plus_value,
                  additional_minus_type, additional_minus_value, buy_gold, buy_date,
                  buy_date_value, quality_count, option_count, lock_count, potential,
                  runes_index, runes_value, skill_runes_index, skill_runes_value,
                  delete_count, unidentified_option_count
           FROM player_hunter_runtime_gear WHERE player_token = $1
           ORDER BY hunter_id, dictionary_key"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in gear_rows {
        let hunter_id = runtime_hunter_id(&row)?;
        runtime_inventory(&mut runtime, hunter_id)
            .gear
            .push(DurableHunterRuntimeGear {
                dictionary_key: row.try_get("dictionary_key")?,
                source_index: row.try_get("source_index")?,
                gear_index: row.try_get("gear_index")?,
                inventory_index: row.try_get("inventory_index")?,
                quality: row.try_get("quality")?,
                new_check: row.try_get("new_check")?,
                level: row.try_get("gear_level")?,
                rating: row.try_get("rating")?,
                group: row.try_get("gear_group")?,
                plus_type: row.try_get("plus_type")?,
                plus_value: row.try_get("plus_value")?,
                minus_type: row.try_get("minus_type")?,
                minus_value: row.try_get("minus_value")?,
                additional_plus_type: row.try_get("additional_plus_type")?,
                additional_plus_value: row.try_get("additional_plus_value")?,
                additional_minus_type: row.try_get("additional_minus_type")?,
                additional_minus_value: row.try_get("additional_minus_value")?,
                buy_gold: row.try_get("buy_gold")?,
                buy_date: row.try_get("buy_date")?,
                buy_date_value: row.try_get("buy_date_value")?,
                quality_count: row.try_get("quality_count")?,
                option_count: row.try_get("option_count")?,
                lock_count: row.try_get("lock_count")?,
                potential: row.try_get("potential")?,
                runes_index: row.try_get("runes_index")?,
                runes_value: row.try_get("runes_value")?,
                skill_runes_index: row.try_get("skill_runes_index")?,
                skill_runes_value: row.try_get("skill_runes_value")?,
                delete_count: row.try_get("delete_count")?,
                unidentified_option_count: row.try_get("unidentified_option_count")?,
            });
    }

    let consumable_rows = sqlx::query(
        r#"SELECT hunter_id, dictionary_key, total_count
           FROM player_hunter_runtime_consumable WHERE player_token = $1
           ORDER BY hunter_id, dictionary_key"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in consumable_rows {
        let hunter_id = runtime_hunter_id(&row)?;
        runtime_inventory(&mut runtime, hunter_id).consumables.push(
            DurableHunterRuntimeConsumable {
                dictionary_key: row.try_get("dictionary_key")?,
                total_count: row.try_get("total_count")?,
            },
        );
    }

    let growth_rows = sqlx::query(
        r#"SELECT hunter_id, source_order, property_level
           FROM player_hunter_runtime_growth WHERE player_token = $1
           ORDER BY hunter_id, source_order"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in growth_rows {
        let hunter_id = runtime_hunter_id(&row)?;
        runtime
            .entry(hunter_id)
            .or_default()
            .growth
            .get_or_insert_with(Vec::new)
            .push(DurableHunterRuntimeGrowth {
                source_order: row.try_get("source_order")?,
                property_level: row.try_get("property_level")?,
            });
    }

    let pet_rows = sqlx::query(
        r#"SELECT hunter_id, pasture_index, source_index, master_index, rating, skill_index,
                  trait_index, trait_level, use_soul, use_growth_stone, locked
           FROM player_hunter_runtime_riding_pet WHERE player_token = $1"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in pet_rows {
        let hunter_id = runtime_hunter_id(&row)?;
        runtime.entry(hunter_id).or_default().riding_pet = Some(DurableHunterRuntimeRidingPet {
            pasture_index: row.try_get("pasture_index")?,
            source_index: row.try_get("source_index")?,
            master_index: row.try_get("master_index")?,
            rating: row.try_get("rating")?,
            skill_index: row.try_get("skill_index")?,
            trait_index: row.try_get("trait_index")?,
            trait_level: row.try_get("trait_level")?,
            use_soul: row.try_get("use_soul")?,
            use_growth_stone: row.try_get("use_growth_stone")?,
            locked: row.try_get("locked")?,
        });
    }
    Ok(runtime)
}

pub(super) fn runtime_hunter_id(row: &PgRow) -> Result<u32, RepositoryError> {
    u32::try_from(row.try_get::<i64, _>("hunter_id")?)
        .map_err(|_| RepositoryError::InvalidOperation)
}

pub(super) fn runtime_inventory(
    runtime: &mut HashMap<u32, DurableHunterRuntimeState>,
    hunter_id: u32,
) -> &mut DurableHunterRuntimeInventory {
    runtime
        .entry(hunter_id)
        .or_default()
        .inventory
        .get_or_insert_with(DurableHunterRuntimeInventory::default)
}
