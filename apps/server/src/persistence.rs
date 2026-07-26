use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    buildings::{
        BaseBuildingId, BuildingRepository, BuildingRepositoryError, BuildingSkinId,
        PostgresBuildingRepository, TownBuildingInstance, TownBuildingInstanceId,
        TownBuildingState, TownMaterialStock, TownProductStock, TownTradeSettlement,
    },
    identity::SessionTokenHash,
    simulation::{
        operational_migration_roster, DurableBuilding, DurableBuildingState, DurableHunterProfile,
        DurableHunterProgress, DurableHunterRosterState, DurableHunterSkill, DurableHunterState,
        DurableHunterTrait, DurableMaterialStock, DurablePlayerAggregate, DurableProductStock,
        DurableTradeSettlement, DurableWaitingHunter, HunterBanishment, HunterServiceGauge,
        OriginalFlowPlayerState, PendingOperation, DURABLE_PLAYER_SCHEMA_VERSION,
        MIGRATION_HUNTER_RELEASE_ID,
    },
};

const ACTIVE_BUILDING_RELEASE_ID: &str = "evil-hunter-1.411.buildings-v1";

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("stored player state is invalid")]
    InvalidState(#[from] serde_json::Error),
    #[error("player state revision conflict")]
    RevisionConflict,
    #[error("durable operation is outside database bounds")]
    InvalidOperation,
    #[error("normalized building state is invalid")]
    Building(#[from] BuildingRepositoryError),
    #[error("world and town revisions diverged")]
    RevisionDivergence,
}

#[derive(Clone, Debug)]
pub struct LoadedPlayerState {
    pub state: DurablePlayerAggregate,
    pub revision: i64,
}

#[async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn resolve_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<Uuid>, RepositoryError>;

    async fn resolve_or_create_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Uuid, RepositoryError>;

    async fn load_or_create(
        &self,
        player_token: Uuid,
    ) -> Result<LoadedPlayerState, RepositoryError>;

    async fn persist(
        &self,
        player_token: Uuid,
        state: &DurablePlayerAggregate,
        expected_revision: i64,
        lease_fence: i64,
        operations: &[PendingOperation],
    ) -> Result<i64, RepositoryError>;

    async fn is_ready(&self) -> bool;
}

pub type SharedPlayerRepository = Arc<dyn PlayerRepository>;

pub struct InMemoryPlayerRepository {
    identities: RwLock<HashMap<SessionTokenHash, Uuid>>,
    durable: RwLock<MemoryDurableState>,
}

#[derive(Default)]
struct MemoryDurableState {
    states: HashMap<Uuid, (DurablePlayerAggregate, i64, i64)>,
    reward_operations: HashSet<(Uuid, Uuid)>,
    command_operations: HashSet<(Uuid, Uuid)>,
}

impl Default for InMemoryPlayerRepository {
    fn default() -> Self {
        Self {
            identities: RwLock::new(HashMap::new()),
            durable: RwLock::new(MemoryDurableState::default()),
        }
    }
}

#[async_trait]
impl PlayerRepository for InMemoryPlayerRepository {
    async fn resolve_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<Uuid>, RepositoryError> {
        Ok(self.identities.read().await.get(&token_hash).copied())
    }

    async fn resolve_or_create_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Uuid, RepositoryError> {
        let mut identities = self.identities.write().await;
        Ok(*identities.entry(token_hash).or_insert_with(Uuid::new_v4))
    }

    async fn load_or_create(
        &self,
        player_token: Uuid,
    ) -> Result<LoadedPlayerState, RepositoryError> {
        let mut durable = self.durable.write().await;
        let (state, revision, _) = durable.states.entry(player_token).or_default();
        Ok(LoadedPlayerState {
            state: state.clone(),
            revision: *revision,
        })
    }

    async fn persist(
        &self,
        player_token: Uuid,
        state: &DurablePlayerAggregate,
        expected_revision: i64,
        lease_fence: i64,
        operations: &[PendingOperation],
    ) -> Result<i64, RepositoryError> {
        let mut durable = self.durable.write().await;
        let entry = durable.states.entry(player_token).or_default();
        if entry.1 != expected_revision || entry.2 > lease_fence {
            return Err(RepositoryError::RevisionConflict);
        }
        entry.0 = state.clone();
        entry.1 += 1;
        entry.2 = lease_fence;
        let next_revision = entry.1;
        for operation in operations {
            match operation {
                PendingOperation::Reward { operation_id, .. } => {
                    durable
                        .reward_operations
                        .insert((player_token, *operation_id));
                }
                PendingOperation::Equip { command_id, .. } => {
                    durable
                        .command_operations
                        .insert((player_token, *command_id));
                }
            }
        }
        Ok(next_revision)
    }

