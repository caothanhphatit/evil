use super::{
    map_configs, Deserialize, DurableHunterRosterState, DurableHunterState, DurablePlayerState,
    HunterAgentState, Serialize, ServiceEffectKind, Uuid,
};

pub const DURABLE_PLAYER_SCHEMA_VERSION: u16 = 17;
pub const MIGRATION_FIXTURE_CONTENT_ID: &str = "migration-fixture.slice1-combat-v1";
pub const MAX_GEAR_ENHANCEMENT_LEVEL: u8 = 20;

pub(super) const TOWN_GRID_MIN: i32 = -32;
pub(super) const TOWN_GRID_MAX: i32 = 32;
pub(super) const MAX_PRODUCTION_QUANTITY: u32 = 1_000;
pub(super) const TOWN_NAV_CELL_WIDTH: i32 = 24;
pub(super) const TOWN_NAV_CELL_HEIGHT: i32 = 18;
pub(super) const TOWN_NAV_ORIGIN_X: i32 = 1627;
pub(super) const TOWN_NAV_ORIGIN_Y: i32 = 600;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalScreen {
    #[default]
    Boot,
    Village,
    HunterRoster,
    Field,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BottomMenuIntent {
    Build,
    Character,
    Archive,
    Store,
    Raid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldMode {
    Inactive,
    Village,
    Field,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldEntityKind {
    Hunter,
    Npc,
    Monster,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldEntityActionState {
    Idle,
    Walking,
    Attacking,
    Dead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    Right,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OriginalFlowPlayerState {
    pub screen: OriginalScreen,
    pub boot_completed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableBuildingState {
    pub town_gold: u64,
    pub buildings: Vec<DurableBuilding>,
    pub hunter_materials: u32,
    pub materials: u32,
    pub runes: u32,
    pub weapons: u32,
    pub armor: u32,
    pub material_stocks: Vec<DurableMaterialStock>,
    pub product_stocks: Vec<DurableProductStock>,
    pub crafted_gear_stocks: Vec<DurableCraftedGearStock>,
    pub hunter_equipment_purchases: u32,
    pub town_seed_version: u16,
    pub next_building_instance_id: u64,
    pub field_trip_id: u64,
    pub settled_field_trip_id: u64,
    pub trade_settlements: Vec<DurableTradeSettlement>,
}

impl Default for DurableBuildingState {
    fn default() -> Self {
        Self {
            town_gold: 1_500,
            buildings: Vec::new(),
            hunter_materials: 20,
            materials: 0,
            runes: 0,
            weapons: 0,
            armor: 0,
            material_stocks: default_material_stocks(),
            product_stocks: Vec::new(),
            crafted_gear_stocks: Vec::new(),
            hunter_equipment_purchases: 0,
            town_seed_version: 0,
            next_building_instance_id: 1,
            field_trip_id: 0,
            settled_field_trip_id: 0,
            trade_settlements: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableMaterialStock {
    pub id: String,
    pub town_quantity: u32,
    pub hunter_quantity: u32,
    pub requested: u32,
    pub unit_price: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableProductStock {
    pub building_instance_id: String,
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableCraftedGearStock {
    pub building_instance_id: String,
    pub gear_instance_id: Uuid,
    pub product_id: String,
    pub gear_kind: String,
    pub rating: u16,
    pub quality: u8,
    pub primary_stat: u32,
    pub option_type: u8,
    pub option_value: u16,
    pub icon_path: String,
    pub ruleset: String,
}

pub(super) fn default_material_stocks() -> Vec<DurableMaterialStock> {
    Vec::new()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableBuilding {
    pub instance_id: String,
    /// Always references a row in building_base_catalog, never a skin asset key.
    pub id: String,
    pub equipped_skin_id: Option<u64>,
    pub level: u8,
    pub uses: u32,
    pub grid_x: i32,
    pub grid_y: i32,
    pub seeded_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableTradeSettlement {
    pub settlement_id: String,
    pub field_trip_id: u64,
    pub material_id: String,
    pub quantity: u32,
    pub unit_price: u64,
    pub total_gold: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurablePlayerAggregate {
    pub schema_version: u16,
    pub navigation: OriginalFlowPlayerState,
    pub migration_fixture_combat: DurablePlayerState,
    pub buildings: DurableBuildingState,
    pub hunter_roster: DurableHunterRosterState,
    pub product_services: DurableProductServiceState,
    pub monster_field_config: DurableMonsterFieldConfig,
    /// Durable continuity for active Hunters only. Monster actors and ground
    /// drops remain reconstructable runtime state.
    pub hunter_world_runtime: Vec<HunterAgentState>,
    #[serde(default, rename = "infirmary", skip_serializing)]
    pub legacy_infirmary: Option<DurableLegacyInfirmaryState>,
}

impl Default for DurablePlayerAggregate {
    fn default() -> Self {
        Self {
            schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
            navigation: OriginalFlowPlayerState::default(),
            migration_fixture_combat: DurablePlayerState::default(),
            buildings: DurableBuildingState::default(),
            hunter_roster: DurableHunterRosterState::default(),
            product_services: DurableProductServiceState::default(),
            monster_field_config: DurableMonsterFieldConfig::default(),
            hunter_world_runtime: Vec::new(),
            legacy_infirmary: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableMonsterFieldConfig {
    pub densities: Vec<DurableMonsterMapDensity>,
    #[serde(default, rename = "tier_id", skip_serializing)]
    pub legacy_map_id: Option<String>,
    #[serde(default, rename = "density_level", skip_serializing)]
    pub legacy_density_level: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableMonsterMapDensity {
    pub map_id: String,
    pub density_level: u8,
}

impl Default for DurableMonsterFieldConfig {
    fn default() -> Self {
        Self {
            densities: map_configs()
                .iter()
                .map(|config| DurableMonsterMapDensity {
                    map_id: config.map_id.to_owned(),
                    density_level: 1,
                })
                .collect(),
            legacy_map_id: None,
            legacy_density_level: None,
        }
    }
}

impl DurableMonsterFieldConfig {
    pub(super) fn normalized_densities(&self) -> Vec<DurableMonsterMapDensity> {
        map_configs()
            .iter()
            .map(|config| {
                let configured = (self.legacy_map_id.as_deref() == Some(config.map_id))
                    .then_some(self.legacy_density_level)
                    .flatten()
                    .or_else(|| {
                        self.densities
                            .iter()
                            .find(|density| density.map_id == config.map_id)
                            .map(|density| density.density_level)
                    })
                    .filter(|level| (1..=3).contains(level))
                    .unwrap_or(1);
                DurableMonsterMapDensity {
                    map_id: config.map_id.to_owned(),
                    density_level: configured,
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableProductServiceState {
    pub visits: Vec<DurableProductServiceVisit>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableLegacyInfirmaryState {
    pub roster_resolved: bool,
    pub hunters: Vec<DurableHunterState>,
    pub treatments: Vec<DurableInfirmaryTreatment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableInfirmaryTreatment {
    pub hunter_id: u32,
    pub building_instance_id: String,
    pub product_id: String,
    pub remaining_ms: u64,
    pub effect_value: u64,
    pub payment_gold: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableProductServiceVisit {
    pub hunter_id: u32,
    pub building_instance_id: String,
    pub building_id: String,
    pub product_id: String,
    pub effect_kind: ServiceEffectKind,
    pub remaining_ms: u64,
    pub effect_value: u64,
    pub payment_gold: u64,
}
