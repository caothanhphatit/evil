//! Building domain, repository ports, and persistence adapters.
//!
//! Runtime code reads the normalized PostgreSQL catalog through this package.
//! Importing a content release is explicit; server startup never rewrites
//! catalog rows from an embedded registry.

mod catalog;
mod error;
mod gear_shops;
mod identifiers;
mod ports;
mod postgres;
mod town;

pub use catalog::{
    AuthoritativeBuildingContent, BaseBuildingDefinition, BuildingCapabilityDefinition,
    BuildingCatalog, BuildingGameplayCatalog, BuildingLevelDefinition, BuildingLevelPrerequisite,
    BuildingSkinDefinition, ConsumableProductDefinition, EconomyAmount, EconomyConversionOption,
    EconomyItemDefinition, EconomyProductDefinition, EconomyProductService, EconomyRandomOutput,
    GearProductDefinition, HunterBasicSkillContentDefinition, HunterClassContentDefinition,
    HunterProgressionDefinition, HunterRarityContentDefinition, HunterStaticContent,
    MonsterDefinition, MonsterMaterialDefinition, OrdinaryMonsterPoolDefinition,
    WorldMapDefinition,
};
pub use error::BuildingRepositoryError;
pub use gear_shops::{gear_product_route, GearProductFamily, GearProductKind, GearProductRoute};
pub use identifiers::{BaseBuildingId, BuildingSkinId, BuildingSkinKey, TownBuildingInstanceId};
pub use ports::{BuildingRepository, LoadedTownBuildingState, TownBuildingRepository};
pub use postgres::PostgresBuildingRepository;
pub use town::{
    TownBuildingInstance, TownBuildingState, TownCraftedGearStock, TownMaterialStock,
    TownProductStock, TownTradeSettlement,
};

#[cfg(test)]
mod tests;