    async fn is_ready(&self) -> bool {
        true
    }
}

pub struct PostgresPlayerRepository {
    pool: PgPool,
    buildings: PostgresBuildingRepository,
}

impl PostgresPlayerRepository {
    pub fn connect_lazy(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect_lazy(database_url)?;
        Ok(Self {
            buildings: PostgresBuildingRepository::from_pool(pool.clone()),
            pool,
        })
    }

    pub async fn validate_active_building_release(
        &self,
        release_id: &str,
        registry_sha256: &str,
    ) -> Result<(), BuildingRepositoryError> {
        self.buildings
            .load_catalog(release_id, registry_sha256)
            .await
            .map(|_| ())
    }
}

#[async_trait]
impl PlayerRepository for PostgresPlayerRepository {
    async fn resolve_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<Uuid>, RepositoryError> {
        Ok(sqlx::query_scalar::<_, Uuid>(
            "SELECT player_token FROM local_identities WHERE token_hash = $1",
        )
        .bind(token_hash.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn resolve_or_create_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Uuid, RepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let player = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO local_identities (token_hash, player_token)
            VALUES ($1, $2)
            ON CONFLICT (token_hash) DO UPDATE
            SET last_seen_at = now()
            RETURNING player_token
            "#,
        )
        .bind(token_hash.as_bytes().as_slice())
        .bind(Uuid::new_v4())
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(player)
    }

    async fn load_or_create(
        &self,
        player_token: Uuid,
    ) -> Result<LoadedPlayerState, RepositoryError> {
        let default_state = DurablePlayerAggregate::default();
        let default_json = encode_non_building_state(&default_state)?;
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            INSERT INTO player_world_state (player_token, state)
            VALUES ($1, $2)
            ON CONFLICT (player_token) DO UPDATE SET player_token = EXCLUDED.player_token
            RETURNING state, revision
            "#,
        )
        .bind(player_token)
        .bind(default_json)
        .fetch_one(&mut *transaction)
        .await?;
        let mut state = decode_player_state(row.try_get("state")?)?;
        let revision: i64 = row.try_get("revision")?;
        let town = match self
            .buildings
            .load_town_in(&mut transaction, player_token)
            .await?
        {
            Some(town) => town,
            None => {
                self.buildings
                    .create_town_from_default_template_in(
                        &mut transaction,
                        player_token,
                        ACTIVE_BUILDING_RELEASE_ID,
                        default_state.buildings.town_gold,
                    )
                    .await?
            }
        };
        if town.state.release_id != ACTIVE_BUILDING_RELEASE_ID {
            return Err(BuildingRepositoryError::RegistryMismatch {
                expected: ACTIVE_BUILDING_RELEASE_ID.to_owned(),
                actual: town.state.release_id,
            }
            .into());
        }
        if town.revision != revision {
            return Err(RepositoryError::RevisionDivergence);
        }
        let mut roster = load_hunter_roster_in(&mut transaction, player_token)
            .await?
            .unwrap_or_default();
        if !roster.roster_resolved && roster.hunters.is_empty() && roster.waiting_queue.is_empty() {
            roster = operational_migration_roster();
            save_hunter_roster_in(&mut transaction, player_token, &roster).await?;
        }
        state.hunter_roster = roster;
        state.buildings = durable_buildings_from_town(town.state)?;
        transaction.commit().await?;
        Ok(LoadedPlayerState { state, revision })
    }

