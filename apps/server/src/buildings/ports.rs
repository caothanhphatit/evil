use async_trait::async_trait;
use uuid::Uuid;

use super::{BuildingCatalog, BuildingGameplayCatalog, BuildingRepositoryError, TownBuildingState};

#[async_trait]
pub trait BuildingRepository: Send + Sync {
    async fn load_catalog(
        &self,
        expected_registry_id: &str,
        expected_registry_sha256: &str,
    ) -> Result<BuildingCatalog, BuildingRepositoryError>;

    async fn load_gameplay_catalog(
        &self,
        expected_registry_id: &str,
        expected_registry_sha256: &str,
    ) -> Result<BuildingGameplayCatalog, BuildingRepositoryError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedTownBuildingState {
    pub state: TownBuildingState,
    pub revision: i64,
}

#[async_trait]
pub trait TownBuildingRepository: Send + Sync {
    async fn load_town(
        &self,
        player_token: Uuid,
    ) -> Result<Option<LoadedTownBuildingState>, BuildingRepositoryError>;

    async fn save_town(
        &self,
        player_token: Uuid,
        state: &TownBuildingState,
        expected_revision: i64,
    ) -> Result<i64, BuildingRepositoryError>;
}
