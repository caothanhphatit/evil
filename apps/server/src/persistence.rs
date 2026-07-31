#[path = "persistence/aggregate_codec.rs"]
mod aggregate_codec;
use aggregate_codec::*;
#[path = "persistence/building_codec.rs"]
mod building_codec;
use building_codec::*;
#[path = "persistence/hunter_roster_save.rs"]
mod hunter_roster_save;
use hunter_roster_save::*;
#[path = "persistence/hunter_owned_items_save.rs"]
mod hunter_owned_items_save;
use hunter_owned_items_save::*;
#[path = "persistence/hunter_runtime_load.rs"]
mod hunter_runtime_load;
use hunter_runtime_load::*;
#[path = "persistence/hunter_runtime_save.rs"]
mod hunter_runtime_save;
use hunter_runtime_save::*;
#[path = "persistence/memory.rs"]
mod memory;
pub use memory::InMemoryPlayerRepository;
#[path = "persistence/postgres.rs"]
mod postgres;
pub use postgres::PostgresPlayerRepository;
#[cfg(test)]
#[path = "persistence/tests.rs"]
mod tests;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use sqlx::{
    postgres::{PgPoolOptions, PgRow},
    PgPool, Postgres, Row, Transaction,
};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    buildings::{
        BaseBuildingId, BuildingRepository, BuildingRepositoryError, BuildingSkinId,
        PostgresBuildingRepository, TownBuildingInstance, TownBuildingInstanceId,
        TownBuildingState, TownCraftedGearStock, TownMaterialStock, TownProductStock,
        TownTradeSettlement,
    },
    identity::SessionTokenHash,
    simulation::{
        new_account_roster, operational_migration_roster, upgrade_operational_fixture_roster,
        DurableBuilding, DurableBuildingState, DurableCraftedGearStock, DurableHunterEquipmentSlot,
        DurableHunterOwnedItem, DurableHunterProfile, DurableHunterProgress,
        DurableHunterRosterState, DurableHunterRuntimeAppearance, DurableHunterRuntimeConsumable,
        DurableHunterRuntimeGear, DurableHunterRuntimeGrowth, DurableHunterRuntimeInventory,
        DurableHunterRuntimeItem, DurableHunterRuntimeRidingPet, DurableHunterRuntimeSkill,
        DurableHunterRuntimeState, DurableHunterRuntimeStatus, DurableHunterSkill,
        DurableHunterState, DurableHunterTrait, DurableMaterialStock, DurablePlayerAggregate,
        DurableProductStock, DurableTradeSettlement, DurableWaitingHunter, HunterBanishment,
        HunterServiceGauge, OriginalFlowPlayerState, PendingOperation,
        DURABLE_PLAYER_SCHEMA_VERSION, MIGRATION_HUNTER_RELEASE_ID,
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
    #[error("account email already exists")]
    AccountExists,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerAccountRecord {
    pub account_id: Uuid,
    pub player_token: Uuid,
    pub normalized_email: String,
    pub display_name: String,
    pub password_hash: String,
    pub is_demo: bool,
}

#[derive(Clone, Debug)]
pub struct LoadedPlayerState {
    pub state: DurablePlayerAggregate,
    pub revision: i64,
}

#[async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn create_account(
        &self,
        normalized_email: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<PlayerAccountRecord, RepositoryError>;

    async fn find_account_by_email(
        &self,
        normalized_email: &str,
    ) -> Result<Option<PlayerAccountRecord>, RepositoryError>;

    async fn bind_session(
        &self,
        token_hash: SessionTokenHash,
        player_token: Uuid,
    ) -> Result<(), RepositoryError>;

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