    async fn persist(
        &self,
        player_token: Uuid,
        state: &DurablePlayerAggregate,
        expected_revision: i64,
        lease_fence: i64,
        operations: &[PendingOperation],
    ) -> Result<i64, RepositoryError> {
        let state_json = encode_non_building_state(state)?;
        let town_state = town_from_durable_buildings(&state.buildings)?;
        let mut transaction = self.pool.begin().await?;
        let revision = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO player_world_state (player_token, state, revision, lease_fence, updated_at)
            VALUES ($1, $2, 1, $4, now())
            ON CONFLICT (player_token) DO UPDATE
            SET state = EXCLUDED.state,
                revision = player_world_state.revision + 1,
                lease_fence = $4,
                updated_at = now()
            WHERE player_world_state.revision = $3
              AND player_world_state.lease_fence <= $4
            RETURNING revision
            "#,
        )
        .bind(player_token)
        .bind(state_json)
        .bind(expected_revision)
        .bind(lease_fence)
        .fetch_optional(&mut *transaction)
        .await?;
        let revision = revision.ok_or(RepositoryError::RevisionConflict)?;
        let town_revision = self
            .buildings
            .save_town_in(
                &mut transaction,
                player_token,
                &town_state,
                expected_revision,
            )
            .await?;
        if town_revision != revision {
            return Err(RepositoryError::RevisionDivergence);
        }
        save_hunter_roster_in(&mut transaction, player_token, &state.hunter_roster).await?;
        for operation in operations {
            match operation {
                PendingOperation::Reward {
                    operation_id,
                    gold,
                    item_id,
                    quantity,
                } => {
                    let gold =
                        i64::try_from(*gold).map_err(|_| RepositoryError::InvalidOperation)?;
                    sqlx::query(
                        r#"
                        INSERT INTO reward_ledger
                            (operation_id, player_token, reason, gold_delta, item_id, quantity)
                        VALUES ($1, $2, 'migration_fixture_drop', $3, $4, $5)
                        ON CONFLICT (player_token, operation_id) DO NOTHING
                        "#,
                    )
                    .bind(operation_id)
                    .bind(player_token)
                    .bind(gold)
                    .bind(i64::from(*item_id))
                    .bind(i64::from(*quantity))
                    .execute(&mut *transaction)
                    .await?;
                }
                PendingOperation::Equip {
                    command_id,
                    item_id,
                } => {
                    sqlx::query(
                        r#"
                        INSERT INTO command_ledger
                            (command_id, player_token, command_type, result)
                        VALUES ($1, $2, 'migration_fixture_equip', $3)
                        ON CONFLICT (player_token, command_id) DO NOTHING
                        "#,
                    )
                    .bind(command_id)
                    .bind(player_token)
                    .bind(serde_json::json!({
                        "accepted": true,
                        "item_id": item_id,
                        "fixture": true
                    }))
                    .execute(&mut *transaction)
                    .await?;
                }
            }
        }
        transaction.commit().await?;
        Ok(revision)
    }

    async fn is_ready(&self) -> bool {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }
}

fn encode_non_building_state(
    state: &DurablePlayerAggregate,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut value = serde_json::to_value(state)?;
    if let Some(object) = value.as_object_mut() {
        object.remove("buildings");
        object.remove("hunter_roster");
    }
    Ok(value)
}

async fn load_hunter_roster_in(
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
               ph.action_state, ph.animation_name
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
               (phs.cooldown_ready_at IS NULL OR phs.cooldown_ready_at <= now()) AS ready
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
            });
    }
    for row in rows {
        let hunter_id = u32::try_from(row.try_get::<i64, _>("hunter_id")?)
            .map_err(|_| RepositoryError::InvalidOperation)?;
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
                action_state: row.try_get("action_state")?,
                animation_name: row.try_get("animation_name")?,
                traits: traits_by_hunter.remove(&hunter_id).unwrap_or_default(),
                skills: skills_by_hunter.remove(&hunter_id).unwrap_or_default(),
            },
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
    roster
        .validate()
        .map_err(|_| RepositoryError::InvalidOperation)?;
    Ok(Some(roster))
}

async fn save_hunter_roster_in(
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
    Ok(())
}

async fn insert_hunter_row(
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
             action_state, animation_name)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26,
                $27, $28, $29, $30, $31, $32, $33, $34, $35, $36)
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
                    CASE WHEN $7 THEN NULL ELSE now() + interval '1 second' END)
            "#,
        )
        .bind(player_token)
        .bind(i64::from(hunter.hunter_id))
        .bind(content_release_id)
        .bind(&skill.skill_id)
        .bind(i16::from(skill.skill_level.max(1)))
        .bind(skill.equipped_slot.map(i16::from))
        .bind(skill.ready)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

fn nonempty_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

fn db_i64(value: u64) -> Result<i64, RepositoryError> {
    i64::try_from(value).map_err(|_| RepositoryError::InvalidOperation)
}

