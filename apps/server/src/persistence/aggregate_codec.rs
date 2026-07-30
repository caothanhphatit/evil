use super::{
    db_u64, load_hunter_runtime_in, optional_db_u32, optional_db_u64, optional_progress,
    runtime_status_from_row, BTreeMap, DurableHunterEquipmentSlot, DurableHunterOwnedItem,
    DurableHunterProfile, DurableHunterRosterState, DurableHunterSkill, DurableHunterState,
    DurableHunterTrait, DurablePlayerAggregate, DurableWaitingHunter, HashMap, HashSet,
    HunterBanishment, HunterServiceGauge, OriginalFlowPlayerState, Postgres, RepositoryError, Row,
    Transaction, Uuid, DURABLE_PLAYER_SCHEMA_VERSION,
};

pub(super) fn encode_non_building_state(
    state: &DurablePlayerAggregate,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut value = serde_json::to_value(state)?;
    if let Some(object) = value.as_object_mut() {
        object.remove("buildings");
        object.remove("hunter_roster");
    }
    Ok(value)
}

pub(super) async fn load_hunter_roster_in(
    transaction: &mut Transaction<'_, Postgres>,
    player_token: Uuid,
) -> Result<Option<DurableHunterRosterState>, RepositoryError> {
    let metadata = sqlx::query(
        "SELECT roster_resolved, wallets_resolved, next_arrival_sequence FROM player_hunter_roster WHERE player_token = $1",
    )
    .bind(player_token)
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(metadata) = metadata else {
        return Ok(None);
    };
    let rows = sqlx::query(
        r#"
        SELECT ph.hunter_id, ph.roster_state, ph.roster_position, ph.arrival_sequence, ph.gold,
               current_hp, max_hp, stamina_current, stamina_maximum,
               satiety_current, satiety_maximum, mood_current, mood_maximum,
               ph.content_release_id, ph.display_name, ph.portrait_asset_id,
               ph.class_id, hc.display_name AS class_name, hc.visual_family,
               ph.rarity_id, hr.display_name AS rarity_name, ph.level, ph.xp,
               ph.xp_to_next_level, ph.attack, ph.defense, ph.dps_milli,
               ph.critical_rate_bps, ph.attack_speed_milli, ph.evasion_rate_bps,
               ph.awakening_current, ph.awakening_maximum,
               ph.reincarnation_current, ph.reincarnation_maximum, ph.is_locked,
               ph.riding_pet_state_resolved,
               hcd.display_name AS characteristic_name,
               ph.action_state, ph.animation_name, ph.hunt_state, ph.owned_items,
               ph.source_dictionary_key, ph.source_index, ph.source_job, ph.source_sub_job,
               ph.source_third_job, ph.source_fourth_job, ph.source_personality,
               ph.source_grade_rank_up, ph.source_dark_soul, ph.source_used_dark_soul,
               ph.source_used_job_trait,
               ph.source_hp, ph.source_now_hp, ph.source_feel, ph.source_now_feel,
               ph.source_hungry, ph.source_now_hungry, ph.source_tire, ph.source_now_tire,
               ph.source_damage, ph.source_armor, ph.source_critical,
               ph.source_attack_speed, ph.source_dodge
        FROM player_hunter ph
        JOIN hunter_class_definition hc
          ON hc.release_id = ph.content_release_id AND hc.class_id = ph.class_id
        JOIN hunter_rarity_definition hr
          ON hr.release_id = ph.content_release_id AND hr.rarity_id = ph.rarity_id
        LEFT JOIN hunter_characteristic_definition hcd
          ON hcd.release_id = ph.characteristic_release_id
         AND hcd.characteristic_id = ph.characteristic_id
        WHERE ph.player_token = $1
        ORDER BY CASE roster_state WHEN 'active' THEN 0 ELSE 1 END,
                 roster_position
        "#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    let mut roster = DurableHunterRosterState {
        roster_resolved: metadata.try_get("roster_resolved")?,
        wallets_resolved: metadata.try_get("wallets_resolved")?,
        next_arrival_sequence: u64::try_from(metadata.try_get::<i64, _>("next_arrival_sequence")?)
            .map_err(|_| RepositoryError::InvalidOperation)?,
        ..DurableHunterRosterState::default()
    };
    let trait_rows = sqlx::query(
        r#"
        SELECT pht.hunter_id, pht.trait_id, htd.display_name, htd.icon_path,
               pht.unlocked_rank, pht.equipped
        FROM player_hunter_trait pht
        JOIN hunter_trait_definition htd
          ON htd.release_id = pht.content_release_id AND htd.trait_id = pht.trait_id
        WHERE pht.player_token = $1
        ORDER BY pht.hunter_id, pht.equipped DESC, pht.trait_id
        "#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    let mut traits_by_hunter: HashMap<u32, Vec<DurableHunterTrait>> = HashMap::new();
    for row in trait_rows {
        let hunter_id = u32::try_from(row.try_get::<i64, _>("hunter_id")?)
            .map_err(|_| RepositoryError::InvalidOperation)?;
        traits_by_hunter
            .entry(hunter_id)
            .or_default()
            .push(DurableHunterTrait {
                trait_id: row.try_get("trait_id")?,
                display_name: row.try_get("display_name")?,
                icon_path: row.try_get("icon_path")?,
                unlocked_rank: u8::try_from(row.try_get::<i16, _>("unlocked_rank")?)
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                equipped: row.try_get("equipped")?,
            });
    }
    let skill_rows = sqlx::query(
        r#"
        SELECT phs.hunter_id, phs.skill_id, hsd.display_name, hsd.icon_path,
               hsd.animation_name, phs.skill_level, phs.equipped_slot,
               (phs.cooldown_ready_at IS NULL OR phs.cooldown_ready_at <= now()) AS ready,
               GREATEST(
                   0,
                   (EXTRACT(EPOCH FROM (phs.cooldown_ready_at - now())) * 1000)::bigint
               ) AS cooldown_remaining_ms
        FROM player_hunter_skill phs
        JOIN hunter_skill_definition hsd
          ON hsd.release_id = phs.content_release_id AND hsd.skill_id = phs.skill_id
        WHERE phs.player_token = $1
        ORDER BY phs.hunter_id, phs.equipped_slot NULLS LAST, phs.skill_id
        "#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    let mut skills_by_hunter: HashMap<u32, Vec<DurableHunterSkill>> = HashMap::new();
    for row in skill_rows {
        let hunter_id = u32::try_from(row.try_get::<i64, _>("hunter_id")?)
            .map_err(|_| RepositoryError::InvalidOperation)?;
        skills_by_hunter
            .entry(hunter_id)
            .or_default()
            .push(DurableHunterSkill {
                skill_id: row.try_get("skill_id")?,
                display_name: row.try_get("display_name")?,
                icon_path: row.try_get("icon_path")?,
                animation_name: row.try_get("animation_name")?,
                skill_level: u8::try_from(row.try_get::<i16, _>("skill_level")?)
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                equipped_slot: row
                    .try_get::<Option<i16>, _>("equipped_slot")?
                    .map(u8::try_from)
                    .transpose()
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                ready: row.try_get("ready")?,
                cooldown_remaining_ms: u64::try_from(
                    row.try_get::<i64, _>("cooldown_remaining_ms")?,
                )
                .map_err(|_| RepositoryError::InvalidOperation)?,
            });
    }
    let equipment_rows = sqlx::query(
        r#"SELECT hunter_id, slot_id, catalog_kind, catalog_index, display_name, icon_path,
                  presentation_gender, required_class_id, locked, evidence_state
           FROM player_hunter_fixture_equipment
           WHERE player_token = $1
           ORDER BY hunter_id, slot_order"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    let mut equipment_by_hunter: HashMap<u32, Vec<DurableHunterEquipmentSlot>> = HashMap::new();
    for row in equipment_rows {
        let hunter_id = u32::try_from(row.try_get::<i64, _>("hunter_id")?)
            .map_err(|_| RepositoryError::InvalidOperation)?;
        equipment_by_hunter
            .entry(hunter_id)
            .or_default()
            .push(DurableHunterEquipmentSlot {
                slot_id: row.try_get("slot_id")?,
                catalog_kind: row.try_get("catalog_kind")?,
                catalog_index: u32::try_from(row.try_get::<i32, _>("catalog_index")?)
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                display_name: row.try_get("display_name")?,
                icon_path: row.try_get("icon_path")?,
                presentation_gender: row.try_get("presentation_gender")?,
                required_class_id: row.try_get("required_class_id")?,
                locked: row.try_get("locked")?,
                evidence_state: row.try_get("evidence_state")?,
            });
    }
    let mut owned_items_by_hunter: HashMap<u32, Vec<DurableHunterOwnedItem>> = HashMap::new();
    let mut normalized_item_hunters = HashSet::new();
    let stack_rows = sqlx::query(
        r#"SELECT hunter_id, product_id, quantity
           FROM player_hunter_item_stack
           WHERE player_token = $1
           ORDER BY hunter_id, product_id"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in stack_rows {
        let hunter_id = u32::try_from(row.try_get::<i64, _>("hunter_id")?)
            .map_err(|_| RepositoryError::InvalidOperation)?;
        normalized_item_hunters.insert(hunter_id);
        owned_items_by_hunter
            .entry(hunter_id)
            .or_default()
            .push(DurableHunterOwnedItem {
                product_id: row.try_get::<String, _>("product_id")?,
                quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                enhancement_level: None,
                gear_instance_id: None,
            });
    }
    let gear_rows = sqlx::query(
        r#"SELECT hunter_id, gear_instance_id, product_id, enhancement_level
           FROM player_hunter_gear_instance
           WHERE player_token = $1
           ORDER BY hunter_id, created_at, gear_instance_id"#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    for row in gear_rows {
        let hunter_id = u32::try_from(row.try_get::<i64, _>("hunter_id")?)
            .map_err(|_| RepositoryError::InvalidOperation)?;
        normalized_item_hunters.insert(hunter_id);
        owned_items_by_hunter
            .entry(hunter_id)
            .or_default()
            .push(DurableHunterOwnedItem {
                product_id: row.try_get::<String, _>("product_id")?,
                quantity: 1,
                enhancement_level: Some(
                    u8::try_from(row.try_get::<i16, _>("enhancement_level")?)
                        .map_err(|_| RepositoryError::InvalidOperation)?,
                ),
                gear_instance_id: Some(row.try_get::<Uuid, _>("gear_instance_id")?),
            });
    }
    let mut runtime_by_hunter = load_hunter_runtime_in(transaction, player_token).await?;
    for row in rows {
        let hunter_id = u32::try_from(row.try_get::<i64, _>("hunter_id")?)
            .map_err(|_| RepositoryError::InvalidOperation)?;
        let mut runtime = runtime_by_hunter.remove(&hunter_id).unwrap_or_default();
        runtime.source_dictionary_key = row.try_get("source_dictionary_key")?;
        runtime.source_index = row.try_get("source_index")?;
        runtime.source_job = row.try_get("source_job")?;
        runtime.source_sub_job = row.try_get("source_sub_job")?;
        runtime.source_third_job = row.try_get("source_third_job")?;
        runtime.source_fourth_job = row.try_get("source_fourth_job")?;
        runtime.source_personality = row.try_get("source_personality")?;
        runtime.source_grade_rank_up = row.try_get("source_grade_rank_up")?;
        runtime.source_dark_soul = row.try_get("source_dark_soul")?;
        runtime.source_used_dark_soul = row.try_get("source_used_dark_soul")?;
        runtime.source_used_job_trait = row.try_get("source_used_job_trait")?;
        runtime.status = runtime_status_from_row(&row)?;
        let hunter = DurableHunterState {
            hunter_id,
            gold: db_u64(&row, "gold")?,
            current_hp: db_u64(&row, "current_hp")?,
            max_hp: db_u64(&row, "max_hp")?,
            stamina: HunterServiceGauge {
                current: db_u64(&row, "stamina_current")?,
                maximum: db_u64(&row, "stamina_maximum")?,
            },
            satiety: HunterServiceGauge {
                current: db_u64(&row, "satiety_current")?,
                maximum: db_u64(&row, "satiety_maximum")?,
            },
            mood: HunterServiceGauge {
                current: db_u64(&row, "mood_current")?,
                maximum: db_u64(&row, "mood_maximum")?,
            },
            hunt: serde_json::from_value(row.try_get("hunt_state")?)?,
            owned_items: if normalized_item_hunters.contains(&hunter_id) {
                owned_items_by_hunter.remove(&hunter_id).unwrap_or_default()
            } else {
                serde_json::from_value(row.try_get("owned_items")?)?
            },
            profile: DurableHunterProfile {
                content_release_id: row.try_get("content_release_id")?,
                display_name: row.try_get("display_name")?,
                portrait_asset_id: row.try_get("portrait_asset_id")?,
                class_id: row.try_get("class_id")?,
                class_name: row.try_get("class_name")?,
                visual_family: row.try_get("visual_family")?,
                rarity_id: row.try_get("rarity_id")?,
                rarity_name: row.try_get("rarity_name")?,
                level: u32::try_from(row.try_get::<i32, _>("level")?)
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                xp: db_u64(&row, "xp")?,
                xp_to_next_level: optional_db_u64(&row, "xp_to_next_level")?,
                attack: db_u64(&row, "attack")?,
                defense: db_u64(&row, "defense")?,
                dps_milli: optional_db_u64(&row, "dps_milli")?,
                critical_rate_bps: optional_db_u32(&row, "critical_rate_bps")?,
                attack_speed_milli: optional_db_u32(&row, "attack_speed_milli")?,
                evasion_rate_bps: optional_db_u32(&row, "evasion_rate_bps")?,
                awakening: optional_progress(&row, "awakening_current", "awakening_maximum")?,
                reincarnation: optional_progress(
                    &row,
                    "reincarnation_current",
                    "reincarnation_maximum",
                )?,
                is_locked: row.try_get("is_locked")?,
                characteristic_name: row.try_get("characteristic_name")?,
                riding_pet_state_resolved: row.try_get("riding_pet_state_resolved")?,
                equipment_slots: equipment_by_hunter.remove(&hunter_id).unwrap_or_default(),
                action_state: row.try_get("action_state")?,
                animation_name: row.try_get("animation_name")?,
                traits: traits_by_hunter.remove(&hunter_id).unwrap_or_default(),
                skills: skills_by_hunter.remove(&hunter_id).unwrap_or_default(),
            },
            runtime,
        };
        match row.try_get::<String, _>("roster_state")?.as_str() {
            "active" => roster.hunters.push(hunter),
            "waiting" => roster.waiting_queue.push(DurableWaitingHunter {
                arrival_sequence: u64::try_from(
                    row.try_get::<Option<i64>, _>("arrival_sequence")?
                        .ok_or(RepositoryError::InvalidOperation)?,
                )
                .map_err(|_| RepositoryError::InvalidOperation)?,
                hunter,
            }),
            _ => return Err(RepositoryError::InvalidOperation),
        }
    }
    let command_rows = sqlx::query(
        r#"
        SELECT command_id, banished_hunter_id, promoted_hunter_id
        FROM player_hunter_roster_command
        WHERE player_token = $1
        ORDER BY created_at, command_id
        "#,
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    roster.banish_commands = command_rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get("command_id")?,
                HunterBanishment {
                    banished_hunter_id: u32::try_from(row.try_get::<i64, _>("banished_hunter_id")?)
                        .map_err(|_| RepositoryError::InvalidOperation)?,
                    promoted_hunter_id: row
                        .try_get::<Option<i64>, _>("promoted_hunter_id")?
                        .map(u32::try_from)
                        .transpose()
                        .map_err(|_| RepositoryError::InvalidOperation)?,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, RepositoryError>>()?;
    let action_rows = sqlx::query(
        "SELECT command_id, command_key FROM player_hunter_action_command WHERE player_token = $1 ORDER BY created_at, command_id",
    )
    .bind(player_token)
    .fetch_all(&mut **transaction)
    .await?;
    roster.hunt_commands = action_rows
        .into_iter()
        .map(|row| Ok((row.try_get("command_id")?, row.try_get("command_key")?)))
        .collect::<Result<BTreeMap<_, _>, RepositoryError>>()?;
    roster
        .validate()
        .map_err(|_| RepositoryError::InvalidOperation)?;
    Ok(Some(roster))
}

pub(super) fn decode_player_state(
    value: serde_json::Value,
) -> Result<DurablePlayerAggregate, serde_json::Error> {
    if value.get("schema_version").is_some() {
        return serde_json::from_value(value);
    }
    let navigation: OriginalFlowPlayerState = serde_json::from_value(value)?;
    Ok(DurablePlayerAggregate {
        schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
        navigation,
        ..DurablePlayerAggregate::default()
    })
}
