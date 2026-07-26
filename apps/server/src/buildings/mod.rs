//! Building domain identifiers and persistence.
//!
//! Runtime code reads the normalized PostgreSQL catalog through this module.
//! Importing a content release is an explicit operation; server startup must not
//! rewrite catalog rows from an embedded registry.

mod gear_shops;

pub use gear_shops::{gear_product_route, GearProductFamily, GearProductKind, GearProductRoute};

use std::{
    collections::{BTreeMap, HashSet},
    fmt,
    str::FromStr,
};

use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use uuid::Uuid;

const CATALOG_LOCK_KEY: &str = "evil_hunter_building_catalog";

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BaseBuildingId(String);

impl BaseBuildingId {
    pub fn parse(value: impl Into<String>) -> Result<Self, BuildingRepositoryError> {
        let value = value.into();
        let suffix = value
            .strip_prefix("build_")
            .ok_or_else(|| BuildingRepositoryError::InvalidBaseId(value.clone()))?;
        if suffix.is_empty()
            || !suffix.bytes().all(|byte| byte.is_ascii_digit())
            || (suffix.len() > 1 && suffix.starts_with('0'))
        {
            return Err(BuildingRepositoryError::InvalidBaseId(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BaseBuildingId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BaseBuildingId {
    type Err = BuildingRepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BuildingSkinId(u64);

impl BuildingSkinId {
    pub fn new(value: u64) -> Result<Self, BuildingRepositoryError> {
        if value == 0 {
            return Err(BuildingRepositoryError::InvalidSkinId(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BuildingSkinKey {
    pub building_id: BaseBuildingId,
    pub skin_id: BuildingSkinId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TownBuildingInstanceId(Uuid);

impl TownBuildingInstanceId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn get(self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseBuildingDefinition {
    pub id: BaseBuildingId,
    pub registry_id: String,
    pub display_name: String,
    pub category: Option<String>,
    pub source_type: i64,
    pub max_instances: u32,
    pub grid_width: u16,
    pub grid_height: u16,
    pub movable: Option<bool>,
    pub constructible: Option<bool>,
    pub base_sprite_asset_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingLevelDefinition {
    pub building_id: BaseBuildingId,
    pub level: u16,
    pub upgrade_duration_ms: Option<u64>,
    pub inventory_capacity: Option<u64>,
    pub production_slots: Option<u16>,
    pub costs: Vec<EconomyAmount>,
    pub prerequisites: Vec<BuildingLevelPrerequisite>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingLevelPrerequisite {
    pub building_id: BaseBuildingId,
    pub required_level: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingSkinDefinition {
    pub key: BuildingSkinKey,
    pub family: String,
    pub display_name: String,
    pub required_level: u16,
    pub visibility: i64,
    pub asset_key: Option<String>,
    pub sprite_prefix: Option<String>,
    pub visual_resolved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingCatalog {
    pub registry_id: String,
    pub bases: Vec<BaseBuildingDefinition>,
    pub levels: Vec<BuildingLevelDefinition>,
    pub skins: Vec<BuildingSkinDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingCapabilityDefinition {
    pub capability_id: String,
    pub building_id: BaseBuildingId,
    pub kind: String,
    pub static_data_ready: bool,
    pub runnable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomyAmount {
    pub resource_id: String,
    pub quantity: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomyItemDefinition {
    pub item_id: String,
    pub internal_name: Option<String>,
    pub item_type: Option<String>,
    pub stack_limit: Option<u64>,
    pub town_pays_hunter_gold_per_unit: Option<u64>,
    pub localized_names: BTreeMap<String, String>,
    pub buy_price: Vec<EconomyAmount>,
    pub sell_price: Vec<EconomyAmount>,
    pub hunter_pays_town_gold_by_tier: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomyProductService {
    pub source_type: u64,
    pub required_level: u16,
    pub service_time_ms: u64,
    pub effect_value: u64,
    pub use_money: u64,
    pub completion_counts: Vec<u64>,
    pub required_cash_count: u64,
    pub cash_completion_count: u64,
    pub required_elemental_count: u64,
    pub elemental_completion_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomyConversionOption {
    pub input_kind: String,
    pub input_resource_id: String,
    pub input_quantity: u64,
    pub output_stock_quantity: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomyRandomOutput {
    pub item_type: String,
    pub grade: u64,
    pub quantity: u64,
    pub rng_ready: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomyProductDefinition {
    pub product_id: String,
    pub building_id: Option<BaseBuildingId>,
    pub duration_ms: Option<u64>,
    pub exact_mutation_ready: bool,
    pub inputs: Vec<EconomyAmount>,
    pub outputs: Vec<EconomyAmount>,
    pub sale_price: Vec<EconomyAmount>,
    pub service: Option<EconomyProductService>,
    pub conversion_options: Vec<EconomyConversionOption>,
    pub random_output: Option<EconomyRandomOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingGameplayCatalog {
    pub registry_id: String,
    pub capabilities: Vec<BuildingCapabilityDefinition>,
    pub items: BTreeMap<String, EconomyItemDefinition>,
    pub products: BTreeMap<String, EconomyProductDefinition>,
}

impl BuildingGameplayCatalog {
    pub fn capabilities_for<'a>(
        &'a self,
        building_id: &'a BaseBuildingId,
    ) -> impl Iterator<Item = &'a BuildingCapabilityDefinition> + 'a {
        self.capabilities
            .iter()
            .filter(move |capability| &capability.building_id == building_id)
    }

    pub fn item(&self, item_id: &str) -> Option<&EconomyItemDefinition> {
        self.items.get(item_id)
    }

    pub fn product(&self, product_id: &str) -> Option<&EconomyProductDefinition> {
        self.products.get(product_id)
    }

    pub fn products_for_building<'a>(
        &'a self,
        building_id: &'a BaseBuildingId,
    ) -> impl Iterator<Item = &'a EconomyProductDefinition> + 'a {
        self.products
            .values()
            .filter(move |product| product.building_id.as_ref() == Some(building_id))
    }

    pub fn validate(&self, catalog: &BuildingCatalog) -> Result<(), BuildingRepositoryError> {
        if self.registry_id != catalog.registry_id {
            return Err(BuildingRepositoryError::RegistryMismatch {
                expected: catalog.registry_id.clone(),
                actual: self.registry_id.clone(),
            });
        }

        let base_ids = catalog
            .bases
            .iter()
            .map(|base| &base.id)
            .collect::<HashSet<_>>();
        let mut capability_ids = HashSet::with_capacity(self.capabilities.len());
        let mut capability_keys = HashSet::with_capacity(self.capabilities.len());
        for capability in &self.capabilities {
            if capability.capability_id.trim().is_empty()
                || capability.kind.trim().is_empty()
                || !capability_ids.insert(capability.capability_id.as_str())
                || !capability_keys.insert((&capability.building_id, capability.kind.as_str()))
            {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "capabilities must have unique non-empty identities",
                ));
            }
            if !base_ids.contains(&capability.building_id) {
                return Err(BuildingRepositoryError::UnknownGameplayBase(
                    capability.building_id.clone(),
                ));
            }
            if capability.runnable && !capability.static_data_ready {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "runnable capabilities require complete static data",
                ));
            }
        }

        for (item_id, item) in &self.items {
            if item_id.trim().is_empty() || item.item_id != *item_id {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "item map keys must match non-empty item identities",
                ));
            }
            if item
                .localized_names
                .keys()
                .any(|locale| locale.trim().is_empty())
            {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "item localization locale must be non-empty",
                ));
            }
        }

        for (product_id, product) in &self.products {
            if product_id.trim().is_empty() || product.product_id != *product_id {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "product map keys must match non-empty product identities",
                ));
            }
            if let Some(building_id) = &product.building_id {
                if !base_ids.contains(building_id) {
                    return Err(BuildingRepositoryError::UnknownGameplayBase(
                        building_id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl BuildingCatalog {
    pub fn base(&self, id: &BaseBuildingId) -> Option<&BaseBuildingDefinition> {
        self.bases.iter().find(|base| &base.id == id)
    }

    /// Level one is the construction cost/condition; later rows are upgrades
    /// targeting that level.
    pub fn level(&self, id: &BaseBuildingId, level: u16) -> Option<&BuildingLevelDefinition> {
        self.levels
            .iter()
            .find(|definition| &definition.building_id == id && definition.level == level)
    }

    pub fn skin(
        &self,
        id: &BaseBuildingId,
        skin_id: BuildingSkinId,
    ) -> Option<&BuildingSkinDefinition> {
        self.skins
            .iter()
            .find(|skin| &skin.key.building_id == id && skin.key.skin_id == skin_id)
    }

    pub fn validate(&self) -> Result<(), BuildingRepositoryError> {
        if self.registry_id.trim().is_empty() || self.bases.is_empty() {
            return Err(BuildingRepositoryError::InvalidCatalog(
                "catalog release and base rows are required",
            ));
        }

        let mut base_ids = HashSet::with_capacity(self.bases.len());
        for base in &self.bases {
            if base.registry_id != self.registry_id {
                return Err(BuildingRepositoryError::MixedRegistryRelease);
            }
            if !base_ids.insert(&base.id) {
                return Err(BuildingRepositoryError::DuplicateBase(base.id.clone()));
            }
            if base.max_instances == 0 || base.grid_width == 0 || base.grid_height == 0 {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "building limits and dimensions must be positive",
                ));
            }
        }

        let mut level_keys = HashSet::with_capacity(self.levels.len());
        for level in &self.levels {
            if !base_ids.contains(&level.building_id) {
                return Err(BuildingRepositoryError::UnknownLevelBase(
                    level.building_id.clone(),
                ));
            }
            if level.level == 0 || !level_keys.insert((&level.building_id, level.level)) {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "building levels must be positive and unique",
                ));
            }
            let mut cost_resources = HashSet::with_capacity(level.costs.len());
            if level.costs.iter().any(|cost| {
                cost.resource_id.trim().is_empty()
                    || !cost_resources.insert(cost.resource_id.as_str())
            }) {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "building level costs require unique non-empty resources",
                ));
            }
        }
        if base_ids
            .iter()
            .any(|base_id| !level_keys.contains(&(base_id, 1)))
        {
            return Err(BuildingRepositoryError::InvalidCatalog(
                "every building requires a level-one definition",
            ));
        }
        for level in &self.levels {
            let mut prerequisite_bases = HashSet::with_capacity(level.prerequisites.len());
            for prerequisite in &level.prerequisites {
                if !prerequisite_bases.insert(&prerequisite.building_id)
                    || !level_keys
                        .contains(&(&prerequisite.building_id, prerequisite.required_level))
                {
                    return Err(BuildingRepositoryError::InvalidCatalog(
                        "building prerequisites must be unique and reference known levels",
                    ));
                }
            }
        }

        let mut skin_keys = HashSet::with_capacity(self.skins.len());
        for skin in &self.skins {
            if !base_ids.contains(&skin.key.building_id) {
                return Err(BuildingRepositoryError::UnknownSkinBase(
                    skin.key.building_id.clone(),
                ));
            }
            if !skin_keys.insert(&skin.key) {
                return Err(BuildingRepositoryError::DuplicateSkin(skin.key.clone()));
            }
            if skin.visual_resolved != (skin.asset_key.is_some() && skin.sprite_prefix.is_some()) {
                return Err(BuildingRepositoryError::InvalidCatalog(
                    "skin level or visual binding is invalid",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoritativeBuildingContent {
    pub catalog: BuildingCatalog,
    pub gameplay: BuildingGameplayCatalog,
}

impl AuthoritativeBuildingContent {
    pub fn new(
        catalog: BuildingCatalog,
        gameplay: BuildingGameplayCatalog,
    ) -> Result<Self, BuildingRepositoryError> {
        if catalog.registry_id != gameplay.registry_id {
            return Err(BuildingRepositoryError::RegistryMismatch {
                expected: catalog.registry_id,
                actual: gameplay.registry_id,
            });
        }
        catalog.validate()?;
        gameplay.validate(&catalog)?;
        Ok(Self { catalog, gameplay })
    }
}

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
pub struct TownBuildingInstance {
    pub instance_id: TownBuildingInstanceId,
    pub building_id: BaseBuildingId,
    pub equipped_skin_id: Option<BuildingSkinId>,
    pub level: u16,
    pub uses: u32,
    pub grid_x: i32,
    pub grid_y: i32,
    pub seeded_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownBuildingState {
    pub release_id: String,
    pub town_gold: u64,
    pub seed_version: u16,
    pub next_building_sequence: u64,
    pub buildings: Vec<TownBuildingInstance>,
    pub hunter_materials: u32,
    pub materials: u32,
    pub runes: u32,
    pub weapons: u32,
    pub armor: u32,
    pub hunter_equipment_purchases: u32,
    pub field_trip_id: u64,
    pub settled_field_trip_id: u64,
    pub material_stocks: Vec<TownMaterialStock>,
    pub product_stocks: Vec<TownProductStock>,
    pub trade_settlements: Vec<TownTradeSettlement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownMaterialStock {
    pub id: String,
    pub town_quantity: u32,
    pub hunter_quantity: u32,
    pub requested: u32,
    pub unit_price: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownProductStock {
    pub building_instance_id: TownBuildingInstanceId,
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownTradeSettlement {
    pub settlement_id: String,
    pub field_trip_id: u64,
    pub material_id: String,
    pub quantity: u32,
    pub unit_price: u64,
    pub total_gold: u64,
}

#[async_trait]
impl TownBuildingRepository for PostgresBuildingRepository {
    async fn load_town(
        &self,
        player_token: Uuid,
    ) -> Result<Option<LoadedTownBuildingState>, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let loaded = self.load_town_in(&mut transaction, player_token).await?;
        transaction.commit().await?;
        Ok(loaded)
    }

    async fn save_town(
        &self,
        player_token: Uuid,
        state: &TownBuildingState,
        expected_revision: i64,
    ) -> Result<i64, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let revision = self
            .save_town_in(&mut transaction, player_token, state, expected_revision)
            .await?;
        transaction.commit().await?;
        Ok(revision)
    }
}

impl TownBuildingState {
    pub fn validate(&self) -> Result<(), BuildingRepositoryError> {
        if self.release_id.trim().is_empty() {
            return Err(BuildingRepositoryError::InvalidTown(
                "content release is required",
            ));
        }
        if self.settled_field_trip_id > self.field_trip_id {
            return Err(BuildingRepositoryError::InvalidTown(
                "settled field trip cannot exceed the latest trip",
            ));
        }
        let mut instance_ids = HashSet::with_capacity(self.buildings.len());
        for building in &self.buildings {
            if building.level == 0 {
                return Err(BuildingRepositoryError::InvalidTown(
                    "building level must be positive",
                ));
            }
            if !instance_ids.insert(building.instance_id) {
                return Err(BuildingRepositoryError::DuplicateInstance(
                    building.instance_id,
                ));
            }
        }
        let mut material_ids = HashSet::with_capacity(self.material_stocks.len());
        for stock in &self.material_stocks {
            if stock.id.trim().is_empty() || !material_ids.insert(stock.id.as_str()) {
                return Err(BuildingRepositoryError::InvalidTown(
                    "material stock ids must be non-empty and unique",
                ));
            }
        }
        let building_ids = self
            .buildings
            .iter()
            .map(|building| building.instance_id)
            .collect::<HashSet<_>>();
        let mut product_keys = HashSet::with_capacity(self.product_stocks.len());
        for stock in &self.product_stocks {
            if stock.product_id.trim().is_empty()
                || !building_ids.contains(&stock.building_instance_id)
                || !product_keys.insert((stock.building_instance_id, stock.product_id.as_str()))
            {
                return Err(BuildingRepositoryError::InvalidTown(
                    "product stocks must reference a building and have unique non-empty products",
                ));
            }
        }
        let mut settlement_ids = HashSet::with_capacity(self.trade_settlements.len());
        for settlement in &self.trade_settlements {
            if settlement.settlement_id.trim().is_empty()
                || settlement.material_id.trim().is_empty()
                || settlement.field_trip_id == 0
                || settlement.quantity == 0
                || !settlement_ids.insert(settlement.settlement_id.as_str())
            {
                return Err(BuildingRepositoryError::InvalidTown(
                    "trade settlements must have valid unique identities",
                ));
            }
        }
        Ok(())
    }
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

#[derive(Clone)]
pub struct PostgresBuildingRepository {
    pool: PgPool,
}

impl PostgresBuildingRepository {
    pub fn connect_lazy(database_url: &str) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect_lazy(database_url)?,
        })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(crate) async fn load_town_in(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        player_token: Uuid,
    ) -> Result<Option<LoadedTownBuildingState>, BuildingRepositoryError> {
        let row = sqlx::query(
            "SELECT town_id, release_id, gold, seed_version::bigint AS seed_version, \
                    next_building_sequence, revision \
             FROM town WHERE player_token = $1 FOR SHARE",
        )
        .bind(player_token)
        .fetch_optional(&mut **transaction)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let town_id: Uuid = row.try_get("town_id")?;
        let rows = sqlx::query(
            r#"SELECT instance_id, building_id, equipped_skin_id,
                      current_level::bigint AS current_level,
                      grid_x, grid_y, use_count, seeded_by
               FROM player_building WHERE town_id = $1 ORDER BY instance_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let mut buildings = Vec::with_capacity(rows.len());
        for building in rows {
            buildings.push(TownBuildingInstance {
                instance_id: TownBuildingInstanceId::new(building.try_get("instance_id")?),
                building_id: BaseBuildingId::parse(building.try_get::<String, _>("building_id")?)?,
                equipped_skin_id: optional_skin_id(building.try_get("equipped_skin_id")?)?,
                level: to_u16(building.try_get("current_level")?)?,
                uses: to_u32(building.try_get("use_count")?)?,
                grid_x: building.try_get("grid_x")?,
                grid_y: building.try_get("grid_y")?,
                seeded_by: building.try_get("seeded_by")?,
            });
        }
        let economy = sqlx::query(
            r#"SELECT hunter_materials, materials, runes, weapons, armor,
                      hunter_equipment_purchases
               FROM town_economy_summary WHERE town_id = $1"#,
        )
        .bind(town_id)
        .fetch_one(&mut **transaction)
        .await?;
        let trade_state = sqlx::query(
            "SELECT field_trip_id, settled_field_trip_id FROM town_trade_state WHERE town_id = $1",
        )
        .bind(town_id)
        .fetch_one(&mut **transaction)
        .await?;
        let stock_rows = sqlx::query(
            r#"SELECT inventory.item_id,
                      inventory.quantity AS town_quantity,
                      COALESCE(hunter.quantity, 0) AS hunter_quantity,
                      COALESCE(orders.requested_quantity - orders.fulfilled_quantity, 0) AS requested,
                      COALESCE(orders.unit_price, 0) AS unit_price
               FROM town_inventory_stack AS inventory
               LEFT JOIN hunter_material_stack AS hunter
                 ON hunter.town_id = inventory.town_id
                AND hunter.material_id = inventory.item_id
               LEFT JOIN building_material_order AS orders
                 ON orders.town_id = inventory.town_id
                AND orders.material_id = inventory.item_id
                AND orders.status = 'open'
               WHERE inventory.town_id = $1
               UNION
               SELECT hunter.material_id, 0, hunter.quantity, 0, 0
               FROM hunter_material_stack AS hunter
               WHERE hunter.town_id = $1
                 AND NOT EXISTS (
                     SELECT 1 FROM town_inventory_stack AS inventory
                     WHERE inventory.town_id = hunter.town_id
                       AND inventory.item_id = hunter.material_id
                 )
               UNION
               SELECT orders.material_id, 0, 0,
                      orders.requested_quantity - orders.fulfilled_quantity, orders.unit_price
               FROM building_material_order AS orders
               WHERE orders.town_id = $1 AND orders.status = 'open'
                 AND NOT EXISTS (
                     SELECT 1 FROM town_inventory_stack AS inventory
                     WHERE inventory.town_id = orders.town_id
                       AND inventory.item_id = orders.material_id
                 )
                 AND NOT EXISTS (
                     SELECT 1 FROM hunter_material_stack AS hunter
                     WHERE hunter.town_id = orders.town_id
                       AND hunter.material_id = orders.material_id
                 )
               ORDER BY item_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let material_stocks = stock_rows
            .into_iter()
            .map(|stock| {
                Ok(TownMaterialStock {
                    id: stock.try_get("item_id")?,
                    town_quantity: to_u32(stock.try_get("town_quantity")?)?,
                    hunter_quantity: to_u32(stock.try_get("hunter_quantity")?)?,
                    requested: to_u32(stock.try_get("requested")?)?,
                    unit_price: to_u64(stock.try_get("unit_price")?)?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let product_stock_rows = sqlx::query(
            r#"SELECT building_instance_id, product_id, quantity
               FROM building_product_stock
               WHERE town_id = $1
               ORDER BY building_instance_id, product_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let product_stocks = product_stock_rows
            .into_iter()
            .map(|stock| {
                Ok(TownProductStock {
                    building_instance_id: TownBuildingInstanceId::new(
                        stock.try_get("building_instance_id")?,
                    ),
                    product_id: stock.try_get("product_id")?,
                    quantity: to_u32(stock.try_get("quantity")?)?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let settlement_rows = sqlx::query(
            r#"SELECT settlement_id, field_trip_id, material_id, quantity,
                      unit_price, total_gold
               FROM hunter_trade_settlement
               WHERE town_id = $1 ORDER BY settled_at, settlement_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let trade_settlements = settlement_rows
            .into_iter()
            .map(|settlement| {
                Ok(TownTradeSettlement {
                    settlement_id: settlement.try_get("settlement_id")?,
                    field_trip_id: to_u64(settlement.try_get("field_trip_id")?)?,
                    material_id: settlement.try_get("material_id")?,
                    quantity: to_u32(settlement.try_get("quantity")?)?,
                    unit_price: to_u64(settlement.try_get("unit_price")?)?,
                    total_gold: to_u64(settlement.try_get("total_gold")?)?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let loaded = LoadedTownBuildingState {
            state: TownBuildingState {
                release_id: row.try_get("release_id")?,
                town_gold: to_u64(row.try_get("gold")?)?,
                seed_version: to_u16(row.try_get("seed_version")?)?,
                next_building_sequence: to_u64(row.try_get("next_building_sequence")?)?,
                buildings,
                hunter_materials: to_u32(economy.try_get("hunter_materials")?)?,
                materials: to_u32(economy.try_get("materials")?)?,
                runes: to_u32(economy.try_get("runes")?)?,
                weapons: to_u32(economy.try_get("weapons")?)?,
                armor: to_u32(economy.try_get("armor")?)?,
                hunter_equipment_purchases: to_u32(economy.try_get("hunter_equipment_purchases")?)?,
                field_trip_id: to_u64(trade_state.try_get("field_trip_id")?)?,
                settled_field_trip_id: to_u64(trade_state.try_get("settled_field_trip_id")?)?,
                material_stocks,
                product_stocks,
                trade_settlements,
            },
            revision: row.try_get("revision")?,
        };
        loaded.state.validate()?;
        Ok(Some(loaded))
    }

    pub(crate) async fn create_town_from_default_template_in(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        player_token: Uuid,
        release_id: &str,
        initial_gold: u64,
    ) -> Result<LoadedTownBuildingState, BuildingRepositoryError> {
        let town_row = sqlx::query(
            r#"INSERT INTO town (
                   player_token, release_id, source_template_id, gold,
                   next_building_sequence, revision
               )
               SELECT $1, template.release_id, template.template_id, $3,
                      count(template_building.slot) + 1, 0
               FROM town_template AS template
               LEFT JOIN town_template_building AS template_building
                 ON template_building.template_id = template.template_id
                AND template_building.release_id = template.release_id
               WHERE template.release_id = $2 AND template.is_default
               GROUP BY template.template_id, template.release_id
               ON CONFLICT (player_token) DO NOTHING
               RETURNING town_id"#,
        )
        .bind(player_token)
        .bind(release_id)
        .bind(i64::try_from(initial_gold).map_err(|_| BuildingRepositoryError::NumericBounds)?)
        .fetch_optional(&mut **transaction)
        .await?;

        if let Some(town_row) = town_row {
            let town_id: Uuid = town_row.try_get("town_id")?;
            sqlx::query(
                "INSERT INTO town_economy_summary (town_id, hunter_materials) VALUES ($1, 20)",
            )
            .bind(town_id)
            .execute(&mut **transaction)
            .await?;
            sqlx::query("INSERT INTO town_trade_state (town_id) VALUES ($1)")
                .bind(town_id)
                .execute(&mut **transaction)
                .await?;
            sqlx::query(
                r#"INSERT INTO player_building (
                       town_id, release_id, building_id, current_level,
                       equipped_skin_id, grid_x, grid_y, use_count, seeded_by
                   )
                   SELECT $1, template.release_id, template.building_id, template.level,
                          template.equipped_skin_id, template.grid_x, template.grid_y,
                          0, source.template_id
                   FROM town_template_building AS template
                   JOIN town_template AS source
                     ON source.template_id = template.template_id
                    AND source.release_id = template.release_id
                   WHERE template.release_id = $2 AND source.is_default
                   ORDER BY template.slot"#,
            )
            .bind(town_id)
            .bind(release_id)
            .execute(&mut **transaction)
            .await?;
        }

        self.load_town_in(transaction, player_token).await?.ok_or(
            BuildingRepositoryError::InvalidTown("active release has no default town template"),
        )
    }

    pub(crate) async fn save_town_in(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        player_token: Uuid,
        state: &TownBuildingState,
        expected_revision: i64,
    ) -> Result<i64, BuildingRepositoryError> {
        state.validate()?;
        let town_row = sqlx::query(
            r#"UPDATE town SET
                   gold = $2,
                   seed_version = $3,
                   next_building_sequence = $4,
                   revision = town.revision + 1,
                   updated_at = now()
               WHERE player_token = $1 AND release_id = $5 AND revision = $6
               RETURNING town_id, revision"#,
        )
        .bind(player_token)
        .bind(i64::try_from(state.town_gold).map_err(|_| BuildingRepositoryError::NumericBounds)?)
        .bind(i64::from(state.seed_version))
        .bind(
            i64::try_from(state.next_building_sequence)
                .map_err(|_| BuildingRepositoryError::NumericBounds)?,
        )
        .bind(&state.release_id)
        .bind(expected_revision)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(BuildingRepositoryError::RevisionConflict)?;
        let town_id: Uuid = town_row.try_get("town_id")?;
        let revision: i64 = town_row.try_get("revision")?;

        sqlx::query(
            r#"INSERT INTO town_economy_summary (
                   town_id, hunter_materials, materials, runes, weapons, armor,
                   hunter_equipment_purchases
               ) VALUES ($1,$2,$3,$4,$5,$6,$7)
               ON CONFLICT (town_id) DO UPDATE SET
                   hunter_materials = EXCLUDED.hunter_materials,
                   materials = EXCLUDED.materials,
                   runes = EXCLUDED.runes,
                   weapons = EXCLUDED.weapons,
                   armor = EXCLUDED.armor,
                   hunter_equipment_purchases = EXCLUDED.hunter_equipment_purchases,
                   updated_at = now()"#,
        )
        .bind(town_id)
        .bind(i64::from(state.hunter_materials))
        .bind(i64::from(state.materials))
        .bind(i64::from(state.runes))
        .bind(i64::from(state.weapons))
        .bind(i64::from(state.armor))
        .bind(i64::from(state.hunter_equipment_purchases))
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            r#"INSERT INTO town_trade_state (town_id, field_trip_id, settled_field_trip_id)
               VALUES ($1,$2,$3)
               ON CONFLICT (town_id) DO UPDATE SET
                   field_trip_id = EXCLUDED.field_trip_id,
                   settled_field_trip_id = EXCLUDED.settled_field_trip_id,
                   updated_at = now()"#,
        )
        .bind(town_id)
        .bind(
            i64::try_from(state.field_trip_id)
                .map_err(|_| BuildingRepositoryError::NumericBounds)?,
        )
        .bind(
            i64::try_from(state.settled_field_trip_id)
                .map_err(|_| BuildingRepositoryError::NumericBounds)?,
        )
        .execute(&mut **transaction)
        .await?;

        sqlx::query(
            "UPDATE town_inventory_stack SET quantity = 0, updated_at = now() WHERE town_id = $1",
        )
        .bind(town_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "UPDATE hunter_material_stack SET quantity = 0, updated_at = now() WHERE town_id = $1",
        )
        .bind(town_id)
        .execute(&mut **transaction)
        .await?;
        let mut requested_material_ids = Vec::new();
        for stock in &state.material_stocks {
            sqlx::query(
                r#"INSERT INTO town_inventory_stack (town_id, item_id, quantity)
                   VALUES ($1,$2,$3)
                   ON CONFLICT (town_id, item_id) DO UPDATE SET
                       quantity = EXCLUDED.quantity, updated_at = now()"#,
            )
            .bind(town_id)
            .bind(&stock.id)
            .bind(i64::from(stock.town_quantity))
            .execute(&mut **transaction)
            .await?;
            sqlx::query(
                r#"INSERT INTO hunter_material_stack (town_id, material_id, quantity)
                   VALUES ($1,$2,$3)
                   ON CONFLICT (town_id, material_id) DO UPDATE SET
                       quantity = EXCLUDED.quantity, updated_at = now()"#,
            )
            .bind(town_id)
            .bind(&stock.id)
            .bind(i64::from(stock.hunter_quantity))
            .execute(&mut **transaction)
            .await?;
            if stock.requested > 0 {
                requested_material_ids.push(stock.id.clone());
                sqlx::query(
                    r#"INSERT INTO building_material_order (
                           town_id, material_id, requested_quantity,
                           fulfilled_quantity, unit_price, status
                       ) VALUES ($1,$2,$3,0,$4,'open')
                       ON CONFLICT (town_id, material_id) WHERE status = 'open'
                       DO UPDATE SET
                           requested_quantity = EXCLUDED.requested_quantity,
                           fulfilled_quantity = 0,
                           unit_price = EXCLUDED.unit_price,
                           updated_at = now()"#,
                )
                .bind(town_id)
                .bind(&stock.id)
                .bind(i64::from(stock.requested))
                .bind(
                    i64::try_from(stock.unit_price)
                        .map_err(|_| BuildingRepositoryError::NumericBounds)?,
                )
                .execute(&mut **transaction)
                .await?;
            }
        }
        sqlx::query(
            "UPDATE building_material_order SET status = 'cancelled', updated_at = now() \
             WHERE town_id = $1 AND status = 'open' AND NOT (material_id = ANY($2))",
        )
        .bind(town_id)
        .bind(&requested_material_ids)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "UPDATE building_product_stock SET quantity = 0, updated_at = now() WHERE town_id = $1",
        )
        .bind(town_id)
        .execute(&mut **transaction)
        .await?;
        for stock in &state.product_stocks {
            sqlx::query(
                r#"INSERT INTO building_product_stock (
                       town_id, building_instance_id, release_id, building_id,
                       product_id, quantity
                   )
                   SELECT $1, building.instance_id, building.release_id,
                          building.building_id, $3, $4
                   FROM player_building AS building
                   WHERE building.town_id = $1 AND building.instance_id = $2
                   ON CONFLICT (town_id, building_instance_id, product_id) DO UPDATE SET
                       quantity = EXCLUDED.quantity, updated_at = now()"#,
            )
            .bind(town_id)
            .bind(stock.building_instance_id.get())
            .bind(&stock.product_id)
            .bind(i64::from(stock.quantity))
            .execute(&mut **transaction)
            .await?;
        }
        for settlement in &state.trade_settlements {
            let result = sqlx::query(
                r#"INSERT INTO hunter_trade_settlement (
                       town_id, settlement_id, field_trip_id, material_id,
                       quantity, unit_price, total_gold
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7)
                   ON CONFLICT (town_id, settlement_id) DO UPDATE SET
                       settlement_id = EXCLUDED.settlement_id
                   WHERE hunter_trade_settlement.field_trip_id = EXCLUDED.field_trip_id
                     AND hunter_trade_settlement.material_id = EXCLUDED.material_id
                     AND hunter_trade_settlement.quantity = EXCLUDED.quantity
                     AND hunter_trade_settlement.unit_price = EXCLUDED.unit_price
                     AND hunter_trade_settlement.total_gold = EXCLUDED.total_gold"#,
            )
            .bind(town_id)
            .bind(&settlement.settlement_id)
            .bind(
                i64::try_from(settlement.field_trip_id)
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .bind(&settlement.material_id)
            .bind(i64::from(settlement.quantity))
            .bind(
                i64::try_from(settlement.unit_price)
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .bind(
                i64::try_from(settlement.total_gold)
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .execute(&mut **transaction)
            .await?;
            if result.rows_affected() != 1 {
                return Err(BuildingRepositoryError::InvalidTown(
                    "trade settlement identity conflicts with persisted data",
                ));
            }
        }

        let mut retained_ids = Vec::with_capacity(state.buildings.len());
        for building in &state.buildings {
            retained_ids.push(building.instance_id.get());
            sqlx::query(
                r#"INSERT INTO player_building
                       (instance_id, town_id, release_id, building_id, current_level,
                        equipped_skin_id, grid_x, grid_y, use_count, seeded_by)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                   ON CONFLICT (instance_id) DO UPDATE SET
                       current_level = EXCLUDED.current_level,
                       equipped_skin_id = EXCLUDED.equipped_skin_id,
                       grid_x = EXCLUDED.grid_x,
                       grid_y = EXCLUDED.grid_y,
                       use_count = EXCLUDED.use_count,
                       seeded_by = EXCLUDED.seeded_by,
                       updated_at = now()
                   WHERE player_building.town_id = EXCLUDED.town_id
                     AND player_building.release_id = EXCLUDED.release_id
                     AND player_building.building_id = EXCLUDED.building_id"#,
            )
            .bind(building.instance_id.get())
            .bind(town_id)
            .bind(&state.release_id)
            .bind(building.building_id.as_str())
            .bind(i64::from(building.level))
            .bind(
                building
                    .equipped_skin_id
                    .map(|skin| i64::try_from(skin.get()))
                    .transpose()
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .bind(building.grid_x)
            .bind(building.grid_y)
            .bind(i64::from(building.uses))
            .bind(&building.seeded_by)
            .execute(&mut **transaction)
            .await?;
        }
        sqlx::query(
            "DELETE FROM player_building WHERE town_id = $1 AND NOT (instance_id = ANY($2))",
        )
        .bind(town_id)
        .bind(&retained_ids)
        .execute(&mut **transaction)
        .await?;
        Ok(revision)
    }

    async fn lock_catalog(
        transaction: &mut Transaction<'_, Postgres>,
        shared: bool,
    ) -> Result<(), sqlx::Error> {
        let function = if shared {
            "pg_advisory_xact_lock_shared"
        } else {
            "pg_advisory_xact_lock"
        };
        let statement = format!("SELECT {function}(hashtext($1))");
        sqlx::query(&statement)
            .bind(CATALOG_LOCK_KEY)
            .execute(&mut **transaction)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl BuildingRepository for PostgresBuildingRepository {
    async fn load_catalog(
        &self,
        expected_registry_id: &str,
        expected_registry_sha256: &str,
    ) -> Result<BuildingCatalog, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        Self::lock_catalog(&mut transaction, true).await?;

        let actual_registry_sha256 = sqlx::query_scalar::<_, String>(
            "SELECT encode(registry_sha256, 'hex') FROM content_release \
             WHERE release_id = $1 AND lifecycle = 'active'",
        )
        .bind(expected_registry_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| {
            BuildingRepositoryError::ActiveReleaseUnavailable(expected_registry_id.to_owned())
        })?;
        if actual_registry_sha256 != expected_registry_sha256 {
            return Err(BuildingRepositoryError::RegistryHashMismatch {
                expected: expected_registry_sha256.to_owned(),
                actual: actual_registry_sha256,
            });
        }

        let base_rows = sqlx::query(
            r#"SELECT building_id, release_id, display_name, category, source_type,
                      max_instances::bigint AS max_instances,
                      grid_width::bigint AS grid_width,
                      grid_height::bigint AS grid_height, movable, constructible,
                      base_sprite_asset_id
               FROM building_definition
               WHERE release_id = $1
               ORDER BY building_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let level_rows = sqlx::query(
            r#"SELECT building_id, level::bigint AS level, upgrade_duration_ms,
                      inventory_capacity, production_slots::bigint AS production_slots
               FROM building_level_definition
               WHERE release_id = $1
               ORDER BY building_id, level"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let level_cost_rows = sqlx::query(
            r#"SELECT building_id, level::bigint AS level, item_id, quantity
               FROM building_level_cost
               WHERE release_id = $1
               ORDER BY building_id, level, item_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let prerequisite_rows = sqlx::query(
            r#"SELECT building_id, level::bigint AS level, required_building_id,
                      required_level::bigint AS required_level
               FROM building_level_prerequisite
               WHERE release_id = $1
               ORDER BY building_id, level, required_building_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let skin_rows = sqlx::query(
            r#"SELECT building_id, skin_id, family, display_name,
                      required_level::bigint AS required_level,
                      visibility, asset_key, sprite_prefix, visual_resolved
               FROM building_skin_definition
               WHERE release_id = $1
               ORDER BY building_id, skin_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;

        let mut bases = Vec::with_capacity(base_rows.len());
        for row in base_rows {
            bases.push(BaseBuildingDefinition {
                id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                registry_id: row.try_get("release_id")?,
                display_name: row.try_get("display_name")?,
                category: row.try_get("category")?,
                source_type: row.try_get("source_type")?,
                max_instances: to_u32(row.try_get("max_instances")?)?,
                grid_width: to_u16(row.try_get("grid_width")?)?,
                grid_height: to_u16(row.try_get("grid_height")?)?,
                movable: row.try_get("movable")?,
                constructible: row.try_get("constructible")?,
                base_sprite_asset_id: row.try_get("base_sprite_asset_id")?,
            });
        }

        let mut levels = Vec::with_capacity(level_rows.len());
        for row in level_rows {
            levels.push(BuildingLevelDefinition {
                building_id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                level: to_u16(row.try_get("level")?)?,
                upgrade_duration_ms: optional_u64(row.try_get("upgrade_duration_ms")?)?,
                inventory_capacity: optional_u64(row.try_get("inventory_capacity")?)?,
                production_slots: optional_u16(row.try_get("production_slots")?)?,
                costs: Vec::new(),
                prerequisites: Vec::new(),
            });
        }
        for row in level_cost_rows {
            let building_id = BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?;
            let level = to_u16(row.try_get("level")?)?;
            levels
                .iter_mut()
                .find(|definition| {
                    definition.building_id == building_id && definition.level == level
                })
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "cost references unknown building level",
                ))?
                .costs
                .push(EconomyAmount {
                    resource_id: row.try_get("item_id")?,
                    quantity: to_u64(row.try_get("quantity")?)?,
                });
        }
        for row in prerequisite_rows {
            let building_id = BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?;
            let level = to_u16(row.try_get("level")?)?;
            levels
                .iter_mut()
                .find(|definition| {
                    definition.building_id == building_id && definition.level == level
                })
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "prerequisite references unknown building level",
                ))?
                .prerequisites
                .push(BuildingLevelPrerequisite {
                    building_id: BaseBuildingId::parse(
                        row.try_get::<String, _>("required_building_id")?,
                    )?,
                    required_level: to_u16(row.try_get("required_level")?)?,
                });
        }

        let mut skins = Vec::with_capacity(skin_rows.len());
        for row in skin_rows {
            skins.push(BuildingSkinDefinition {
                key: BuildingSkinKey {
                    building_id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                    skin_id: BuildingSkinId::new(to_u64(row.try_get("skin_id")?)?)?,
                },
                family: row.try_get("family")?,
                display_name: row.try_get("display_name")?,
                required_level: to_u16(row.try_get("required_level")?)?,
                visibility: row.try_get("visibility")?,
                asset_key: row.try_get("asset_key")?,
                sprite_prefix: row.try_get("sprite_prefix")?,
                visual_resolved: row.try_get("visual_resolved")?,
            });
        }
        transaction.commit().await?;

        let catalog = BuildingCatalog {
            registry_id: expected_registry_id.to_owned(),
            bases,
            levels,
            skins,
        };
        catalog.validate()?;
        if let Some(actual) = catalog.bases.first().map(|base| base.registry_id.as_str()) {
            if actual != expected_registry_id {
                return Err(BuildingRepositoryError::RegistryMismatch {
                    expected: expected_registry_id.to_owned(),
                    actual: actual.to_owned(),
                });
            }
        }
        Ok(catalog)
    }

    async fn load_gameplay_catalog(
        &self,
        expected_registry_id: &str,
        expected_registry_sha256: &str,
    ) -> Result<BuildingGameplayCatalog, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        Self::lock_catalog(&mut transaction, true).await?;
        let actual_hash = sqlx::query_scalar::<_, String>(
            "SELECT encode(registry_sha256, 'hex') FROM content_release \
             WHERE release_id = $1 AND lifecycle = 'active'",
        )
        .bind(expected_registry_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| {
            BuildingRepositoryError::ActiveReleaseUnavailable(expected_registry_id.to_owned())
        })?;
        if actual_hash != expected_registry_sha256 {
            return Err(BuildingRepositoryError::RegistryHashMismatch {
                expected: expected_registry_sha256.to_owned(),
                actual: actual_hash,
            });
        }

        let capability_rows = sqlx::query(
            r#"SELECT capability_id, building_id, capability_kind,
                      static_data_ready, runnable
               FROM building_capability_definition
               WHERE release_id = $1
               ORDER BY building_id, capability_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let item_rows = sqlx::query(
            r#"SELECT item_id, internal_name, item_type, stack_limit,
                      town_pays_hunter_gold_per_unit
               FROM economy_item_definition
               WHERE release_id = $1
               ORDER BY item_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let localization_rows = sqlx::query(
            r#"SELECT item_id, locale, display_name
               FROM economy_item_localization
               WHERE release_id = $1
               ORDER BY item_id, locale"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let item_price_rows = sqlx::query(
            r#"SELECT item_id, price_direction, ordinal, resource_id, quantity
               FROM economy_item_price_component
               WHERE release_id = $1
               ORDER BY item_id, price_direction, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let tier_price_rows = sqlx::query(
            r#"SELECT item_id, tier, gold
               FROM economy_item_hunter_tier_price
               WHERE release_id = $1
               ORDER BY item_id, tier"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let product_rows = sqlx::query(
            r#"SELECT product_id, building_id, duration_ms, exact_mutation_ready
               FROM economy_product_definition
               WHERE release_id = $1
               ORDER BY product_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let amount_rows = sqlx::query(
            r#"SELECT product_id, amount_kind, ordinal, resource_id, quantity
               FROM economy_product_amount
               WHERE release_id = $1
               ORDER BY product_id, amount_kind, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let service_rows = sqlx::query(
            r#"SELECT product_id, source_type, required_level::bigint AS required_level,
                      service_time_ms, effect_value, use_money, required_cash_count,
                      cash_completion_count, required_elemental_count,
                      elemental_completion_count
               FROM economy_product_service
               WHERE release_id = $1
               ORDER BY product_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let completion_rows = sqlx::query(
            r#"SELECT product_id, ordinal, quantity
               FROM economy_product_service_completion
               WHERE release_id = $1
               ORDER BY product_id, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let conversion_rows = sqlx::query(
            r#"SELECT product_id, ordinal, input_kind, input_resource_id,
                      input_quantity, output_stock_quantity
               FROM economy_product_conversion_option
               WHERE release_id = $1
               ORDER BY product_id, ordinal"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;
        let random_rows = sqlx::query(
            r#"SELECT product_id, item_type, grade, quantity, rng_ready
               FROM economy_product_random_output
               WHERE release_id = $1
               ORDER BY product_id"#,
        )
        .bind(expected_registry_id)
        .fetch_all(&mut *transaction)
        .await?;

        let capabilities = capability_rows
            .into_iter()
            .map(|row| {
                Ok(BuildingCapabilityDefinition {
                    capability_id: row.try_get("capability_id")?,
                    building_id: BaseBuildingId::parse(row.try_get::<String, _>("building_id")?)?,
                    kind: row.try_get("capability_kind")?,
                    static_data_ready: row.try_get("static_data_ready")?,
                    runnable: row.try_get("runnable")?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;

        let mut items = BTreeMap::new();
        for row in item_rows {
            let item_id: String = row.try_get("item_id")?;
            items.insert(
                item_id.clone(),
                EconomyItemDefinition {
                    item_id,
                    internal_name: row.try_get("internal_name")?,
                    item_type: row.try_get("item_type")?,
                    stack_limit: optional_u64(row.try_get("stack_limit")?)?,
                    town_pays_hunter_gold_per_unit: optional_u64(
                        row.try_get("town_pays_hunter_gold_per_unit")?,
                    )?,
                    localized_names: BTreeMap::new(),
                    buy_price: Vec::new(),
                    sell_price: Vec::new(),
                    hunter_pays_town_gold_by_tier: Vec::new(),
                },
            );
        }
        for row in localization_rows {
            if let Some(item) = items.get_mut(row.try_get::<String, _>("item_id")?.as_str()) {
                item.localized_names
                    .insert(row.try_get("locale")?, row.try_get("display_name")?);
            }
        }
        for row in item_price_rows {
            let item_id: String = row.try_get("item_id")?;
            let amount = EconomyAmount {
                resource_id: row.try_get("resource_id")?,
                quantity: to_u64(row.try_get("quantity")?)?,
            };
            let item = items
                .get_mut(&item_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "item price references unknown item",
                ))?;
            match row.try_get::<String, _>("price_direction")?.as_str() {
                "buy" => item.buy_price.push(amount),
                "sell" => item.sell_price.push(amount),
                _ => {
                    return Err(BuildingRepositoryError::InvalidCatalog(
                        "invalid item price direction",
                    ))
                }
            }
        }
        for row in tier_price_rows {
            let item_id: String = row.try_get("item_id")?;
            items
                .get_mut(&item_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "tier price references unknown item",
                ))?
                .hunter_pays_town_gold_by_tier
                .push(to_u64(row.try_get("gold")?)?);
        }

        let mut products = BTreeMap::new();
        for row in product_rows {
            let product_id: String = row.try_get("product_id")?;
            products.insert(
                product_id.clone(),
                EconomyProductDefinition {
                    product_id,
                    building_id: row
                        .try_get::<Option<String>, _>("building_id")?
                        .map(BaseBuildingId::parse)
                        .transpose()?,
                    duration_ms: optional_u64(row.try_get("duration_ms")?)?,
                    exact_mutation_ready: row.try_get("exact_mutation_ready")?,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    sale_price: Vec::new(),
                    service: None,
                    conversion_options: Vec::new(),
                    random_output: None,
                },
            );
        }
        for row in amount_rows {
            let product_id: String = row.try_get("product_id")?;
            let product =
                products
                    .get_mut(&product_id)
                    .ok_or(BuildingRepositoryError::InvalidCatalog(
                        "amount references unknown product",
                    ))?;
            let amount = EconomyAmount {
                resource_id: row.try_get("resource_id")?,
                quantity: to_u64(row.try_get("quantity")?)?,
            };
            match row.try_get::<String, _>("amount_kind")?.as_str() {
                "input" => product.inputs.push(amount),
                "output" => product.outputs.push(amount),
                "sale_price" => product.sale_price.push(amount),
                _ => {
                    return Err(BuildingRepositoryError::InvalidCatalog(
                        "invalid product amount kind",
                    ))
                }
            }
        }
        for row in service_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "service references unknown product",
                ))?
                .service = Some(EconomyProductService {
                source_type: to_u64(row.try_get("source_type")?)?,
                required_level: to_u16(row.try_get("required_level")?)?,
                service_time_ms: to_u64(row.try_get("service_time_ms")?)?,
                effect_value: to_u64(row.try_get("effect_value")?)?,
                use_money: to_u64(row.try_get("use_money")?)?,
                completion_counts: Vec::new(),
                required_cash_count: to_u64(row.try_get("required_cash_count")?)?,
                cash_completion_count: to_u64(row.try_get("cash_completion_count")?)?,
                required_elemental_count: to_u64(row.try_get("required_elemental_count")?)?,
                elemental_completion_count: to_u64(row.try_get("elemental_completion_count")?)?,
            });
        }
        for row in completion_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .and_then(|product| product.service.as_mut())
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "completion references unknown service",
                ))?
                .completion_counts
                .push(to_u64(row.try_get("quantity")?)?);
        }
        for row in conversion_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "conversion references unknown product",
                ))?
                .conversion_options
                .push(EconomyConversionOption {
                    input_kind: row.try_get("input_kind")?,
                    input_resource_id: row.try_get("input_resource_id")?,
                    input_quantity: to_u64(row.try_get("input_quantity")?)?,
                    output_stock_quantity: to_u64(row.try_get("output_stock_quantity")?)?,
                });
        }
        for row in random_rows {
            let product_id: String = row.try_get("product_id")?;
            products
                .get_mut(&product_id)
                .ok_or(BuildingRepositoryError::InvalidCatalog(
                    "random output references unknown product",
                ))?
                .random_output = Some(EconomyRandomOutput {
                item_type: row.try_get("item_type")?,
                grade: to_u64(row.try_get("grade")?)?,
                quantity: to_u64(row.try_get("quantity")?)?,
                rng_ready: row.try_get("rng_ready")?,
            });
        }
        transaction.commit().await?;
        let gameplay = BuildingGameplayCatalog {
            registry_id: expected_registry_id.to_owned(),
            capabilities,
            items,
            products,
        };
        let catalog = self
            .load_catalog(expected_registry_id, expected_registry_sha256)
            .await?;
        gameplay.validate(&catalog)?;
        Ok(gameplay)
    }
}

fn to_u64(value: i64) -> Result<u64, BuildingRepositoryError> {
    value
        .try_into()
        .map_err(|_| BuildingRepositoryError::NumericBounds)
}

fn to_u32(value: i64) -> Result<u32, BuildingRepositoryError> {
    value
        .try_into()
        .map_err(|_| BuildingRepositoryError::NumericBounds)
}

fn to_u16(value: i64) -> Result<u16, BuildingRepositoryError> {
    value
        .try_into()
        .map_err(|_| BuildingRepositoryError::NumericBounds)
}

fn optional_u64(value: Option<i64>) -> Result<Option<u64>, BuildingRepositoryError> {
    value.map(to_u64).transpose()
}

fn optional_u16(value: Option<i64>) -> Result<Option<u16>, BuildingRepositoryError> {
    value.map(to_u16).transpose()
}

fn optional_skin_id(value: Option<i64>) -> Result<Option<BuildingSkinId>, BuildingRepositoryError> {
    value
        .map(|value| BuildingSkinId::new(to_u64(value)?))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(id: &str) -> BaseBuildingDefinition {
        BaseBuildingDefinition {
            id: BaseBuildingId::parse(id).unwrap(),
            registry_id: "release-1".into(),
            display_name: "Town Hall".into(),
            category: Some("town".into()),
            source_type: 1,
            max_instances: 1,
            grid_width: 3,
            grid_height: 3,
            movable: Some(true),
            constructible: Some(true),
            base_sprite_asset_id: Some("town-hall".into()),
        }
    }

    fn level(id: &str) -> BuildingLevelDefinition {
        BuildingLevelDefinition {
            building_id: BaseBuildingId::parse(id).unwrap(),
            level: 1,
            upgrade_duration_ms: Some(0),
            inventory_capacity: None,
            production_slots: None,
            costs: Vec::new(),
            prerequisites: Vec::new(),
        }
    }

    #[test]
    fn base_and_skin_identifiers_cannot_be_confused() {
        assert!(BaseBuildingId::parse("build_0").is_ok());
        assert!(BaseBuildingId::parse("build_1").is_ok());
        assert!(BaseBuildingId::parse("buildSkin_1_1").is_err());
        assert!(BaseBuildingId::parse("build_01").is_err());
        assert!(BuildingSkinId::new(0).is_err());
    }

    #[test]
    fn catalog_rejects_skin_without_a_base_row() {
        let catalog = BuildingCatalog {
            registry_id: "release-1".into(),
            bases: vec![base("build_1")],
            levels: vec![level("build_1")],
            skins: vec![BuildingSkinDefinition {
                key: BuildingSkinKey {
                    building_id: BaseBuildingId::parse("build_2").unwrap(),
                    skin_id: BuildingSkinId::new(1).unwrap(),
                },
                family: "default".into(),
                display_name: "Skin".into(),
                required_level: 1,
                visibility: 1,
                asset_key: None,
                sprite_prefix: None,
                visual_resolved: false,
            }],
        };

        assert!(matches!(
            catalog.validate(),
            Err(BuildingRepositoryError::UnknownSkinBase(_))
        ));
    }

    #[test]
    fn catalog_rejects_mixed_content_releases() {
        let mut second = base("build_2");
        second.registry_id = "release-2".into();
        let catalog = BuildingCatalog {
            registry_id: "release-1".into(),
            bases: vec![base("build_1"), second],
            levels: vec![level("build_1"), level("build_2")],
            skins: Vec::new(),
        };

        assert!(matches!(
            catalog.validate(),
            Err(BuildingRepositoryError::MixedRegistryRelease)
        ));
    }

    #[test]
    fn town_rejects_duplicate_instance_ids_before_persistence() {
        let instance_id = TownBuildingInstanceId::new(Uuid::new_v4());
        let building = TownBuildingInstance {
            instance_id,
            building_id: BaseBuildingId::parse("build_1").unwrap(),
            equipped_skin_id: None,
            level: 1,
            uses: 0,
            grid_x: 0,
            grid_y: 0,
            seeded_by: None,
        };
        let state = TownBuildingState {
            release_id: "release-1".into(),
            town_gold: 0,
            seed_version: 0,
            next_building_sequence: 2,
            buildings: vec![building.clone(), building],
            hunter_materials: 0,
            materials: 0,
            runes: 0,
            weapons: 0,
            armor: 0,
            hunter_equipment_purchases: 0,
            field_trip_id: 0,
            settled_field_trip_id: 0,
            material_stocks: Vec::new(),
            product_stocks: Vec::new(),
            trade_settlements: Vec::new(),
        };

        assert!(matches!(
            state.validate(),
            Err(BuildingRepositoryError::DuplicateInstance(id)) if id == instance_id
        ));
    }

    #[test]
    fn gameplay_catalog_rejects_unknown_building_references() {
        let catalog = BuildingCatalog {
            registry_id: "release-1".into(),
            bases: vec![base("build_1")],
            levels: vec![level("build_1")],
            skins: Vec::new(),
        };
        let gameplay = BuildingGameplayCatalog {
            registry_id: "release-1".into(),
            capabilities: vec![BuildingCapabilityDefinition {
                capability_id: "capability:craft".into(),
                building_id: BaseBuildingId::parse("build_2").unwrap(),
                kind: "craft".into(),
                static_data_ready: true,
                runnable: false,
            }],
            items: BTreeMap::new(),
            products: BTreeMap::new(),
        };

        assert!(matches!(
            gameplay.validate(&catalog),
            Err(BuildingRepositoryError::UnknownGameplayBase(id))
                if id == BaseBuildingId::parse("build_2").unwrap()
        ));
    }

    #[tokio::test]
    async fn migrated_catalog_loads_complete_normalized_content() {
        let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
            return;
        };
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .unwrap();
        let release_id = "evil-hunter-1.411.buildings-v1";
        let registry_hash = sqlx::query_scalar::<_, String>(
            "SELECT encode(registry_sha256, 'hex') FROM content_release WHERE release_id = $1",
        )
        .bind(release_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let repository = PostgresBuildingRepository::from_pool(pool);

        let catalog = repository
            .load_catalog(release_id, &registry_hash)
            .await
            .unwrap();
        assert_eq!(catalog.bases.len(), 79);
        assert_eq!(catalog.levels.len(), 227);
        assert_eq!(catalog.skins.len(), 61);
        assert_eq!(
            catalog
                .levels
                .iter()
                .map(|definition| definition.costs.len())
                .sum::<usize>(),
            402
        );
        assert_eq!(
            catalog
                .levels
                .iter()
                .map(|definition| definition.prerequisites.len())
                .sum::<usize>(),
            227
        );

        let gameplay = repository
            .load_gameplay_catalog(release_id, &registry_hash)
            .await
            .unwrap();
        assert_eq!(gameplay.capabilities.len(), 10);
        assert_eq!(gameplay.items.len(), 1_107);
        assert_eq!(gameplay.products.len(), 3_457);
        gameplay.validate(&catalog).unwrap();
    }
}
