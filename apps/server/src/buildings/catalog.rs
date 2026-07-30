use std::collections::{BTreeMap, HashSet};

use super::{BaseBuildingId, BuildingRepositoryError, BuildingSkinId, BuildingSkinKey};

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
