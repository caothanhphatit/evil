use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::simulation::OriginalFlowPlayerState;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("stored player state is invalid")]
    InvalidState(#[from] serde_json::Error),
    #[error("player state revision conflict")]
    RevisionConflict,
}

#[derive(Clone, Debug)]
pub struct LoadedPlayerState {
    pub state: OriginalFlowPlayerState,
    pub revision: i64,
}

#[async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn load_or_create(
        &self,
        player_token: Uuid,
    ) -> Result<LoadedPlayerState, RepositoryError>;

    async fn persist(
        &self,
        player_token: Uuid,
        state: &OriginalFlowPlayerState,
        expected_revision: i64,
        lease_fence: i64,
    ) -> Result<i64, RepositoryError>;

    async fn is_ready(&self) -> bool;
}

pub type SharedPlayerRepository = Arc<dyn PlayerRepository>;

#[derive(Default)]
pub struct InMemoryPlayerRepository {
    states: RwLock<HashMap<Uuid, (OriginalFlowPlayerState, i64, i64)>>,
}

#[async_trait]
impl PlayerRepository for InMemoryPlayerRepository {
    async fn load_or_create(
        &self,
        player_token: Uuid,
    ) -> Result<LoadedPlayerState, RepositoryError> {
        let mut states = self.states.write().await;
        let (state, revision, _) = states.entry(player_token).or_default();
        Ok(LoadedPlayerState {
            state: state.clone(),
            revision: *revision,
        })
    }

    async fn persist(
        &self,
        player_token: Uuid,
        state: &OriginalFlowPlayerState,
        expected_revision: i64,
        lease_fence: i64,
    ) -> Result<i64, RepositoryError> {
        let mut states = self.states.write().await;
        let entry = states.entry(player_token).or_default();
        if entry.1 != expected_revision || entry.2 > lease_fence {
            return Err(RepositoryError::RevisionConflict);
        }
        entry.0 = state.clone();
        entry.1 += 1;
        entry.2 = lease_fence;
        Ok(entry.1)
    }

    async fn is_ready(&self) -> bool {
        true
    }
}

pub struct PostgresPlayerRepository {
    pool: PgPool,
}

impl PostgresPlayerRepository {
    pub fn connect_lazy(database_url: &str) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(10)
                .connect_lazy(database_url)?,
        })
    }
}

#[async_trait]
impl PlayerRepository for PostgresPlayerRepository {
    async fn load_or_create(
        &self,
        player_token: Uuid,
    ) -> Result<LoadedPlayerState, RepositoryError> {
        let default_json = serde_json::to_value(OriginalFlowPlayerState::default())?;
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
        .fetch_one(&self.pool)
        .await?;
        let state: serde_json::Value = row.try_get("state")?;
        Ok(LoadedPlayerState {
            state: serde_json::from_value(state)?,
            revision: row.try_get("revision")?,
        })
    }

    async fn persist(
        &self,
        player_token: Uuid,
        state: &OriginalFlowPlayerState,
        expected_revision: i64,
        lease_fence: i64,
    ) -> Result<i64, RepositoryError> {
        let state_json = serde_json::to_value(state)?;
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
        .fetch_optional(&self.pool)
        .await?;
        revision.ok_or(RepositoryError::RevisionConflict)
    }

    async fn is_ready(&self) -> bool {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::OriginalScreen;

    #[tokio::test]
    async fn flow_state_persists_across_reconnect() {
        let repository = InMemoryPlayerRepository::default();
        let player_token = Uuid::from_u128(1);
        let state = OriginalFlowPlayerState {
            screen: OriginalScreen::HunterRoster,
            boot_completed: true,
        };
        repository
            .persist(player_token, &state, 0, 1)
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
        let state = OriginalFlowPlayerState::default();
        assert_eq!(repository.persist(player, &state, 0, 5).await.unwrap(), 1);
        assert!(matches!(
            repository.persist(player, &state, 0, 5).await,
            Err(RepositoryError::RevisionConflict)
        ));
        assert!(matches!(
            repository.persist(player, &state, 1, 4).await,
            Err(RepositoryError::RevisionConflict)
        ));
    }
}
