use super::{
    async_trait, decode_player_state, durable_buildings_from_town, encode_non_building_state,
    load_hunter_roster_in, new_account_roster, operational_migration_roster, save_hunter_roster_in,
    town_from_durable_buildings, upgrade_operational_fixture_roster, BuildingRepository,
    BuildingRepositoryError, DurablePlayerAggregate, LoadedPlayerState, PendingOperation, PgPool,
    PgPoolOptions, PlayerAccountRecord, PlayerRepository, PostgresBuildingRepository,
    RepositoryError, Row, SessionTokenHash, Uuid, ACTIVE_BUILDING_RELEASE_ID,
};

pub struct PostgresPlayerRepository {
    pub(super) pool: PgPool,
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
    async fn create_account(
        &self,
        normalized_email: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<PlayerAccountRecord, RepositoryError> {
        let account_id = Uuid::new_v4();
        let player_token = Uuid::new_v4();
        let result = sqlx::query(
            r#"
            INSERT INTO player_account
                (account_id, player_token, normalized_email, display_name, password_hash)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING account_id, player_token, normalized_email, display_name, password_hash, is_demo
            "#,
        )
        .bind(account_id)
        .bind(player_token)
        .bind(normalized_email)
        .bind(display_name)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await;
        let row = match result {
            Ok(row) => row,
            Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23505") => {
                return Err(RepositoryError::AccountExists)
            }
            Err(error) => return Err(error.into()),
        };
        Ok(PlayerAccountRecord {
            account_id: row.try_get("account_id")?,
            player_token: row.try_get("player_token")?,
            normalized_email: row.try_get("normalized_email")?,
            display_name: row.try_get("display_name")?,
            password_hash: row.try_get("password_hash")?,
            is_demo: row.try_get("is_demo")?,
        })
    }

    async fn find_account_by_email(
        &self,
        normalized_email: &str,
    ) -> Result<Option<PlayerAccountRecord>, RepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT account_id, player_token, normalized_email, display_name, password_hash, is_demo
            FROM player_account
            WHERE normalized_email = $1
            "#,
        )
        .bind(normalized_email)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(PlayerAccountRecord {
                account_id: row.try_get("account_id")?,
                player_token: row.try_get("player_token")?,
                normalized_email: row.try_get("normalized_email")?,
                display_name: row.try_get("display_name")?,
                password_hash: row.try_get("password_hash")?,
                is_demo: row.try_get("is_demo")?,
            })
        })
        .transpose()
    }

    async fn bind_session(
        &self,
        token_hash: SessionTokenHash,
        player_token: Uuid,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO local_identities (token_hash, player_token)
            VALUES ($1, $2)
            ON CONFLICT (token_hash) DO UPDATE
            SET player_token = EXCLUDED.player_token, last_seen_at = now()
            "#,
        )
        .bind(token_hash.as_bytes().as_slice())
        .bind(player_token)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

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
        let is_demo_account = sqlx::query_scalar::<_, bool>(
            "SELECT COALESCE((SELECT is_demo FROM player_account WHERE player_token = $1), FALSE)",
        )
        .bind(player_token)
        .fetch_one(&self.pool)
        .await?;
        let mut default_state = DurablePlayerAggregate::default();
        let is_new_account = !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM player_world_state WHERE player_token = $1)",
        )
        .bind(player_token)
        .fetch_one(&self.pool)
        .await?;
        if is_new_account {
            default_state.buildings.town_gold = 100_000;
        }
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
        let mut town = match self
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
            roster = if is_new_account {
                new_account_roster(player_token)
            } else {
                operational_migration_roster()
            };
            save_hunter_roster_in(&mut transaction, player_token, &roster).await?;
        } else if upgrade_operational_fixture_roster(&mut roster) {
            save_hunter_roster_in(&mut transaction, player_token, &roster).await?;
        }
        if is_new_account && is_demo_account {
            sqlx::query("SELECT seed_full_demo_account_stock($1)")
                .bind(player_token)
                .execute(&mut *transaction)
                .await?;
            town = self
                .buildings
                .load_town_in(&mut transaction, player_token)
                .await?
                .ok_or(RepositoryError::RevisionDivergence)?;
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