fn db_u64(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, RepositoryError> {
    u64::try_from(row.try_get::<i64, _>(column)?).map_err(|_| RepositoryError::InvalidOperation)
}

fn optional_db_u64(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<u64>, RepositoryError> {
    row.try_get::<Option<i64>, _>(column)?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| RepositoryError::InvalidOperation)
}

fn optional_db_u32(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<u32>, RepositoryError> {
    row.try_get::<Option<i32>, _>(column)?
        .map(u32::try_from)
        .transpose()
        .map_err(|_| RepositoryError::InvalidOperation)
}

fn optional_progress(
    row: &sqlx::postgres::PgRow,
    current_column: &str,
    maximum_column: &str,
) -> Result<Option<DurableHunterProgress>, RepositoryError> {
    match (
        optional_db_u32(row, current_column)?,
        optional_db_u32(row, maximum_column)?,
    ) {
        (None, None) => Ok(None),
        (Some(current), Some(maximum)) => Ok(Some(DurableHunterProgress { current, maximum })),
        _ => Err(RepositoryError::InvalidOperation),
    }
}

fn town_from_durable_buildings(
    state: &DurableBuildingState,
) -> Result<TownBuildingState, RepositoryError> {
    let buildings = state
        .buildings
        .iter()
        .map(|building| {
            Ok(TownBuildingInstance {
                instance_id: TownBuildingInstanceId::new(
                    Uuid::parse_str(&building.instance_id).map_err(|_| {
                        BuildingRepositoryError::InvalidTown("instance id must be UUID")
                    })?,
                ),
                building_id: BaseBuildingId::parse(building.id.clone())?,
                equipped_skin_id: building
                    .equipped_skin_id
                    .map(BuildingSkinId::new)
                    .transpose()?,
                level: u16::from(building.level),
                uses: building.uses,
                grid_x: building.grid_x,
                grid_y: building.grid_y,
                seeded_by: building.seeded_by.clone(),
            })
        })
        .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
    Ok(TownBuildingState {
        release_id: ACTIVE_BUILDING_RELEASE_ID.to_owned(),
        town_gold: state.town_gold,
        seed_version: state.town_seed_version,
        next_building_sequence: state.next_building_instance_id,
        buildings,
        hunter_materials: state.hunter_materials,
        materials: state.materials,
        runes: state.runes,
        weapons: state.weapons,
        armor: state.armor,
        hunter_equipment_purchases: state.hunter_equipment_purchases,
        field_trip_id: state.field_trip_id,
        settled_field_trip_id: state.settled_field_trip_id,
        material_stocks: state
            .material_stocks
            .iter()
            .map(|stock| TownMaterialStock {
                id: stock.id.clone(),
                town_quantity: stock.town_quantity,
                hunter_quantity: stock.hunter_quantity,
                requested: stock.requested,
                unit_price: stock.unit_price,
            })
            .collect(),
        product_stocks: state
            .product_stocks
            .iter()
            .map(|stock| {
                Ok(TownProductStock {
                    building_instance_id: TownBuildingInstanceId::new(
                        Uuid::parse_str(&stock.building_instance_id).map_err(|_| {
                            BuildingRepositoryError::InvalidTown(
                                "product stock building instance id must be UUID",
                            )
                        })?,
                    ),
                    product_id: stock.product_id.clone(),
                    quantity: stock.quantity,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?,
        trade_settlements: state
            .trade_settlements
            .iter()
            .map(|settlement| TownTradeSettlement {
                settlement_id: settlement.settlement_id.clone(),
                field_trip_id: settlement.field_trip_id,
                material_id: settlement.material_id.clone(),
                quantity: settlement.quantity,
                unit_price: settlement.unit_price,
                total_gold: settlement.total_gold,
            })
            .collect(),
    })
}

fn durable_buildings_from_town(
    state: TownBuildingState,
) -> Result<DurableBuildingState, RepositoryError> {
    let buildings = state
        .buildings
        .into_iter()
        .map(|building| {
            Ok(DurableBuilding {
                instance_id: building.instance_id.get().to_string(),
                id: building.building_id.to_string(),
                equipped_skin_id: building.equipped_skin_id.map(BuildingSkinId::get),
                level: u8::try_from(building.level)
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                uses: building.uses,
                grid_x: building.grid_x,
                grid_y: building.grid_y,
                seeded_by: building.seeded_by,
            })
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;
    Ok(DurableBuildingState {
        town_gold: state.town_gold,
        buildings,
        hunter_materials: state.hunter_materials,
        materials: state.materials,
        runes: state.runes,
        weapons: state.weapons,
        armor: state.armor,
        material_stocks: state
            .material_stocks
            .into_iter()
            .map(|stock| DurableMaterialStock {
                id: stock.id,
                town_quantity: stock.town_quantity,
                hunter_quantity: stock.hunter_quantity,
                requested: stock.requested,
                unit_price: stock.unit_price,
            })
            .collect(),
        product_stocks: state
            .product_stocks
            .into_iter()
            .map(|stock| DurableProductStock {
                building_instance_id: stock.building_instance_id.get().to_string(),
                product_id: stock.product_id,
                quantity: stock.quantity,
            })
            .collect(),
        hunter_equipment_purchases: state.hunter_equipment_purchases,
        town_seed_version: state.seed_version,
        next_building_instance_id: state.next_building_sequence,
        field_trip_id: state.field_trip_id,
        settled_field_trip_id: state.settled_field_trip_id,
        trade_settlements: state
            .trade_settlements
            .into_iter()
            .map(|settlement| DurableTradeSettlement {
                settlement_id: settlement.settlement_id,
                field_trip_id: settlement.field_trip_id,
                material_id: settlement.material_id,
                quantity: settlement.quantity,
                unit_price: settlement.unit_price,
                total_gold: settlement.total_gold,
            })
            .collect(),
    })
}

fn decode_player_state(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildings::BuildingRepository;
    use crate::content::building_registry::EMBEDDED_REGISTRY_SHA256;
    use crate::simulation::{DurablePlayerState, OriginalFlowSession, OriginalScreen};

    #[tokio::test]
    async fn local_identity_is_idempotent_and_separate_from_cache_state() {
        let repository = InMemoryPlayerRepository::default();
        let token_hash = SessionTokenHash::from_token(Uuid::new_v4());

        assert_eq!(
            repository.resolve_local_identity(token_hash).await.unwrap(),
            None
        );
        let player = repository
            .resolve_or_create_local_identity(token_hash)
            .await
            .unwrap();
        assert_eq!(
            repository
                .resolve_or_create_local_identity(token_hash)
                .await
                .unwrap(),
            player
        );
        assert_eq!(
            repository.resolve_local_identity(token_hash).await.unwrap(),
            Some(player)
        );
    }

    #[tokio::test]
    async fn different_local_identity_hashes_receive_different_players() {
        let repository = InMemoryPlayerRepository::default();
        let first = repository
            .resolve_or_create_local_identity(SessionTokenHash::from_token(Uuid::new_v4()))
            .await
            .unwrap();
        let second = repository
            .resolve_or_create_local_identity(SessionTokenHash::from_token(Uuid::new_v4()))
            .await
            .unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn durable_identity_migration_stores_only_a_fixed_length_hash() {
        let migration =
            include_str!("../../../infra/db/migrations/0004_durable_local_identities.sql");

        assert!(migration.contains("token_hash BYTEA PRIMARY KEY"));
        assert!(migration.contains("octet_length(token_hash) = 32"));
        assert!(!migration.contains("session_token"));
    }

    #[test]
    fn normalized_building_schema_separates_content_and_player_state() {
        let migration =
            include_str!("../../../infra/db/migrations/0007_normalized_building_domain.sql");

        assert!(migration.contains("CREATE TABLE building_definition"));
        assert!(migration.contains("CREATE TABLE building_skin_definition"));
        assert!(migration.contains("CREATE TABLE player_building"));
        assert!(migration.contains("CREATE TABLE town_economy_summary"));
        assert!(migration.contains("CREATE TABLE hunter_trade_settlement"));
        assert!(migration.contains("REFERENCES building_skin_definition"));
    }

    #[test]
    fn normalized_hunter_roster_schema_preserves_capacity_fifo_and_idempotency() {
        let migration =
            include_str!("../../../infra/db/migrations/0013_normalized_hunter_roster.sql");

        assert!(migration.contains("CREATE TABLE player_hunter_roster"));
        assert!(migration.contains("CREATE TABLE player_hunter"));
        assert!(migration.contains("roster_position < 8"));
        assert!(migration.contains("player_hunter_waiting_sequence_unique"));
        assert!(migration.contains("CREATE TABLE player_hunter_roster_command"));
    }

    #[test]
    fn hunter_profile_schema_separates_content_owned_state_and_seeds_eight_demo_hunters() {
        let migration =
            include_str!("../../../infra/db/migrations/0014_hunter_profiles_and_demo_account.sql");

        assert!(migration.contains("CREATE TABLE hunter_class_definition"));
        assert!(migration.contains("CREATE TABLE hunter_trait_definition"));
        assert!(migration.contains("CREATE TABLE hunter_skill_definition"));
        assert!(migration.contains("CREATE TABLE player_profile"));
        assert!(migration.contains("CREATE TABLE player_hunter_trait"));
        assert!(migration.contains("CREATE TABLE player_hunter_skill"));
        assert_eq!(
            migration
                .matches("'00000000-0000-4000-8000-00000000a001', 1, 'active'")
                .count(),
            1
        );
        assert_eq!(
            migration
                .matches("'00000000-0000-4000-8000-00000000a001', 8, 'active'")
                .count(),
            1
        );
        assert!(migration.contains("'hunter-lab:20260724'"));
    }

    #[test]
    fn hunter_info_schema_separates_definitions_from_nullable_player_state() {
        let migration = include_str!("../../../infra/db/migrations/0016_hunter_info_domain.sql");

        assert!(migration.contains("CREATE TABLE hunter_characteristic_definition"));
        assert!(migration.contains("CREATE TABLE hunter_growth_property_definition"));
        assert!(migration.contains("CREATE TABLE hunter_riding_pet_definition"));
        assert!(migration.contains("CREATE TABLE player_hunter_growth"));
        assert!(migration.contains("CREATE TABLE player_hunter_material_stack"));
        assert!(migration.contains("CREATE TABLE player_hunter_riding_pet"));
        assert_eq!(migration.matches("'resolved', 'basic',").count(), 10);
        assert_eq!(migration.matches("'resolved', 'class_change',").count(), 40);
        assert_eq!(migration.matches("'growth:").count(), 15);
        assert!(migration.contains("icon_path, animation_name"));
        assert!(!migration.contains("skill_h1_01"));
    }

    #[tokio::test]
    async fn postgres_loads_only_the_pinned_active_building_release_when_configured() {
        let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
            return;
        };
        let repository = PostgresBuildingRepository::connect_lazy(&database_url).unwrap();
        let catalog = repository
            .load_catalog(ACTIVE_BUILDING_RELEASE_ID, EMBEDDED_REGISTRY_SHA256)
            .await
            .unwrap();
        assert_eq!(catalog.registry_id, ACTIVE_BUILDING_RELEASE_ID);
        assert_eq!(catalog.bases.len(), 79);
        assert_eq!(catalog.skins.len(), 61);
    }

    #[tokio::test]
    async fn postgres_local_identity_contract_when_test_database_is_configured() {
        let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
            return;
        };
        let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
        let token_hash = SessionTokenHash::from_token(Uuid::new_v4());

        let player = repository
            .resolve_or_create_local_identity(token_hash)
            .await
            .unwrap();
        assert_eq!(
            repository.resolve_local_identity(token_hash).await.unwrap(),
            Some(player)
        );
        let stored_length = sqlx::query_scalar::<_, i32>(
            "SELECT octet_length(token_hash) FROM local_identities WHERE player_token = $1",
        )
        .bind(player)
        .fetch_one(&repository.pool)
        .await
        .unwrap();
        assert_eq!(stored_length, 32);

        sqlx::query("DELETE FROM local_identities WHERE player_token = $1")
            .bind(player)
            .execute(&repository.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn postgres_demo_account_loads_eight_diverse_hunter_profiles_when_configured() {
        let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
            return;
        };
        let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
        let player = Uuid::parse_str("00000000-0000-4000-8000-00000000a001").unwrap();

        let loaded = repository.load_or_create(player).await.unwrap();
        let hunters = &loaded.state.hunter_roster.hunters;
        assert_eq!(hunters.len(), 8);
        assert_eq!(hunters[0].profile.display_name, "Astra");
        assert_eq!(hunters[0].profile.visual_family, "H4");
        assert_eq!(hunters[7].profile.display_name, "Hale");
        assert!(hunters
            .iter()
            .all(|hunter| !hunter.profile.traits.is_empty()));
        assert!(hunters
            .iter()
            .all(|hunter| !hunter.profile.skills.is_empty()));
        assert_eq!(
            hunters
                .iter()
                .map(|hunter| hunter.profile.class_id.as_str())
                .collect::<HashSet<_>>()
                .len(),
            5
        );
        assert_eq!(
            hunters
                .iter()
                .map(|hunter| hunter.profile.rarity_id.as_str())
                .collect::<HashSet<_>>()
                .len(),
            5
        );
    }

    #[tokio::test]
    async fn postgres_aggregate_and_ledgers_are_atomic_when_test_database_is_configured() {
        let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
            return;
        };
        let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
        let player = Uuid::new_v4();
        let reward_id = Uuid::new_v4();
        let command_id = Uuid::new_v4();
        let operations = vec![
            PendingOperation::Reward {
                operation_id: reward_id,
                gold: 10,
                item_id: 2001,
                quantity: 1,
            },
            PendingOperation::Equip {
                command_id,
                item_id: 2001,
            },
        ];

        let loaded = repository.load_or_create(player).await.unwrap();
        assert_eq!(loaded.revision, 0);
        let mut state = loaded.state;
        state.navigation = OriginalFlowPlayerState {
            screen: OriginalScreen::Field,
            boot_completed: true,
        };
        state.buildings.town_gold = 1_234;
        state.buildings.materials = 7;
        state.buildings.field_trip_id = 1;
        state.buildings.settled_field_trip_id = 1;
        state.buildings.material_stocks = vec![DurableMaterialStock {
            id: "material_1".into(),
            town_quantity: 3,
            hunter_quantity: 2,
            requested: 4,
            unit_price: 5,
        }];
        state.buildings.trade_settlements = vec![DurableTradeSettlement {
            settlement_id: "settlement-1".into(),
            field_trip_id: 1,
            material_id: "material_1".into(),
            quantity: 3,
            unit_price: 5,
            total_gold: 15,
        }];
        assert_eq!(
            repository
                .persist(player, &state, 0, 1, &operations)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            repository
                .persist(player, &state, 1, 1, &operations)
                .await
                .unwrap(),
            2
        );

        let reward_count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reward_ledger WHERE player_token = $1 AND operation_id = $2",
        )
        .bind(player)
        .bind(reward_id)
        .fetch_one(&repository.pool)
        .await
        .unwrap();
        let command_count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM command_ledger WHERE player_token = $1 AND command_id = $2",
        )
        .bind(player)
        .bind(command_id)
        .fetch_one(&repository.pool)
        .await
        .unwrap();
        assert_eq!(reward_count, 1);
        assert_eq!(command_count, 1);
        let stored_has_buildings = sqlx::query_scalar::<_, bool>(
            "SELECT state ? 'buildings' FROM player_world_state WHERE player_token = $1",
        )
        .bind(player)
        .fetch_one(&repository.pool)
        .await
        .unwrap();
        assert!(!stored_has_buildings);
        let reloaded = repository.load_or_create(player).await.unwrap();
        assert_eq!(reloaded.state.buildings, state.buildings);
        let settlement_count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM hunter_trade_settlement \
             WHERE town_id = (SELECT town_id FROM town WHERE player_token = $1)",
        )
        .bind(player)
        .fetch_one(&repository.pool)
        .await
        .unwrap();
        assert_eq!(settlement_count, 1);

        let rejected_reward = Uuid::new_v4();
        let conflict = repository
            .persist(
                player,
                &state,
                0,
                1,
                &[PendingOperation::Reward {
                    operation_id: rejected_reward,
                    gold: 10,
                    item_id: 2001,
                    quantity: 1,
                }],
            )
            .await;
        assert!(matches!(conflict, Err(RepositoryError::RevisionConflict)));
        let rejected_count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM reward_ledger WHERE player_token = $1 AND operation_id = $2",
        )
        .bind(player)
        .bind(rejected_reward)
        .fetch_one(&repository.pool)
        .await
        .unwrap();
        let revision = sqlx::query_scalar::<_, i64>(
            "SELECT revision FROM player_world_state WHERE player_token = $1",
        )
        .bind(player)
        .fetch_one(&repository.pool)
        .await
        .unwrap();
        assert_eq!(rejected_count, 0);
        assert_eq!(revision, 2);

        sqlx::query("DELETE FROM player_world_state WHERE player_token = $1")
            .bind(player)
            .execute(&repository.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn flow_state_persists_across_reconnect() {
        let repository = InMemoryPlayerRepository::default();
        let player_token = Uuid::from_u128(1);
        let state = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::HunterRoster,
                boot_completed: true,
            },
            ..DurablePlayerAggregate::default()
        };
        repository
            .persist(player_token, &state, 0, 1, &[])
            .await
            .expect("persist");
        assert_eq!(
            repository
                .load_or_create(player_token)
                .await
                .expect("load")
                .state,
            state
        );
    }

    #[tokio::test]
    async fn stale_revision_or_fence_cannot_overwrite_state() {
        let repository = InMemoryPlayerRepository::default();
        let player = Uuid::new_v4();
        let state = DurablePlayerAggregate::default();
        assert_eq!(
            repository.persist(player, &state, 0, 5, &[]).await.unwrap(),
            1
        );
        assert!(matches!(
            repository.persist(player, &state, 0, 5, &[]).await,
            Err(RepositoryError::RevisionConflict)
        ));
        assert!(matches!(
            repository.persist(player, &state, 1, 4, &[]).await,
            Err(RepositoryError::RevisionConflict)
        ));
    }

    #[test]
    fn legacy_navigation_json_upgrades_to_versioned_aggregate() {
        let aggregate = decode_player_state(serde_json::json!({
            "screen": "field",
            "boot_completed": true
        }))
        .unwrap();

        assert_eq!(aggregate.schema_version, DURABLE_PLAYER_SCHEMA_VERSION);
        assert_eq!(aggregate.navigation.screen, OriginalScreen::Field);
        assert_eq!(
            aggregate.migration_fixture_combat,
            DurablePlayerState::default()
        );
    }

    #[test]
    fn authoritative_json_excludes_the_normalized_building_domain() {
        let mut aggregate = DurablePlayerAggregate::default();
        aggregate.navigation.boot_completed = true;
        aggregate.buildings.town_gold = 99;

        let encoded = encode_non_building_state(&aggregate).unwrap();

        assert_eq!(encoded["navigation"]["boot_completed"], true);
        assert!(encoded.get("buildings").is_none());
    }

    #[tokio::test]
    async fn fixture_ledgers_are_idempotent_with_state_revision() {
        let repository = InMemoryPlayerRepository::default();
        let player = Uuid::new_v4();
        let reward_id = Uuid::from_u128(100);
        let command_id = Uuid::from_u128(200);
        let operations = vec![
            PendingOperation::Reward {
                operation_id: reward_id,
                gold: 10,
                item_id: 2001,
                quantity: 1,
            },
            PendingOperation::Equip {
                command_id,
                item_id: 2001,
            },
        ];
        let state = DurablePlayerAggregate::default();

        assert_eq!(
            repository
                .persist(player, &state, 0, 1, &operations)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            repository
                .persist(player, &state, 1, 1, &operations)
                .await
                .unwrap(),
            2
        );
        let durable = repository.durable.read().await;
        assert_eq!(durable.reward_operations.len(), 1);
        assert_eq!(durable.command_operations.len(), 1);
    }

    #[tokio::test]
    async fn field_checkpoint_restores_combat_and_pending_reward_together() {
        let repository = InMemoryPlayerRepository::default();
        let player = Uuid::new_v4();
        let aggregate = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Field,
                boot_completed: true,
            },
            ..DurablePlayerAggregate::default()
        };
        let mut session = OriginalFlowSession::from_aggregate(aggregate, 7);
        let mut operations = Vec::new();
        for _ in 0..100 {
            let tick = session.advance_simulation_tick().expect("field tick");
            operations.extend(tick.operations);
            if !operations.is_empty() {
                break;
            }
        }
        assert!(operations
            .iter()
            .any(|operation| matches!(operation, PendingOperation::Reward { .. })));

        let mut expected = session.snapshot().migration_fixture_combat.world;
        expected.events.clear();
        repository
            .persist(player, &session.durable_state(), 0, 1, &operations)
            .await
            .unwrap();
        let loaded = repository.load_or_create(player).await.unwrap();
        let restored = OriginalFlowSession::from_aggregate(loaded.state, 7);

        assert_eq!(restored.snapshot().migration_fixture_combat.world, expected);
        assert_eq!(repository.durable.read().await.reward_operations.len(), 1);
    }
}
