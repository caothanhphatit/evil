use super::{
    db_i64, fixture_equipment_slot_order, nonempty_or, save_hunter_owned_items_in,
    save_hunter_runtime_in, DurableHunterProfile, DurableHunterRosterState, DurableHunterState,
    Postgres, RepositoryError, Transaction, Uuid, MIGRATION_HUNTER_RELEASE_ID,
};

pub(super) async fn save_hunter_roster_in(
    transaction: &mut Transaction<'_, Postgres>,
    player_token: Uuid,
    roster: &DurableHunterRosterState,
) -> Result<(), RepositoryError> {
    roster
        .validate()
        .map_err(|_| RepositoryError::InvalidOperation)?;
    let next_arrival_sequence = i64::try_from(roster.next_arrival_sequence.max(1))
        .map_err(|_| RepositoryError::InvalidOperation)?;
    sqlx::query(
        r#"
        INSERT INTO player_hunter_roster
            (player_token, roster_resolved, wallets_resolved, next_arrival_sequence, updated_at)
        VALUES ($1, $2, $3, $4, now())
        ON CONFLICT (player_token) DO UPDATE
        SET roster_resolved = EXCLUDED.roster_resolved,
            wallets_resolved = EXCLUDED.wallets_resolved,
            next_arrival_sequence = EXCLUDED.next_arrival_sequence,
            updated_at = now()
        "#,
    )
    .bind(player_token)
    .bind(roster.roster_resolved)
    .bind(roster.wallets_resolved)
    .bind(next_arrival_sequence)
    .execute(&mut **transaction)
    .await?;
    let retained_hunter_ids = roster
        .hunters
        .iter()
        .chain(roster.waiting_queue.iter().map(|waiting| &waiting.hunter))
        .map(|hunter| i64::from(hunter.hunter_id))
        .collect::<Vec<_>>();
    sqlx::query("DELETE FROM player_hunter WHERE player_token = $1 AND NOT (hunter_id = ANY($2))")
        .bind(player_token)
        .bind(&retained_hunter_ids)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM player_hunter_roster_command WHERE player_token = $1")
        .bind(player_token)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM player_hunter_action_command WHERE player_token = $1")
        .bind(player_token)
        .execute(&mut **transaction)
        .await?;

    for (position, hunter) in roster.hunters.iter().enumerate() {
        insert_hunter_row(transaction, player_token, hunter, "active", position, None).await?;
    }
    for (position, waiting) in roster.waiting_queue.iter().enumerate() {
        insert_hunter_row(
            transaction,
            player_token,
            &waiting.hunter,
            "waiting",
            position,
            Some(waiting.arrival_sequence),
        )
        .await?;
    }
    for (command_id, result) in &roster.banish_commands {
        sqlx::query(
            r#"
            INSERT INTO player_hunter_roster_command
                (player_token, command_id, banished_hunter_id, promoted_hunter_id)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(player_token)
        .bind(command_id)
        .bind(i64::from(result.banished_hunter_id))
        .bind(result.promoted_hunter_id.map(i64::from))
        .execute(&mut **transaction)
        .await?;
    }
    for (command_id, command_key) in &roster.hunt_commands {
        sqlx::query("INSERT INTO player_hunter_action_command (player_token, command_id, command_key) VALUES ($1, $2, $3)")
            .bind(player_token)
            .bind(command_id)
            .bind(command_key)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(())
}

pub(super) async fn insert_hunter_row(
    transaction: &mut Transaction<'_, Postgres>,
    player_token: Uuid,
    hunter: &DurableHunterState,
    roster_state: &str,
    position: usize,
    arrival_sequence: Option<u64>,
) -> Result<(), RepositoryError> {
    let fallback_profile = DurableHunterProfile::migration_default(hunter.hunter_id);
    let profile = &hunter.profile;
    let content_release_id = nonempty_or(&profile.content_release_id, MIGRATION_HUNTER_RELEASE_ID);
    let display_name = nonempty_or(&profile.display_name, &fallback_profile.display_name);
    let class_id = nonempty_or(&profile.class_id, "h1");
    let rarity_id = nonempty_or(&profile.rarity_id, "normal");
    let action_state = nonempty_or(&profile.action_state, "idle");
    let animation_name = nonempty_or(&profile.animation_name, "hunter_stay");
    sqlx::query(
        r#"
        INSERT INTO player_hunter
            (player_token, hunter_id, roster_state, roster_position, arrival_sequence,
             gold, current_hp, max_hp, stamina_current, stamina_maximum,
             satiety_current, satiety_maximum, mood_current, mood_maximum,
             content_release_id, display_name, portrait_asset_id, class_id, rarity_id,
             level, xp, xp_to_next_level, attack, defense, dps_milli,
             critical_rate_bps, attack_speed_milli, evasion_rate_bps,
             awakening_current, awakening_maximum, reincarnation_current,
             reincarnation_maximum, is_locked, riding_pet_state_resolved,
             action_state, animation_name, hunt_state, owned_items)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26,
                $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38)
        ON CONFLICT (player_token, hunter_id) DO UPDATE
        SET roster_state = EXCLUDED.roster_state,
            roster_position = EXCLUDED.roster_position,
            arrival_sequence = EXCLUDED.arrival_sequence,
            gold = EXCLUDED.gold,
            current_hp = EXCLUDED.current_hp,
            max_hp = EXCLUDED.max_hp,
            stamina_current = EXCLUDED.stamina_current,
            stamina_maximum = EXCLUDED.stamina_maximum,
            satiety_current = EXCLUDED.satiety_current,
            satiety_maximum = EXCLUDED.satiety_maximum,
            mood_current = EXCLUDED.mood_current,
            mood_maximum = EXCLUDED.mood_maximum,
            content_release_id = EXCLUDED.content_release_id,
            display_name = EXCLUDED.display_name,
            portrait_asset_id = EXCLUDED.portrait_asset_id,
            class_id = EXCLUDED.class_id,
            rarity_id = EXCLUDED.rarity_id,
            level = EXCLUDED.level,
            xp = EXCLUDED.xp,
            xp_to_next_level = EXCLUDED.xp_to_next_level,
            attack = EXCLUDED.attack,
            defense = EXCLUDED.defense,
            dps_milli = EXCLUDED.dps_milli,
            critical_rate_bps = EXCLUDED.critical_rate_bps,
            attack_speed_milli = EXCLUDED.attack_speed_milli,
            evasion_rate_bps = EXCLUDED.evasion_rate_bps,
            awakening_current = EXCLUDED.awakening_current,
            awakening_maximum = EXCLUDED.awakening_maximum,
            reincarnation_current = EXCLUDED.reincarnation_current,
            reincarnation_maximum = EXCLUDED.reincarnation_maximum,
            is_locked = EXCLUDED.is_locked,
            riding_pet_state_resolved = EXCLUDED.riding_pet_state_resolved,
            action_state = EXCLUDED.action_state,
            animation_name = EXCLUDED.animation_name,
            hunt_state = EXCLUDED.hunt_state,
            owned_items = EXCLUDED.owned_items,
            state_revision = player_hunter.state_revision + 1
        "#,
    )
    .bind(player_token)
    .bind(i64::from(hunter.hunter_id))
    .bind(roster_state)
    .bind(i32::try_from(position).map_err(|_| RepositoryError::InvalidOperation)?)
    .bind(
        arrival_sequence
            .map(i64::try_from)
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(db_i64(hunter.gold)?)
    .bind(db_i64(hunter.current_hp)?)
    .bind(db_i64(hunter.max_hp)?)
    .bind(db_i64(hunter.stamina.current)?)
    .bind(db_i64(hunter.stamina.maximum)?)
    .bind(db_i64(hunter.satiety.current)?)
    .bind(db_i64(hunter.satiety.maximum)?)
    .bind(db_i64(hunter.mood.current)?)
    .bind(db_i64(hunter.mood.maximum)?)
    .bind(content_release_id)
    .bind(display_name)
    .bind(&profile.portrait_asset_id)
    .bind(class_id)
    .bind(rarity_id)
    .bind(i32::try_from(profile.level.max(1)).map_err(|_| RepositoryError::InvalidOperation)?)
    .bind(db_i64(profile.xp)?)
    .bind(profile.xp_to_next_level.map(db_i64).transpose()?)
    .bind(db_i64(profile.attack)?)
    .bind(db_i64(profile.defense)?)
    .bind(profile.dps_milli.map(db_i64).transpose()?)
    .bind(
        profile
            .critical_rate_bps
            .map(i32::try_from)
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(
        profile
            .attack_speed_milli
            .map(i32::try_from)
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(
        profile
            .evasion_rate_bps
            .map(i32::try_from)
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(
        profile
            .awakening
            .map(|value| i32::try_from(value.current))
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(
        profile
            .awakening
            .map(|value| i32::try_from(value.maximum))
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(
        profile
            .reincarnation
            .map(|value| i32::try_from(value.current))
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(
        profile
            .reincarnation
            .map(|value| i32::try_from(value.maximum))
            .transpose()
            .map_err(|_| RepositoryError::InvalidOperation)?,
    )
    .bind(profile.is_locked)
    .bind(profile.riding_pet_state_resolved)
    .bind(action_state)
    .bind(animation_name)
    .bind(serde_json::to_value(&hunter.hunt)?)
    // The normalized inventory tables are authoritative after migration
    // 0034. Keep the legacy column empty so old fallback reads cannot
    // resurrect stale ownership rows.
    .bind(serde_json::json!([]))
    .execute(&mut **transaction)
    .await?;
    sqlx::query("DELETE FROM player_hunter_trait WHERE player_token = $1 AND hunter_id = $2")
        .bind(player_token)
        .bind(i64::from(hunter.hunter_id))
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM player_hunter_skill WHERE player_token = $1 AND hunter_id = $2")
        .bind(player_token)
        .bind(i64::from(hunter.hunter_id))
        .execute(&mut **transaction)
        .await?;
    sqlx::query(
        "DELETE FROM player_hunter_fixture_equipment WHERE player_token = $1 AND hunter_id = $2",
    )
    .bind(player_token)
    .bind(i64::from(hunter.hunter_id))
    .execute(&mut **transaction)
    .await?;
    for hunter_trait in &profile.traits {
        sqlx::query(
            r#"
            INSERT INTO player_hunter_trait
                (player_token, hunter_id, content_release_id, trait_id, unlocked_rank, equipped)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(player_token)
        .bind(i64::from(hunter.hunter_id))
        .bind(content_release_id)
        .bind(&hunter_trait.trait_id)
        .bind(i16::from(hunter_trait.unlocked_rank.max(1)))
        .bind(hunter_trait.equipped)
        .execute(&mut **transaction)
        .await?;
    }
    for skill in &profile.skills {
        sqlx::query(
            r#"
            INSERT INTO player_hunter_skill
                (player_token, hunter_id, content_release_id, skill_id, skill_level, equipped_slot,
                 cooldown_ready_at)
            VALUES ($1, $2, $3, $4, $5, $6,
                    CASE WHEN $7 THEN NULL
                         ELSE now() + ($8::bigint * interval '1 millisecond')
                    END)
            "#,
        )
        .bind(player_token)
        .bind(i64::from(hunter.hunter_id))
        .bind(content_release_id)
        .bind(&skill.skill_id)
        .bind(i16::from(skill.skill_level.max(1)))
        .bind(skill.equipped_slot.map(i16::from))
        .bind(skill.ready)
        .bind(
            i64::try_from(skill.cooldown_remaining_ms)
                .map_err(|_| RepositoryError::InvalidOperation)?,
        )
        .execute(&mut **transaction)
        .await?;
    }
    for equipment in &profile.equipment_slots {
        sqlx::query(
            r#"INSERT INTO player_hunter_fixture_equipment
               (player_token, hunter_id, slot_id, slot_order, catalog_kind, catalog_index,
                display_name, icon_path, presentation_gender, required_class_id, locked,
                evidence_state)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)"#,
        )
        .bind(player_token)
        .bind(i64::from(hunter.hunter_id))
        .bind(&equipment.slot_id)
        .bind(fixture_equipment_slot_order(&equipment.slot_id)?)
        .bind(&equipment.catalog_kind)
        .bind(
            i32::try_from(equipment.catalog_index)
                .map_err(|_| RepositoryError::InvalidOperation)?,
        )
        .bind(&equipment.display_name)
        .bind(&equipment.icon_path)
        .bind(&equipment.presentation_gender)
        .bind(&equipment.required_class_id)
        .bind(equipment.locked)
        .bind(&equipment.evidence_state)
        .execute(&mut **transaction)
        .await?;
    }
    save_hunter_runtime_in(transaction, player_token, hunter).await?;
    save_hunter_owned_items_in(transaction, player_token, hunter).await?;
    Ok(())
}
