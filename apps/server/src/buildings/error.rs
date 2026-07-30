use thiserror::Error;

use super::{BaseBuildingId, BuildingSkinKey, TownBuildingInstanceId};

#[derive(Debug, Error)]
pub enum BuildingRepositoryError {
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("invalid base building id: {0}")]
    InvalidBaseId(String),
    #[error("invalid building skin id: {0}")]
    InvalidSkinId(u64),
    #[error("invalid building catalog: {0}")]
    InvalidCatalog(&'static str),
    #[error("building catalog contains more than one registry release")]
    MixedRegistryRelease,
    #[error("duplicate base building: {0}")]
    DuplicateBase(BaseBuildingId),
    #[error("duplicate building skin: {0:?}")]
    DuplicateSkin(BuildingSkinKey),
    #[error("skin references unknown base building: {0}")]
    UnknownSkinBase(BaseBuildingId),
    #[error("level references unknown base building: {0}")]
    UnknownLevelBase(BaseBuildingId),
    #[error("gameplay content references unknown base building: {0}")]
    UnknownGameplayBase(BaseBuildingId),
    #[error("building catalog release mismatch: expected {expected}, found {actual}")]
    RegistryMismatch { expected: String, actual: String },
    #[error("active building catalog release is unavailable: {0}")]
    ActiveReleaseUnavailable(String),
    #[error("building catalog hash mismatch: expected {expected}, found {actual}")]
    RegistryHashMismatch { expected: String, actual: String },
    #[error("integer stored in building catalog is outside domain bounds")]
    NumericBounds,
    #[error("invalid town building state: {0}")]
    InvalidTown(&'static str),
    #[error("duplicate town building instance: {0:?}")]
    DuplicateInstance(TownBuildingInstanceId),
    #[error("town building state revision conflict")]
    RevisionConflict,
}
