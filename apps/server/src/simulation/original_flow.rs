use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::buildings::{
    gear_product_route, AuthoritativeBuildingContent, BaseBuildingDefinition, BaseBuildingId,
    BuildingGameplayCatalog, BuildingLevelDefinition, EconomyAmount,
};
#[cfg(test)]
use crate::buildings::{
    BuildingCapabilityDefinition, BuildingCatalog, BuildingLevelPrerequisite,
    EconomyItemDefinition, EconomyProductDefinition, EconomyProductService,
};

#[cfg(test)]
use super::hunter_roster::operational_migration_roster;
#[cfg(test)]
use super::hunter_roster::DurableHunterProfile;
use super::hunter_roster::{
    DurableGearEnhancementTask, DurableHunterRosterState, DurableHunterState,
    GearEnhancementTaskStatus, HunterRosterError, GEAR_ENHANCEMENT_WORKFLOW_VERSION,
    HUNT_TICKS_TO_RETURN, MAX_ACTIVE_TOWN_HUNTERS,
};
use super::product_service::{capacity_for_level, HunterServiceGauge, ServiceEffectKind};
#[cfg(test)]
use super::trading_post::ACTIVE_MATERIAL_REQUEST;
use super::trading_post::{
    material_catalog_stocks, material_difficulty_rating, settle_returning_hunters,
};
use super::{
    map_config, ClientCommand, DurablePlayerState, FixtureCommand, HunterActionState,
    HunterAgentState, HunterEvidenceState, MonsterActionState, MonsterState, MonsterWorldState,
    NavigationObstacle, PendingOperation, ServerMessage, Simulation, WorldSnapshot, MAP_CONFIGS,
    MONSTER_RULESET,
};

pub const DURABLE_PLAYER_SCHEMA_VERSION: u16 = 15;
pub const MIGRATION_FIXTURE_CONTENT_ID: &str = "migration-fixture.slice1-combat-v1";
pub const MAX_GEAR_ENHANCEMENT_LEVEL: u8 = 20;

const TOWN_GRID_MIN: i32 = -32;
const TOWN_GRID_MAX: i32 = 32;
const MAX_PRODUCTION_QUANTITY: u32 = 1_000;
const TOWN_NAV_CELL_SIZE: i32 = 24;
const TOWN_NAV_ORIGIN_X: i32 = 1627;
const TOWN_NAV_ORIGIN_Y: i32 = 600;

const QUEST_BLOCKERS: [&str; 2] = ["quest_catalog_binding", "quest_reward_binding"];
const SHOP_BLOCKERS: [&str; 2] = ["shop_catalog_binding", "shop_price_binding"];
const MAIL_BLOCKERS: [&str; 2] = ["mail_schema_binding", "mail_grant_binding"];
const REWARDED_AD_BLOCKERS: [&str; 2] = ["ad_placement_binding", "ad_reward_binding"];
const TOPUP_BLOCKERS: [&str; 3] = [
    "product_catalog_binding",
    "provider_receipt_binding",
    "entitlement_rules_binding",
];
const BUILDING_CAPABILITY_BLOCKERS: [&str; 2] = [
    "building_capability_dispatch_binding",
    "building_economy_settlement_binding",
];
const GEAR_ENHANCEMENT_BLOCKERS: [&str; 3] = [
    "enhancement_cost_binding",
    "enhancement_probability_binding",
    "enhancement_material_binding",
];

#[derive(Clone, Copy)]
struct BasicHunterSkillDefinition {
    skill_id: &'static str,
    display_name: &'static str,
    class_id: &'static str,
    class_family: &'static str,
    cooldown_ms: u64,
    confirmed_icon_path: Option<&'static str>,
}

fn basic_hunter_skill_definition(skill_id: &str) -> Option<BasicHunterSkillDefinition> {
    Some(match skill_id {
        "skill_h1_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h1_01",
            display_name: "Fury",
            class_id: "h1",
            class_family: "H1",
            cooldown_ms: 15_000,
            confirmed_icon_path: Some("sprites/skill_h1_01__1395.png"),
        },
        "skill_h1_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h1_02",
            display_name: "War Cry",
            class_id: "h1",
            class_family: "H1",
            cooldown_ms: 16_000,
            confirmed_icon_path: Some("sprites/skill_h1_02__5620.png"),
        },
        "skill_h2_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h2_01",
            display_name: "Holy Light",
            class_id: "h2",
            class_family: "H2",
            cooldown_ms: 8_000,
            confirmed_icon_path: None,
        },
        "skill_h2_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h2_02",
            display_name: "Barrier",
            class_id: "h2",
            class_family: "H2",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        "skill_h3_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h3_01",
            display_name: "Multishot",
            class_id: "h3",
            class_family: "H3",
            cooldown_ms: 6_000,
            confirmed_icon_path: None,
        },
        "skill_h3_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h3_02",
            display_name: "Dodge",
            class_id: "h3",
            class_family: "H3",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        "skill_h4_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h4_01",
            display_name: "Thunderbolt",
            class_id: "h4",
            class_family: "H4",
            cooldown_ms: 6_000,
            confirmed_icon_path: None,
        },
        "skill_h4_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h4_02",
            display_name: "Ice Armor",
            class_id: "h4",
            class_family: "H4",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        "skill_h5_01" => BasicHunterSkillDefinition {
            skill_id: "skill_h5_01",
            display_name: "Round Slash",
            class_id: "h5",
            class_family: "H5",
            cooldown_ms: 6_000,
            confirmed_icon_path: None,
        },
        "skill_h5_02" => BasicHunterSkillDefinition {
            skill_id: "skill_h5_02",
            display_name: "Concentrate",
            class_id: "h5",
            class_family: "H5",
            cooldown_ms: 16_000,
            confirmed_icon_path: None,
        },
        _ => return None,
    })
}

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

fn default_material_stocks() -> Vec<DurableMaterialStock> {
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
            densities: MAP_CONFIGS
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
    fn normalized_densities(&self) -> Vec<DurableMonsterMapDensity> {
        MAP_CONFIGS
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingConfidence {
    Confirmed,
    StronglyInferred,
    Tentative,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvidenceBinding {
    pub id: &'static str,
    pub confidence: BindingConfidence,
    pub resolved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VillageSnapshot {
    pub source_scene: &'static str,
    pub canvas_nodes: Vec<&'static str>,
    pub world_nodes: Vec<&'static str>,
    pub bottom_menu: Vec<BottomMenuIntent>,
    pub bindings: Vec<EvidenceBinding>,
    pub building_system: BuildingSystemSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuildingDefinitionSnapshot {
    pub id: String,
    pub name: String,
    pub feature: String,
    pub max_level: u8,
    pub construct_cost: u64,
    pub prerequisite_id: Option<String>,
    pub prerequisite_level: u8,
    pub max_build: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub sprite_asset_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuildingStateSnapshot {
    pub id: String,
    pub constructed: bool,
    pub level: u8,
    pub upgrade_cost: Option<u64>,
    pub can_construct: bool,
    pub can_upgrade: bool,
    pub condition: Option<String>,
    pub uses: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuildingInstanceSnapshot {
    pub instance_id: String,
    pub building_id: String,
    pub level: u8,
    pub grid_x: i32,
    pub grid_y: i32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub sprite_asset_id: Option<String>,
    pub upgrade_cost: Option<u64>,
    pub can_upgrade: bool,
    pub condition: Option<String>,
    pub uses: u32,
    pub seeded_by: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuildingSystemSnapshot {
    pub evidence_label: &'static str,
    pub town_gold: u64,
    pub definitions: Vec<BuildingDefinitionSnapshot>,
    pub states: Vec<BuildingStateSnapshot>,
    pub instances: Vec<BuildingInstanceSnapshot>,
    pub hunter_materials: u32,
    pub materials: u32,
    pub runes: u32,
    pub weapons: u32,
    pub armor: u32,
    pub hunter_equipment_purchases: u32,
    pub material_stocks: Vec<MaterialStockSnapshot>,
    pub recipes: Vec<ShopRecipeSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MaterialStockSnapshot {
    pub id: String,
    pub display_name: String,
    pub icon: String,
    pub town_quantity: u32,
    pub hunter_quantity: u32,
    pub requested: u32,
    pub unit_price: u64,
    pub difficulty: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecipeMaterialCostSnapshot {
    pub material_id: String,
    pub display_name: String,
    pub quantity: u32,
    pub output_quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShopRecipeSnapshot {
    pub id: String,
    pub shop_id: String,
    pub icon: String,
    pub product_name: String,
    pub material_costs: Vec<RecipeMaterialCostSnapshot>,
    pub stock: u32,
    pub sale_price: u64,
    pub kind: &'static str,
    pub required_level: u16,
    pub duration_ms: u64,
    pub effect_value: u64,
    pub effect_kind: &'static str,
    pub capacity: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HunterRosterSnapshot {
    pub scene_nodes: Vec<&'static str>,
    pub hunter_spine_source_confirmed: bool,
    pub starter_composition_resolved: bool,
    pub starter_stats_resolved: bool,
    pub bindings: Vec<EvidenceBinding>,
    pub active_capacity: usize,
    pub active_hunters: Vec<HunterRosterMemberSnapshot>,
    pub waiting_hunters: Vec<HunterRosterMemberSnapshot>,
    pub infirmary: InfirmaryServiceSnapshot,
    pub product_services: Vec<ProductServiceSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HunterRosterMemberSnapshot {
    pub hunter_id: u32,
    pub display_name: String,
    pub portrait_asset_id: Option<String>,
    pub class_id: String,
    pub class_name: String,
    pub class_family: String,
    pub rarity_id: String,
    pub rarity_name: String,
    pub level: u32,
    pub xp: u64,
    pub gold: u64,
    pub current_hp: u64,
    pub max_hp: u64,
    pub stamina: u64,
    pub max_stamina: u64,
    pub satiety: u64,
    pub max_satiety: u64,
    pub mood: u64,
    pub max_mood: u64,
    pub attack: u64,
    pub defense: u64,
    pub action_state: String,
    pub animation: String,
    pub trait_name: Option<String>,
    pub traits: Vec<HunterTraitSnapshot>,
    pub skills: Vec<HunterSkillSnapshot>,
    pub hunt: HunterHuntSnapshot,
    pub hunter_info: HunterInfoSnapshot,
    pub gear_enhancements: Vec<GearEnhancementSnapshot>,
    pub gear_enhancement_task: Option<GearEnhancementTaskSnapshot>,
    pub runtime_evidence: HunterRuntimeEvidenceSnapshot,
    pub roster_state: &'static str,
    pub position: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GearEnhancementSnapshot {
    pub product_id: String,
    pub level: Option<u8>,
    pub max_level: u8,
    pub instance_id: Option<Uuid>,
    pub evidence_state: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GearEnhancementResourceSnapshot {
    pub material_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GearEnhancementAttemptSnapshot {
    pub attempt: u32,
    pub starting_level: u8,
    pub resulting_level: u8,
    pub succeeded: bool,
    pub gold_spent: u64,
    pub materials_spent: Vec<GearEnhancementResourceSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GearEnhancementTaskSnapshot {
    pub building_instance_id: String,
    pub status: &'static str,
    pub interaction_ready: bool,
    pub selected_gear_instance_id: Option<Uuid>,
    pub selected_product_id: Option<String>,
    pub mode: Option<String>,
    pub target_level: Option<u8>,
    pub optional_material_ids: Vec<String>,
    pub next_attempt_gold_cost: Option<u64>,
    pub next_attempt_success_bps: Option<u32>,
    pub required_materials: Vec<GearEnhancementResourceSnapshot>,
    pub attempts: Vec<GearEnhancementAttemptSnapshot>,
    pub spent_gold: u64,
    pub spent_materials: Vec<GearEnhancementResourceSnapshot>,
    pub final_level: Option<u8>,
    pub stop_reason: Option<String>,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterLootSnapshot {
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterHuntSnapshot {
    pub status: String,
    pub zone_id: Option<String>,
    pub progress_ticks: u32,
    pub required_ticks: u32,
    pub loot: Vec<HunterLootSnapshot>,
    pub ruleset: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HunterEvidenceSection<T> {
    pub evidence_state: HunterEvidenceState,
    pub value: Option<T>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HunterRuntimeEvidenceSnapshot {
    pub source_key: Option<String>,
    pub source_index: Option<i32>,
    pub job: HunterEvidenceSection<HunterRuntimeJobSnapshot>,
    pub status: HunterEvidenceSection<HunterRuntimeStatusSnapshot>,
    pub skills: HunterEvidenceSection<Vec<HunterRuntimeSkillSnapshot>>,
    pub appearance: HunterEvidenceSection<HunterRuntimeAppearanceSnapshot>,
    pub inventory: HunterEvidenceSection<HunterRuntimeInventorySnapshot>,
    pub growth: HunterEvidenceSection<Vec<HunterRuntimeGrowthSnapshot>>,
    pub riding_pet: HunterEvidenceSection<HunterRuntimeRidingPetSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRuntimeAppearanceSnapshot {
    pub body_index: i32,
    pub costume_index: i32,
    pub costume_hidden: bool,
    pub fairy_index: i32,
    pub fairy_hidden: bool,
    pub weapon_costume_index: i32,
    pub weapon_costume_hidden: bool,
    pub wing_costume_index: i32,
    pub wing_costume_hidden: bool,
    pub seal_costume_index: i32,
    pub seal_costume_hidden: bool,
    pub companion_index: i32,
    pub companion_hidden: bool,
    pub hat_hidden: bool,
    pub costume_hat_hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRuntimeJobSnapshot {
    pub job: i32,
    pub sub_job: i32,
    pub third_job: i32,
    pub fourth_job: i32,
    pub personality: i32,
    pub grade_rank_up: Option<i32>,
    pub dark_soul: Option<i64>,
    pub used_dark_soul: Option<i64>,
    pub used_job_trait: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HunterRuntimeStatusSnapshot {
    pub maximum_hp: i64,
    pub current_hp: i64,
    pub maximum_mood: f32,
    pub current_mood: f32,
    pub maximum_satiety: f32,
    pub current_satiety: f32,
    pub maximum_stamina: f32,
    pub current_stamina: f32,
    pub attack: i64,
    pub defense: i64,
    pub critical: i32,
    pub attack_speed: f32,
    pub evasion: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HunterRuntimeSkillSnapshot {
    pub source_key: String,
    pub source_index: i32,
    pub skill_definition_index: i32,
    pub cooldown_raw: f64,
    pub level: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HunterRuntimeInventorySnapshot {
    pub items: Vec<HunterRuntimeItemSnapshot>,
    pub gear: Vec<HunterRuntimeGearSnapshot>,
    pub consumables: Vec<HunterRuntimeConsumableSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRuntimeItemSnapshot {
    pub source_key: String,
    pub definition_index: i32,
    pub count: i64,
    pub reserved_count: i64,
    pub is_new: bool,
    pub is_infinite: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRuntimeGearSnapshot {
    pub source_key: String,
    pub definition_index: i32,
    pub inventory_index: i32,
    pub quality: i32,
    pub level: i32,
    pub rating: i32,
    pub group: i32,
    pub is_new: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRuntimeConsumableSnapshot {
    pub source_key: String,
    pub total_count: i32,
    pub nested_values_resolved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRuntimeGrowthSnapshot {
    pub property_order: i16,
    pub level: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRuntimeRidingPetSnapshot {
    pub pasture_index: i32,
    pub definition_index: i32,
    pub master_key: String,
    pub rating: i32,
    pub skill_index: i32,
    pub trait_index: i32,
    pub trait_level: i32,
    pub used_soul: i32,
    pub used_growth_stone: i32,
    pub locked: bool,
    pub gear_values_resolved: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HunterInfoSnapshot {
    pub characteristic_name: Option<String>,
    pub locked: Option<bool>,
    pub reincarnation: Option<HunterProgressSnapshot>,
    pub experience: Option<HunterProgressSnapshot>,
    pub status: HunterStatusSnapshot,
    pub equipment_slots: Option<Vec<HunterEquipmentSlotSnapshot>>,
    pub skills: Option<Vec<HunterInfoSkillSnapshot>>,
    pub growth: Option<HunterGrowthSnapshot>,
    pub riding_pet: Option<HunterRidingPetSnapshot>,
    pub materials: Option<Vec<HunterMaterialSnapshot>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HunterProgressSnapshot {
    pub current: u64,
    pub maximum: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HunterStatusSnapshot {
    pub dps_milli: Option<u64>,
    pub critical_rate_bps: Option<u32>,
    pub attack_speed_milli: Option<u32>,
    pub evasion_rate_bps: Option<u32>,
    pub awakening: Option<HunterProgressSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterEquipmentSlotSnapshot {
    pub slot_id: String,
    pub catalog_kind: String,
    pub catalog_index: u32,
    pub display_name: String,
    pub icon_path: Option<String>,
    pub placeholder_icon_path: Option<String>,
    pub presentation_gender: String,
    pub required_class_id: Option<String>,
    pub locked: Option<bool>,
    pub evidence_state: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterInfoSkillSnapshot {
    pub skill_id: String,
    pub display_name: String,
    pub icon_path: Option<String>,
    pub level: Option<u8>,
    pub description: Option<String>,
    pub group: Option<String>,
    pub unlocked: Option<bool>,
    pub unlock_requirement: Option<String>,
    pub ready: Option<bool>,
    pub cooldown_remaining_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterGrowthSnapshot {
    pub secret_points: u32,
    pub nodes: Vec<HunterGrowthNodeSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterGrowthNodeSnapshot {
    pub node_id: String,
    pub icon_path: Option<String>,
    pub points: u32,
    pub max_points: Option<u32>,
    pub order: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum HunterRidingPetSnapshot {
    Empty {
        mounted: bool,
        can_move_to_ranch: bool,
    },
    Mounted {
        mounted: bool,
        display_name: Option<String>,
        icon_path: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterMaterialSnapshot {
    pub material_id: String,
    pub display_name: Option<String>,
    pub icon_path: String,
    pub quantity: u64,
    pub order: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterTraitSnapshot {
    pub trait_id: String,
    pub display_name: String,
    pub icon_path: String,
    pub unlocked_rank: u8,
    pub equipped: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterSkillSnapshot {
    pub skill_id: String,
    pub display_name: String,
    pub icon_path: Option<String>,
    pub animation_name: Option<String>,
    pub level: u8,
    pub equipped_slot: Option<u8>,
    pub ready: bool,
    pub cooldown_remaining_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProductServiceSnapshot {
    pub building_id: &'static str,
    pub effect_kind: &'static str,
    pub roster_resolved: bool,
    pub slots: u16,
    pub available_slots: u16,
    pub hunters: Vec<ProductServiceHunterSnapshot>,
    pub active: Vec<ProductServiceVisitSnapshot>,
    pub blockers: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProductServiceHunterSnapshot {
    pub hunter_id: u32,
    pub gold: u64,
    pub current_value: u64,
    pub maximum_value: u64,
    pub service_state: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProductServiceVisitSnapshot {
    pub hunter_id: u32,
    pub building_instance_id: String,
    pub product_id: String,
    pub remaining_ms: u64,
    pub effect_value: u64,
    pub payment_gold: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InfirmaryServiceSnapshot {
    pub roster_resolved: bool,
    pub slots: u16,
    pub available_slots: u16,
    pub hunters: Vec<InfirmaryHunterSnapshot>,
    pub active: Vec<InfirmaryTreatmentSnapshot>,
    pub blockers: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InfirmaryHunterSnapshot {
    pub hunter_id: u32,
    pub current_hp: u64,
    pub max_hp: u64,
    pub treatment_state: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InfirmaryTreatmentSnapshot {
    pub hunter_id: u32,
    pub building_instance_id: String,
    pub product_id: String,
    pub remaining_ms: u64,
    pub effect_value: u64,
    pub payment_gold: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldSnapshot {
    pub scene_nodes: Vec<&'static str>,
    pub visual_projection_runnable: bool,
    pub gameplay_runnable: bool,
    pub blockers: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldEntityDescriptor {
    pub entity_id: String,
    pub kind: WorldEntityKind,
    pub asset_bundle_id: &'static str,
    pub source_skeleton_name: &'static str,
    pub role: &'static str,
    pub source_binding: EvidenceBinding,
    pub placement_binding: EvidenceBinding,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldEntityProjection {
    pub descriptor: WorldEntityDescriptor,
    pub x: i32,
    pub y: i32,
    pub facing: Facing,
    pub action_state: WorldEntityActionState,
    pub animation: String,
    pub class_family: Option<String>,
    pub target_entity_id: Option<String>,
    pub action_sequence: u64,
    pub loot_sequence: u64,
    pub loot_label: Option<String>,
    pub attack_effect_key: Option<&'static str>,
    pub skill_presentation_key: Option<String>,
    pub current_hp: Option<u64>,
    pub maximum_hp: Option<u64>,
    pub interaction_prompt_key: Option<&'static str>,
    pub selectable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatPresentationSnapshot {
    pub sequence: u64,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub kind: super::CombatPresentationKind,
    pub amount: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldDropProjection {
    pub drop_id: String,
    pub item_id: String,
    pub quantity: u32,
    pub x: i32,
    pub y: i32,
    pub icon_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldProjection {
    pub mode: WorldMode,
    pub visual_tick: u64,
    pub coordinate_space: &'static str,
    pub authority_scope: &'static str,
    pub entities: Vec<WorldEntityProjection>,
    pub selected_entity_id: Option<String>,
    pub drops: Vec<WorldDropProjection>,
    pub combat_presentations: Vec<CombatPresentationSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OriginalFlowSnapshot {
    pub screen: OriginalScreen,
    pub content_release_id: &'static str,
    pub content_release_runnable: bool,
    pub flow_order: Vec<OriginalScreen>,
    pub village: VillageSnapshot,
    pub hunter_roster: HunterRosterSnapshot,
    pub field: FieldSnapshot,
    pub world: WorldProjection,
    pub monster_world: MonsterWorldSnapshot,
    pub migration_fixture_combat: MigrationFixtureCombatProjection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationFixtureCombatProjection {
    pub content_id: &'static str,
    pub evidence_label: &'static str,
    pub active: bool,
    pub world: WorldSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonsterDropSnapshot {
    pub monster_entity_id: String,
    pub item_id: String,
    pub quantity: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonsterSnapshot {
    pub entity_id: String,
    pub monster_id: String,
    pub source_index: u32,
    pub asset_bundle_id: String,
    pub hp: u64,
    pub max_hp: u64,
    pub damage: u64,
    pub armor: u64,
    pub experience: u64,
    pub gold: u64,
    pub x: i32,
    pub y: i32,
    pub action_state: String,
    pub animation: String,
    pub target_hunter_id: Option<u32>,
    pub respawn_ticks: Option<u16>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonsterWorldSnapshot {
    pub ruleset: &'static str,
    pub tick: u64,
    pub map_id: String,
    pub monster_tier: u8,
    pub map_asset_id: String,
    pub world_difficulty: u8,
    pub maps: Vec<MonsterMapSnapshot>,
    pub density_level: u8,
    pub spawn_count: u32,
    pub spawn_min: u32,
    pub spawn_max: u32,
    pub cluster_active: bool,
    pub banner_message: Option<&'static str>,
    pub monsters: Vec<MonsterSnapshot>,
    pub drops: Vec<MonsterDropSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonsterMapSnapshot {
    pub map_id: String,
    pub monster_tier: u8,
    pub map_asset_id: String,
    pub density_level: u8,
}

fn monster_world_snapshot(world: &MonsterWorldState) -> MonsterWorldSnapshot {
    let field = world.current_field();
    let config = map_config(&field.map_id).expect("fixture monster map must have a config");
    let cluster_active = field.density_level == 3;
    MonsterWorldSnapshot {
        ruleset: MONSTER_RULESET,
        tick: world.tick,
        map_id: field.map_id.clone(),
        monster_tier: config.monster_tier,
        map_asset_id: config.map_asset_id.to_owned(),
        world_difficulty: world.world_difficulty,
        maps: world
            .fields
            .iter()
            .map(|field| {
                let config =
                    map_config(&field.map_id).expect("fixture monster map must have a config");
                MonsterMapSnapshot {
                    map_id: field.map_id.clone(),
                    monster_tier: config.monster_tier,
                    map_asset_id: config.map_asset_id.to_owned(),
                    density_level: field.density_level,
                }
            })
            .collect(),
        density_level: field.density_level,
        spawn_count: field.spawn_count,
        spawn_min: config.density_counts[0],
        spawn_max: config.density_counts[2],
        cluster_active,
        banner_message: None,
        monsters: field
            .monsters
            .iter()
            .map(|monster| MonsterSnapshot {
                entity_id: monster.entity_id.clone(),
                monster_id: monster.monster_id.clone(),
                source_index: monster.source_index,
                asset_bundle_id: monster.asset_bundle_id.clone(),
                hp: monster.hp,
                max_hp: monster.max_hp,
                damage: monster.damage,
                armor: monster.armor,
                experience: monster.experience,
                gold: monster.gold,
                x: monster.x,
                y: monster.y,
                action_state: monster_action_name(monster.action_state).to_owned(),
                animation: monster.animation.clone(),
                target_hunter_id: monster.target_hunter_id,
                respawn_ticks: monster.respawn_ticks,
            })
            .collect(),
        drops: field
            .drops
            .iter()
            .map(|drop| MonsterDropSnapshot {
                monster_entity_id: drop.monster_entity_id.clone(),
                item_id: drop.item_id.clone(),
                quantity: drop.quantity,
            })
            .collect(),
    }
}

#[derive(Debug)]
pub struct OriginalFlowSession {
    state: OriginalFlowPlayerState,
    simulation: Simulation,
    combat_snapshot: WorldSnapshot,
    selected_entity_id: Option<String>,
    visual_tick: u64,
    simulation_remainder_ns: u64,
    buildings: DurableBuildingState,
    hunter_roster: DurableHunterRosterState,
    product_services: DurableProductServiceState,
    monster_world: MonsterWorldState,
    building_content: Arc<AuthoritativeBuildingContent>,
}

#[derive(Debug)]
pub struct OriginalFlowCommandResult {
    pub message: ServerMessage,
    pub durable_state_changed: bool,
    pub operations: Vec<PendingOperation>,
}

#[derive(Debug)]
pub struct OriginalFlowTickResult {
    pub world: WorldProjection,
    pub simulation_tick: u64,
    pub operations: Vec<PendingOperation>,
}

impl OriginalFlowSession {
    #[cfg(test)]
    pub fn from_state(state: OriginalFlowPlayerState) -> Self {
        let mut aggregate = DurablePlayerAggregate {
            navigation: state,
            ..DurablePlayerAggregate::default()
        };
        aggregate.buildings = test_town_building_state();
        Self::from_aggregate(aggregate, 7)
    }

    #[cfg(test)]
    pub fn from_aggregate(state: DurablePlayerAggregate, seed: u64) -> Self {
        Self::from_aggregate_with_content(state, seed, test_authoritative_building_content())
    }

    pub fn from_aggregate_with_content(
        mut state: DurablePlayerAggregate,
        seed: u64,
        building_content: Arc<AuthoritativeBuildingContent>,
    ) -> Self {
        if state.schema_version < 3 && state.buildings.hunter_materials == 0 {
            state.buildings.hunter_materials = 20;
        }
        if state.schema_version < 4 || state.buildings.material_stocks.is_empty() {
            state.buildings.material_stocks = default_material_stocks();
        }
        if state.schema_version < 11 {
            state.hunter_roster.upgrade_legacy_capacity();
        }
        // Older runtime builds accidentally copied collected gold drops into the
        // material inventory after already crediting the Hunter wallet.
        for hunter in state.hunter_roster.hunters.iter_mut().chain(
            state
                .hunter_roster
                .waiting_queue
                .iter_mut()
                .map(|waiting| &mut waiting.hunter),
        ) {
            hunter.hunt.loot.retain(|loot| loot.item_id != "gold");
            if hunter.hunt.status == "returning_for_infirmary" && hunter.hunt.zone_id.is_none() {
                hunter.hunt.status = "idle".to_owned();
                hunter.profile.action_state = "idle".to_owned();
                hunter.profile.animation_name = "hunter_stay".to_owned();
            }
            let incompatible_or_terminal_enhancement =
                hunter.hunt.gear_enhancement.as_ref().is_some_and(|task| {
                    task.workflow_version != GEAR_ENHANCEMENT_WORKFLOW_VERSION
                        || enhancement_task_terminal(task)
                });
            let orphaned_enhancement_action = hunter.hunt.gear_enhancement.is_none()
                && is_enhancement_action_state(&hunter.profile.action_state);
            if incompatible_or_terminal_enhancement || orphaned_enhancement_action {
                release_hunter_from_enhancement(hunter);
            }
        }
        if let Some(legacy) = state.legacy_infirmary.take() {
            if state.hunter_roster.hunters.is_empty() {
                state.hunter_roster.roster_resolved = legacy.roster_resolved;
                state.hunter_roster.hunters = legacy.hunters;
            }
            state
                .product_services
                .visits
                .extend(legacy.treatments.into_iter().map(|treatment| {
                    DurableProductServiceVisit {
                        hunter_id: treatment.hunter_id,
                        building_instance_id: treatment.building_instance_id,
                        building_id: "build_12".to_owned(),
                        product_id: treatment.product_id,
                        effect_kind: ServiceEffectKind::Hp,
                        remaining_ms: treatment.remaining_ms,
                        effect_value: treatment.effect_value,
                        payment_gold: treatment.payment_gold,
                    }
                }));
        }
        let monster_densities = state.monster_field_config.normalized_densities();
        let mut monster_world = MonsterWorldState::with_densities(
            monster_densities
                .iter()
                .map(|density| (density.map_id.as_str(), density.density_level)),
        );
        monster_world.restore_hunter_runtime(&state.hunter_roster, state.hunter_world_runtime);
        let simulation = Simulation::from_state(seed, state.migration_fixture_combat);
        let combat_snapshot = simulation.snapshot();
        Self {
            state: state.navigation,
            simulation,
            combat_snapshot,
            selected_entity_id: None,
            visual_tick: 0,
            simulation_remainder_ns: 0,
            buildings: state.buildings,
            hunter_roster: state.hunter_roster,
            product_services: state.product_services,
            monster_world,
            building_content,
        }
    }

    pub fn state(&self) -> &OriginalFlowPlayerState {
        &self.state
    }

    pub fn durable_state(&self) -> DurablePlayerAggregate {
        DurablePlayerAggregate {
            schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
            navigation: self.state.clone(),
            migration_fixture_combat: self.simulation.durable_state(),
            buildings: self.buildings.clone(),
            hunter_roster: self.hunter_roster.clone(),
            product_services: self.product_services.clone(),
            monster_field_config: DurableMonsterFieldConfig {
                densities: self
                    .monster_world
                    .fields
                    .iter()
                    .map(|field| DurableMonsterMapDensity {
                        map_id: field.map_id.clone(),
                        density_level: field.density_level,
                    })
                    .collect(),
                legacy_map_id: None,
                legacy_density_level: None,
            },
            hunter_world_runtime: self.monster_world.hunters.clone(),
            legacy_infirmary: None,
        }
    }

    pub fn advance_simulation_tick(&mut self) -> Option<OriginalFlowTickResult> {
        self.advance_simulation_step(100_000_000)
    }

    /// Advances the deterministic 10 Hz gameplay domain using scheduler time.
    /// Network cadence may vary without changing movement, combat, or cooldown speed.
    pub fn advance_simulation_step(&mut self, elapsed_ns: u64) -> Option<OriginalFlowTickResult> {
        if matches!(
            self.state.screen,
            OriginalScreen::Boot | OriginalScreen::HunterRoster
        ) {
            return None;
        }
        const DOMAIN_STEP_NS: u64 = 100_000_000;
        self.simulation_remainder_ns = self.simulation_remainder_ns.saturating_add(elapsed_ns);
        let step_count = self.simulation_remainder_ns / DOMAIN_STEP_NS;
        self.simulation_remainder_ns %= DOMAIN_STEP_NS;
        if step_count == 0 {
            return None;
        }

        let mut operations = Vec::new();
        for _ in 0..step_count {
            operations.extend(self.advance_domain_tick());
        }
        Some(OriginalFlowTickResult {
            world: self.world_projection(),
            simulation_tick: self.monster_world.tick.max(self.combat_snapshot.tick),
            operations,
        })
    }

    fn advance_domain_tick(&mut self) -> Vec<PendingOperation> {
        if self.state.screen == OriginalScreen::Field {
            self.combat_snapshot = self.simulation.step();
        }
        self.refresh_skill_cooldowns(100);
        for hunter in &mut self.hunter_roster.hunters {
            hunter.hunt.healing_potion_cooldown_ms = hunter
                .hunt
                .healing_potion_cooldown_ms
                .saturating_sub(100);
        }
        self.auto_cast_ready_hunter_skills();
        self.visual_tick = self.visual_tick.wrapping_add(1);
        let navigation_obstacles =
            town_navigation_obstacles(&self.buildings.buildings, &self.building_content.catalog);
        self.apply_autonomous_hunter_healing_policy();
        for hunter in &mut self.hunter_roster.hunters {
            let terminal_enhancement = hunter
                .hunt
                .gear_enhancement
                .as_ref()
                .is_some_and(enhancement_task_terminal);
            if terminal_enhancement {
                release_hunter_from_enhancement(hunter);
            }
        }
        let mut operations = self
            .monster_world
            .tick_with_obstacles(&mut self.hunter_roster, &navigation_obstacles);
        self.advance_legacy_hunter_hunts(1);
        self.auto_sell_requested_hunter_loot();
        if self.state.screen == OriginalScreen::Field {
            let mut combined = self.simulation.drain_operations();
            combined.append(&mut operations);
            combined
        } else {
            operations
        }
    }

    /// Applies the explicit rebuild healing policy at the simulation boundary.
    ///
    /// The original package exposes potion slots and HP/service methods, but no
    /// captured threshold or autonomous decision body. Until that evidence is
    /// recovered, the product rule is: below 10% HP, consume a Healing Potion
    /// first; when none is owned, leave the hunting region and return to town for
    /// the Infirmary route. This mutation is server-owned and deterministic.
    fn apply_autonomous_hunter_healing_policy(&mut self) {
        const HEALING_POTION_VALUES: [u64; 8] = [
            4_000, 12_000, 32_400, 77_800, 163_300, 294_000, 1_562_500, 9_375_000,
        ];

        for hunter in &mut self.hunter_roster.hunters {
            if hunter.current_hp == 0
                || hunter.max_hp == 0
                || u128::from(hunter.current_hp) * 100 >= u128::from(hunter.max_hp) * 10
                || hunter.hunt.gear_enhancement.is_some()
                || hunter.hunt.healing_potion_cooldown_ms > 0
                || self
                    .product_services
                    .visits
                    .iter()
                    .any(|visit| visit.hunter_id == hunter.hunter_id)
            {
                continue;
            }

            let potion = hunter
                .owned_items
                .iter_mut()
                .filter_map(|item| {
                    let prefix = "recipe:consumable:0:level:";
                    let level = item
                        .product_id
                        .strip_prefix(prefix)?
                        .parse::<usize>()
                        .ok()?;
                    (level < HEALING_POTION_VALUES.len() && item.quantity > 0)
                        .then_some((level, item))
                })
                .max_by_key(|(level, _)| *level);
            if let Some((level, item)) = potion {
                item.quantity = item.quantity.saturating_sub(1);
                hunter.current_hp = hunter
                    .current_hp
                    .saturating_add(HEALING_POTION_VALUES[level])
                    .min(hunter.max_hp);
                hunter.profile.action_state = "using_healing_potion".to_owned();
                hunter.profile.animation_name = "hunter_stay".to_owned();
                hunter.hunt.healing_potion_cooldown_ms = 20_000;
                continue;
            }

            // No consumable is available. Unassign the farm region so the world
            // agent returns to town; service stock/payment remains authoritative
            // and can be started by the Infirmary flow once it arrives.
            let leaving_field = hunter.hunt.zone_id.take().is_some();
            // The exact autonomous service-selection body is still unresolved.
            // Keep the Hunter commandable instead of persisting a terminal state
            // that no subsystem can complete.
            hunter.hunt.status = "idle".to_owned();
            hunter.profile.action_state = if leaving_field {
                "returning_for_infirmary"
            } else {
                "idle"
            }
            .to_owned();
            hunter.profile.animation_name = if leaving_field {
                "hunter_walk"
            } else {
                "hunter_stay"
            }
            .to_owned();
        }
    }

    pub fn advance_visual_tick(&mut self) -> Option<OriginalFlowSnapshot> {
        self.advance_visual_tick_by(200)
    }

    pub fn advance_visual_tick_by(&mut self, elapsed_ms: u64) -> Option<OriginalFlowSnapshot> {
        if !self.advance_visual_clock_by(elapsed_ms) {
            return None;
        }
        Some(self.snapshot())
    }

    pub fn advance_visual_clock_by(&mut self, elapsed_ms: u64) -> bool {
        if self.state.screen != OriginalScreen::Village {
            return false;
        }
        self.advance_product_services(elapsed_ms);
        true
    }

    fn advance_legacy_hunter_hunts(&mut self, ticks: u32) {
        let ids = self
            .hunter_roster
            .hunters
            .iter()
            .filter(|hunter| {
                hunter.hunt.zone_id.as_deref() == Some(super::hunter_roster::FIXTURE_HUNT_ZONE_ID)
            })
            .map(|hunter| hunter.hunter_id)
            .collect::<Vec<_>>();
        for hunter_id in ids {
            let _ = self.hunter_roster.advance_hunt(hunter_id, ticks);
        }
    }

    pub fn snapshot(&self) -> OriginalFlowSnapshot {
        OriginalFlowSnapshot {
            screen: self.state.screen,
            content_release_id: "original-flow-v1",
            content_release_runnable: false,
            flow_order: vec![
                OriginalScreen::Boot,
                OriginalScreen::Village,
                OriginalScreen::HunterRoster,
                OriginalScreen::Field,
            ],
            village: VillageSnapshot {
                source_scene: "level1",
                canvas_nodes: vec!["UICanvas", "MainCanvas", "WorldCanvas"],
                world_nodes: vec!["MapManager", "BuildGroup", "BottomView"],
                bottom_menu: vec![
                    BottomMenuIntent::Build,
                    BottomMenuIntent::Character,
                    BottomMenuIntent::Archive,
                    BottomMenuIntent::Store,
                    BottomMenuIntent::Raid,
                ],
                bindings: vec![
                    binding("scene.level1", BindingConfidence::Confirmed, true),
                    binding("village.background", BindingConfidence::Tentative, false),
                    binding("village.camera_bounds", BindingConfidence::Unknown, false),
                    binding(
                        "village.building_anchors",
                        BindingConfidence::Unknown,
                        false,
                    ),
                ],
                building_system: self.building_snapshot(),
            },
            hunter_roster: HunterRosterSnapshot {
                scene_nodes: vec!["HunterManager", "HunterGroup", "HunterBorder"],
                hunter_spine_source_confirmed: true,
                starter_composition_resolved: false,
                starter_stats_resolved: false,
                bindings: vec![
                    binding("hunter.spine_bundle", BindingConfidence::Confirmed, true),
                    binding(
                        "hunter.roster_ui",
                        BindingConfidence::StronglyInferred,
                        false,
                    ),
                    binding(
                        "hunter.starter_composition",
                        BindingConfidence::Unknown,
                        false,
                    ),
                    binding("hunter.starter_stats", BindingConfidence::Unknown, false),
                ],
                active_capacity: MAX_ACTIVE_TOWN_HUNTERS,
                active_hunters: self
                    .hunter_roster
                    .hunters
                    .iter()
                    .enumerate()
                    .map(|(position, hunter)| hunter_roster_member(hunter, "active", position))
                    .collect(),
                waiting_hunters: self
                    .hunter_roster
                    .waiting_queue
                    .iter()
                    .enumerate()
                    .map(|(position, waiting)| {
                        hunter_roster_member(&waiting.hunter, "waiting", position)
                    })
                    .collect(),
                infirmary: self.infirmary_snapshot(),
                product_services: ["build_9", "build_12", "build_13", "build_19"]
                    .into_iter()
                    .filter_map(|building_id| self.product_service_snapshot(building_id))
                    .collect(),
            },
            field: FieldSnapshot {
                scene_nodes: vec!["World", "Hunter", "Evil", "HpBar", "StatusGroup"],
                visual_projection_runnable: true,
                gameplay_runnable: true,
                blockers: Vec::new(),
            },
            world: self.world_projection(),
            monster_world: monster_world_snapshot(&self.monster_world),
            migration_fixture_combat: MigrationFixtureCombatProjection {
                content_id: MIGRATION_FIXTURE_CONTENT_ID,
                evidence_label: "deterministic_migration_fixture_not_legacy_balance",
                active: self.state.screen == OriginalScreen::Field,
                world: self.combat_snapshot.clone(),
            },
        }
    }

    pub fn handle_command(&mut self, command: ClientCommand) -> Option<OriginalFlowCommandResult> {
        self.handle_command_with_id(command, Uuid::nil())
    }

    pub fn handle_command_with_id(
        &mut self,
        command: ClientCommand,
        command_id: Uuid,
    ) -> Option<OriginalFlowCommandResult> {
        let previous_state = self.durable_state();
        let message = match command {
            ClientCommand::SubmitFarmReport { .. } => self.rejected(
                "submit_farm_report",
                "farm reports are handled by the queue ingress",
            ),
            ClientCommand::RequestResync => return None,
            ClientCommand::StartBuildingService {
                instance_id,
                hunter_id,
                product_id,
            } => self.start_product_service(&instance_id, hunter_id, &product_id),
            ClientCommand::StartInfirmaryTreatment {
                instance_id,
                hunter_id,
                product_id,
            } => self.start_product_service(&instance_id, hunter_id, &product_id),
            ClientCommand::CompleteBoot => {
                if self.state.screen != OriginalScreen::Boot {
                    self.rejected("complete_boot", "boot_already_completed")
                } else {
                    self.state.boot_completed = true;
                    self.state.screen = OriginalScreen::Village;
                    self.accepted("complete_boot")
                }
            }
            ClientCommand::SelectBottomMenu { menu } => self.select_bottom_menu(menu),
            ClientCommand::NavigateBack => self.navigate_back(),
            ClientCommand::EnterField => self.enter_field(),
            ClientCommand::EnterMonsterMap { map_id } => self.enter_monster_map(&map_id),
            ClientCommand::SetMonsterDensity { level } => self.set_monster_density(level),
            ClientCommand::SetMonsterRegionDensity { region_id, level } => {
                self.set_monster_region_density(&region_id, level)
            }
            ClientCommand::SelectMonsterTarget {
                monster_id,
                hunter_id,
            } => self.select_monster_target(&monster_id, hunter_id),
            ClientCommand::SelectEntity { entity_id } => self.select_entity(&entity_id),
            ClientCommand::ConstructBuilding { building_id } => {
                self.construct_building(&building_id)
            }
            ClientCommand::ConstructBuildingAt {
                building_id,
                grid_x,
                grid_y,
            } => self.construct_building_at(&building_id, grid_x, grid_y),
            ClientCommand::UpgradeBuilding { instance_id } => self.upgrade_building(&instance_id),
            ClientCommand::MoveBuilding {
                instance_id,
                grid_x,
                grid_y,
            } => self.move_building(&instance_id, grid_x, grid_y),
            ClientCommand::UseBuilding { instance_id } => self.use_building(&instance_id),
            ClientCommand::SetMaterialRequest {
                instance_id,
                material_id,
                quantity,
            } => self.set_material_request(&instance_id, &material_id, quantity),
            ClientCommand::CancelMaterialRequest {
                instance_id,
                material_id,
            } => self.cancel_material_request(&instance_id, &material_id),
            ClientCommand::CraftShopItem {
                instance_id,
                recipe_id,
                material_id,
                quantity,
            } => self.craft_shop_item(&instance_id, &recipe_id, material_id.as_deref(), quantity),
            ClientCommand::OpenHunterProgression { .. } => self.accepted("open_hunter_progression"),
            ClientCommand::AssignHunterHunt { hunter_id, zone_id } => self.apply_hunter_command(
                command_id,
                &format!("assign_hunter_hunt:{hunter_id}:{zone_id}"),
                "assign_hunter_hunt",
                |roster| roster.assign_hunt(hunter_id, &zone_id),
            ),
            ClientCommand::ReturnHunterHunt { hunter_id } => self.apply_hunter_command(
                command_id,
                &format!("return_hunter_hunt:{hunter_id}"),
                "return_hunter_hunt",
                |roster| roster.return_from_hunt(hunter_id),
            ),
            ClientCommand::SellHunterLoot { hunter_id } => {
                self.sell_hunter_loot(command_id, hunter_id)
            }
            ClientCommand::ReviveHunter { hunter_id } => self.apply_hunter_command(
                command_id,
                &format!("revive_hunter:{hunter_id}"),
                "revive_hunter",
                |roster| roster.revive_hunter(hunter_id),
            ),
            ClientCommand::LearnHunterSkill {
                hunter_id,
                skill_id,
            } => self.learn_hunter_skill(command_id, hunter_id, &skill_id),
            ClientCommand::UseHunterSkill {
                hunter_id,
                skill_id,
                target_entity_id,
            } => self
                .use_hunter_skill(
                    command_id,
                    hunter_id,
                    &skill_id,
                    target_entity_id.as_deref(),
                    true,
                )
                .expect("player skill commands always produce a response"),
            ClientCommand::BanishHunter { hunter_id } => self.banish_hunter(command_id, hunter_id),
            ClientCommand::EquipHunterItem { hunter_id, item_id } => {
                self.equip_fixture_item(command_id, hunter_id, item_id)
            }
            ClientCommand::StartHunterEnhancement { hunter_id } => {
                self.start_hunter_enhancement(command_id, hunter_id)
            }
            ClientCommand::EnhanceHunterGear {
                hunter_id,
                gear_instance_id,
                mode,
                optional_material_ids,
            } => self.enhance_hunter_gear(
                command_id,
                hunter_id,
                gear_instance_id,
                &mode,
                &optional_material_ids,
            ),
            ClientCommand::ClaimQuestReward { .. } => {
                self.binding_blocked("claim_quest_reward", &QUEST_BLOCKERS)
            }
            ClientCommand::OpenShop { .. } => self.binding_blocked("open_shop", &SHOP_BLOCKERS),
            ClientCommand::PurchaseShopItem {
                hunter_id,
                shop_id,
                product_id,
            } => self.purchase_shop_item(command_id, hunter_id, &shop_id, &product_id),
            ClientCommand::SellShopItem {
                shop_id,
                product_id,
            } => self.sell_shop_item(&shop_id, &product_id),
            ClientCommand::ClaimMail { .. } => self.binding_blocked("claim_mail", &MAIL_BLOCKERS),
            ClientCommand::ClaimRewardedAd { .. } => {
                self.binding_blocked("claim_rewarded_ad", &REWARDED_AD_BLOCKERS)
            }
            ClientCommand::StartTopupPurchase { .. } => {
                self.binding_blocked("start_topup_purchase", &TOPUP_BLOCKERS)
            }
        };
        let operations = self.simulation.drain_operations();
        Some(OriginalFlowCommandResult {
            message,
            durable_state_changed: self.durable_state() != previous_state,
            operations,
        })
    }

    fn building_snapshot(&self) -> BuildingSystemSnapshot {
        let content = &self.building_content;
        let definitions = content
            .catalog
            .bases
            .iter()
            .map(|building| building_definition_snapshot(building, content))
            .collect::<Vec<_>>();
        let states = content
            .catalog
            .bases
            .iter()
            .map(|definition| {
                let built = self
                    .buildings
                    .buildings
                    .iter()
                    .find(|item| item.id == definition.id.as_str());
                let constructed = built.is_some();
                let level = built.map_or(0, |item| item.level);
                let target_level = if constructed {
                    u16::from(level).saturating_add(1)
                } else {
                    1
                };
                let target_row = content.catalog.level(&definition.id, target_level);
                let gold_cost = target_row.and_then(gold_cost);
                let condition = mutation_condition(self, target_row);
                BuildingStateSnapshot {
                    id: definition.id.to_string(),
                    constructed,
                    level,
                    upgrade_cost: constructed.then_some(gold_cost).flatten(),
                    can_construct: !constructed && target_row.is_some() && condition.is_none(),
                    can_upgrade: constructed && target_row.is_some() && condition.is_none(),
                    condition: if target_row.is_none() && constructed {
                        Some("maximum_level".to_owned())
                    } else {
                        condition
                    },
                    uses: built.map_or(0, |item| item.uses),
                }
            })
            .collect();
        let instances = self
            .buildings
            .buildings
            .iter()
            .filter_map(|instance| {
                let building_id = BaseBuildingId::parse(&instance.id).ok()?;
                let definition = content.catalog.base(&building_id)?;
                let (grid_width, grid_height) = building_grid_size(definition)?;
                let target_row = content
                    .catalog
                    .level(&building_id, u16::from(instance.level).saturating_add(1));
                let condition = mutation_condition(self, target_row);
                Some(BuildingInstanceSnapshot {
                    instance_id: instance.instance_id.clone(),
                    building_id: instance.id.clone(),
                    level: instance.level,
                    grid_x: instance.grid_x,
                    grid_y: instance.grid_y,
                    grid_width,
                    grid_height,
                    sprite_asset_id: definition.base_sprite_asset_id.clone(),
                    upgrade_cost: target_row.and_then(gold_cost),
                    can_upgrade: target_row.is_some() && condition.is_none(),
                    condition,
                    uses: instance.uses,
                    seeded_by: instance.seeded_by.clone(),
                })
            })
            .collect();
        BuildingSystemSnapshot {
            evidence_label: "evil-hunter-1.411-postgresql-authoritative-content",
            town_gold: self.buildings.town_gold,
            definitions,
            states,
            instances,
            hunter_materials: self.buildings.hunter_materials,
            materials: self.buildings.materials,
            runes: self.buildings.runes,
            weapons: self.buildings.weapons,
            armor: self.buildings.armor,
            hunter_equipment_purchases: self.buildings.hunter_equipment_purchases,
            material_stocks: material_catalog_stocks(
                &content.gameplay,
                &self.buildings.material_stocks,
            )
            .iter()
            .map(|stock| MaterialStockSnapshot {
                id: stock.id.clone(),
                display_name: content
                    .gameplay
                    .item(&stock.id)
                    .and_then(|item| item.localized_names.get("en").cloned())
                    .unwrap_or_else(|| stock.id.clone()),
                icon: material_icon_path(&stock.id).unwrap_or_default(),
                town_quantity: stock.town_quantity,
                hunter_quantity: stock.hunter_quantity,
                requested: stock.requested,
                unit_price: stock.unit_price,
                difficulty: material_difficulty_rating(&stock.id).unwrap_or(u8::MAX),
            })
            .collect(),
            recipes: {
                let mut recipes = Vec::new();
                for definition in &content.catalog.bases {
                    let Some(building_id) = BaseBuildingId::parse(definition.id.as_str()).ok()
                    else {
                        continue;
                    };
                    let building = self
                        .buildings
                        .buildings
                        .iter()
                        .find(|building| building.id == definition.id.as_str());
                    let mut seen = HashSet::new();
                    let mut gear_buckets = HashMap::new();
                    for product in content.gameplay.products.values() {
                        let gear_route = gear_product_route(&content.gameplay, product);
                        let sale_building_id = product_sale_building_id(&content.gameplay, product);
                        let is_native_product = product.building_id.as_ref() == Some(&building_id);
                        let is_sale_product = sale_building_id.as_ref() == Some(&building_id);
                        if !is_native_product && !is_sale_product {
                            continue;
                        }
                        if is_native_product {
                            if let Some(route) = &gear_route {
                                let count = gear_buckets
                                    .entry((route.kind, route.rating))
                                    .or_insert(0_u8);
                                if *count >= 6 {
                                    continue;
                                }
                                *count += 1;
                            } else if seen.len() >= 24 {
                                continue;
                            }
                        }

                        let stock_building = sale_building_id
                            .as_ref()
                            .and_then(|sale_building_id| {
                                self.buildings
                                    .buildings
                                    .iter()
                                    .find(|candidate| candidate.id == sale_building_id.as_str())
                            })
                            .or(building);
                        let stored_stock = stock_building.map_or(0, |stock_building| {
                            self.buildings
                                .product_stocks
                                .iter()
                                .find(|stock| {
                                    stock.building_instance_id == stock_building.instance_id
                                        && stock.product_id == product.product_id
                                })
                                .map_or(0, |stock| stock.quantity)
                        });
                        if is_sale_product && stored_stock == 0 {
                            continue;
                        }
                        let stock = stored_stock;
                        let product_name = product_display_name(&product.product_id)
                            .map(str::to_owned)
                            .or_else(|| {
                                product
                                    .outputs
                                    .first()
                                    .and_then(|output| content.gameplay.item(&output.resource_id))
                                    .and_then(|item| item.localized_names.get("en").cloned())
                            })
                            .unwrap_or_else(|| "Unresolved product".to_owned());
                        let dedupe_key = if gear_route.is_some() {
                            product.product_id.clone()
                        } else {
                            product_name.clone()
                        };
                        if !seen.insert(dedupe_key) {
                            continue;
                        }
                        let stock_level =
                            stock_building.map_or(1, |building| u16::from(building.level));
                        let stock_building_id = stock_building
                            .and_then(|building| BaseBuildingId::parse(&building.id).ok())
                            .unwrap_or_else(|| building_id.clone());
                        let capacity = content
                            .catalog
                            .level(&stock_building_id, stock_level)
                            .and_then(|level| level.production_slots)
                            .or_else(|| capacity_for_level(stock_building_id.as_str(), stock_level))
                            .unwrap_or(0);
                        // Preserve the service route when a legacy DB row has the
                        // optional service payload missing but still has recovered
                        // conversion inputs under a service building.
                        let service_product = product.service.is_some()
                            || (ServiceEffectKind::for_building(stock_building_id.as_str())
                                .is_some()
                                && !product.conversion_options.is_empty());
                        recipes.push(ShopRecipeSnapshot {
                            id: product.product_id.clone(),
                            shop_id: definition.id.to_string(),
                            icon: product_icon_path(&product.product_id)
                                .map(str::to_owned)
                                .unwrap_or_default(),
                            product_name,
                            material_costs: product
                                .inputs
                                .iter()
                                .map(|cost| (cost.resource_id.clone(), cost.quantity, 1))
                                .chain(
                                    service_product
                                        .then(|| {
                                            product.conversion_options.iter().map(|option| {
                                                (
                                                    option.input_resource_id.clone(),
                                                    option.input_quantity,
                                                    option.output_stock_quantity,
                                                )
                                            })
                                        })
                                        .into_iter()
                                        .flatten(),
                                )
                                .filter_map(|(material_id, amount, output)| {
                                    u32::try_from(amount)
                                        .ok()
                                        .zip(u32::try_from(output).ok())
                                        .map(|(quantity, output_quantity)| {
                                            let display_name = content
                                                .gameplay
                                                .item(&material_id)
                                                .and_then(|item| {
                                                    item.localized_names.get("en").cloned()
                                                })
                                                .or_else(|| match material_id.as_str() {
                                                    "currency:gem" => Some("Gem".to_owned()),
                                                    "currency:elemental" => {
                                                        Some("Elemental".to_owned())
                                                    }
                                                    _ => None,
                                                })
                                                .unwrap_or_else(|| {
                                                    "Unresolved material".to_owned()
                                                });
                                            RecipeMaterialCostSnapshot {
                                                material_id,
                                                display_name,
                                                quantity,
                                                output_quantity,
                                            }
                                        })
                                })
                                .collect(),
                            stock,
                            sale_price: product.service.as_ref().map_or_else(
                                || {
                                    product
                                        .sale_price
                                        .first()
                                        .map(|price| price.quantity)
                                        .or_else(|| {
                                            consumable_purchase_price(&content.gameplay, product)
                                        })
                                        .unwrap_or(0)
                                },
                                |service| service.use_money,
                            ),
                            kind: if service_product { "service" } else { "craft" },
                            required_level: gear_route.as_ref().map_or_else(
                                || {
                                    product
                                        .service
                                        .as_ref()
                                        .map_or(0, |service| service.required_level)
                                },
                                |route| route.rating,
                            ),
                            duration_ms: product.duration_ms.unwrap_or(0),
                            effect_value: product
                                .service
                                .as_ref()
                                .map_or(0, |service| service.effect_value),
                            effect_kind: service_effect_kind(definition.id.as_str()),
                            capacity,
                        });
                    }
                }
                recipes
            },
        }
    }

    fn construct_building(&mut self, building_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("construct_building", "village_unavailable");
        }
        let Ok(building_id) = BaseBuildingId::parse(building_id) else {
            return self.rejected("construct_building", "building_unknown");
        };
        if self.building_content.catalog.base(&building_id).is_none() {
            return self.rejected("construct_building", "building_unknown");
        }
        self.rejected("construct_building", "placement_required")
    }

    fn construct_building_at(
        &mut self,
        building_id: &str,
        grid_x: i32,
        grid_y: i32,
    ) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("construct_building_at", "village_unavailable");
        }
        let Ok(base_id) = BaseBuildingId::parse(building_id) else {
            return self.rejected("construct_building_at", "building_unknown");
        };
        let Some(definition) = self.building_content.catalog.base(&base_id) else {
            return self.rejected("construct_building_at", "building_unknown");
        };
        let max_build = definition.max_instances;
        if max_build == 0
            || self
                .buildings
                .buildings
                .iter()
                .filter(|building| building.id == building_id)
                .count()
                >= max_build as usize
        {
            return self.rejected("construct_building_at", "max_build_reached");
        }
        let Some((grid_width, grid_height)) = building_grid_size(definition) else {
            return self.rejected("construct_building_at", "grid_size_unresolved");
        };
        if !placement_is_valid(
            &self.buildings.buildings,
            &self.building_content.catalog,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
            None,
        ) {
            return self.rejected("construct_building_at", "placement_blocked");
        }
        let Some(row) = self.building_content.catalog.level(&base_id, 1) else {
            return self.rejected("construct_building_at", "build_row_unresolved");
        };
        if let Some(reason) = mutation_condition(self, Some(row)) {
            return self.rejected("construct_building_at", &reason);
        }
        if !can_pay_costs(&self.buildings, &row.costs) {
            return self.rejected("construct_building_at", "insufficient_building_cost");
        }
        pay_costs(&mut self.buildings, &row.costs);
        let instance_id = Uuid::new_v4().to_string();
        self.buildings.next_building_instance_id += 1;
        self.buildings.buildings.push(DurableBuilding {
            instance_id,
            id: building_id.to_owned(),
            equipped_skin_id: None,
            level: 1,
            uses: 0,
            grid_x,
            grid_y,
            seeded_by: None,
        });
        self.accepted("construct_building_at")
    }

    fn upgrade_building(&mut self, instance_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("upgrade_building", "village_unavailable");
        }
        let Some((index, building)) = self
            .buildings
            .buildings
            .iter()
            .enumerate()
            .find(|(_, building)| building.instance_id == instance_id)
        else {
            return self.rejected("upgrade_building", "building_instance_unknown");
        };
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("upgrade_building", "building_unknown");
        };
        let Some(row) = self
            .building_content
            .catalog
            .level(&building_id, u16::from(building.level).saturating_add(1))
        else {
            return self.rejected("upgrade_building", "maximum_level");
        };
        if let Some(reason) = mutation_condition(self, Some(row)) {
            return self.rejected("upgrade_building", &reason);
        }
        if !can_pay_costs(&self.buildings, &row.costs) {
            return self.rejected("upgrade_building", "insufficient_building_cost");
        }
        pay_costs(&mut self.buildings, &row.costs);
        let Ok(level) = u8::try_from(row.level) else {
            return self.rejected("upgrade_building", "building_level_out_of_range");
        };
        self.buildings.buildings[index].level = level;
        self.accepted("upgrade_building")
    }

    fn move_building(&mut self, instance_id: &str, grid_x: i32, grid_y: i32) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("move_building", "village_unavailable");
        }
        let Some((index, building)) = self
            .buildings
            .buildings
            .iter()
            .enumerate()
            .find(|(_, building)| building.instance_id == instance_id)
        else {
            return self.rejected("move_building", "building_instance_unknown");
        };
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("move_building", "building_unknown");
        };
        let Some(definition) = self.building_content.catalog.base(&building_id) else {
            return self.rejected("move_building", "building_unknown");
        };
        let Some((grid_width, grid_height)) = building_grid_size(definition) else {
            return self.rejected("move_building", "grid_size_unresolved");
        };
        if !placement_is_valid(
            &self.buildings.buildings,
            &self.building_content.catalog,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
            Some(index),
        ) {
            return self.rejected("move_building", "placement_blocked");
        }
        self.buildings.buildings[index].grid_x = grid_x;
        self.buildings.buildings[index].grid_y = grid_y;
        self.accepted("move_building")
    }

    fn use_building(&mut self, instance_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("use_building", "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("use_building", "building_instance_unknown");
        };
        self.capability_blocked("use_building", &building.id, &[])
    }

    fn set_material_request(
        &mut self,
        instance_id: &str,
        material_id: &str,
        quantity: u32,
    ) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("set_material_request", "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("set_material_request", "building_instance_unknown");
        };
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("set_material_request", "building_unknown");
        };
        if !self
            .building_content
            .gameplay
            .capabilities_for(&building_id)
            .any(|capability| capability.kind == "loot-purchase-reservations")
        {
            return self.rejected("set_material_request", "building_capability_mismatch");
        }
        let Some(difficulty) = material_difficulty_rating(material_id) else {
            return self.rejected("set_material_request", "material_difficulty_unresolved");
        };
        if difficulty >= building.level {
            return self.rejected("set_material_request", "material_difficulty_locked");
        }
        if quantity == 0 {
            return self.rejected("set_material_request", "material_quantity_invalid");
        }
        let Some(authoritative_price) = self
            .building_content
            .gameplay
            .item(material_id)
            .and_then(|item| item.town_pays_hunter_gold_per_unit)
        else {
            return self.rejected("set_material_request", "material_price_unresolved");
        };
        if let Some(stock) = self
            .buildings
            .material_stocks
            .iter_mut()
            .find(|stock| stock.id == material_id)
        {
            stock.requested = quantity;
            stock.unit_price = authoritative_price;
        } else {
            self.buildings.material_stocks.push(DurableMaterialStock {
                id: material_id.to_owned(),
                town_quantity: 0,
                hunter_quantity: 0,
                requested: quantity,
                unit_price: authoritative_price,
            });
            self.buildings
                .material_stocks
                .sort_by(|left, right| left.id.cmp(&right.id));
        }
        self.accepted("set_material_request")
    }

    fn cancel_material_request(&mut self, instance_id: &str, material_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("cancel_material_request", "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("cancel_material_request", "building_instance_unknown");
        };
        let capability_matches = BaseBuildingId::parse(&building.id).ok().is_some_and(|id| {
            self.building_content
                .gameplay
                .capabilities_for(&id)
                .any(|capability| capability.kind == "loot-purchase-reservations")
        });
        if !capability_matches {
            return self.rejected("cancel_material_request", "building_capability_mismatch");
        }
        let Some(stock) = self
            .buildings
            .material_stocks
            .iter_mut()
            .find(|stock| stock.id == material_id)
        else {
            return self.rejected("cancel_material_request", "material_request_unknown");
        };
        stock.requested = 0;
        self.accepted("cancel_material_request")
    }

    fn craft_shop_item(
        &mut self,
        instance_id: &str,
        recipe_id: &str,
        material_id: Option<&str>,
        quantity: u32,
    ) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("craft_shop_item", "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("craft_shop_item", "building_instance_unknown");
        };
        if !(1..=MAX_PRODUCTION_QUANTITY).contains(&quantity) {
            return self.rejected("craft_shop_item", "quantity_invalid");
        }
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("craft_shop_item", "building_unknown");
        };
        let Some(product) = self.building_content.gameplay.product(recipe_id) else {
            return self.rejected("craft_shop_item", "recipe_unknown");
        };
        if product.building_id.as_ref() != Some(&building_id) {
            return self.rejected("craft_shop_item", "recipe_building_mismatch");
        }
        // A partially migrated service row may have conversion options but no
        // decoded service payload. The building route still identifies it as a
        // service product, which must remain uncapped.
        let service_product = product.service.is_some()
            || (ServiceEffectKind::for_building(building_id.as_str()).is_some()
                && !product.conversion_options.is_empty());
        let gear_route = gear_product_route(&self.building_content.gameplay, product);
        if gear_route
            .as_ref()
            .is_some_and(|route| route.difficulty_group > u16::from(building.level))
        {
            return self.rejected("craft_shop_item", "product_level_locked");
        }
        let crafting_capability = self
            .building_content
            .gameplay
            .capabilities_for(&building_id)
            .any(|capability| {
                capability.kind == "weapon-and-armor-crafting"
                    || capability.kind == "potion-crafting"
                    || capability.kind == "accessory-crafting"
            });
        if !crafting_capability && !service_product {
            return self.rejected("craft_shop_item", "building_capability_mismatch");
        }
        let sale_building_id = product_sale_building_id(&self.building_content.gameplay, product);
        let stock_building = if let Some(sale_building_id) = &sale_building_id {
            let Some(sale_building) = self
                .buildings
                .buildings
                .iter()
                .find(|candidate| candidate.id == sale_building_id.as_str())
            else {
                return self.rejected("craft_shop_item", "sale_building_instance_unknown");
            };
            sale_building
        } else {
            building
        };
        let stock_building_id = BaseBuildingId::parse(&stock_building.id)
            .expect("validated building state references a canonical base id");
        let stock_building_instance_id = stock_building.instance_id.clone();
        let capacity = self
            .building_content
            .catalog
            .level(&stock_building_id, u16::from(stock_building.level))
            .and_then(|level| level.production_slots)
            .or_else(|| {
                capacity_for_level(stock_building_id.as_str(), u16::from(stock_building.level))
            })
            .map(u32::from)
            .unwrap_or(0);
        let stocked = self
            .buildings
            .product_stocks
            .iter()
            .filter(|stock| stock.building_instance_id == stock_building_instance_id)
            .fold(0_u32, |total, stock| total.saturating_add(stock.quantity));
        // Service products are consumed by the service flow and are not
        // constrained by the building's display-stock cap. Crafted gear and
        // sale inventory still use the authoritative capacity check.
        if !service_product && capacity > 0 && stocked.saturating_add(quantity) > capacity {
            return self.rejected("craft_shop_item", "product_capacity_exceeded");
        }
        let costs = if service_product {
            let Some(material_id) = material_id else {
                return self.rejected("craft_shop_item", "material_selection_required");
            };
            let Some(option) = product
                .conversion_options
                .iter()
                .find(|option| option.input_resource_id == material_id)
            else {
                return self.rejected("craft_shop_item", "material_selection_invalid");
            };
            let batches = u64::from(quantity)
                .saturating_add(option.output_stock_quantity.saturating_sub(1))
                / option.output_stock_quantity.max(1);
            vec![EconomyAmount {
                resource_id: option.input_resource_id.clone(),
                quantity: option.input_quantity.saturating_mul(batches),
            }]
        } else {
            product
                .inputs
                .iter()
                .map(|cost| EconomyAmount {
                    resource_id: cost.resource_id.clone(),
                    quantity: cost.quantity.saturating_mul(u64::from(quantity)),
                })
                .collect::<Vec<_>>()
        };
        if !can_pay_costs(&self.buildings, &costs) {
            return self.rejected("craft_shop_item", "insufficient_materials");
        }
        pay_costs(&mut self.buildings, &costs);
        if let Some(stock) = self.buildings.product_stocks.iter_mut().find(|stock| {
            stock.building_instance_id == stock_building_instance_id
                && stock.product_id == recipe_id
        }) {
            stock.quantity = stock.quantity.saturating_add(quantity);
        } else {
            self.buildings.product_stocks.push(DurableProductStock {
                building_instance_id: stock_building_instance_id,
                product_id: recipe_id.to_owned(),
                quantity,
            });
        }
        self.accepted("craft_shop_item")
    }

    fn start_product_service(
        &mut self,
        instance_id: &str,
        hunter_id: u32,
        product_id: &str,
    ) -> ServerMessage {
        const INTENT: &str = "start_building_service";
        if self.state.screen != OriginalScreen::Village {
            return self.rejected(INTENT, "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected(INTENT, "service_instance_unknown");
        };
        let building_id = building.id.clone();
        let level = u16::from(building.level);
        let Some(effect_kind) = ServiceEffectKind::for_building(&building_id) else {
            return self.rejected(INTENT, "service_building_unsupported");
        };
        if !self.product_service_roster_resolved(effect_kind) {
            return self.binding_blocked(
                INTENT,
                &[
                    "hunter_roster_binding",
                    effect_kind.state_binding(),
                    "hunter_wallet_state_binding",
                ],
            );
        }
        let slots = capacity_for_level(&building_id, level).unwrap_or(0);
        let occupied_slots = self
            .product_services
            .visits
            .iter()
            .filter(|visit| visit.building_instance_id == instance_id)
            .count();
        if slots == 0 || occupied_slots >= usize::from(slots) {
            return self.rejected(INTENT, "service_slots_full");
        }
        if self
            .product_services
            .visits
            .iter()
            .any(|visit| visit.hunter_id == hunter_id)
        {
            return self.rejected(INTENT, "hunter_already_in_service");
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(INTENT, "hunter_unknown");
        };
        if !hunter_service_gauge(hunter, effect_kind).needs_service() {
            return self.rejected(INTENT, "service_not_required");
        }
        let Some(product) = self.building_content.gameplay.product(product_id) else {
            return self.rejected(INTENT, "recipe_unknown");
        };
        if product.building_id.as_ref().map(BaseBuildingId::as_str) != Some(building_id.as_str()) {
            return self.rejected(INTENT, "recipe_building_mismatch");
        }
        let Some(service) = product.service.as_ref() else {
            return self.rejected(INTENT, "service_recipe_required");
        };
        if service.required_level >= level {
            return self.rejected(INTENT, "product_level_locked");
        }
        let Some(stock) = self.buildings.product_stocks.iter_mut().find(|stock| {
            stock.building_instance_id == instance_id && stock.product_id == product_id
        }) else {
            return self.rejected(INTENT, "product_out_of_stock");
        };
        if stock.quantity == 0 {
            return self.rejected(INTENT, "product_out_of_stock");
        }
        let hunter = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .expect("validated service hunter");
        if hunter.gold < service.use_money {
            return self.rejected(INTENT, "insufficient_hunter_gold");
        }
        hunter.gold -= service.use_money;
        stock.quantity -= 1;
        self.product_services
            .visits
            .push(DurableProductServiceVisit {
                hunter_id,
                building_instance_id: instance_id.to_owned(),
                building_id,
                product_id: product_id.to_owned(),
                effect_kind,
                remaining_ms: service.service_time_ms,
                effect_value: service.effect_value,
                payment_gold: service.use_money,
            });
        self.accepted(INTENT)
    }

    fn advance_product_services(&mut self, elapsed_ms: u64) {
        for visit in &mut self.product_services.visits {
            visit.remaining_ms = visit.remaining_ms.saturating_sub(elapsed_ms);
        }
        let completed_visits = self
            .product_services
            .visits
            .iter()
            .filter(|visit| visit.remaining_ms == 0)
            .cloned()
            .collect::<Vec<_>>();
        for visit in &completed_visits {
            if let Some(hunter) = self
                .hunter_roster
                .hunters
                .iter_mut()
                .find(|hunter| hunter.hunter_id == visit.hunter_id)
            {
                restore_hunter_service_gauge(hunter, visit.effect_kind, visit.effect_value);
                self.buildings.town_gold =
                    self.buildings.town_gold.saturating_add(visit.payment_gold);
            }
        }
        self.product_services
            .visits
            .retain(|visit| visit.remaining_ms > 0);
    }

    fn product_service_roster_resolved(&self, effect_kind: ServiceEffectKind) -> bool {
        if !self.hunter_roster.roster_resolved || !self.hunter_roster.wallets_resolved {
            return false;
        }
        let mut hunter_ids = HashSet::with_capacity(self.hunter_roster.hunters.len());
        self.hunter_roster.hunters.iter().all(|hunter| {
            hunter_service_gauge(hunter, effect_kind).is_resolved()
                && hunter_ids.insert(hunter.hunter_id)
        })
    }

    fn infirmary_snapshot(&self) -> InfirmaryServiceSnapshot {
        let service = self
            .product_service_snapshot("build_12")
            .expect("static service building");
        InfirmaryServiceSnapshot {
            roster_resolved: service.roster_resolved,
            slots: service.slots,
            available_slots: service.available_slots,
            hunters: service
                .hunters
                .into_iter()
                .map(|hunter| InfirmaryHunterSnapshot {
                    hunter_id: hunter.hunter_id,
                    current_hp: hunter.current_value,
                    max_hp: hunter.maximum_value,
                    treatment_state: if hunter.service_state == "serving" {
                        "treating"
                    } else {
                        "idle"
                    },
                })
                .collect(),
            active: service
                .active
                .into_iter()
                .map(|visit| InfirmaryTreatmentSnapshot {
                    hunter_id: visit.hunter_id,
                    building_instance_id: visit.building_instance_id,
                    product_id: visit.product_id,
                    remaining_ms: visit.remaining_ms,
                    effect_value: visit.effect_value,
                    payment_gold: visit.payment_gold,
                })
                .collect(),
            blockers: service.blockers,
        }
    }

    fn product_service_snapshot(
        &self,
        building_id: &'static str,
    ) -> Option<ProductServiceSnapshot> {
        let effect_kind = ServiceEffectKind::for_building(building_id)?;
        let roster_resolved = self.product_service_roster_resolved(effect_kind);
        let slots = self
            .buildings
            .buildings
            .iter()
            .filter(|building| building.id == building_id)
            .filter_map(|building| capacity_for_level(building_id, u16::from(building.level)))
            .fold(0_u16, u16::saturating_add);
        let active = self
            .product_services
            .visits
            .iter()
            .filter(|visit| visit.building_id == building_id)
            .map(|visit| ProductServiceVisitSnapshot {
                hunter_id: visit.hunter_id,
                building_instance_id: visit.building_instance_id.clone(),
                product_id: visit.product_id.clone(),
                remaining_ms: visit.remaining_ms,
                effect_value: visit.effect_value,
                payment_gold: visit.payment_gold,
            })
            .collect::<Vec<_>>();
        let occupied_slots = u16::try_from(active.len()).unwrap_or(u16::MAX);
        Some(ProductServiceSnapshot {
            building_id,
            effect_kind: effect_kind.as_str(),
            roster_resolved,
            slots,
            available_slots: slots.saturating_sub(occupied_slots),
            hunters: self
                .hunter_roster
                .hunters
                .iter()
                .map(|hunter| {
                    let gauge = hunter_service_gauge(hunter, effect_kind);
                    ProductServiceHunterSnapshot {
                        hunter_id: hunter.hunter_id,
                        gold: hunter.gold,
                        current_value: gauge.current,
                        maximum_value: gauge.maximum,
                        service_state: if self
                            .product_services
                            .visits
                            .iter()
                            .any(|visit| visit.hunter_id == hunter.hunter_id)
                        {
                            "serving"
                        } else {
                            "idle"
                        },
                    }
                })
                .collect(),
            active,
            blockers: if roster_resolved {
                Vec::new()
            } else {
                vec![
                    "hunter_roster_binding",
                    effect_kind.state_binding(),
                    "hunter_wallet_state_binding",
                ]
            },
        })
    }

    fn settle_returning_hunters(&mut self) {
        settle_returning_hunters(&mut self.buildings);
    }

    fn purchase_shop_item(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        shop_id: &str,
        product_id: &str,
    ) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("purchase_shop_item", "village_unavailable");
        }
        let Ok(building_id) = BaseBuildingId::parse(shop_id) else {
            return self.rejected("purchase_shop_item", "building_unknown");
        };
        let Some(product) = self.building_content.gameplay.product(product_id) else {
            return self.rejected("purchase_shop_item", "recipe_unknown");
        };
        let route = gear_product_route(&self.building_content.gameplay, product);
        let sale_building_id = product_sale_building_id(&self.building_content.gameplay, product);
        if sale_building_id.as_ref() != Some(&building_id) {
            return self.rejected("purchase_shop_item", "recipe_building_mismatch");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == shop_id)
        else {
            return self.rejected("purchase_shop_item", "building_instance_unknown");
        };
        if route
            .as_ref()
            .is_some_and(|route| route.difficulty_group > u16::from(building.level))
        {
            return self.rejected("purchase_shop_item", "product_level_locked");
        }
        let building_instance_id = building.instance_id.clone();
        let Some(stock) = self.buildings.product_stocks.iter_mut().find(|stock| {
            stock.building_instance_id == building_instance_id && stock.product_id == product_id
        }) else {
            return self.rejected("purchase_shop_item", "product_stock_empty");
        };
        if stock.quantity == 0 {
            return self.rejected("purchase_shop_item", "product_stock_empty");
        }
        let price = route
            .as_ref()
            .and_then(|_| product.sale_price.first().map(|amount| amount.quantity))
            .or_else(|| consumable_purchase_price(&self.building_content.gameplay, product))
            .unwrap_or(0);
        if price == 0 {
            return self.rejected("purchase_shop_item", "sale_price_unresolved");
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected("purchase_shop_item", "hunter_unknown");
        };
        if !hunter.hunt.is_idle() {
            return self.rejected("purchase_shop_item", "hunter_not_in_town");
        }
        if hunter.gold < price {
            return self.rejected("purchase_shop_item", "insufficient_hunter_gold");
        }
        let key = format!("purchase_shop_item:{hunter_id}:{shop_id}:{product_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted("purchase_shop_item")
                } else {
                    self.rejected("purchase_shop_item", "command_id_conflict")
                };
            }
        }

        hunter.gold -= price;
        stock.quantity -= 1;
        self.buildings.town_gold = self.buildings.town_gold.saturating_add(price);
        if let Some(owned) = hunter
            .owned_items
            .iter_mut()
            .find(|owned| owned.product_id == product_id)
        {
            owned.quantity = owned.quantity.saturating_add(1);
        } else {
            hunter
                .owned_items
                .push(super::hunter_roster::DurableHunterOwnedItem {
                    product_id: product_id.to_owned(),
                    quantity: 1,
                    enhancement_level: None,
                    gear_instance_id: (command_id != Uuid::nil() && route.is_some())
                        .then_some(command_id),
                });
        }
        if route.is_some() {
            self.buildings.hunter_equipment_purchases =
                self.buildings.hunter_equipment_purchases.saturating_add(1);
        }
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted("purchase_shop_item")
    }

    fn sell_shop_item(&mut self, shop_id: &str, product_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("sell_shop_item", "village_unavailable");
        }
        let _ = (shop_id, product_id);
        self.capability_blocked(
            "sell_shop_item",
            shop_id,
            &[
                "weapon-display-and-sale",
                "armor-display-and-sale",
                "potion-display-and-sale",
                "accessory-display-and-sale",
            ],
        )
    }

    fn start_hunter_enhancement(&mut self, command_id: Uuid, hunter_id: u32) -> ServerMessage {
        const INTENT: &str = "start_hunter_enhancement";
        if self.state.screen != OriginalScreen::Village {
            return self.rejected(INTENT, "village_unavailable");
        }
        let key = format!("{INTENT}:{hunter_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted(INTENT)
                } else {
                    self.rejected(INTENT, "command_id_conflict")
                };
            }
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_15")
        else {
            return self.rejected(INTENT, "enhancement_forge_unavailable");
        };
        let Some(definition) = BaseBuildingId::parse("build_15")
            .ok()
            .and_then(|building_id| self.building_content.catalog.base(&building_id))
        else {
            return self.rejected(INTENT, "enhancement_forge_definition_unavailable");
        };
        let Some((grid_width, grid_height)) = building_grid_size(definition) else {
            return self.rejected(INTENT, "enhancement_forge_geometry_unavailable");
        };
        let Ok(grid_width) = i32::try_from(grid_width) else {
            return self.rejected(INTENT, "enhancement_forge_geometry_unavailable");
        };
        let Ok(grid_height) = i32::try_from(grid_height) else {
            return self.rejected(INTENT, "enhancement_forge_geometry_unavailable");
        };
        let interaction_x = TOWN_NAV_ORIGIN_X
            + building.grid_x * TOWN_NAV_CELL_SIZE
            + grid_width * TOWN_NAV_CELL_SIZE / 2;
        let interaction_y =
            TOWN_NAV_ORIGIN_Y + (building.grid_y + grid_height + 1) * TOWN_NAV_CELL_SIZE;
        let building_instance_id = building.instance_id.clone();
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(INTENT, "hunter_unknown");
        };
        if !hunter.hunt.is_idle() {
            return self.rejected(INTENT, "hunter_not_in_town");
        }
        if hunter.current_hp == 0 {
            return self.rejected(INTENT, "hunter_unavailable");
        }
        if hunter.hunt.gear_enhancement.is_some() {
            return self.rejected(INTENT, "enhancement_task_already_active");
        }
        hunter.hunt.gear_enhancement = Some(DurableGearEnhancementTask {
            workflow_version: GEAR_ENHANCEMENT_WORKFLOW_VERSION,
            building_instance_id,
            status: GearEnhancementTaskStatus::Traveling,
            interaction_x,
            interaction_y,
            blockers: GEAR_ENHANCEMENT_BLOCKERS
                .iter()
                .map(|blocker| (*blocker).to_owned())
                .collect(),
            ..DurableGearEnhancementTask::default()
        });
        hunter.profile.action_state = "traveling_to_enhancement_forge".to_owned();
        hunter.profile.animation_name = "hunter_walk".to_owned();
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted(INTENT)
    }

    fn enhance_hunter_gear(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        gear_instance_id: Uuid,
        mode: &str,
        optional_material_ids: &[String],
    ) -> ServerMessage {
        const INTENT: &str = "enhance_hunter_gear";
        if self.state.screen != OriginalScreen::Village {
            return self.rejected(INTENT, "village_unavailable");
        }
        let target_level = match mode {
            "single" => None,
            "to_10" => Some(10),
            "to_15" => Some(15),
            "to_20" => Some(MAX_GEAR_ENHANCEMENT_LEVEL),
            _ => return self.rejected(INTENT, "enhancement_mode_invalid"),
        };
        let mut unique_materials = HashSet::new();
        for material_id in optional_material_ids {
            if !unique_materials.insert(material_id.as_str()) {
                return self.rejected(INTENT, "enhancement_optional_material_duplicated");
            }
            if self.building_content.gameplay.item(material_id).is_none() {
                return self.rejected(INTENT, "enhancement_optional_material_unknown");
            }
        }
        let key = format!(
            "{INTENT}:{hunter_id}:{gear_instance_id}:{mode}:{}",
            optional_material_ids.join(",")
        );
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.binding_blocked(INTENT, &GEAR_ENHANCEMENT_BLOCKERS)
                } else {
                    self.rejected(INTENT, "command_id_conflict")
                };
            }
        }
        let Some(hunter_index) = self
            .hunter_roster
            .hunters
            .iter()
            .position(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(INTENT, "hunter_unknown");
        };
        let hunter = &self.hunter_roster.hunters[hunter_index];
        if !hunter.hunt.is_idle() {
            return self.rejected(INTENT, "hunter_not_in_town");
        }
        let Some(task) = hunter.hunt.gear_enhancement.as_ref() else {
            return self.rejected(INTENT, "enhancement_visit_not_started");
        };
        if !matches!(
            task.status,
            GearEnhancementTaskStatus::WaitingForInteraction
                | GearEnhancementTaskStatus::Configuring
        ) {
            return self.rejected(INTENT, "hunter_not_ready_at_enhancement_forge");
        }
        let Some(owned) = hunter
            .owned_items
            .iter()
            .find(|owned| owned.gear_instance_id == Some(gear_instance_id) && owned.quantity > 0)
        else {
            return self.rejected(INTENT, "gear_instance_not_owned");
        };
        let Some(product) = self.building_content.gameplay.product(&owned.product_id) else {
            return self.rejected(INTENT, "gear_definition_unavailable");
        };
        if gear_product_route(&self.building_content.gameplay, product).is_none() {
            return self.rejected(INTENT, "product_is_not_gear");
        }
        if owned
            .enhancement_level
            .is_some_and(|level| level >= MAX_GEAR_ENHANCEMENT_LEVEL)
        {
            return self.rejected(INTENT, "gear_enhancement_cap_reached");
        }
        let product_id = owned.product_id.clone();
        let current_level = owned.enhancement_level;
        let hunter = &mut self.hunter_roster.hunters[hunter_index];
        let task = hunter
            .hunt
            .gear_enhancement
            .as_mut()
            .expect("enhancement task was validated above");
        task.status = GearEnhancementTaskStatus::Configuring;
        task.selected_gear_instance_id = Some(gear_instance_id);
        task.selected_product_id = Some(product_id);
        task.mode = Some(mode.to_owned());
        task.target_level = target_level;
        task.optional_material_ids = optional_material_ids.to_vec();
        task.attempts.clear();
        task.spent_gold = 0;
        task.spent_materials.clear();
        task.final_level = current_level;
        task.stop_reason = Some("evidence_disabled".to_owned());
        task.blockers = GEAR_ENHANCEMENT_BLOCKERS
            .iter()
            .map(|blocker| (*blocker).to_owned())
            .collect();
        hunter.profile.action_state = "configuring_enhancement".to_owned();
        hunter.profile.animation_name = "hunter_stay".to_owned();
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        // This is a terminal fail-closed result: no economy mutation occurred,
        // so the Hunter must be released instead of remaining pinned to the forge.
        release_hunter_from_enhancement(hunter);
        self.binding_blocked(INTENT, &GEAR_ENHANCEMENT_BLOCKERS)
    }

    fn equip_fixture_item(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        item_id: u32,
    ) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field || hunter_id != 1 {
            return self.rejected("equip_hunter_item", "fixture_hunter_unavailable");
        }
        let outcome = self
            .simulation
            .handle_command(FixtureCommand::EquipItem {
                command_id,
                item_id,
            })
            .expect("fixture equip always returns a command outcome");
        self.combat_snapshot = self.simulation.snapshot();
        ServerMessage::IntentResult {
            intent: "equip_hunter_item".to_owned(),
            accepted: outcome.accepted,
            reason: outcome.reason,
            snapshot: self.snapshot(),
        }
    }

    fn banish_hunter(&mut self, command_id: Uuid, hunter_id: u32) -> ServerMessage {
        if !matches!(
            self.state.screen,
            OriginalScreen::Village | OriginalScreen::HunterRoster
        ) {
            return self.rejected("banish_hunter", "hunter_roster_unavailable");
        }
        if !self.hunter_roster.roster_resolved {
            return self.rejected("banish_hunter", "hunter_roster_unresolved");
        }
        if self
            .product_services
            .visits
            .iter()
            .any(|visit| visit.hunter_id == hunter_id)
        {
            return self.rejected("banish_hunter", "hunter_busy");
        }
        match self
            .hunter_roster
            .banish_active_idempotent(command_id, hunter_id)
        {
            Ok(_) => {
                let banished_entity_id = village_hunter_entity_id(hunter_id);
                if self.selected_entity_id.as_deref() == Some(banished_entity_id.as_str()) {
                    self.selected_entity_id = None;
                }
                self.accepted("banish_hunter")
            }
            Err(HunterRosterError::ActiveHunterUnknown) => {
                self.rejected("banish_hunter", "active_hunter_unknown")
            }
            Err(HunterRosterError::CommandConflict) => {
                self.rejected("banish_hunter", "banish_command_conflict")
            }
            Err(HunterRosterError::DuplicateHunter | HunterRosterError::InvalidState(_)) => {
                self.rejected("banish_hunter", "hunter_roster_invalid")
            }
        }
    }

    fn select_bottom_menu(&mut self, menu: BottomMenuIntent) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("select_bottom_menu", "bottom_menu_unavailable");
        }
        match menu {
            BottomMenuIntent::Character => {
                self.state.screen = OriginalScreen::HunterRoster;
                self.selected_entity_id = None;
                self.accepted("select_bottom_menu.character")
            }
            BottomMenuIntent::Build => self.accepted("select_bottom_menu.build"),
            BottomMenuIntent::Archive => {
                self.binding_blocked("select_bottom_menu.archive", &["archive_rules_binding"])
            }
            BottomMenuIntent::Store => {
                self.binding_blocked("select_bottom_menu.store", &["store_catalog_binding"])
            }
            BottomMenuIntent::Raid => {
                self.binding_blocked("select_bottom_menu.raid", &["raid_rules_binding"])
            }
        }
    }

    fn navigate_back(&mut self) -> ServerMessage {
        match self.state.screen {
            OriginalScreen::HunterRoster | OriginalScreen::Field => {
                if self.state.screen == OriginalScreen::Field {
                    self.settle_returning_hunters();
                }
                self.state.screen = OriginalScreen::Village;
                self.selected_entity_id = None;
                self.accepted("navigate_back")
            }
            _ => self.rejected("navigate_back", "navigation_unavailable"),
        }
    }

    fn enter_field(&mut self) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("enter_field", "field_entry_unavailable");
        }
        self.state.screen = OriginalScreen::Field;
        self.buildings.field_trip_id = self.buildings.field_trip_id.saturating_add(1);
        self.selected_entity_id = None;
        self.accepted("enter_field")
    }

    fn enter_monster_map(&mut self, map_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field {
            return self.rejected("enter_monster_map", "field_required");
        }
        match self.monster_world.enter_map(map_id) {
            Ok(()) => {
                self.selected_entity_id = None;
                self.accepted("enter_monster_map")
            }
            Err(reason) => self.rejected("enter_monster_map", reason),
        }
    }

    fn set_monster_density(&mut self, level: u8) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field {
            return self.rejected("set_monster_density", "field_required");
        }
        match self.monster_world.set_density(level) {
            Ok(()) => self.accepted("set_monster_density"),
            Err(reason) => self.rejected("set_monster_density", reason),
        }
    }

    fn set_monster_region_density(&mut self, region_id: &str, level: u8) -> ServerMessage {
        if !matches!(
            self.state.screen,
            OriginalScreen::Village | OriginalScreen::Field
        ) {
            return self.rejected("set_monster_region_density", "world_required");
        }
        match self.monster_world.set_region_density(region_id, level) {
            Ok(()) => self.accepted("set_monster_region_density"),
            Err(reason) => self.rejected("set_monster_region_density", reason),
        }
    }

    fn select_monster_target(&mut self, monster_id: &str, hunter_id: u32) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field {
            return self.rejected("select_monster_target", "field_required");
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected("select_monster_target", "hunter_unavailable");
        };
        if hunter.current_hp == 0 || hunter.profile.action_state == "dead" {
            return self.rejected("select_monster_target", "hunter_unavailable");
        }
        match self.monster_world.select_target(monster_id, hunter_id) {
            Ok(()) => self.accepted("select_monster_target"),
            Err(reason) => self.rejected("select_monster_target", reason),
        }
    }

    fn select_entity(&mut self, entity_id: &str) -> ServerMessage {
        let selected = self
            .world_entities()
            .into_iter()
            .find(|entity| entity.descriptor.entity_id == entity_id && entity.selectable)
            .map(|entity| entity.descriptor.entity_id);
        let Some(selected) = selected else {
            return self.rejected("select_entity", "entity_unavailable");
        };
        self.selected_entity_id = Some(selected);
        self.accepted("select_entity")
    }

    fn apply_hunter_command<F>(
        &mut self,
        command_id: Uuid,
        key: &str,
        intent: &str,
        apply: F,
    ) -> ServerMessage
    where
        F: FnOnce(&mut DurableHunterRosterState) -> Result<(), HunterRosterError>,
    {
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == key {
                    self.accepted(intent)
                } else {
                    self.rejected(
                        intent,
                        "command id was already used for a different hunter action",
                    )
                };
            }
        }
        match apply(&mut self.hunter_roster) {
            Ok(()) => {
                if command_id != Uuid::nil() {
                    self.hunter_roster
                        .hunt_commands
                        .insert(command_id, key.to_owned());
                }
                self.accepted(intent)
            }
            Err(error) => self.rejected(intent, &error.to_string()),
        }
    }

    fn learn_hunter_skill(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        skill_id: &str,
    ) -> ServerMessage {
        let key = format!("learn_hunter_skill:{hunter_id}:{skill_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted("learn_hunter_skill")
                } else {
                    self.rejected(
                        "learn_hunter_skill",
                        "command id was already used for a different hunter action",
                    )
                };
            }
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(
                "learn_hunter_skill",
                "hunter is not in the active town roster",
            );
        };
        let Some(definition) = basic_hunter_skill_definition(skill_id) else {
            return self.rejected("learn_hunter_skill", "skill definition is unavailable");
        };
        if hunter.profile.class_id != definition.class_id
            || hunter.profile.visual_family != definition.class_family
        {
            return self.rejected("learn_hunter_skill", "skill is unavailable for hunter job");
        }
        if hunter
            .profile
            .skills
            .iter()
            .any(|skill| skill.skill_id == skill_id)
        {
            return self.rejected("learn_hunter_skill", "skill is already learned");
        }
        // Job ownership and cooldown come from the packaged basic-skill catalog.
        // Only the two H1 icon bindings are independently confirmed.
        hunter
            .profile
            .skills
            .push(super::hunter_roster::DurableHunterSkill {
                skill_id: definition.skill_id.to_owned(),
                display_name: definition.display_name.to_owned(),
                icon_path: definition.confirmed_icon_path.map(str::to_owned),
                animation_name: None,
                skill_level: 1,
                equipped_slot: None,
                ready: true,
                cooldown_remaining_ms: 0,
            });
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted("learn_hunter_skill")
    }

    /// Activates all ten packaged basic skills while leaving unresolved effect
    /// formulas unavailable instead of substituting guessed combat outcomes.
    fn use_hunter_skill(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        skill_id: &str,
        target_entity_id: Option<&str>,
        produce_response: bool,
    ) -> Option<ServerMessage> {
        let key = format!("use_hunter_skill:{hunter_id}:{skill_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    produce_response.then(|| self.accepted("use_hunter_skill"))
                } else {
                    produce_response
                        .then(|| self.rejected("use_hunter_skill", "command id was already used"))
                };
            }
        }
        let Some(definition) = basic_hunter_skill_definition(skill_id) else {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill definition is unavailable"));
        };
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return produce_response.then(|| {
                self.rejected(
                    "use_hunter_skill",
                    "hunter is not in the active town roster",
                )
            });
        };
        if hunter.profile.class_id != definition.class_id
            || hunter.profile.visual_family != definition.class_family
        {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill is unavailable for hunter job"));
        }
        let Some(skill) = hunter
            .profile
            .skills
            .iter()
            .find(|skill| skill.skill_id == skill_id)
        else {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill is not learned"));
        };
        if !skill.ready || skill.cooldown_remaining_ms > 0 {
            return produce_response
                .then(|| self.rejected("use_hunter_skill", "skill is on cooldown"));
        }
        if let Err(reason) = self.monster_world.validate_hunter_skill_effect(
            &self.hunter_roster,
            hunter_id,
            skill_id,
            target_entity_id,
        ) {
            return produce_response.then(|| self.rejected("use_hunter_skill", reason));
        }
        if let Err(reason) = self.monster_world.trigger_hunter_skill(
            hunter_id,
            target_entity_id,
            definition.class_family,
            definition.skill_id,
        ) {
            return produce_response.then(|| self.rejected("use_hunter_skill", reason));
        }
        if let Err(reason) =
            self.monster_world
                .apply_hunter_skill_effect(&self.hunter_roster, hunter_id, skill_id)
        {
            return produce_response.then(|| self.rejected("use_hunter_skill", reason));
        }
        if let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        {
            if let Some(skill) = hunter
                .profile
                .skills
                .iter_mut()
                .find(|skill| skill.skill_id == skill_id)
            {
                skill.ready = false;
                skill.cooldown_remaining_ms = definition.cooldown_ms;
            }
        }
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        produce_response.then(|| self.accepted("use_hunter_skill"))
    }

    fn refresh_skill_cooldowns(&mut self, elapsed_ms: u64) {
        for hunter in &mut self.hunter_roster.hunters {
            for skill in &mut hunter.profile.skills {
                if skill.cooldown_remaining_ms == 0 {
                    skill.ready = true;
                    continue;
                }
                skill.cooldown_remaining_ms =
                    skill.cooldown_remaining_ms.saturating_sub(elapsed_ms);
                skill.ready = skill.cooldown_remaining_ms == 0;
            }
        }
    }

    fn auto_cast_ready_hunter_skills(&mut self) {
        let casts = self
            .monster_world
            .hunters
            .iter()
            .filter_map(|agent| {
                if agent.action_state != HunterActionState::Attacking {
                    return None;
                }
                let target = agent.target_monster_id.clone()?;
                if agent.active_skill_id.is_some() || agent.region_id.is_none() {
                    return None;
                }
                let hunter = self
                    .hunter_roster
                    .hunters
                    .iter()
                    .find(|hunter| hunter.hunter_id == agent.hunter_id)?;
                let skill = hunter
                    .profile
                    .skills
                    .iter()
                    .find(|skill| skill.ready && skill.cooldown_remaining_ms == 0)?;
                Some((agent.hunter_id, skill.skill_id.clone(), target))
            })
            .collect::<Vec<_>>();
        for (hunter_id, skill_id, target) in casts {
            let _ = self.use_hunter_skill(Uuid::nil(), hunter_id, &skill_id, Some(&target), false);
        }
    }

    fn sell_hunter_loot(&mut self, command_id: Uuid, hunter_id: u32) -> ServerMessage {
        let requested_only = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .is_some_and(|hunter| {
                hunter.hunt.status == "hunting"
                    && hunter.hunt.zone_id.as_deref().is_some_and(|zone_id| {
                        super::hunter_roster::ORDINARY_HUNT_REGION_IDS.contains(&zone_id)
                    })
            });
        self.sell_hunter_loot_internal(command_id, hunter_id, requested_only)
    }

    fn sell_hunter_loot_internal(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        requested_only: bool,
    ) -> ServerMessage {
        let key = format!("sell_hunter_loot:{hunter_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted("sell_hunter_loot")
                } else {
                    self.rejected(
                        "sell_hunter_loot",
                        "command id was already used for a different hunter action",
                    )
                };
            }
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(
                "sell_hunter_loot",
                "hunter is not in the active town roster",
            );
        };
        let ordinary_field_auto_sale = requested_only
            && hunter.hunt.status == "hunting"
            && hunter.hunt.zone_id.as_deref().is_some_and(|zone_id| {
                super::hunter_roster::ORDINARY_HUNT_REGION_IDS.contains(&zone_id)
            });
        if !hunter.hunt.is_idle() && !ordinary_field_auto_sale {
            return self.rejected("sell_hunter_loot", "hunter must be idle to sell loot");
        }
        let mut sale_lines = BTreeMap::<String, u32>::new();
        for loot in &hunter.hunt.loot {
            if loot.quantity == 0 {
                continue;
            }
            if loot.item_id == "gold" {
                continue;
            }
            if !loot.item_id.starts_with("material:") {
                return self.rejected("sell_hunter_loot", "loot definition is unavailable");
            }
            let Some(item) = self.building_content.gameplay.item(&loot.item_id) else {
                return self.rejected("sell_hunter_loot", "loot definition is unavailable");
            };
            let already_selected = sale_lines.get(&loot.item_id).copied().unwrap_or(0);
            let sale_quantity = if requested_only {
                let remaining_request = self
                    .buildings
                    .material_stocks
                    .iter()
                    .find(|stock| stock.id == loot.item_id)
                    .map_or(0, |stock| stock.requested)
                    .saturating_sub(already_selected);
                loot.quantity.min(remaining_request)
            } else {
                loot.quantity
            };
            if sale_quantity == 0 {
                continue;
            }
            if item.item_type.as_deref() != Some("material")
                || item.town_pays_hunter_gold_per_unit.is_none()
            {
                return self.rejected("sell_hunter_loot", "loot price is unavailable");
            }
            let quantity = sale_lines.entry(loot.item_id.clone()).or_default();
            let Some(total) = quantity.checked_add(sale_quantity) else {
                return self.rejected("sell_hunter_loot", "loot quantity overflow");
            };
            *quantity = total;
        }
        if sale_lines.is_empty() {
            return self.rejected("sell_hunter_loot", "hunter has no hunt loot");
        }
        let mut priced_lines = Vec::with_capacity(sale_lines.len());
        let mut total_gold = 0_u64;
        let mut available_town_gold = self.buildings.town_gold;
        for (material_id, quantity) in sale_lines {
            let Some(unit_price) = self
                .building_content
                .gameplay
                .item(&material_id)
                .and_then(|item| item.town_pays_hunter_gold_per_unit)
            else {
                return self.rejected("sell_hunter_loot", "loot price is unavailable");
            };
            if unit_price == 0 {
                return self.rejected("sell_hunter_loot", "loot price is unavailable");
            }
            let affordable_quantity = available_town_gold / unit_price;
            let quantity = quantity.min(u32::try_from(affordable_quantity).unwrap_or(u32::MAX));
            if quantity == 0 {
                continue;
            }
            let Some(line_gold) = u64::from(quantity).checked_mul(unit_price) else {
                return self.rejected("sell_hunter_loot", "loot price overflow");
            };
            let Some(next_total) = total_gold.checked_add(line_gold) else {
                return self.rejected("sell_hunter_loot", "loot price overflow");
            };
            total_gold = next_total;
            available_town_gold -= line_gold;
            priced_lines.push((material_id, quantity, unit_price, line_gold));
        }
        if priced_lines.is_empty() {
            return self.rejected("sell_hunter_loot", "town wallet cannot afford loot");
        }
        let settlement_field_trip_id = self.buildings.field_trip_id.max(1);
        self.buildings.field_trip_id = settlement_field_trip_id;
        self.buildings.town_gold -= total_gold;
        for (material_id, quantity, unit_price, _) in &priced_lines {
            if let Some(stock) = self
                .buildings
                .material_stocks
                .iter_mut()
                .find(|stock| stock.id == *material_id)
            {
                stock.town_quantity = stock.town_quantity.saturating_add(*quantity);
                stock.requested = stock.requested.saturating_sub(*quantity);
                stock.unit_price = *unit_price;
            } else {
                self.buildings.material_stocks.push(DurableMaterialStock {
                    id: material_id.clone(),
                    town_quantity: *quantity,
                    hunter_quantity: 0,
                    requested: 0,
                    unit_price: *unit_price,
                });
            }
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            unreachable!()
        };
        hunter.gold = hunter.gold.saturating_add(total_gold);
        let mut sold_quantities = priced_lines
            .iter()
            .map(|(material_id, quantity, _, _)| (material_id.clone(), *quantity))
            .collect::<BTreeMap<_, _>>();
        for loot in &mut hunter.hunt.loot {
            if let Some(sold) = sold_quantities.get_mut(&loot.item_id) {
                let deducted = loot.quantity.min(*sold);
                loot.quantity -= deducted;
                *sold -= deducted;
            }
        }
        hunter.hunt.loot.retain(|loot| loot.quantity > 0);
        for (line_index, (material_id, quantity, unit_price, line_gold)) in
            priced_lines.into_iter().enumerate()
        {
            let settlement_id = if line_index == 0 {
                command_id.to_string()
            } else {
                format!("{command_id}:{line_index}")
            };
            self.buildings
                .trade_settlements
                .push(DurableTradeSettlement {
                    settlement_id,
                    field_trip_id: settlement_field_trip_id,
                    material_id,
                    quantity,
                    unit_price,
                    total_gold: line_gold,
                });
        }
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted("sell_hunter_loot")
    }

    fn auto_sell_requested_hunter_loot(&mut self) {
        let hunter_ids = self
            .hunter_roster
            .hunters
            .iter()
            .filter(|hunter| {
                let can_settle_requested_loot = hunter.hunt.is_idle()
                    || (hunter.hunt.status == "hunting"
                        && hunter.hunt.zone_id.as_deref().is_some_and(|zone_id| {
                            super::hunter_roster::ORDINARY_HUNT_REGION_IDS.contains(&zone_id)
                        }));
                can_settle_requested_loot && self.has_affordable_auto_sale(hunter)
            })
            .map(|hunter| hunter.hunter_id)
            .collect::<Vec<_>>();
        for hunter_id in hunter_ids {
            let settlement_id = self.next_auto_trade_settlement_id(hunter_id);
            let _ = self.sell_hunter_loot_internal(settlement_id, hunter_id, true);
        }
    }

    /// Avoid invoking the rejection path on every simulation tick when a
    /// requested sale cannot succeed. Rejections build a complete snapshot,
    /// which is too expensive for the 10 Hz movement loop.
    fn has_affordable_auto_sale(&self, hunter: &DurableHunterState) -> bool {
        for loot in &hunter.hunt.loot {
            if loot.quantity == 0 || loot.item_id == "gold" {
                continue;
            }
            let Some(item) = self.building_content.gameplay.item(&loot.item_id) else {
                return false;
            };
            let Some(unit_price) = item.town_pays_hunter_gold_per_unit else {
                return false;
            };
            if unit_price == 0 {
                return false;
            }
            let Some(stock) = self
                .buildings
                .material_stocks
                .iter()
                .find(|stock| stock.id == loot.item_id)
            else {
                continue;
            };
            if stock.requested > 0
                && loot.quantity.min(stock.requested) > 0
                && self.buildings.town_gold >= unit_price
            {
                return true;
            }
        }
        false
    }

    fn next_auto_trade_settlement_id(&self, hunter_id: u32) -> Uuid {
        let mut sequence = u32::try_from(self.buildings.trade_settlements.len())
            .unwrap_or(u32::MAX)
            .saturating_add(1);
        loop {
            let id = Uuid::from_u128(
                (u128::from(self.buildings.field_trip_id.max(1)) << 64)
                    | (u128::from(hunter_id) << 32)
                    | u128::from(sequence),
            );
            let id_text = id.to_string();
            let settlement_exists = self.buildings.trade_settlements.iter().any(|settlement| {
                settlement.settlement_id == id_text
                    || settlement.settlement_id.starts_with(&format!("{id_text}:"))
            });
            if !settlement_exists && !self.hunter_roster.hunt_commands.contains_key(&id) {
                return id;
            }
            sequence = sequence.wrapping_add(1);
        }
    }

    fn world_projection(&self) -> WorldProjection {
        WorldProjection {
            mode: match self.state.screen {
                OriginalScreen::Village => WorldMode::Village,
                OriginalScreen::Field => WorldMode::Field,
                OriginalScreen::Boot | OriginalScreen::HunterRoster => WorldMode::Inactive,
            },
            visual_tick: self.visual_tick,
            coordinate_space: "scene_pixels_v1",
            authority_scope: "server_authoritative_simulation",
            entities: self.world_entities(),
            selected_entity_id: self.selected_entity_id.clone(),
            drops: self
                .monster_world
                .fields
                .iter()
                .flat_map(|field| &field.drops)
                .map(|drop| WorldDropProjection {
                    drop_id: drop.drop_id.clone(),
                    item_id: drop.item_id.clone(),
                    quantity: drop.quantity,
                    x: drop.x,
                    y: drop.y,
                    icon_path: drop_icon_path(&drop.item_id),
                })
                .collect(),
            combat_presentations: self
                .monster_world
                .combat_presentations
                .iter()
                .map(|event| CombatPresentationSnapshot {
                    sequence: event.sequence,
                    source_entity_id: event.source_entity_id.clone(),
                    target_entity_id: event.target_entity_id.clone(),
                    kind: event.kind,
                    amount: event.amount,
                })
                .collect(),
        }
    }

    fn world_entities(&self) -> Vec<WorldEntityProjection> {
        match self.state.screen {
            OriginalScreen::Village => {
                let mut entities = self
                    .hunter_roster
                    .hunters
                    .iter()
                    .enumerate()
                    .map(|(slot, hunter)| {
                        let mut entity = if let Some(agent) = self
                            .monster_world
                            .hunters
                            .iter()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)
                        {
                            hunter_visual_entity(agent, hunter.current_hp, hunter.max_hp)
                        } else {
                            let motion = village_hunter_motion(self.visual_tick, slot);
                            visual_entity(
                                village_hunter_entity_id(hunter.hunter_id),
                                WorldEntityKind::Hunter,
                                "hunter",
                                "hunter",
                                BindingConfidence::Confirmed,
                                motion.x,
                                motion.y,
                                motion.facing,
                                motion.action_state,
                                motion.animation,
                            )
                        };
                        entity.class_family = Some(hunter.profile.visual_family.clone());
                        entity.loot_label =
                            self.monster_world
                                .hunters
                                .iter()
                                .find(|agent| agent.hunter_id == hunter.hunter_id)
                                .and_then(|agent| {
                                    agent
                                        .loot_item_id
                                        .as_deref()
                                        .map(|item_id| (item_id, agent.loot_quantity))
                                })
                                .and_then(|(item_id, loot_quantity)| {
                                    if item_id == "gold" {
                                        Some(format!("Gold +{loot_quantity}"))
                                    } else {
                                        self.building_content.gameplay.item(item_id).and_then(
                                            |item| {
                                                item.localized_names
                                                    .get("en")
                                                    .cloned()
                                                    .or_else(|| item.internal_name.clone())
                                                    .map(|name| format!("{name} x{loot_quantity}"))
                                            },
                                        )
                                    }
                                });
                        entity.attack_effect_key =
                            match (entity.action_state, hunter.profile.visual_family.as_str()) {
                                (WorldEntityActionState::Attacking, "H3")
                                    if entity.skill_presentation_key.is_none()
                                        && !entity.animation.ends_with("_skill") =>
                                {
                                    Some("ranger_basic_arrow")
                                }
                                _ => None,
                            };
                        entity.current_hp = Some(hunter.current_hp);
                        entity.maximum_hp = Some(hunter.max_hp);
                        entity.interaction_prompt_key = hunter
                            .hunt
                            .gear_enhancement
                            .as_ref()
                            .filter(|task| {
                                matches!(
                                    task.status,
                                    GearEnhancementTaskStatus::WaitingForInteraction
                                        | GearEnhancementTaskStatus::Configuring
                                        | GearEnhancementTaskStatus::Result
                                )
                            })
                            .map(|_| "hunter_enhancement_ready");
                        entity
                    })
                    .collect::<Vec<_>>();
                entities.push(visual_entity(
                    "village-npc-01",
                    WorldEntityKind::Npc,
                    "npc",
                    "Npc",
                    BindingConfidence::Confirmed,
                    1760,
                    684,
                    Facing::Left,
                    WorldEntityActionState::Idle,
                    "npc_stay",
                ));
                entities.extend(
                    self.monster_world
                        .fields
                        .iter()
                        .flat_map(|field| field.monsters.iter().map(monster_visual_entity)),
                );
                entities
            }
            OriginalScreen::Field => {
                // Field rendering must use the authoritative hunting agents.
                // The former roaming fixture had a different entity id, which
                // hid movement and caused target-bound EXP events to expire.
                let mut entities = self
                    .hunter_roster
                    .hunters
                    .iter()
                    .filter_map(|hunter| {
                        let agent = self
                            .monster_world
                            .hunters
                            .iter()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)?;
                        let mut entity =
                            hunter_visual_entity(agent, hunter.current_hp, hunter.max_hp);
                        entity.class_family = Some(hunter.profile.visual_family.clone());
                        entity.loot_label = agent.loot_item_id.as_deref().and_then(|item_id| {
                            if item_id == "gold" {
                                Some(format!("Gold +{}", agent.loot_quantity))
                            } else {
                                self.building_content
                                    .gameplay
                                    .item(item_id)
                                    .and_then(|item| {
                                        item.localized_names
                                            .get("en")
                                            .cloned()
                                            .or_else(|| item.internal_name.clone())
                                            .map(|name| format!("{name} x{}", agent.loot_quantity))
                                    })
                            }
                        });
                        entity.current_hp = Some(hunter.current_hp);
                        entity.maximum_hp = Some(hunter.max_hp);
                        Some(entity)
                    })
                    .collect::<Vec<_>>();
                entities.extend(
                    self.monster_world
                        .fields
                        .iter()
                        .flat_map(|field| field.monsters.iter().map(monster_visual_entity)),
                );
                entities
            }
            OriginalScreen::Boot | OriginalScreen::HunterRoster => Vec::new(),
        }
    }

    fn accepted(&self, intent: &str) -> ServerMessage {
        ServerMessage::IntentResult {
            intent: intent.to_owned(),
            accepted: true,
            reason: None,
            snapshot: self.snapshot(),
        }
    }

    fn rejected(&self, intent: &str, reason: &str) -> ServerMessage {
        ServerMessage::IntentResult {
            intent: intent.to_owned(),
            accepted: false,
            reason: Some(reason.to_owned()),
            snapshot: self.snapshot(),
        }
    }

    fn binding_blocked(&self, intent: &str, blockers: &[&str]) -> ServerMessage {
        ServerMessage::BindingBlocked {
            intent: intent.to_owned(),
            blockers: blockers
                .iter()
                .map(|blocker| (*blocker).to_owned())
                .collect(),
            snapshot: self.snapshot(),
        }
    }

    fn capability_blocked(
        &self,
        intent: &str,
        building_id: &str,
        expected_kinds: &[&str],
    ) -> ServerMessage {
        let Ok(building_id) = BaseBuildingId::parse(building_id) else {
            return self.binding_blocked(intent, &["building_base_id_parse"]);
        };
        let matching = self
            .building_content
            .gameplay
            .capabilities_for(&building_id)
            .filter(|capability| {
                expected_kinds.is_empty() || expected_kinds.contains(&capability.kind.as_str())
            })
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return self.binding_blocked(intent, &["building_capability_identity_binding"]);
        }
        if matching.iter().any(|capability| capability.runnable) {
            return self.binding_blocked(intent, &["building_capability_executor_binding"]);
        }
        self.binding_blocked(intent, &BUILDING_CAPABILITY_BLOCKERS)
    }
}

fn drop_icon_path(item_id: &str) -> String {
    if item_id == "gold" {
        return "/content/releases/original-flow-v1/sprites/top_ic_01_gold_24__4677.png".to_owned();
    }
    material_icon_path(item_id).unwrap_or_default()
}

fn building_definition_snapshot(
    building: &BaseBuildingDefinition,
    content: &AuthoritativeBuildingContent,
) -> BuildingDefinitionSnapshot {
    let construction = content.catalog.level(&building.id, 1);
    let prerequisite = construction.and_then(|level| level.prerequisites.first());
    BuildingDefinitionSnapshot {
        id: building.id.to_string(),
        name: building.display_name.clone(),
        feature: building
            .category
            .clone()
            .or_else(|| {
                content
                    .gameplay
                    .capabilities_for(&building.id)
                    .find(|capability| capability.static_data_ready)
                    .map(|capability| capability.kind.clone())
            })
            .unwrap_or_else(|| "unresolved".to_owned()),
        max_level: content
            .catalog
            .levels
            .iter()
            .filter(|level| level.building_id == building.id)
            .filter_map(|level| u8::try_from(level.level).ok())
            .max()
            .unwrap_or(0),
        construct_cost: construction.and_then(gold_cost).unwrap_or(0),
        prerequisite_id: prerequisite.map(|value| value.building_id.to_string()),
        prerequisite_level: prerequisite
            .and_then(|value| u8::try_from(value.required_level).ok())
            .unwrap_or(0),
        max_build: building.max_instances,
        grid_width: building_grid_size(building).map_or(0, |size| size.0),
        grid_height: building_grid_size(building).map_or(0, |size| size.1),
        sprite_asset_id: building.base_sprite_asset_id.clone(),
    }
}

fn building_grid_size(building: &BaseBuildingDefinition) -> Option<(u32, u32)> {
    Some((
        u32::from(building.grid_width),
        u32::from(building.grid_height),
    ))
}

fn mutation_condition(
    flow: &OriginalFlowSession,
    row: Option<&BuildingLevelDefinition>,
) -> Option<String> {
    let row = row?;
    row.prerequisites.iter().find_map(|prerequisite| {
        let current_level = flow
            .buildings
            .buildings
            .iter()
            .filter(|building| building.id == prerequisite.building_id.as_str())
            .map(|building| u16::from(building.level))
            .max()
            .unwrap_or(0);
        (current_level < prerequisite.required_level).then(|| {
            format!(
                "building_prerequisite_required:{}",
                prerequisite.building_id
            )
        })
    })
}

fn can_pay_costs(state: &DurableBuildingState, costs: &[EconomyAmount]) -> bool {
    costs.iter().all(|cost| {
        if cost.resource_id == "currency:gold" {
            state.town_gold >= cost.quantity
        } else {
            state
                .material_stocks
                .iter()
                .find(|stock| stock.id == cost.resource_id)
                .is_some_and(|stock| u64::from(stock.town_quantity) >= cost.quantity)
        }
    })
}

/// Resolves the recovered consumable price table for a crafted potion row.
/// Product `salePrice` is intentionally absent for these rows in the catalog.
fn product_sale_building_id(
    gameplay: &BuildingGameplayCatalog,
    product: &crate::buildings::EconomyProductDefinition,
) -> Option<BaseBuildingId> {
    if let Some(route) = gear_product_route(gameplay, product) {
        return Some(route.sale_building_id);
    }
    let is_potion = product
        .building_id
        .as_ref()
        .is_some_and(|id| id.as_str() == "build_14")
        && product.outputs.len() == 1
        && product.outputs[0].resource_id.starts_with("consumable:");
    is_potion.then(|| BaseBuildingId::parse("build_11").expect("potion shop id is canonical"))
}

fn consumable_purchase_price(
    gameplay: &BuildingGameplayCatalog,
    product: &crate::buildings::EconomyProductDefinition,
) -> Option<u64> {
    let output = product.outputs.first()?;
    if !output.resource_id.starts_with("consumable:") || output.quantity == 0 {
        return None;
    }
    let level = product
        .product_id
        .split_once(":level:")?
        .1
        .parse::<usize>()
        .ok()?;
    gameplay
        .item(&output.resource_id)?
        .hunter_pays_town_gold_by_tier
        .get(level)
        .copied()
}

fn product_display_name(product_id: &str) -> Option<&'static str> {
    Some(match product_id {
        "product:0" => "Small Room",
        "product:1" => "Standard Room",
        "product:2" => "Superior Room",
        "product:3" => "Deluxe Room",
        "product:4" => "Suite Room",
        "product:5" => "Linen Bandage",
        "product:6" => "Wool Bandage",
        "product:7" => "Silk Bandage",
        "product:8" => "Magic Bandage",
        "product:9" => "Hell Bandage",
        "product:10" => "Cake",
        "product:11" => "Parfait",
        "product:12" => "Handmade Burger",
        "product:13" => "Tomato Pasta",
        "product:14" => "Tenderloin Steak",
        "product:15" => "Orange Juice",
        "product:16" => "Beer",
        "product:17" => "Red Wine",
        "product:18" => "Cocktail",
        "product:19" => "Whiskey",
        "product:29" => "Luxury Room",
        "product:30" => "Shiny Bandage",
        "product:31" => "Three Course Meal",
        "product:32" => "Vodka",
        "product:48" => "Special Room",
        "product:49" => "Pink Silk Bandage",
        "product:50" => "Afternoon Meal",
        "product:51" => "Tequila",
        _ => return None,
    })
}

fn product_icon_path(product_id: &str) -> Option<&'static str> {
    Some(match product_id {
        "product:0" => "/content/releases/original-flow-v1/sprites/product_00__3523.png",
        "product:1" => "/content/releases/original-flow-v1/sprites/product_01__4988.png",
        "product:2" => "/content/releases/original-flow-v1/sprites/product_02__4912.png",
        "product:3" => "/content/releases/original-flow-v1/sprites/product_03__2634.png",
        "product:4" => "/content/releases/original-flow-v1/sprites/product_04__7168.png",
        "product:5" => "/content/releases/original-flow-v1/sprites/product_05__2957.png",
        "product:6" => "/content/releases/original-flow-v1/sprites/product_06__3994.png",
        "product:7" => "/content/releases/original-flow-v1/sprites/product_07__2037.png",
        "product:8" => "/content/releases/original-flow-v1/sprites/product_08__6490.png",
        "product:9" => "/content/releases/original-flow-v1/sprites/product_09__1935.png",
        "product:10" => "/content/releases/original-flow-v1/sprites/product_10__6271.png",
        "product:11" => "/content/releases/original-flow-v1/sprites/product_11__2026.png",
        "product:12" => "/content/releases/original-flow-v1/sprites/product_12__1368.png",
        "product:13" => "/content/releases/original-flow-v1/sprites/product_13__3637.png",
        "product:14" => "/content/releases/original-flow-v1/sprites/product_14__1604.png",
        "product:15" => "/content/releases/original-flow-v1/sprites/product_15__6488.png",
        "product:16" => "/content/releases/original-flow-v1/sprites/product_16__3707.png",
        "product:17" => "/content/releases/original-flow-v1/sprites/product_17__6592.png",
        "product:18" => "/content/releases/original-flow-v1/sprites/product_18__6216.png",
        "product:19" => "/content/releases/original-flow-v1/sprites/product_19__5193.png",
        "product:29" => "/content/releases/original-flow-v1/sprites/product_29__6396.png",
        "product:30" => "/content/releases/original-flow-v1/sprites/product_30__3026.png",
        "product:31" => "/content/releases/original-flow-v1/sprites/product_31__1771.png",
        "product:32" => "/content/releases/original-flow-v1/sprites/product_32__7065.png",
        "product:48" => "/content/releases/original-flow-v1/sprites/product_48__4411.png",
        "product:49" => "/content/releases/original-flow-v1/sprites/product_49__4142.png",
        "product:50" => "/content/releases/original-flow-v1/sprites/product_50__4905.png",
        "product:51" => "/content/releases/original-flow-v1/sprites/product_51__6664.png",
        _ => return None,
    })
}

fn material_icon_path(material_id: &str) -> Option<String> {
    if let Some(index) = material_id
        .strip_prefix("material:")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|index| *index < 369)
    {
        return Some(format!(
            "/content/releases/evil-hunter-1.411/material-icons/material-{index}.png"
        ));
    }
    Some(
        match material_id {
            "currency:gem" => "/content/releases/original-flow-v1/sprites/top_ic_02_gem__6963.png",
            "currency:elemental" => {
                "/content/releases/original-flow-v1/sprites/top_ic_03_element__4250.png"
            }
            _ => return None,
        }
        .to_owned(),
    )
}

fn service_effect_kind(building_id: &str) -> &'static str {
    match building_id {
        "build_9" => "stamina",
        "build_12" => "HP",
        "build_13" => "satiety",
        "build_19" => "mood",
        _ => "service",
    }
}

fn hunter_service_gauge(
    hunter: &DurableHunterState,
    effect_kind: ServiceEffectKind,
) -> HunterServiceGauge {
    match effect_kind {
        ServiceEffectKind::Hp => HunterServiceGauge {
            current: hunter.current_hp,
            maximum: hunter.max_hp,
        },
        ServiceEffectKind::Stamina => hunter.stamina,
        ServiceEffectKind::Satiety => hunter.satiety,
        ServiceEffectKind::Mood => hunter.mood,
    }
}

fn hunter_roster_member(
    hunter: &DurableHunterState,
    roster_state: &'static str,
    position: usize,
) -> HunterRosterMemberSnapshot {
    let experience = hunter
        .profile
        .xp_to_next_level
        .map(|maximum| HunterProgressSnapshot {
            current: hunter.profile.xp,
            maximum,
        });
    let progress = |value: crate::simulation::DurableHunterProgress| HunterProgressSnapshot {
        current: u64::from(value.current),
        maximum: u64::from(value.maximum),
    };
    HunterRosterMemberSnapshot {
        hunter_id: hunter.hunter_id,
        display_name: hunter.profile.display_name.clone(),
        portrait_asset_id: hunter.profile.portrait_asset_id.clone(),
        class_id: hunter.profile.class_id.clone(),
        class_name: hunter.profile.class_name.clone(),
        class_family: hunter.profile.visual_family.clone(),
        rarity_id: hunter.profile.rarity_id.clone(),
        rarity_name: hunter.profile.rarity_name.clone(),
        level: hunter.profile.level,
        xp: hunter.profile.xp,
        gold: hunter.gold,
        current_hp: hunter.current_hp,
        max_hp: hunter.max_hp,
        stamina: hunter.stamina.current,
        max_stamina: hunter.stamina.maximum,
        satiety: hunter.satiety.current,
        max_satiety: hunter.satiety.maximum,
        mood: hunter.mood.current,
        max_mood: hunter.mood.maximum,
        attack: hunter.profile.attack,
        defense: hunter.profile.defense,
        action_state: hunter.profile.action_state.clone(),
        animation: hunter.profile.animation_name.clone(),
        trait_name: hunter
            .profile
            .traits
            .iter()
            .find(|hunter_trait| hunter_trait.equipped)
            .map(|hunter_trait| hunter_trait.display_name.clone()),
        traits: hunter
            .profile
            .traits
            .iter()
            .map(|hunter_trait| HunterTraitSnapshot {
                trait_id: hunter_trait.trait_id.clone(),
                display_name: hunter_trait.display_name.clone(),
                icon_path: hunter_trait.icon_path.clone(),
                unlocked_rank: hunter_trait.unlocked_rank,
                equipped: hunter_trait.equipped,
            })
            .collect(),
        skills: hunter
            .profile
            .skills
            .iter()
            .map(|skill| HunterSkillSnapshot {
                skill_id: skill.skill_id.clone(),
                display_name: skill.display_name.clone(),
                icon_path: skill.icon_path.clone(),
                animation_name: skill.animation_name.clone(),
                level: skill.skill_level,
                equipped_slot: skill.equipped_slot,
                ready: skill.ready,
                cooldown_remaining_ms: skill.cooldown_remaining_ms,
            })
            .collect(),
        hunt: HunterHuntSnapshot {
            status: if hunter.hunt.is_idle() {
                "idle".to_owned()
            } else {
                hunter.hunt.status.clone()
            },
            zone_id: hunter.hunt.zone_id.clone(),
            progress_ticks: hunter.hunt.progress_ticks,
            required_ticks: HUNT_TICKS_TO_RETURN,
            loot: hunter
                .hunt
                .loot
                .iter()
                .map(|loot| HunterLootSnapshot {
                    item_id: loot.item_id.clone(),
                    quantity: loot.quantity,
                })
                .collect(),
            ruleset: "web-rebuild-v1-fixture",
        },
        hunter_info: HunterInfoSnapshot {
            characteristic_name: hunter.profile.characteristic_name.clone(),
            locked: hunter.profile.is_locked,
            reincarnation: hunter.profile.reincarnation.map(progress),
            experience,
            status: HunterStatusSnapshot {
                dps_milli: hunter.profile.dps_milli,
                critical_rate_bps: hunter.profile.critical_rate_bps,
                attack_speed_milli: hunter.profile.attack_speed_milli,
                evasion_rate_bps: hunter.profile.evasion_rate_bps,
                awakening: hunter.profile.awakening.map(progress),
            },
            equipment_slots: Some(
                hunter
                    .profile
                    .equipment_slots
                    .iter()
                    .map(|equipment| HunterEquipmentSlotSnapshot {
                        slot_id: equipment.slot_id.clone(),
                        catalog_kind: equipment.catalog_kind.clone(),
                        catalog_index: equipment.catalog_index,
                        display_name: equipment.display_name.clone(),
                        icon_path: Some(equipment.icon_path.clone()),
                        placeholder_icon_path: None,
                        presentation_gender: equipment.presentation_gender.clone(),
                        required_class_id: equipment.required_class_id.clone(),
                        locked: Some(equipment.locked),
                        evidence_state: equipment.evidence_state.clone(),
                    })
                    .collect(),
            ),
            skills: Some(hunter_skill_catalog_preview(hunter)),
            growth: None,
            riding_pet: hunter.profile.riding_pet_state_resolved.then_some(
                HunterRidingPetSnapshot::Empty {
                    mounted: false,
                    can_move_to_ranch: false,
                },
            ),
            materials: Some(
                hunter
                    .hunt
                    .loot
                    .iter()
                    .enumerate()
                    .filter(|(_, loot)| loot.quantity > 0 && loot.item_id.starts_with("material:"))
                    .map(|(order, loot)| HunterMaterialSnapshot {
                        material_id: loot.item_id.clone(),
                        display_name: None,
                        icon_path: drop_icon_path(&loot.item_id),
                        quantity: u64::from(loot.quantity),
                        order: u32::try_from(order).unwrap_or(u32::MAX),
                    })
                    .collect(),
            ),
        },
        gear_enhancements: hunter
            .owned_items
            .iter()
            .filter(|owned| owned.quantity > 0 && owned.gear_instance_id.is_some())
            .map(|owned| GearEnhancementSnapshot {
                product_id: owned.product_id.clone(),
                level: owned.enhancement_level,
                max_level: MAX_GEAR_ENHANCEMENT_LEVEL,
                instance_id: owned.gear_instance_id,
                evidence_state: "unresolved",
            })
            .collect(),
        gear_enhancement_task: hunter
            .hunt
            .gear_enhancement
            .as_ref()
            .map(gear_enhancement_task_snapshot),
        runtime_evidence: runtime_evidence_snapshot(hunter),
        roster_state,
        position,
    }
}

fn gear_enhancement_task_snapshot(
    task: &DurableGearEnhancementTask,
) -> GearEnhancementTaskSnapshot {
    let status = match task.status {
        GearEnhancementTaskStatus::Traveling => "traveling",
        GearEnhancementTaskStatus::WaitingForInteraction => "waiting_for_interaction",
        GearEnhancementTaskStatus::Configuring => "configuring",
        GearEnhancementTaskStatus::Processing => "processing",
        GearEnhancementTaskStatus::Result => "result",
    };
    let resources = |rows: &[super::hunter_roster::DurableHunterLoot]| {
        rows.iter()
            .map(|row| GearEnhancementResourceSnapshot {
                material_id: row.item_id.clone(),
                quantity: row.quantity,
            })
            .collect::<Vec<_>>()
    };
    GearEnhancementTaskSnapshot {
        building_instance_id: task.building_instance_id.clone(),
        status,
        interaction_ready: matches!(
            task.status,
            GearEnhancementTaskStatus::WaitingForInteraction
                | GearEnhancementTaskStatus::Configuring
                | GearEnhancementTaskStatus::Result
        ),
        selected_gear_instance_id: task.selected_gear_instance_id,
        selected_product_id: task.selected_product_id.clone(),
        mode: task.mode.clone(),
        target_level: task.target_level,
        optional_material_ids: task.optional_material_ids.clone(),
        next_attempt_gold_cost: None,
        next_attempt_success_bps: None,
        required_materials: Vec::new(),
        attempts: task
            .attempts
            .iter()
            .map(|attempt| GearEnhancementAttemptSnapshot {
                attempt: attempt.attempt,
                starting_level: attempt.starting_level,
                resulting_level: attempt.resulting_level,
                succeeded: attempt.succeeded,
                gold_spent: attempt.gold_spent,
                materials_spent: resources(&attempt.materials_spent),
            })
            .collect(),
        spent_gold: task.spent_gold,
        spent_materials: resources(&task.spent_materials),
        final_level: task.final_level,
        stop_reason: task.stop_reason.clone(),
        blockers: task.blockers.clone(),
    }
}

fn hunter_skill_catalog_preview(hunter: &DurableHunterState) -> Vec<HunterInfoSkillSnapshot> {
    let rows: [(&str, &str, Option<&str>, &str); 2] = match hunter.profile.class_id.as_str() {
        "h1" => [
            (
                "skill_h1_01",
                "Fury",
                Some("skills/skill_h1_01__1395.png"),
                "Attacks quickly for a certain time and increases Attack Speed.",
            ),
            (
                "skill_h1_02",
                "War Cry",
                Some("skills/skill_h1_02__5620.png"),
                "Charge to enemy and Stun it.",
            ),
        ],
        "h2" => [
            (
                "skill_h2_01",
                "Holy Light",
                None,
                "Hits and provokes nearby enemies with holy power.",
            ),
            (
                "skill_h2_02",
                "Barrier",
                None,
                "Defends against enemy attacks by summoning a barrier.",
            ),
        ],
        "h3" => [
            (
                "skill_h3_01",
                "Multishot",
                None,
                "Rapidly shoots multiple arrows.",
            ),
            (
                "skill_h3_02",
                "Dodge",
                None,
                "Increases Evasion for a certain time.",
            ),
        ],
        "h4" => [
            (
                "skill_h4_01",
                "Thunderbolt",
                None,
                "Lightning inflicts damage to nearby enemies.",
            ),
            (
                "skill_h4_02",
                "Ice Armor",
                None,
                "Enemy who attacked hunter will lose ATK SPD.",
            ),
        ],
        _ => [
            (
                "skill_h5_01",
                "Round Slash",
                None,
                "Swing the spear horizontally to release energy, dealing damage to enemies nearby.",
            ),
            (
                "skill_h5_02",
                "Concentrate",
                None,
                "Concentrate to increase the chance of dealing a critical strike.",
            ),
        ],
    };
    rows.into_iter()
        .map(|(skill_id, display_name, icon, description)| {
            let learned = hunter
                .profile
                .skills
                .iter()
                .find(|skill| skill.skill_id == skill_id);
            HunterInfoSkillSnapshot {
                skill_id: skill_id.to_owned(),
                display_name: display_name.to_owned(),
                icon_path: icon.map(|icon| {
                    format!("/content/releases/evil-hunter-1.411/hunter-assets/ui/{icon}")
                }),
                level: learned.map(|skill| skill.skill_level),
                description: Some(format!(
                    "{description} Catalog definition; learned state remains server-owned."
                )),
                group: Some("Basic Skills".to_owned()),
                unlocked: learned.map(|_| true),
                unlock_requirement: learned
                    .is_none()
                    .then(|| "Learned-state fixture unresolved".to_owned()),
                ready: learned.map(|skill| skill.ready),
                cooldown_remaining_ms: learned.map(|skill| skill.cooldown_remaining_ms),
            }
        })
        .collect()
}

fn release_hunter_from_enhancement(hunter: &mut DurableHunterState) {
    hunter.hunt.gear_enhancement = None;
    hunter.profile.action_state = "idle".to_owned();
    hunter.profile.animation_name = "hunter_stay".to_owned();
}

fn enhancement_task_terminal(task: &DurableGearEnhancementTask) -> bool {
    task.status == GearEnhancementTaskStatus::Result
        || (task.status == GearEnhancementTaskStatus::Configuring && task.stop_reason.is_some())
}

fn is_enhancement_action_state(action_state: &str) -> bool {
    matches!(
        action_state,
        "traveling_to_enhancement_forge"
            | "waiting_for_enhancement_interaction"
            | "configuring_enhancement"
    )
}

fn runtime_evidence_snapshot(hunter: &DurableHunterState) -> HunterRuntimeEvidenceSnapshot {
    let runtime = &hunter.runtime;
    let job = match (
        runtime.source_job,
        runtime.source_sub_job,
        runtime.source_third_job,
        runtime.source_fourth_job,
        runtime.source_personality,
    ) {
        (Some(job), Some(sub_job), Some(third_job), Some(fourth_job), Some(personality)) => {
            Some(HunterRuntimeJobSnapshot {
                job,
                sub_job,
                third_job,
                fourth_job,
                personality,
                grade_rank_up: runtime.source_grade_rank_up,
                dark_soul: runtime.source_dark_soul,
                used_dark_soul: runtime.source_used_dark_soul,
                used_job_trait: runtime.source_used_job_trait,
            })
        }
        _ => None,
    };
    let status = runtime
        .status
        .as_ref()
        .map(|status| HunterRuntimeStatusSnapshot {
            maximum_hp: status.hp,
            current_hp: status.now_hp,
            maximum_mood: status.feel,
            current_mood: status.now_feel,
            maximum_satiety: status.hungry,
            current_satiety: status.now_hungry,
            maximum_stamina: status.tire,
            current_stamina: status.now_tire,
            attack: status.damage,
            defense: status.armor,
            critical: status.critical,
            attack_speed: status.attack_speed,
            evasion: status.dodge,
        });
    let skills = runtime.skills.as_ref().map(|skills| {
        skills
            .iter()
            .map(|skill| HunterRuntimeSkillSnapshot {
                source_key: skill.dictionary_key.clone(),
                source_index: skill.source_index,
                skill_definition_index: skill.skill_index,
                cooldown_raw: skill.cool_time,
                level: skill.level,
            })
            .collect()
    });
    let appearance =
        runtime
            .appearance
            .as_ref()
            .map(|appearance| HunterRuntimeAppearanceSnapshot {
                body_index: appearance.body_index,
                costume_index: appearance.costume_index,
                costume_hidden: appearance.costume_hidden,
                fairy_index: appearance.fairy_index,
                fairy_hidden: appearance.fairy_hidden,
                weapon_costume_index: appearance.weapon_costume_index,
                weapon_costume_hidden: appearance.weapon_costume_hidden,
                wing_costume_index: appearance.wing_costume_index,
                wing_costume_hidden: appearance.wing_costume_hidden,
                seal_costume_index: appearance.seal_costume_index,
                seal_costume_hidden: appearance.seal_costume_hidden,
                companion_index: appearance.ramble_pet_index,
                companion_hidden: appearance.ramble_pet_hidden,
                hat_hidden: appearance.hat_hidden,
                costume_hat_hidden: appearance.costume_hat_hidden,
            });
    let inventory = runtime
        .inventory
        .as_ref()
        .map(|inventory| HunterRuntimeInventorySnapshot {
            items: inventory
                .items
                .iter()
                .map(|item| HunterRuntimeItemSnapshot {
                    source_key: item.dictionary_key.clone(),
                    definition_index: item.source_index,
                    count: item.count,
                    reserved_count: item.reservation,
                    is_new: item.new_check,
                    is_infinite: item.infinity_check,
                })
                .collect(),
            gear: inventory
                .gear
                .iter()
                .map(|gear| HunterRuntimeGearSnapshot {
                    source_key: gear.dictionary_key.clone(),
                    definition_index: gear.gear_index,
                    inventory_index: gear.inventory_index,
                    quality: gear.quality,
                    level: gear.level,
                    rating: gear.rating,
                    group: gear.group,
                    is_new: gear.new_check,
                })
                .collect(),
            consumables: inventory
                .consumables
                .iter()
                .map(|consumable| HunterRuntimeConsumableSnapshot {
                    source_key: consumable.dictionary_key.clone(),
                    total_count: consumable.total_count,
                    nested_values_resolved: false,
                })
                .collect(),
        });
    let growth = runtime.growth.as_ref().map(|growth| {
        growth
            .iter()
            .map(|property| HunterRuntimeGrowthSnapshot {
                property_order: property.source_order,
                level: property.property_level,
            })
            .collect()
    });
    let riding_pet = runtime
        .riding_pet
        .as_ref()
        .map(|pet| HunterRuntimeRidingPetSnapshot {
            pasture_index: pet.pasture_index,
            definition_index: pet.source_index,
            master_key: pet.master_index.clone(),
            rating: pet.rating,
            skill_index: pet.skill_index,
            trait_index: pet.trait_index,
            trait_level: pet.trait_level,
            used_soul: pet.use_soul,
            used_growth_stone: pet.use_growth_stone,
            locked: pet.locked,
            gear_values_resolved: false,
        });
    HunterRuntimeEvidenceSnapshot {
        source_key: runtime.source_dictionary_key.clone(),
        source_index: runtime.source_index,
        job: evidence_section(job),
        status: evidence_section(status),
        skills: evidence_section(skills),
        appearance: evidence_section(appearance),
        inventory: evidence_section(inventory),
        growth: evidence_section(growth),
        riding_pet: evidence_section(riding_pet),
    }
}

fn evidence_section<T>(value: Option<T>) -> HunterEvidenceSection<T> {
    HunterEvidenceSection {
        evidence_state: if value.is_some() {
            HunterEvidenceState::ValueCaptured
        } else {
            HunterEvidenceState::SchemaConfirmed
        },
        value,
    }
}

fn restore_hunter_service_gauge(
    hunter: &mut DurableHunterState,
    effect_kind: ServiceEffectKind,
    amount: u64,
) {
    match effect_kind {
        ServiceEffectKind::Hp => {
            hunter.current_hp = hunter.current_hp.saturating_add(amount).min(hunter.max_hp);
        }
        ServiceEffectKind::Stamina => hunter.stamina.restore(amount),
        ServiceEffectKind::Satiety => hunter.satiety.restore(amount),
        ServiceEffectKind::Mood => hunter.mood.restore(amount),
    }
}

fn pay_costs(state: &mut DurableBuildingState, costs: &[EconomyAmount]) {
    for cost in costs {
        if cost.resource_id == "currency:gold" {
            state.town_gold -= cost.quantity;
        } else if let Some(stock) = state
            .material_stocks
            .iter_mut()
            .find(|stock| stock.id == cost.resource_id)
        {
            stock.town_quantity -= u32::try_from(cost.quantity)
                .expect("validated building cost fits available u32 stock");
        }
    }
}

fn placement_is_valid(
    buildings: &[DurableBuilding],
    catalog: &crate::buildings::BuildingCatalog,
    grid_x: i32,
    grid_y: i32,
    grid_width: u32,
    grid_height: u32,
    ignored_index: Option<usize>,
) -> bool {
    let Ok(width) = i32::try_from(grid_width) else {
        return false;
    };
    let Ok(height) = i32::try_from(grid_height) else {
        return false;
    };
    let Some(right) = grid_x.checked_add(width) else {
        return false;
    };
    let Some(bottom) = grid_y.checked_add(height) else {
        return false;
    };
    if grid_x < TOWN_GRID_MIN
        || grid_y < TOWN_GRID_MIN
        || right > TOWN_GRID_MAX
        || bottom > TOWN_GRID_MAX
    {
        return false;
    }
    buildings.iter().enumerate().all(|(index, building)| {
        if ignored_index == Some(index) {
            return true;
        }
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return false;
        };
        let Some(definition) = catalog.base(&building_id) else {
            return false;
        };
        let Some((other_width, other_height)) = building_grid_size(definition) else {
            return false;
        };
        let Ok(other_width) = i32::try_from(other_width) else {
            return false;
        };
        let Ok(other_height) = i32::try_from(other_height) else {
            return false;
        };
        right <= building.grid_x
            || grid_x >= building.grid_x + other_width
            || bottom <= building.grid_y
            || grid_y >= building.grid_y + other_height
    })
}

fn town_navigation_obstacles(
    buildings: &[DurableBuilding],
    catalog: &crate::buildings::BuildingCatalog,
) -> Vec<NavigationObstacle> {
    buildings
        .iter()
        .filter_map(|building| {
            let building_id = BaseBuildingId::parse(&building.id).ok()?;
            let definition = catalog.base(&building_id)?;
            let (width, height) = building_grid_size(definition)?;
            let width = i32::try_from(width).ok()?;
            let height = i32::try_from(height).ok()?;
            Some(NavigationObstacle {
                min_x: TOWN_NAV_ORIGIN_X + building.grid_x * TOWN_NAV_CELL_SIZE,
                max_x: TOWN_NAV_ORIGIN_X + (building.grid_x + width) * TOWN_NAV_CELL_SIZE,
                min_y: TOWN_NAV_ORIGIN_Y + building.grid_y * TOWN_NAV_CELL_SIZE,
                max_y: TOWN_NAV_ORIGIN_Y + (building.grid_y + height) * TOWN_NAV_CELL_SIZE,
            })
        })
        .collect()
}

fn gold_cost(row: &BuildingLevelDefinition) -> Option<u64> {
    row.costs
        .iter()
        .find(|cost| cost.resource_id == "currency:gold")
        .map(|cost| cost.quantity)
}

fn binding(id: &'static str, confidence: BindingConfidence, resolved: bool) -> EvidenceBinding {
    EvidenceBinding {
        id,
        confidence,
        resolved,
    }
}

#[allow(clippy::too_many_arguments)]
fn visual_entity(
    entity_id: impl Into<String>,
    kind: WorldEntityKind,
    asset_bundle_id: &'static str,
    source_skeleton_name: &'static str,
    source_confidence: BindingConfidence,
    x: i32,
    y: i32,
    facing: Facing,
    action_state: WorldEntityActionState,
    animation: impl Into<String>,
) -> WorldEntityProjection {
    WorldEntityProjection {
        descriptor: WorldEntityDescriptor {
            entity_id: entity_id.into(),
            kind,
            asset_bundle_id,
            source_skeleton_name,
            role: "migration_visual_candidate",
            source_binding: binding("actor.spine_bundle", source_confidence, true),
            // Exact legacy spawn coordinates are still unavailable; these anchors are presentation-only.
            placement_binding: binding("actor.world_placement", BindingConfidence::Unknown, false),
        },
        x,
        y,
        facing,
        action_state,
        animation: animation.into(),
        class_family: None,
        target_entity_id: None,
        action_sequence: 0,
        loot_sequence: 0,
        loot_label: None,
        attack_effect_key: None,
        skill_presentation_key: None,
        current_hp: None,
        maximum_hp: None,
        interaction_prompt_key: None,
        selectable: true,
    }
}

fn monster_visual_entity(monster: &MonsterState) -> WorldEntityProjection {
    let family = match monster.asset_bundle_id.as_str() {
        "mon_goldblin" => "mon_goldblin",
        _ => "mon_a_01_1",
    };
    let animation = match monster.animation.as_str() {
        "atk" => "atk",
        "atk_b" => "atk_b",
        "die" => "die",
        "walk" => "walk",
        "walk_b" => "walk_b",
        _ => "stay",
    };
    let mut entity = visual_entity(
        monster.entity_id.clone(),
        WorldEntityKind::Monster,
        family,
        family,
        BindingConfidence::Confirmed,
        monster.x,
        monster.y,
        if monster.facing_left {
            Facing::Left
        } else {
            Facing::Right
        },
        match monster.action_state {
            MonsterActionState::Idle => WorldEntityActionState::Idle,
            MonsterActionState::Patrolling | MonsterActionState::Chasing
                if monster.animation == "walk" || monster.animation == "walk_b" =>
            {
                WorldEntityActionState::Walking
            }
            MonsterActionState::Attacking => WorldEntityActionState::Attacking,
            MonsterActionState::Dead => WorldEntityActionState::Dead,
            MonsterActionState::Patrolling | MonsterActionState::Chasing => {
                WorldEntityActionState::Idle
            }
        },
        animation,
    );
    // Monsters remain server-owned combat actors, but are not player-selectable UI entities.
    entity.selectable = false;
    entity.target_entity_id = monster.target_hunter_id.map(village_hunter_entity_id);
    entity.action_sequence = monster.attack_sequence;
    entity.current_hp = Some(monster.hp);
    entity.maximum_hp = Some(monster.max_hp);
    entity
}

fn monster_action_name(state: MonsterActionState) -> &'static str {
    match state {
        MonsterActionState::Idle => "idle",
        MonsterActionState::Patrolling => "patrolling",
        MonsterActionState::Chasing => "chasing",
        MonsterActionState::Attacking => "attacking",
        MonsterActionState::Dead => "dead",
    }
}

fn hunter_visual_entity(
    agent: &HunterAgentState,
    current_hp: u64,
    maximum_hp: u64,
) -> WorldEntityProjection {
    use super::HunterActionState;
    let mut entity = visual_entity(
        village_hunter_entity_id(agent.hunter_id),
        WorldEntityKind::Hunter,
        "hunter",
        "hunter",
        BindingConfidence::Confirmed,
        agent.x,
        agent.y,
        if agent.facing_left {
            Facing::Left
        } else {
            Facing::Right
        },
        match agent.action_state {
            HunterActionState::EnteringRegion
            | HunterActionState::Chasing
            | HunterActionState::CollectingLoot => WorldEntityActionState::Walking,
            HunterActionState::Attacking => WorldEntityActionState::Attacking,
            HunterActionState::Dead => WorldEntityActionState::Dead,
            HunterActionState::TownIdle if agent.animation == "hunter_walk" => {
                WorldEntityActionState::Walking
            }
            HunterActionState::TownIdle | HunterActionState::AcquiringTarget => {
                WorldEntityActionState::Idle
            }
        },
        agent.animation.clone(),
    );
    entity.target_entity_id = agent.target_monster_id.clone();
    entity.action_sequence = agent.attack_sequence;
    entity.loot_sequence = agent.loot_sequence;
    entity.skill_presentation_key = agent.active_skill_id.clone();
    entity.current_hp = Some(current_hp);
    entity.maximum_hp = Some(maximum_hp);
    entity
}

fn village_hunter_entity_id(hunter_id: u32) -> String {
    format!("village-hunter-{hunter_id}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VillageHunterMotion {
    x: i32,
    y: i32,
    facing: Facing,
    action_state: WorldEntityActionState,
    animation: &'static str,
}

fn village_hunter_motion(tick: u64, active_slot: usize) -> VillageHunterMotion {
    const WALK_TICKS: u64 = 42;
    const IDLE_TICKS: u64 = 12;
    const CYCLE_TICKS: u64 = (WALK_TICKS + IDLE_TICKS) * 2;

    let slot = active_slot as u64;
    let min_x = 1345 + i32::try_from(slot % 4).unwrap_or(0) * 145;
    let max_x = min_x + 72;
    // Separate lanes guarantee that active Hunters never share a world position.
    let y = 645 + i32::try_from(slot).unwrap_or(0) * 24;
    let phase = (tick + slot * 17) % CYCLE_TICKS;

    if phase < WALK_TICKS {
        VillageHunterMotion {
            x: interpolate_lane(min_x, max_x, phase, WALK_TICKS),
            y,
            facing: Facing::Right,
            action_state: WorldEntityActionState::Walking,
            animation: "hunter_walk",
        }
    } else if phase < WALK_TICKS + IDLE_TICKS {
        VillageHunterMotion {
            x: max_x,
            y,
            facing: Facing::Right,
            action_state: WorldEntityActionState::Idle,
            animation: "hunter_stay",
        }
    } else if phase < WALK_TICKS * 2 + IDLE_TICKS {
        VillageHunterMotion {
            x: interpolate_lane(max_x, min_x, phase - WALK_TICKS - IDLE_TICKS, WALK_TICKS),
            y,
            facing: Facing::Left,
            action_state: WorldEntityActionState::Walking,
            animation: "hunter_walk",
        }
    } else {
        VillageHunterMotion {
            x: min_x,
            y,
            facing: Facing::Left,
            action_state: WorldEntityActionState::Idle,
            animation: "hunter_stay",
        }
    }
}

fn interpolate_lane(start: i32, end: i32, elapsed: u64, duration: u64) -> i32 {
    let delta = i64::from(end - start);
    let offset =
        delta * i64::try_from(elapsed).unwrap_or(0) / i64::try_from(duration.max(1)).unwrap_or(1);
    start + i32::try_from(offset).unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn test_authoritative_building_content() -> Arc<AuthoritativeBuildingContent> {
    use std::collections::{BTreeMap, HashSet};
    use std::sync::OnceLock;

    use crate::content::building_registry::canonical_building_content;

    static CONTENT: OnceLock<Arc<AuthoritativeBuildingContent>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let embedded = canonical_building_content().expect("test building registry");
            let mut bases = Vec::with_capacity(embedded.buildings.len());
            let mut levels = Vec::new();
            for building in &embedded.buildings {
                let building_id = BaseBuildingId::parse(&building.id).expect("base building id");
                let [grid_width, grid_height] = building.source_data.grid_size.as_slice() else {
                    panic!("test building grid size");
                };
                bases.push(BaseBuildingDefinition {
                    id: building_id.clone(),
                    registry_id: embedded.registry_id.clone(),
                    display_name: building.display_name.clone(),
                    category: building.category.clone(),
                    source_type: building.source_data.source_type,
                    max_instances: u32::try_from(building.source_data.max_build)
                        .expect("max instances"),
                    grid_width: u16::try_from(*grid_width).expect("grid width"),
                    grid_height: u16::try_from(*grid_height).expect("grid height"),
                    movable: Some(building.source_data.movable != 0),
                    constructible: None,
                    base_sprite_asset_id: building.base_sprite_asset_id.clone(),
                });
                let mut seen_levels = HashSet::new();
                for row in building.build_rows.iter().chain(&building.levels) {
                    if !seen_levels.insert(row.level) {
                        continue;
                    }
                    levels.push(BuildingLevelDefinition {
                        building_id: building_id.clone(),
                        level: u16::from(row.level),
                        upgrade_duration_ms: None,
                        inventory_capacity: None,
                        production_slots: None,
                        costs: row
                            .costs
                            .iter()
                            .map(|cost| EconomyAmount {
                                resource_id: cost.item_id.clone(),
                                quantity: cost.quantity,
                            })
                            .collect(),
                        prerequisites: row
                            .required_town_hall_level
                            .map(|required_level| BuildingLevelPrerequisite {
                                building_id: BaseBuildingId::parse("build_1")
                                    .expect("town hall id"),
                                required_level: u16::from(required_level),
                            })
                            .into_iter()
                            .collect(),
                    });
                }
            }
            let catalog = BuildingCatalog {
                registry_id: embedded.registry_id.clone(),
                bases,
                levels,
                skins: Vec::new(),
            };
            let capabilities = embedded
                .capabilities
                .iter()
                .enumerate()
                .map(|(index, capability)| BuildingCapabilityDefinition {
                    capability_id: format!("test-capability-{index}"),
                    building_id: BaseBuildingId::parse(&capability.building_id)
                        .expect("capability building id"),
                    kind: capability.kind.clone(),
                    static_data_ready: capability.static_data_ready,
                    runnable: capability.runnable,
                })
                .collect();
            let items = embedded
                .items
                .iter()
                .map(|(item_id, item)| {
                    (
                        item_id.clone(),
                        EconomyItemDefinition {
                            item_id: item_id.clone(),
                            internal_name: item.internal_name.clone(),
                            item_type: item.item_type.clone(),
                            stack_limit: item.stack_limit,
                            town_pays_hunter_gold_per_unit: item.town_pays_hunter_gold_per_unit,
                            localized_names: item
                                .display_name
                                .as_ref()
                                .map(|name| ("en".to_owned(), name.clone()))
                                .into_iter()
                                .collect::<BTreeMap<_, _>>(),
                            buy_price: item
                                .buy_price
                                .as_deref()
                                .unwrap_or_default()
                                .iter()
                                .map(|amount| EconomyAmount {
                                    resource_id: amount.item_id.clone(),
                                    quantity: amount.quantity,
                                })
                                .collect(),
                            sell_price: item
                                .sell_price
                                .as_deref()
                                .unwrap_or_default()
                                .iter()
                                .map(|amount| EconomyAmount {
                                    resource_id: amount.item_id.clone(),
                                    quantity: amount.quantity,
                                })
                                .collect(),
                            hunter_pays_town_gold_by_tier: item
                                .hunter_pays_town_gold_by_tier
                                .clone()
                                .unwrap_or_default(),
                        },
                    )
                })
                .collect();
            Arc::new(
                AuthoritativeBuildingContent::new(
                    catalog,
                    BuildingGameplayCatalog {
                        registry_id: embedded.registry_id.clone(),
                        capabilities,
                        items,
                        products: BTreeMap::new(),
                    },
                )
                .expect("test authoritative building content"),
            )
        })
        .clone()
}

#[cfg(test)]
fn test_town_building_state() -> DurableBuildingState {
    let mut state = DurableBuildingState {
        town_seed_version: 2,
        ..DurableBuildingState::default()
    };
    for id in 1_u128..=28 {
        let slot = i32::try_from(id - 1).unwrap();
        state.buildings.push(DurableBuilding {
            instance_id: Uuid::from_u128(id).to_string(),
            id: format!("build_{id}"),
            equipped_skin_id: None,
            level: 1,
            uses: 0,
            grid_x: ((slot % 7) - 3) * 4,
            grid_y: ((slot / 7) * 4) - 6,
            seeded_by: Some("town-template:default-town-v2".to_owned()),
        });
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BANDAGE_ID: &str = "product:5";

    fn gear_flow(product_id: &str, sale_price: u64) -> OriginalFlowSession {
        gear_flow_for_building(product_id, sale_price, "build_10")
    }

    fn gear_flow_for_building(
        product_id: &str,
        sale_price: u64,
        producer_building_id: &str,
    ) -> OriginalFlowSession {
        let mut aggregate = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            buildings: test_town_building_state(),
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        };
        aggregate
            .buildings
            .material_stocks
            .push(DurableMaterialStock {
                id: "material:11".to_owned(),
                town_quantity: 20,
                hunter_quantity: 0,
                requested: 0,
                unit_price: 1,
            });
        let mut content = (*test_authoritative_building_content()).clone();
        content.gameplay.products.insert(
            product_id.to_owned(),
            EconomyProductDefinition {
                product_id: product_id.to_owned(),
                building_id: Some(BaseBuildingId::parse(producer_building_id).unwrap()),
                duration_ms: None,
                exact_mutation_ready: false,
                inputs: vec![EconomyAmount {
                    resource_id: "material:11".to_owned(),
                    quantity: 2,
                }],
                outputs: Vec::new(),
                sale_price: vec![EconomyAmount {
                    resource_id: "currency:gold".to_owned(),
                    quantity: sale_price,
                }],
                service: None,
                conversion_options: Vec::new(),
                random_output: None,
            },
        );
        OriginalFlowSession::from_aggregate_with_content(aggregate, 7, Arc::new(content))
    }

    fn building_instance_id(flow: &OriginalFlowSession, building_id: &str) -> String {
        flow.buildings
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap()
            .instance_id
            .clone()
    }

    #[test]
    fn blacksmith_stock_purchase_conserves_hunter_and_town_gold() {
        let product_id = "recipe:weapon:0:rating:0";
        let mut flow = gear_flow(product_id, 75);
        let blacksmith_id = building_instance_id(&flow, "build_10");
        let weapon_shop_id = building_instance_id(&flow, "build_7");

        let crafted = flow
            .handle_command(ClientCommand::CraftShopItem {
                instance_id: blacksmith_id,
                recipe_id: product_id.to_owned(),
                material_id: None,
                quantity: 2,
            })
            .unwrap();
        assert!(matches!(
            crafted.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 16);
        assert_eq!(
            flow.buildings.product_stocks,
            vec![DurableProductStock {
                building_instance_id: weapon_shop_id,
                product_id: product_id.to_owned(),
                quantity: 2,
            }]
        );

        let gold_before = flow.buildings.town_gold;
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
        let purchase = flow
            .handle_command(ClientCommand::PurchaseShopItem {
                hunter_id: 1,
                shop_id: "build_7".to_owned(),
                product_id: product_id.to_owned(),
            })
            .unwrap();
        assert!(matches!(
            purchase.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.buildings.product_stocks[0].quantity, 1);
        assert_eq!(flow.buildings.town_gold, gold_before + 75);
        assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before - 75);
        assert_eq!(
            flow.hunter_roster.hunters[0].owned_items[0].product_id,
            product_id
        );
        assert_eq!(flow.hunter_roster.hunters[0].owned_items[0].quantity, 1);
        assert_eq!(flow.buildings.hunter_equipment_purchases, 1);

        let recipes = flow.snapshot().village.building_system.recipes;
        assert!(recipes
            .iter()
            .any(|recipe| recipe.id == product_id && recipe.shop_id == "build_10"));
        assert!(recipes.iter().any(|recipe| {
            recipe.id == product_id && recipe.shop_id == "build_7" && recipe.stock == 1
        }));
    }

    #[test]
    fn gear_enhancement_fails_closed_without_resolved_cost_and_rng_evidence() {
        let product_id = "recipe:weapon:0:rating:0";
        let mut flow = gear_flow(product_id, 75);
        let blacksmith_id = building_instance_id(&flow, "build_10");
        let crafted = flow
            .handle_command(ClientCommand::CraftShopItem {
                instance_id: blacksmith_id,
                recipe_id: product_id.to_owned(),
                material_id: None,
                quantity: 1,
            })
            .unwrap();
        assert!(matches!(
            crafted.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        let gear_instance_id = Uuid::from_u128(8_001);
        let purchase = flow
            .handle_command_with_id(
                ClientCommand::PurchaseShopItem {
                    hunter_id: 1,
                    shop_id: "build_7".to_owned(),
                    product_id: product_id.to_owned(),
                },
                gear_instance_id,
            )
            .unwrap();
        assert!(matches!(
            purchase.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        let premature = flow
            .handle_command(ClientCommand::EnhanceHunterGear {
                hunter_id: 1,
                gear_instance_id,
                mode: "single".to_owned(),
                optional_material_ids: Vec::new(),
            })
            .unwrap();
        assert!(matches!(
            premature.message,
            ServerMessage::IntentResult {
                accepted: false,
                reason: Some(ref reason),
                ..
            } if reason == "enhancement_visit_not_started"
        ));
        let started = flow
            .handle_command_with_id(
                ClientCommand::StartHunterEnhancement { hunter_id: 1 },
                Uuid::from_u128(8_002),
            )
            .unwrap();
        assert!(matches!(
            started.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(
            flow.hunter_roster.hunters[0]
                .hunt
                .gear_enhancement
                .as_ref()
                .map(|task| task.status),
            Some(GearEnhancementTaskStatus::Traveling)
        );
        for _ in 0..500 {
            flow.advance_simulation_tick();
            if flow.hunter_roster.hunters[0]
                .hunt
                .gear_enhancement
                .as_ref()
                .is_some_and(|task| task.status == GearEnhancementTaskStatus::WaitingForInteraction)
            {
                break;
            }
        }
        let ready_world = flow.snapshot();
        let ready_snapshot = ready_world.hunter_roster.active_hunters[0]
            .gear_enhancement_task
            .as_ref()
            .expect("enhancement task is projected");
        assert_eq!(ready_snapshot.status, "waiting_for_interaction");
        assert!(ready_snapshot.interaction_ready);
        let gold_before = flow.hunter_roster.hunters[0].gold;
        let result = flow
            .handle_command_with_id(
                ClientCommand::EnhanceHunterGear {
                    hunter_id: 1,
                    gear_instance_id,
                    mode: "single".to_owned(),
                    optional_material_ids: Vec::new(),
                },
                Uuid::from_u128(8_003),
            )
            .unwrap();
        assert!(matches!(
            result.message,
            ServerMessage::BindingBlocked { .. }
        ));
        assert_eq!(flow.hunter_roster.hunters[0].gold, gold_before);
        assert_eq!(
            flow.hunter_roster.hunters[0].owned_items[0].enhancement_level,
            None
        );
        let released = flow.snapshot().hunter_roster.active_hunters[0].clone();
        assert!(released.gear_enhancement_task.is_none());
        assert_eq!(flow.hunter_roster.hunters[0].profile.action_state, "idle");
    }

    #[test]
    fn enhancement_visit_survives_reconnect_and_resumes_until_interaction_ready() {
        let aggregate = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            buildings: test_town_building_state(),
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        };
        let mut flow = OriginalFlowSession::from_aggregate(aggregate, 7);
        let started = flow
            .handle_command_with_id(
                ClientCommand::StartHunterEnhancement { hunter_id: 1 },
                Uuid::from_u128(8_101),
            )
            .unwrap();
        assert!(matches!(
            started.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        for _ in 0..5 {
            flow.advance_simulation_tick();
        }

        let durable = flow.durable_state();
        let mut restored = OriginalFlowSession::from_aggregate(durable, 7);
        assert_eq!(
            restored.hunter_roster.hunters[0]
                .hunt
                .gear_enhancement
                .as_ref()
                .map(|task| task.status),
            Some(GearEnhancementTaskStatus::Traveling)
        );
        for _ in 0..500 {
            restored.advance_simulation_tick();
            if restored.hunter_roster.hunters[0]
                .hunt
                .gear_enhancement
                .as_ref()
                .is_some_and(|task| task.status == GearEnhancementTaskStatus::WaitingForInteraction)
            {
                break;
            }
        }
        let task = restored.hunter_roster.hunters[0]
            .hunt
            .gear_enhancement
            .as_ref()
            .expect("enhancement task survives reconnect");
        assert_eq!(
            task.status,
            GearEnhancementTaskStatus::WaitingForInteraction
        );
        assert_eq!(
            restored.hunter_roster.hunters[0].profile.action_state,
            "waiting_for_enhancement_interaction"
        );
    }

    #[test]
    fn terminal_enhancement_task_is_released_when_restoring_a_session() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.gear_enhancement = Some(DurableGearEnhancementTask {
            status: GearEnhancementTaskStatus::Configuring,
            stop_reason: Some("evidence_disabled".to_owned()),
            ..DurableGearEnhancementTask::default()
        });
        roster.hunters[0].profile.action_state = "configuring_enhancement".to_owned();

        let flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                buildings: test_town_building_state(),
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );

        assert!(flow.hunter_roster.hunters[0]
            .hunt
            .gear_enhancement
            .is_none());
        assert_eq!(flow.hunter_roster.hunters[0].profile.action_state, "idle");
        assert_eq!(
            flow.hunter_roster.hunters[0].profile.animation_name,
            "hunter_stay"
        );
    }

    #[test]
    fn legacy_enhancement_task_and_orphaned_action_are_released_on_restore() {
        let mut roster = operational_migration_roster();
        let task = serde_json::json!({
            "building_instance_id": "forge-legacy",
            "status": "waiting_for_interaction",
            "interaction_x": 1,
            "interaction_y": 2
        });
        roster.hunters[0].hunt.gear_enhancement =
            Some(serde_json::from_value(task).expect("legacy task shape remains readable"));
        roster.hunters[0].profile.action_state = "waiting_for_enhancement_interaction".to_owned();
        roster.hunters[1].profile.action_state = "configuring_enhancement".to_owned();

        let flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                buildings: test_town_building_state(),
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );

        assert!(flow.hunter_roster.hunters[0]
            .hunt
            .gear_enhancement
            .is_none());
        assert!(flow.hunter_roster.hunters[1]
            .hunt
            .gear_enhancement
            .is_none());
        assert_eq!(flow.hunter_roster.hunters[0].profile.action_state, "idle");
        assert_eq!(flow.hunter_roster.hunters[1].profile.action_state, "idle");
    }

    #[test]
    fn alchemist_crafts_and_sells_catalog_potion_at_recovered_price() {
        let product_id = "recipe:consumable:0:level:0";
        let mut aggregate = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            buildings: test_town_building_state(),
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        };
        aggregate
            .buildings
            .material_stocks
            .push(DurableMaterialStock {
                id: "material:139".to_owned(),
                town_quantity: 3,
                hunter_quantity: 0,
                requested: 0,
                unit_price: 1,
            });
        let mut content = (*test_authoritative_building_content()).clone();
        content.gameplay.products.insert(
            product_id.to_owned(),
            EconomyProductDefinition {
                product_id: product_id.to_owned(),
                building_id: Some(BaseBuildingId::parse("build_14").unwrap()),
                duration_ms: None,
                exact_mutation_ready: false,
                inputs: vec![EconomyAmount {
                    resource_id: "material:139".to_owned(),
                    quantity: 3,
                }],
                outputs: vec![EconomyAmount {
                    resource_id: "consumable:0".to_owned(),
                    quantity: 1,
                }],
                sale_price: Vec::new(),
                service: None,
                conversion_options: Vec::new(),
                random_output: None,
            },
        );
        let mut flow =
            OriginalFlowSession::from_aggregate_with_content(aggregate, 7, Arc::new(content));
        let alchemist_id = building_instance_id(&flow, "build_14");
        let crafted = flow
            .handle_command(ClientCommand::CraftShopItem {
                instance_id: alchemist_id,
                recipe_id: product_id.to_owned(),
                material_id: None,
                quantity: 1,
            })
            .unwrap();
        assert!(matches!(
            crafted.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        let potion_shop_id = building_instance_id(&flow, "build_11");
        assert_eq!(
            flow.buildings
                .product_stocks
                .iter()
                .find(|stock| stock.building_instance_id == potion_shop_id
                    && stock.product_id == product_id)
                .map(|stock| stock.quantity),
            Some(1)
        );
        let potion_row = flow
            .snapshot()
            .village
            .building_system
            .recipes
            .into_iter()
            .find(|recipe| recipe.id == product_id && recipe.shop_id == "build_11")
            .expect("potion shop row");
        assert_eq!(potion_row.stock, 1);
        assert_eq!(potion_row.sale_price, 68);

        let town_gold_before = flow.buildings.town_gold;
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
        let equipment_purchase_count_before = flow.buildings.hunter_equipment_purchases;
        let purchased = flow
            .handle_command(ClientCommand::PurchaseShopItem {
                hunter_id: 1,
                shop_id: "build_11".to_owned(),
                product_id: product_id.to_owned(),
            })
            .unwrap();
        assert!(matches!(
            purchased.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(
            flow.buildings
                .product_stocks
                .iter()
                .find(|stock| stock.building_instance_id == potion_shop_id
                    && stock.product_id == product_id)
                .map(|stock| stock.quantity),
            Some(0)
        );
        assert_eq!(flow.buildings.town_gold, town_gold_before + 68);
        assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before - 68);
        assert_eq!(
            flow.buildings.hunter_equipment_purchases,
            equipment_purchase_count_before
        );
        assert!(flow.snapshot().hunter_roster.active_hunters[0]
            .gear_enhancements
            .is_empty());
        assert_eq!(
            flow.hunter_roster.hunters[0].owned_items[0].product_id,
            product_id
        );
    }

    #[test]
    fn jeweler_crafts_accessories_into_accessory_shop_stock() {
        let product_id = "recipe:ring:0:rating:0";
        let mut flow = gear_flow_for_building(product_id, 80, "build_21");
        let jeweler_id = building_instance_id(&flow, "build_21");
        let accessory_shop_id = building_instance_id(&flow, "build_20");

        let crafted = flow
            .handle_command(ClientCommand::CraftShopItem {
                instance_id: jeweler_id,
                recipe_id: product_id.to_owned(),
                material_id: None,
                quantity: 2,
            })
            .unwrap();
        assert!(matches!(
            crafted.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(
            flow.buildings.product_stocks,
            vec![DurableProductStock {
                building_instance_id: accessory_shop_id,
                product_id: product_id.to_owned(),
                quantity: 2,
            }]
        );
        let recipes = flow.snapshot().village.building_system.recipes;
        assert!(recipes
            .iter()
            .any(|recipe| recipe.id == product_id && recipe.shop_id == "build_21"));
        assert!(recipes.iter().any(|recipe| {
            recipe.id == product_id && recipe.shop_id == "build_20" && recipe.stock == 2
        }));
    }

    #[test]
    fn blacksmith_routes_wearable_armor_to_armor_shop_and_enforces_difficulty_levels() {
        // Helmet 10 belongs to difficulty group 2. Its quality/rating is not
        // the building-level gate.
        let product_id = "recipe:helmet:10:rating:1";
        let mut flow = gear_flow(product_id, 90);
        let blacksmith_id = building_instance_id(&flow, "build_10");
        let before = flow.buildings.clone();

        let locked = flow
            .handle_command(ClientCommand::CraftShopItem {
                instance_id: blacksmith_id.clone(),
                recipe_id: product_id.to_owned(),
                material_id: None,
                quantity: 1,
            })
            .unwrap();
        assert!(matches!(
            locked.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("product_level_locked")
        ));
        assert_eq!(flow.buildings, before);

        flow.buildings
            .buildings
            .iter_mut()
            .find(|building| building.instance_id == blacksmith_id)
            .unwrap()
            .level = 2;
        let crafted = flow
            .handle_command(ClientCommand::CraftShopItem {
                instance_id: building_instance_id(&flow, "build_10"),
                recipe_id: product_id.to_owned(),
                material_id: None,
                quantity: 1,
            })
            .unwrap();
        assert!(matches!(
            crafted.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(
            flow.buildings.product_stocks[0].building_instance_id,
            building_instance_id(&flow, "build_8")
        );

        let shop_locked = flow
            .handle_command(ClientCommand::PurchaseShopItem {
                hunter_id: 1,
                shop_id: "build_8".to_owned(),
                product_id: product_id.to_owned(),
            })
            .unwrap();
        assert!(matches!(
            shop_locked.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("product_level_locked")
        ));
    }

    fn infirmary_flow(roster_resolved: bool) -> OriginalFlowSession {
        let mut aggregate = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            buildings: test_town_building_state(),
            hunter_roster: DurableHunterRosterState {
                roster_resolved,
                wallets_resolved: roster_resolved,
                hunters: (1..=5)
                    .map(|hunter_id| DurableHunterState {
                        hunter_id,
                        gold: 1_000,
                        current_hp: 100,
                        max_hp: 1_000,
                        stamina: HunterServiceGauge {
                            current: 100,
                            maximum: 1_000,
                        },
                        satiety: HunterServiceGauge {
                            current: 100,
                            maximum: 1_000,
                        },
                        mood: HunterServiceGauge {
                            current: 100,
                            maximum: 1_000,
                        },
                        profile: DurableHunterProfile::migration_default(hunter_id),
                        hunt: Default::default(),
                        runtime: Default::default(),
                        owned_items: Vec::new(),
                    })
                    .collect(),
                ..DurableHunterRosterState::default()
            },
            product_services: DurableProductServiceState { visits: Vec::new() },
            ..DurablePlayerAggregate::default()
        };
        let infirmary_instance_id = aggregate
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_12")
            .expect("test infirmary")
            .instance_id
            .clone();
        aggregate
            .buildings
            .product_stocks
            .push(DurableProductStock {
                building_instance_id: infirmary_instance_id,
                product_id: TEST_BANDAGE_ID.to_owned(),
                quantity: 5,
            });

        let mut content = (*test_authoritative_building_content()).clone();
        content.gameplay.products.insert(
            TEST_BANDAGE_ID.to_owned(),
            EconomyProductDefinition {
                product_id: TEST_BANDAGE_ID.to_owned(),
                building_id: Some(BaseBuildingId::parse("build_12").expect("infirmary id")),
                duration_ms: Some(600),
                exact_mutation_ready: false,
                inputs: Vec::new(),
                outputs: Vec::new(),
                sale_price: Vec::new(),
                service: Some(EconomyProductService {
                    source_type: 0,
                    required_level: 0,
                    service_time_ms: 600,
                    effect_value: 250,
                    use_money: 90,
                    completion_counts: vec![1, 2, 10],
                    required_cash_count: 3,
                    cash_completion_count: 1,
                    required_elemental_count: 150,
                    elemental_completion_count: 1,
                }),
                conversion_options: Vec::new(),
                random_output: None,
            },
        );
        OriginalFlowSession::from_aggregate_with_content(aggregate, 7, Arc::new(content))
    }

    fn infirmary_instance_id(flow: &OriginalFlowSession) -> String {
        flow.buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_12")
            .expect("test infirmary")
            .instance_id
            .clone()
    }

    fn add_test_service_product(
        flow: &mut OriginalFlowSession,
        building_id: &str,
        product_id: &str,
    ) -> String {
        let instance_id = flow
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .expect("test service building")
            .instance_id
            .clone();
        flow.buildings.product_stocks.push(DurableProductStock {
            building_instance_id: instance_id.clone(),
            product_id: product_id.to_owned(),
            quantity: 5,
        });
        Arc::make_mut(&mut flow.building_content)
            .gameplay
            .products
            .insert(
                product_id.to_owned(),
                EconomyProductDefinition {
                    product_id: product_id.to_owned(),
                    building_id: Some(
                        BaseBuildingId::parse(building_id).expect("service building id"),
                    ),
                    duration_ms: Some(600),
                    exact_mutation_ready: false,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    sale_price: Vec::new(),
                    service: Some(EconomyProductService {
                        source_type: 0,
                        required_level: 0,
                        service_time_ms: 600,
                        effect_value: 250,
                        use_money: 90,
                        completion_counts: vec![1, 10],
                        required_cash_count: 3,
                        cash_completion_count: 1,
                        required_elemental_count: 150,
                        elemental_completion_count: 1,
                    }),
                    conversion_options: Vec::new(),
                    random_output: None,
                },
            );
        instance_id
    }

    #[test]
    fn infirmary_fails_closed_when_hunter_roster_is_unresolved() {
        let mut flow = infirmary_flow(false);
        let instance_id = infirmary_instance_id(&flow);
        let stock_before = flow.buildings.product_stocks[0].quantity;
        let result = flow
            .handle_command(ClientCommand::StartInfirmaryTreatment {
                instance_id,
                hunter_id: 1,
                product_id: TEST_BANDAGE_ID.to_owned(),
            })
            .expect("binding-blocked result");

        assert!(matches!(
            result.message,
            ServerMessage::BindingBlocked { .. }
        ));
        assert!(!result.durable_state_changed);
        assert_eq!(flow.buildings.product_stocks[0].quantity, stock_before);
        let snapshot = flow.infirmary_snapshot();
        assert!(!snapshot.roster_resolved);
        assert_eq!(
            snapshot.blockers,
            vec![
                "hunter_roster_binding",
                "hunter_health_state_binding",
                "hunter_wallet_state_binding",
            ]
        );
        assert!(snapshot.active.is_empty());
    }

    #[test]
    fn infirmary_consumes_stock_then_applies_healing_and_payment_on_completion() {
        let mut flow = infirmary_flow(true);
        let instance_id = infirmary_instance_id(&flow);
        let gold_before = flow.buildings.town_gold;
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
        let accepted = flow
            .handle_command(ClientCommand::StartInfirmaryTreatment {
                instance_id,
                hunter_id: 1,
                product_id: TEST_BANDAGE_ID.to_owned(),
            })
            .expect("treatment result");

        assert!(matches!(
            accepted.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert!(accepted.durable_state_changed);
        assert_eq!(flow.buildings.product_stocks[0].quantity, 4);
        assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before - 90);
        let started = flow.infirmary_snapshot();
        assert_eq!(started.slots, 3);
        assert_eq!(started.available_slots, 2);
        assert_eq!(started.active[0].remaining_ms, 600);
        assert_eq!(started.active[0].effect_value, 250);
        assert_eq!(started.active[0].payment_gold, 90);
        assert_eq!(started.hunters[0].treatment_state, "treating");

        flow.advance_visual_tick();
        flow.advance_visual_tick();
        assert_eq!(flow.infirmary_snapshot().active[0].remaining_ms, 200);
        assert_eq!(flow.hunter_roster.hunters[0].current_hp, 100);
        assert_eq!(flow.buildings.town_gold, gold_before);

        flow.advance_visual_tick();
        assert!(flow.infirmary_snapshot().active.is_empty());
        assert_eq!(flow.hunter_roster.hunters[0].current_hp, 350);
        assert_eq!(flow.buildings.town_gold, gold_before + 90);
        assert_eq!(
            flow.buildings.town_gold + flow.hunter_roster.hunters[0].gold,
            gold_before + hunter_gold_before
        );
        assert_eq!(flow.infirmary_snapshot().hunters[0].treatment_state, "idle");
    }

    #[test]
    fn product_service_rejects_insufficient_hunter_gold_without_consuming_stock() {
        let mut flow = infirmary_flow(true);
        let instance_id = infirmary_instance_id(&flow);
        flow.hunter_roster.hunters[0].gold = 89;

        let result = flow
            .handle_command(ClientCommand::StartBuildingService {
                instance_id,
                hunter_id: 1,
                product_id: TEST_BANDAGE_ID.to_owned(),
            })
            .expect("service result");

        assert!(matches!(
            result.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("insufficient_hunter_gold")
        ));
        assert_eq!(flow.buildings.product_stocks[0].quantity, 5);
        assert_eq!(flow.hunter_roster.hunters[0].gold, 89);
        assert!(flow.product_services.visits.is_empty());
    }

    #[test]
    fn inn_restaurant_and_tavern_restore_their_recovered_gauges() {
        for (building_id, product_id, effect_kind) in [
            ("build_9", "product:0", ServiceEffectKind::Stamina),
            ("build_13", "product:10", ServiceEffectKind::Satiety),
            ("build_19", "product:15", ServiceEffectKind::Mood),
        ] {
            let mut flow = infirmary_flow(true);
            let instance_id = add_test_service_product(&mut flow, building_id, product_id);
            let town_gold_before = flow.buildings.town_gold;

            let result = flow
                .handle_command(ClientCommand::StartBuildingService {
                    instance_id,
                    hunter_id: 1,
                    product_id: product_id.to_owned(),
                })
                .expect("service result");
            assert!(matches!(
                result.message,
                ServerMessage::IntentResult { accepted: true, .. }
            ));
            assert_eq!(flow.hunter_roster.hunters[0].gold, 910);
            assert_eq!(
                hunter_service_gauge(&flow.hunter_roster.hunters[0], effect_kind).current,
                100
            );

            for _ in 0..3 {
                flow.advance_visual_tick();
            }
            assert_eq!(
                hunter_service_gauge(&flow.hunter_roster.hunters[0], effect_kind).current,
                350
            );
            assert_eq!(flow.buildings.town_gold, town_gold_before + 90);
            assert!(flow.product_services.visits.is_empty());
        }
    }

    #[test]
    fn infirmary_enforces_slots_per_building_instance_and_rejects_unknown_hunters() {
        let mut flow = infirmary_flow(true);
        let first_instance_id = infirmary_instance_id(&flow);
        let mut second = flow
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_12")
            .expect("test infirmary")
            .clone();
        second.instance_id = Uuid::from_u128(1200).to_string();
        second.grid_x = 24;
        flow.buildings.buildings.push(second.clone());
        flow.buildings.product_stocks.push(DurableProductStock {
            building_instance_id: second.instance_id.clone(),
            product_id: TEST_BANDAGE_ID.to_owned(),
            quantity: 2,
        });

        for hunter_id in 1..=3 {
            let result = flow
                .handle_command(ClientCommand::StartInfirmaryTreatment {
                    instance_id: first_instance_id.clone(),
                    hunter_id,
                    product_id: TEST_BANDAGE_ID.to_owned(),
                })
                .expect("treatment result");
            assert!(matches!(
                result.message,
                ServerMessage::IntentResult { accepted: true, .. }
            ));
        }
        let full = flow
            .handle_command(ClientCommand::StartInfirmaryTreatment {
                instance_id: first_instance_id,
                hunter_id: 4,
                product_id: TEST_BANDAGE_ID.to_owned(),
            })
            .expect("full result");
        assert!(matches!(
            full.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("service_slots_full")
        ));

        let second_instance_id = second.instance_id.clone();
        let other_instance = flow
            .handle_command(ClientCommand::StartInfirmaryTreatment {
                instance_id: second_instance_id.clone(),
                hunter_id: 4,
                product_id: TEST_BANDAGE_ID.to_owned(),
            })
            .expect("second infirmary result");
        assert!(matches!(
            other_instance.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));

        let unknown = flow
            .handle_command(ClientCommand::StartInfirmaryTreatment {
                instance_id: second_instance_id,
                hunter_id: 999,
                product_id: TEST_BANDAGE_ID.to_owned(),
            })
            .expect("unknown hunter result");
        assert!(matches!(
            unknown.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("hunter_unknown")
        ));
    }

    #[test]
    fn infirmary_protocol_snapshot_exposes_hunters_queue_and_capacity() {
        let flow = infirmary_flow(true);
        let value = serde_json::to_value(flow.snapshot()).expect("serialize world snapshot");
        let infirmary = &value["hunter_roster"]["infirmary"];

        assert_eq!(infirmary["roster_resolved"], true);
        assert_eq!(infirmary["slots"], 3);
        assert_eq!(infirmary["available_slots"], 3);
        assert_eq!(infirmary["hunters"][0]["hunter_id"], 1);
        assert_eq!(infirmary["hunters"][0]["treatment_state"], "idle");
        assert_eq!(infirmary["active"], serde_json::json!([]));
        assert_eq!(infirmary["blockers"], serde_json::json!([]));
    }

    #[test]
    fn infirmary_protocol_decodes_treatment_command() {
        let command: ClientCommand = serde_json::from_value(serde_json::json!({
            "type": "start_infirmary_treatment",
            "instance_id": "infirmary-1",
            "hunter_id": 7,
            "product_id": "product:5"
        }))
        .expect("decode treatment command");

        assert!(matches!(
            command,
            ClientCommand::StartInfirmaryTreatment {
                instance_id,
                hunter_id: 7,
                product_id,
            } if instance_id == "infirmary-1" && product_id == "product:5"
        ));
    }

    #[test]
    fn infirmary_production_ignores_display_capacity_but_consumes_materials() {
        let mut flow = infirmary_flow(true);
        let instance_id = infirmary_instance_id(&flow);
        flow.buildings.material_stocks.push(DurableMaterialStock {
            id: "material:11".to_owned(),
            town_quantity: 20,
            hunter_quantity: 0,
            requested: 0,
            unit_price: 1,
        });
        Arc::make_mut(&mut flow.building_content)
            .gameplay
            .products
            .get_mut(TEST_BANDAGE_ID)
            .expect("test bandage product")
            .service = None;
        Arc::make_mut(&mut flow.building_content)
            .gameplay
            .products
            .get_mut(TEST_BANDAGE_ID)
            .expect("test bandage product")
            .conversion_options = vec![crate::buildings::EconomyConversionOption {
            input_kind: "material".to_owned(),
            input_resource_id: "material:11".to_owned(),
            input_quantity: 1,
            output_stock_quantity: 1,
        }];

        let result = flow
            .handle_command(ClientCommand::CraftShopItem {
                instance_id: instance_id.clone(),
                recipe_id: TEST_BANDAGE_ID.to_owned(),
                material_id: Some("material:11".to_owned()),
                quantity: 10,
            })
            .expect("bandage production result");

        assert!(matches!(
            result.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 10);
        assert_eq!(
            flow.buildings
                .product_stocks
                .iter()
                .find(|stock| {
                    stock.building_instance_id == instance_id && stock.product_id == TEST_BANDAGE_ID
                })
                .expect("bandage stock")
                .quantity,
            15
        );
    }

    #[test]
    fn autonomous_healing_uses_owned_healing_potion_below_ten_percent() {
        let mut flow = infirmary_flow(true);
        let hunter = &mut flow.hunter_roster.hunters[0];
        hunter.current_hp = 90;
        hunter.max_hp = 1_000;
        hunter.hunt.status = "hunting".to_owned();
        hunter.hunt.zone_id = Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned());
        hunter
            .owned_items
            .push(super::super::hunter_roster::DurableHunterOwnedItem {
                product_id: "recipe:consumable:0:level:0".to_owned(),
                quantity: 2,
                ..super::super::hunter_roster::DurableHunterOwnedItem::default()
            });

        flow.apply_autonomous_hunter_healing_policy();

        assert_eq!(flow.hunter_roster.hunters[0].current_hp, 1_000);
        assert_eq!(flow.hunter_roster.hunters[0].owned_items[0].quantity, 1);
        assert_eq!(
            flow.hunter_roster.hunters[0].hunt.zone_id.as_deref(),
            Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID)
        );
        assert_eq!(
            flow.hunter_roster.hunters[0].profile.action_state,
            "using_healing_potion"
        );
    }

    #[test]
    fn autonomous_healing_respects_recovered_potion_cooldown() {
        let mut flow = infirmary_flow(true);
        let hunter = &mut flow.hunter_roster.hunters[0];
        hunter.current_hp = 100;
        hunter.max_hp = 100_000;
        hunter.hunt.status = "hunting".to_owned();
        hunter.hunt.zone_id = Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned());
        hunter
            .owned_items
            .push(super::super::hunter_roster::DurableHunterOwnedItem {
                product_id: "recipe:consumable:0:level:0".to_owned(),
                quantity: 2,
                ..super::super::hunter_roster::DurableHunterOwnedItem::default()
            });

        flow.apply_autonomous_hunter_healing_policy();
        flow.apply_autonomous_hunter_healing_policy();

        assert_eq!(flow.hunter_roster.hunters[0].current_hp, 4_100);
        assert_eq!(flow.hunter_roster.hunters[0].owned_items[0].quantity, 1);
        assert_eq!(
            flow.hunter_roster.hunters[0]
                .hunt
                .healing_potion_cooldown_ms,
            20_000
        );

        let restored = OriginalFlowSession::from_aggregate(flow.durable_state(), 7);
        assert_eq!(
            restored.hunter_roster.hunters[0]
                .hunt
                .healing_potion_cooldown_ms,
            20_000
        );
    }

    #[test]
    fn autonomous_healing_returns_to_infirmary_when_no_potion_is_owned() {
        let mut flow = infirmary_flow(true);
        let hunter = &mut flow.hunter_roster.hunters[0];
        hunter.current_hp = 99;
        hunter.max_hp = 1_000;
        hunter.hunt.status = "hunting".to_owned();
        hunter.hunt.zone_id = Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned());

        flow.apply_autonomous_hunter_healing_policy();

        assert_eq!(flow.hunter_roster.hunters[0].current_hp, 99);
        assert!(flow.hunter_roster.hunters[0].hunt.zone_id.is_none());
        assert_eq!(flow.hunter_roster.hunters[0].hunt.status, "idle");
        assert_eq!(
            flow.hunter_roster.hunters[0].profile.action_state,
            "returning_for_infirmary"
        );
    }

    #[test]
    fn original_flow_reaches_village_and_roster_without_fixture_combat() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
        flow.handle_command(ClientCommand::CompleteBoot);
        assert_eq!(flow.snapshot().screen, OriginalScreen::Village);

        flow.handle_command(ClientCommand::SelectBottomMenu {
            menu: BottomMenuIntent::Character,
        });
        assert_eq!(flow.snapshot().screen, OriginalScreen::HunterRoster);
    }

    #[test]
    fn normalized_town_template_projects_28_core_bases_and_upgrades_by_instance() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
        flow.handle_command(ClientCommand::CompleteBoot);
        assert_eq!(
            flow.buildings
                .buildings
                .iter()
                .map(|building| building.id.as_str())
                .collect::<Vec<_>>(),
            (1..=28).map(|id| format!("build_{id}")).collect::<Vec<_>>()
        );
        assert!(flow.buildings.buildings.iter().all(|building| {
            building.seeded_by.as_deref() == Some("town-template:default-town-v2")
        }));

        flow.buildings.material_stocks.push(DurableMaterialStock {
            id: "material:11".to_owned(),
            town_quantity: 10,
            hunter_quantity: 0,
            requested: 0,
            unit_price: 0,
        });
        let town_hall_instance_id = flow.buildings.buildings[0].instance_id.clone();
        let result = flow
            .handle_command(ClientCommand::UpgradeBuilding {
                instance_id: town_hall_instance_id,
            })
            .expect("upgrade returns a result");
        assert!(matches!(
            result.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.buildings.buildings.len(), 28);
        assert_eq!(flow.buildings.buildings[0].level, 2);

        let system = &flow.snapshot().village.building_system;
        assert_eq!(system.definitions.len(), 79);
        assert_eq!(system.instances.len(), 28);
        let town_hall = system
            .definitions
            .iter()
            .find(|building| building.id == "build_1")
            .unwrap();
        assert_eq!(town_hall.name, "Town Hall");
        assert_eq!(town_hall.max_level, 17);
        assert_eq!(town_hall.construct_cost, 500);
        let state = system
            .states
            .iter()
            .find(|building| building.id == "build_1")
            .unwrap();
        assert_eq!(state.level, 2);
    }

    #[test]
    fn trading_post_reservation_is_authoritative_and_sale_fails_closed_without_seller() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
        flow.handle_command(ClientCommand::CompleteBoot);
        let town_hall_instance_id = flow
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_1")
            .unwrap()
            .instance_id
            .clone();
        let trading_post_instance_id = flow
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_3")
            .unwrap()
            .instance_id
            .clone();
        assert!(flow
            .snapshot()
            .village
            .building_system
            .material_stocks
            .iter()
            .any(|stock| stock.id == "material:1" && stock.town_quantity == 0));
        let wrong_building = flow
            .handle_command(ClientCommand::SetMaterialRequest {
                instance_id: town_hall_instance_id,
                material_id: "material:1".to_owned(),
                quantity: 3,
            })
            .unwrap();
        assert!(matches!(
            wrong_building.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("building_capability_mismatch")
        ));
        let quantity_request = flow
            .handle_command(ClientCommand::SetMaterialRequest {
                instance_id: trading_post_instance_id.clone(),
                material_id: "material:1".to_owned(),
                quantity: 2,
            })
            .unwrap();
        assert!(matches!(
            quantity_request.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(
            flow.buildings
                .material_stocks
                .iter()
                .find(|stock| stock.id == "material:1")
                .unwrap()
                .requested,
            2
        );
        let locked_difficulty = flow
            .handle_command(ClientCommand::SetMaterialRequest {
                instance_id: trading_post_instance_id.clone(),
                material_id: "material:2".to_owned(),
                quantity: ACTIVE_MATERIAL_REQUEST,
            })
            .unwrap();
        assert!(matches!(
            locked_difficulty.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("material_difficulty_locked")
        ));
        flow.handle_command(ClientCommand::SetMaterialRequest {
            instance_id: trading_post_instance_id.clone(),
            material_id: "material:1".to_owned(),
            quantity: ACTIVE_MATERIAL_REQUEST,
        });
        flow.handle_command(ClientCommand::CancelMaterialRequest {
            instance_id: trading_post_instance_id.clone(),
            material_id: "material:1".to_owned(),
        });
        let material_index = flow
            .buildings
            .material_stocks
            .iter()
            .position(|stock| stock.id == "material:1")
            .unwrap();
        assert_eq!(flow.buildings.material_stocks[material_index].requested, 0);
        flow.handle_command(ClientCommand::SetMaterialRequest {
            instance_id: trading_post_instance_id,
            material_id: "material:1".to_owned(),
            quantity: ACTIVE_MATERIAL_REQUEST,
        });
        flow.buildings.material_stocks[material_index].hunter_quantity = 5;
        flow.handle_command(ClientCommand::EnterField);
        flow.handle_command(ClientCommand::NavigateBack);
        assert_eq!(flow.buildings.town_gold, 1_500);
        assert_eq!(
            flow.buildings.material_stocks[material_index].town_quantity,
            0
        );
        assert_eq!(
            flow.buildings.material_stocks[material_index].hunter_quantity,
            5
        );
        assert_eq!(
            flow.buildings.material_stocks[material_index].requested,
            ACTIVE_MATERIAL_REQUEST
        );
        assert!(flow.buildings.trade_settlements.is_empty());
        flow.settle_returning_hunters();
        assert!(flow.buildings.trade_settlements.is_empty());
    }

    #[test]
    fn session_constructor_does_not_repair_or_seed_building_state() {
        let mut aggregate = DurablePlayerAggregate::default();
        aggregate.buildings.buildings.push(DurableBuilding {
            instance_id: Uuid::from_u128(99).to_string(),
            id: "build_4".to_owned(),
            equipped_skin_id: None,
            level: 1,
            uses: 0,
            grid_x: 0,
            grid_y: 10,
            seeded_by: None,
        });
        let restored = OriginalFlowSession::from_aggregate(aggregate, 7);
        assert_eq!(restored.buildings.buildings.len(), 1);
        assert_eq!(restored.buildings.buildings[0].id, "build_4");
        assert_eq!(restored.buildings.buildings[0].grid_y, 10);
    }

    #[test]
    fn field_entry_projects_visual_entities_without_enabling_gameplay() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        let result = flow
            .handle_command(ClientCommand::EnterField)
            .expect("field intent returns a result");
        assert!(result.durable_state_changed);
        let ServerMessage::IntentResult {
            accepted, snapshot, ..
        } = result.message
        else {
            panic!("field navigation should be accepted");
        };
        assert!(accepted);
        assert_eq!(snapshot.screen, OriginalScreen::Field);
        assert_eq!(snapshot.world.mode, WorldMode::Field);
        assert_eq!(
            snapshot.world.authority_scope,
            "server_authoritative_simulation"
        );
        assert_eq!(
            snapshot
                .world
                .entities
                .iter()
                .filter(|entity| entity.descriptor.kind == WorldEntityKind::Hunter)
                .count(),
            8
        );
        assert_eq!(
            snapshot
                .world
                .entities
                .iter()
                .filter(|entity| entity.descriptor.kind == WorldEntityKind::Monster)
                .count(),
            9
        );
        assert!(snapshot.field.visual_projection_runnable);
        assert!(snapshot.field.gameplay_runnable);
        assert!(snapshot.field.blockers.is_empty());
        assert!(snapshot
            .world
            .entities
            .iter()
            .all(|entity| !matches!(entity.animation.as_str(), "atk" | "die" | "dying")));
        for region in ["map_new01", "background_08", "background_11"] {
            assert!(snapshot.world.entities.iter().any(|entity| entity
                .descriptor
                .entity_id
                .starts_with(&format!("monster-{region}-"))));
        }
        flow.advance_simulation_tick();
        let live = flow.snapshot();
        assert!(live
            .world
            .entities
            .iter()
            .any(|entity| entity.descriptor.entity_id == "village-hunter-1"));
        assert!(live
            .world
            .entities
            .iter()
            .all(|entity| entity.descriptor.entity_id != "field-hunter-01"));
    }

    #[test]
    fn village_projection_uses_authoritative_health_for_combat_actors_only() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        });
        flow.hunter_roster.hunters.push(DurableHunterState {
            hunter_id: 7,
            gold: 0,
            current_hp: 19,
            max_hp: 250,
            stamina: HunterServiceGauge::default(),
            satiety: HunterServiceGauge::default(),
            mood: HunterServiceGauge::default(),
            profile: DurableHunterProfile::migration_default(7),
            runtime: Default::default(),
            hunt: Default::default(),
            owned_items: Vec::new(),
        });

        let world = flow.snapshot().world;
        let hunter = world
            .entities
            .iter()
            .find(|entity| entity.descriptor.entity_id == "village-hunter-7")
            .expect("durable Hunter is projected into the village");
        assert_eq!(
            (hunter.current_hp, hunter.maximum_hp),
            (Some(19), Some(250))
        );

        assert!(world
            .entities
            .iter()
            .filter(|entity| entity.descriptor.kind == WorldEntityKind::Monster)
            .all(|entity| entity.current_hp.is_some() && entity.maximum_hp.is_some()));

        let npc = world
            .entities
            .iter()
            .find(|entity| entity.descriptor.kind == WorldEntityKind::Npc)
            .expect("village NPC remains projected");
        assert_eq!((npc.current_hp, npc.maximum_hp), (None, None));
    }

    #[test]
    fn entity_selection_is_authoritative_and_not_persisted() {
        let state = OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        };
        let mut flow = OriginalFlowSession::from_state(state.clone());
        let selected = flow
            .handle_command(ClientCommand::SelectEntity {
                entity_id: "village-npc-01".to_owned(),
            })
            .expect("selection result");
        assert!(!selected.durable_state_changed);
        assert!(matches!(
            selected.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(
            flow.snapshot().world.selected_entity_id,
            Some("village-npc-01".to_owned())
        );
        assert_eq!(flow.state(), &state);

        let rejected = flow
            .handle_command(ClientCommand::SelectEntity {
                entity_id: "client-invented-entity".to_owned(),
            })
            .expect("selection rejection");
        assert!(!rejected.durable_state_changed);
        assert!(matches!(
            rejected.message,
            ServerMessage::IntentResult {
                accepted: false,
                ..
            }
        ));
        assert_eq!(
            flow.snapshot().world.selected_entity_id,
            Some("village-npc-01".to_owned())
        );
    }

    #[test]
    fn monsters_project_health_without_becoming_selectable_entities() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.advance_simulation_tick();
        let monster = flow
            .snapshot()
            .world
            .entities
            .into_iter()
            .find(|entity| entity.descriptor.kind == WorldEntityKind::Monster)
            .expect("monster remains in the visible-world projection");

        assert!(!monster.selectable);
        assert!(monster.current_hp.is_some());
        assert!(monster.maximum_hp.is_some());

        let rejected = flow
            .handle_command(ClientCommand::SelectEntity {
                entity_id: monster.descriptor.entity_id,
            })
            .expect("selection rejection");
        assert!(matches!(
            rejected.message,
            ServerMessage::IntentResult {
                accepted: false,
                ..
            }
        ));
        assert_eq!(flow.snapshot().world.selected_entity_id, None);
    }

    #[test]
    fn back_from_field_persists_only_the_village_screen() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Field,
            boot_completed: true,
        });
        flow.handle_command(ClientCommand::NavigateBack);
        assert_eq!(flow.state().screen, OriginalScreen::Village);
        assert_eq!(flow.snapshot().world.mode, WorldMode::Village);
    }

    #[test]
    fn fixed_simulation_tick_moves_entities_without_changing_navigation_state() {
        let state = OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        };
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: state.clone(),
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        let before = flow.snapshot();
        let after = flow
            .advance_simulation_tick()
            .expect("active world tick")
            .world;
        assert_eq!(after.visual_tick, before.world.visual_tick + 1);
        assert_ne!(after.entities, before.world.entities);
        assert_eq!(flow.state(), &state);
    }

    #[test]
    fn assigned_hunter_routes_never_cross_authoritative_building_footprints() {
        const ACTOR_CLEARANCE: i32 = 14;
        for config in &MAP_CONFIGS {
            let mut flow = OriginalFlowSession::from_aggregate(
                DurablePlayerAggregate {
                    navigation: OriginalFlowPlayerState {
                        screen: OriginalScreen::Village,
                        boot_completed: true,
                    },
                    buildings: test_town_building_state(),
                    hunter_roster: operational_migration_roster(),
                    ..DurablePlayerAggregate::default()
                },
                7,
            );
            flow.handle_command(ClientCommand::AssignHunterHunt {
                hunter_id: 1,
                zone_id: config.map_id.to_owned(),
            });
            let obstacles = town_navigation_obstacles(
                &flow.buildings.buildings,
                &flow.building_content.catalog,
            );
            let mut entered_field = false;

            for _ in 0..400 {
                flow.advance_simulation_tick().expect("active village tick");
                let hunter = flow
                    .monster_world
                    .hunters
                    .iter()
                    .find(|hunter| hunter.hunter_id == 1)
                    .unwrap();
                assert!(obstacles.iter().all(|obstacle| {
                    hunter.x < obstacle.min_x - ACTOR_CLEARANCE
                        || hunter.x > obstacle.max_x + ACTOR_CLEARANCE
                        || hunter.y < obstacle.min_y - ACTOR_CLEARANCE
                        || hunter.y > obstacle.max_y + ACTOR_CLEARANCE
                }));
                if hunter.x >= config.bounds.min_x
                    && hunter.x <= config.bounds.max_x
                    && hunter.y >= config.bounds.min_y
                    && hunter.y <= config.bounds.max_y
                {
                    entered_field = true;
                    break;
                }
            }
            assert!(
                entered_field,
                "Hunter did not reach {} without crossing a building",
                config.map_id
            );
        }
    }

    #[test]
    fn village_projects_only_active_hunters_in_deterministic_non_overlapping_lanes() {
        let flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        let snapshot = flow.snapshot();
        let hunters = snapshot
            .world
            .entities
            .iter()
            .filter(|entity| entity.descriptor.kind == WorldEntityKind::Hunter)
            .collect::<Vec<_>>();

        assert_eq!(hunters.len(), MAX_ACTIVE_TOWN_HUNTERS);
        assert!(hunters
            .iter()
            .all(|entity| entity.descriptor.entity_id != "village-hunter-9"));
        assert_eq!(
            hunters
                .iter()
                .map(|entity| (entity.x, entity.y))
                .collect::<HashSet<_>>()
                .len(),
            MAX_ACTIVE_TOWN_HUNTERS
        );
        assert!(hunters.iter().all(|entity| matches!(
            (entity.action_state, entity.animation.as_str()),
            (WorldEntityActionState::Idle, "hunter_stay")
                | (WorldEntityActionState::Walking, "hunter_walk")
        )));
    }

    #[test]
    fn town_roaming_hunter_projects_walking_motion_for_client_interpolation() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        world.tick(&mut roster);
        let mut agent = world.hunters[0].clone();
        agent.action_state = crate::simulation::HunterActionState::TownIdle;
        agent.animation = "hunter_walk".to_owned();

        let walking = hunter_visual_entity(&agent, 100, 100);
        assert_eq!(walking.action_state, WorldEntityActionState::Walking);

        agent.animation = "hunter_stay".to_owned();
        let paused = hunter_visual_entity(&agent, 100, 100);
        assert_eq!(paused.action_state, WorldEntityActionState::Idle);
    }

    #[test]
    fn hunter_info_projects_fixture_equipment_without_claiming_runtime_capture() {
        let flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        let snapshot = flow.snapshot();
        let hunter = &snapshot.hunter_roster.active_hunters[0];
        let equipment = hunter.hunter_info.equipment_slots.as_ref().unwrap();

        assert_eq!(equipment.len(), 4);
        let weapon = equipment
            .iter()
            .find(|slot| slot.slot_id == "weapon")
            .unwrap();
        assert_eq!(
            weapon.required_class_id.as_deref(),
            Some(hunter.class_id.as_str())
        );
        assert_eq!(weapon.evidence_state, "web_rebuild_test_fixture");
        assert_eq!(hunter.runtime_evidence.inventory.value, None);
    }

    #[test]
    fn unresolved_bottom_menu_does_not_change_screen() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        });
        let result = flow
            .handle_command(ClientCommand::SelectBottomMenu {
                menu: BottomMenuIntent::Store,
            })
            .expect("store intent returns a result");
        assert!(!result.durable_state_changed);
        assert!(matches!(
            result.message,
            ServerMessage::BindingBlocked { .. }
        ));
        assert_eq!(flow.snapshot().screen, OriginalScreen::Village);
    }

    #[test]
    fn unresolved_progression_and_economy_intents_never_grant_state() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        });
        let commands = [
            ClientCommand::OpenHunterProgression { hunter_id: 1 },
            ClientCommand::ClaimQuestReward {
                quest_id: "quest-1".to_owned(),
            },
            ClientCommand::OpenShop {
                shop_id: "main".to_owned(),
            },
            ClientCommand::PurchaseShopItem {
                hunter_id: 1,
                shop_id: "main".to_owned(),
                product_id: "product-1".to_owned(),
            },
            ClientCommand::ClaimMail {
                mail_id: "mail-1".to_owned(),
            },
            ClientCommand::ClaimRewardedAd {
                placement: "unknown".to_owned(),
            },
            ClientCommand::StartTopupPurchase {
                product_id: "unknown".to_owned(),
            },
        ];

        for command in commands {
            let result = flow.handle_command(command).expect("intent result");
            assert!(!result.durable_state_changed);
            match &result.message {
                ServerMessage::BindingBlocked { .. }
                | ServerMessage::IntentResult {
                    accepted: false, ..
                } => {}
                ServerMessage::IntentResult {
                    accepted: true,
                    intent,
                    ..
                } if intent == "open_hunter_progression" => {}
                _ => panic!("unresolved intent unexpectedly granted state"),
            }
            assert_eq!(flow.state().screen, OriginalScreen::Village);
        }
    }

    #[test]
    fn fixture_combat_is_deterministic_and_restores_from_durable_aggregate() {
        let aggregate = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Field,
                boot_completed: true,
            },
            ..DurablePlayerAggregate::default()
        };
        let mut first = OriginalFlowSession::from_aggregate(aggregate, 7);
        for _ in 0..80 {
            first.advance_simulation_tick().expect("field tick");
        }
        let durable = first.durable_state();
        let restored = OriginalFlowSession::from_aggregate(durable, 7);

        assert_eq!(
            restored.snapshot().migration_fixture_combat.world,
            first.snapshot().migration_fixture_combat.world
        );
        assert_eq!(
            restored.snapshot().migration_fixture_combat.evidence_label,
            "deterministic_migration_fixture_not_legacy_balance"
        );
    }

    #[test]
    fn fixture_equip_command_is_idempotent_across_restore() {
        let mut combat = DurablePlayerState::default();
        combat.inventory.insert(2001, 1);
        let aggregate = DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Field,
                boot_completed: true,
            },
            migration_fixture_combat: combat,
            ..DurablePlayerAggregate::default()
        };
        let command_id = Uuid::from_u128(42);
        let mut first = OriginalFlowSession::from_aggregate(aggregate, 7);
        let accepted = first
            .handle_command_with_id(
                ClientCommand::EquipHunterItem {
                    hunter_id: 1,
                    item_id: 2001,
                },
                command_id,
            )
            .expect("equip result");
        assert!(accepted.durable_state_changed);
        assert_eq!(accepted.operations.len(), 1);

        let mut restored = OriginalFlowSession::from_aggregate(first.durable_state(), 7);
        let duplicate = restored
            .handle_command_with_id(
                ClientCommand::EquipHunterItem {
                    hunter_id: 1,
                    item_id: 2001,
                },
                command_id,
            )
            .expect("duplicate equip result");
        assert!(!duplicate.durable_state_changed);
        assert!(duplicate.operations.is_empty());
    }

    #[test]
    fn banish_promotes_fifo_and_is_idempotent_across_restore() {
        let mut roster = DurableHunterRosterState {
            roster_resolved: true,
            wallets_resolved: true,
            ..DurableHunterRosterState::default()
        };
        for hunter_id in 1..=10 {
            roster
                .arrive(DurableHunterState {
                    hunter_id,
                    gold: 100,
                    current_hp: 100,
                    max_hp: 100,
                    stamina: HunterServiceGauge {
                        current: 100,
                        maximum: 100,
                    },
                    satiety: HunterServiceGauge {
                        current: 100,
                        maximum: 100,
                    },
                    mood: HunterServiceGauge {
                        current: 100,
                        maximum: 100,
                    },
                    profile: DurableHunterProfile::migration_default(hunter_id),
                    runtime: Default::default(),
                    hunt: Default::default(),
                    owned_items: Vec::new(),
                })
                .unwrap();
        }
        let aggregate = DurablePlayerAggregate {
            schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        };
        let command_id = Uuid::from_u128(9001);
        let mut flow = OriginalFlowSession::from_aggregate(aggregate, 7);
        let before = flow.snapshot();
        assert!(before
            .world
            .entities
            .iter()
            .any(|entity| entity.descriptor.entity_id == "village-hunter-3"));
        assert!(!before
            .world
            .entities
            .iter()
            .any(|entity| entity.descriptor.entity_id == "village-hunter-9"));
        let accepted = flow
            .handle_command_with_id(ClientCommand::BanishHunter { hunter_id: 3 }, command_id)
            .unwrap();
        assert!(accepted.durable_state_changed);
        assert!(matches!(
            accepted.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        let roster = &accepted_snapshot(&accepted.message).hunter_roster;
        assert_eq!(roster.active_capacity, 8);
        assert_eq!(roster.active_hunters.len(), 8);
        assert_eq!(roster.active_hunters.last().unwrap().hunter_id, 9);
        assert_eq!(roster.waiting_hunters[0].hunter_id, 10);
        let world = &accepted_snapshot(&accepted.message).world;
        assert!(!world
            .entities
            .iter()
            .any(|entity| entity.descriptor.entity_id == "village-hunter-3"));
        assert!(world
            .entities
            .iter()
            .any(|entity| entity.descriptor.entity_id == "village-hunter-9"));
        assert!(!world
            .entities
            .iter()
            .any(|entity| entity.descriptor.entity_id == "village-hunter-10"));

        let mut restored = OriginalFlowSession::from_aggregate(flow.durable_state(), 7);
        let duplicate = restored
            .handle_command_with_id(ClientCommand::BanishHunter { hunter_id: 3 }, command_id)
            .unwrap();
        assert!(!duplicate.durable_state_changed);
        assert!(matches!(
            duplicate.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));

        let not_active = restored
            .handle_command_with_id(
                ClientCommand::BanishHunter { hunter_id: 10 },
                Uuid::from_u128(9002),
            )
            .unwrap();
        assert!(!not_active.durable_state_changed);
        assert!(matches!(
            not_active.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("active_hunter_unknown")
        ));
    }

    #[test]
    fn schema_ten_roster_overflow_upgrades_to_waiting_fifo() {
        let roster = DurableHunterRosterState {
            roster_resolved: true,
            wallets_resolved: true,
            hunters: (1..=10)
                .map(|hunter_id| DurableHunterState {
                    hunter_id,
                    gold: 0,
                    current_hp: 1,
                    max_hp: 1,
                    stamina: HunterServiceGauge::default(),
                    satiety: HunterServiceGauge::default(),
                    mood: HunterServiceGauge::default(),
                    profile: DurableHunterProfile::migration_default(hunter_id),
                    runtime: Default::default(),
                    hunt: Default::default(),
                    owned_items: Vec::new(),
                })
                .collect(),
            ..DurableHunterRosterState::default()
        };
        let flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                schema_version: 10,
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        let durable = flow.durable_state();
        assert_eq!(durable.schema_version, 15);
        assert_eq!(durable.hunter_roster.hunters.len(), 8);
        assert_eq!(durable.hunter_roster.waiting_queue.len(), 2);
        assert_eq!(durable.hunter_roster.waiting_queue[0].hunter.hunter_id, 9);
        assert_eq!(durable.hunter_roster.waiting_queue[1].hunter.hunter_id, 10);
    }

    #[test]
    fn runtime_evidence_keeps_uncaptured_sections_null_and_projects_captured_status() {
        let mut hunter = operational_migration_roster().hunters.remove(0);
        let unresolved = runtime_evidence_snapshot(&hunter);
        assert_eq!(
            unresolved.status.evidence_state,
            HunterEvidenceState::SchemaConfirmed
        );
        assert!(unresolved.status.value.is_none());
        assert!(unresolved.skills.value.is_none());
        assert!(unresolved.appearance.value.is_none());
        assert!(unresolved.inventory.value.is_none());
        assert!(unresolved.growth.value.is_none());
        assert!(unresolved.riding_pet.value.is_none());

        hunter.runtime.status = Some(super::super::DurableHunterRuntimeStatus {
            hp: 120,
            now_hp: 75,
            feel: 90.0,
            now_feel: 45.0,
            hungry: 80.0,
            now_hungry: 40.0,
            tire: 70.0,
            now_tire: 35.0,
            damage: 22,
            armor: 11,
            critical: 7,
            attack_speed: 1.25,
            dodge: 3,
        });
        let captured = runtime_evidence_snapshot(&hunter);
        assert_eq!(
            captured.status.evidence_state,
            HunterEvidenceState::ValueCaptured
        );
        let status = captured.status.value.expect("captured status is projected");
        assert_eq!(status.maximum_hp, 120);
        assert_eq!(status.current_hp, 75);
        assert_eq!(status.attack_speed, 1.25);
    }

    #[test]
    fn authoritative_hunt_tick_returns_loot_and_sale_conserves_economy() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.handle_command_with_id(
            ClientCommand::AssignHunterHunt {
                hunter_id: 1,
                zone_id: super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned(),
            },
            Uuid::from_u128(100),
        )
        .unwrap();
        for _ in 0..HUNT_TICKS_TO_RETURN {
            flow.advance_simulation_tick().unwrap();
        }
        assert_eq!(flow.hunter_roster.hunters[0].hunt.status, "returning");
        flow.handle_command_with_id(
            ClientCommand::ReturnHunterHunt { hunter_id: 1 },
            Uuid::from_u128(101),
        )
        .unwrap();
        let town_gold_before = flow.buildings.town_gold;
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
        let expected_price = flow
            .building_content
            .gameplay
            .item("material:1")
            .and_then(|item| item.town_pays_hunter_gold_per_unit)
            .unwrap();
        let sell_id = Uuid::from_u128(102);
        let sold = flow
            .handle_command_with_id(ClientCommand::SellHunterLoot { hunter_id: 1 }, sell_id)
            .unwrap();
        assert!(matches!(
            sold.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.buildings.town_gold, town_gold_before - expected_price);
        assert_eq!(
            flow.hunter_roster.hunters[0].gold,
            hunter_gold_before + expected_price
        );
        assert_eq!(
            flow.buildings
                .material_stocks
                .iter()
                .find(|stock| stock.id == "material:1")
                .unwrap()
                .town_quantity,
            1
        );
        let after_sale = flow.durable_state();
        let duplicate = flow
            .handle_command_with_id(ClientCommand::SellHunterLoot { hunter_id: 1 }, sell_id)
            .unwrap();
        assert!(matches!(
            duplicate.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.durable_state(), after_sale);
        let conflict = flow
            .handle_command_with_id(ClientCommand::ReturnHunterHunt { hunter_id: 1 }, sell_id)
            .unwrap();
        assert!(matches!(
            conflict.message,
            ServerMessage::IntentResult {
                accepted: false,
                ..
            }
        ));
    }

    #[test]
    fn hunter_sells_multiple_catalog_materials_in_one_authoritative_settlement() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.loot = vec![
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:32".to_owned(),
                quantity: 2,
            },
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:92".to_owned(),
                quantity: 3,
            },
        ];
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        let town_gold_before = flow.buildings.town_gold;
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

        let sold = flow
            .handle_command_with_id(
                ClientCommand::SellHunterLoot { hunter_id: 1 },
                Uuid::from_u128(103),
            )
            .unwrap();

        assert!(matches!(
            sold.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.buildings.town_gold, town_gold_before - 80);
        assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 80);
        assert!(flow.hunter_roster.hunters[0].hunt.loot.is_empty());
        assert_eq!(flow.buildings.trade_settlements.len(), 2);
        assert_eq!(
            flow.buildings.trade_settlements[0].material_id,
            "material:32"
        );
        assert_eq!(flow.buildings.trade_settlements[0].total_gold, 20);
        assert_eq!(
            flow.buildings.trade_settlements[1].material_id,
            "material:92"
        );
        assert_eq!(flow.buildings.trade_settlements[1].total_gold, 60);
        assert_eq!(
            flow.buildings
                .material_stocks
                .iter()
                .find(|stock| stock.id == "material:32")
                .unwrap()
                .town_quantity,
            2
        );
        assert_eq!(
            flow.buildings
                .material_stocks
                .iter()
                .find(|stock| stock.id == "material:92")
                .unwrap()
                .town_quantity,
            3
        );
    }

    #[test]
    fn idle_hunter_auto_sells_only_requested_materials() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.loot = vec![
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:32".to_owned(),
                quantity: 2,
            },
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:92".to_owned(),
                quantity: 3,
            },
        ];
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.buildings.material_stocks = vec![DurableMaterialStock {
            id: "material:32".to_owned(),
            town_quantity: 0,
            hunter_quantity: 0,
            requested: 1,
            unit_price: 10,
        }];
        let town_gold_before = flow.buildings.town_gold;
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

        flow.advance_simulation_tick().expect("village tick");

        assert_eq!(flow.buildings.town_gold, town_gold_before - 10);
        assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 10);
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 1);
        assert_eq!(flow.buildings.material_stocks[0].requested, 0);
        assert_eq!(
            flow.hunter_roster.hunters[0].hunt.loot,
            vec![
                super::super::hunter_roster::DurableHunterLoot {
                    item_id: "material:32".to_owned(),
                    quantity: 1,
                },
                super::super::hunter_roster::DurableHunterLoot {
                    item_id: "material:92".to_owned(),
                    quantity: 3,
                },
            ]
        );
        assert_eq!(flow.buildings.trade_settlements.len(), 1);
    }

    #[test]
    fn idle_hunter_can_auto_sell_again_without_starting_a_new_field_trip() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:32".to_owned(),
            quantity: 2,
        }];
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.buildings.material_stocks = vec![DurableMaterialStock {
            id: "material:32".to_owned(),
            town_quantity: 0,
            hunter_quantity: 0,
            requested: 1,
            unit_price: 10,
        }];

        flow.advance_simulation_tick().expect("first village tick");
        flow.buildings.material_stocks[0].requested = 1;
        flow.advance_simulation_tick().expect("second village tick");

        assert_eq!(flow.buildings.town_gold, 1_480);
        assert_eq!(flow.hunter_roster.hunters[0].gold, 1_020);
        assert!(flow.hunter_roster.hunters[0].hunt.loot.is_empty());
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 2);
        assert_eq!(flow.buildings.material_stocks[0].requested, 0);
        assert_eq!(flow.buildings.trade_settlements.len(), 2);
        assert_eq!(flow.buildings.field_trip_id, 1);
        assert!(flow
            .buildings
            .trade_settlements
            .iter()
            .all(|settlement| settlement.field_trip_id == 1));
        assert_ne!(
            flow.buildings.trade_settlements[0].settlement_id,
            flow.buildings.trade_settlements[1].settlement_id
        );
    }

    #[test]
    fn ordinary_field_hunter_auto_sells_requested_material_and_ignores_legacy_gold_row() {
        let mut roster = operational_migration_roster();
        roster
            .assign_hunt(1, super::super::hunter_roster::ORDINARY_HUNT_REGION_IDS[0])
            .unwrap();
        roster.hunters[0].hunt.loot = vec![
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "gold".to_owned(),
                quantity: 500,
            },
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:32".to_owned(),
                quantity: 2,
            },
        ];
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.buildings.material_stocks = vec![DurableMaterialStock {
            id: "material:32".to_owned(),
            town_quantity: 0,
            hunter_quantity: 0,
            requested: 2,
            unit_price: 0,
        }];
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

        flow.advance_simulation_tick().expect("village tick");

        assert_eq!(flow.buildings.town_gold, 1_480);
        assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 20);
        assert_eq!(flow.buildings.material_stocks[0].unit_price, 10);
        assert!(flow.hunter_roster.hunters[0].hunt.loot.is_empty());
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 2);
        assert_eq!(flow.buildings.material_stocks[0].requested, 0);
    }

    #[test]
    fn ordinary_field_hunter_sell_command_uses_the_requested_material_lane() {
        let mut roster = operational_migration_roster();
        roster
            .assign_hunt(1, super::super::hunter_roster::ORDINARY_HUNT_REGION_IDS[0])
            .unwrap();
        roster.hunters[0].hunt.loot = vec![
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:32".to_owned(),
                quantity: 2,
            },
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:92".to_owned(),
                quantity: 3,
            },
        ];
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.buildings.material_stocks = vec![DurableMaterialStock {
            id: "material:32".to_owned(),
            town_quantity: 0,
            hunter_quantity: 0,
            requested: 1,
            unit_price: 10,
        }];

        let result = flow
            .handle_command_with_id(
                ClientCommand::SellHunterLoot { hunter_id: 1 },
                Uuid::from_u128(8_001),
            )
            .unwrap();

        assert!(matches!(
            result.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 1);
        assert_eq!(flow.buildings.material_stocks[0].requested, 0);
        assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[0].quantity, 1);
        assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[1].quantity, 3);
    }

    #[test]
    fn auto_sale_buys_only_the_quantity_the_town_wallet_can_afford() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:32".to_owned(),
            quantity: 2,
        }];
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.buildings.town_gold = 15;
        flow.buildings.material_stocks = vec![DurableMaterialStock {
            id: "material:32".to_owned(),
            town_quantity: 0,
            hunter_quantity: 0,
            requested: 2,
            unit_price: 10,
        }];
        let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

        flow.advance_simulation_tick().expect("village tick");

        assert_eq!(flow.buildings.town_gold, 5);
        assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 10);
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 1);
        assert_eq!(flow.buildings.material_stocks[0].requested, 1);
        assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[0].quantity, 1);
    }

    #[test]
    fn auto_sale_does_not_enter_rejection_path_when_wallet_cannot_buy_one_unit() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:32".to_owned(),
            quantity: 2,
        }];
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.buildings.town_gold = 9;
        flow.buildings.material_stocks = vec![DurableMaterialStock {
            id: "material:32".to_owned(),
            town_quantity: 0,
            hunter_quantity: 0,
            requested: 2,
            unit_price: 10,
        }];

        let hunter = &flow.hunter_roster.hunters[0];
        assert!(!flow.has_affordable_auto_sale(hunter));
        flow.advance_simulation_tick().expect("village tick");
        assert_eq!(flow.buildings.town_gold, 9);
        assert_eq!(flow.buildings.trade_settlements.len(), 0);
        assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[0].quantity, 2);
    }

    #[test]
    fn skill_and_revive_commands_are_whitelisted_and_persisted() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        let rejected = flow
            .handle_command_with_id(
                ClientCommand::LearnHunterSkill {
                    hunter_id: 1,
                    skill_id: "arbitrary".to_owned(),
                },
                Uuid::from_u128(201),
            )
            .unwrap();
        assert!(matches!(
            rejected.message,
            ServerMessage::IntentResult {
                accepted: false,
                ..
            }
        ));
        flow.handle_command_with_id(
            ClientCommand::LearnHunterSkill {
                hunter_id: 1,
                skill_id: "skill_h1_01".to_owned(),
            },
            Uuid::from_u128(202),
        )
        .unwrap();
        flow.hunter_roster.hunters[1].profile.class_id = "h2".to_owned();
        flow.hunter_roster.hunters[1].profile.visual_family = "H2".to_owned();
        let cross_job = flow
            .handle_command_with_id(
                ClientCommand::LearnHunterSkill {
                    hunter_id: 2,
                    skill_id: "skill_h1_01".to_owned(),
                },
                Uuid::from_u128(204),
            )
            .unwrap();
        assert!(matches!(
            cross_job.message,
            ServerMessage::IntentResult {
                accepted: false,
                ..
            }
        ));
        flow.hunter_roster.defeat_hunter(1).unwrap();
        flow.handle_command_with_id(
            ClientCommand::ReviveHunter { hunter_id: 1 },
            Uuid::from_u128(203),
        )
        .unwrap();
        let restored = OriginalFlowSession::from_aggregate(flow.durable_state(), 7);
        assert_eq!(
            restored.hunter_roster.hunters[0].profile.skills[0].skill_id,
            "skill_h1_01"
        );
        assert_eq!(
            restored.hunter_roster.hunters[0].current_hp,
            restored.hunter_roster.hunters[0].max_hp
        );
    }

    #[test]
    fn all_basic_jobs_start_with_and_can_activate_their_two_catalog_skills() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.state.screen = OriginalScreen::Village;
        let jobs = [
            (1, ["skill_h1_01", "skill_h1_02"], 15_000),
            (2, ["skill_h2_01", "skill_h2_02"], 8_000),
            (3, ["skill_h3_01", "skill_h3_02"], 6_000),
            (4, ["skill_h4_01", "skill_h4_02"], 6_000),
            (5, ["skill_h5_01", "skill_h5_02"], 6_000),
        ];
        for (hunter_id, skill_ids, _) in jobs {
            let hunter = &flow.hunter_roster.hunters[usize::try_from(hunter_id - 1).unwrap()];
            assert_eq!(
                hunter
                    .profile
                    .skills
                    .iter()
                    .map(|skill| skill.skill_id.as_str())
                    .collect::<Vec<_>>(),
                skill_ids
            );
        }

        flow.monster_world.tick(&mut flow.hunter_roster);
        let (target_id, target_x, target_y) = {
            let target = &flow.monster_world.fields[0].monsters[0];
            (target.entity_id.clone(), target.x, target.y)
        };
        let ranger_agent = flow
            .monster_world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 3)
            .unwrap();
        ranger_agent.region_id = Some(flow.monster_world.fields[0].map_id.clone());
        ranger_agent.x = target_x;
        ranger_agent.y = target_y;
        ranger_agent.target_monster_id = Some(target_id);
        let mut command = 211_u128;
        for (hunter_id, skill_ids, cooldown_ms) in jobs {
            let used = flow
                .handle_command_with_id(
                    ClientCommand::UseHunterSkill {
                        hunter_id,
                        skill_id: skill_ids[0].to_owned(),
                        target_entity_id: None,
                    },
                    Uuid::from_u128(command),
                )
                .unwrap();
            command += 1;
            assert!(
                matches!(
                    used.message,
                    ServerMessage::IntentResult { accepted: true, .. }
                ),
                "hunter {hunter_id} failed to activate {}: {:?}",
                skill_ids[0],
                used.message
            );
            let skill = &flow.hunter_roster.hunters[usize::try_from(hunter_id - 1).unwrap()]
                .profile
                .skills[0];
            assert!(!skill.ready);
            assert_eq!(skill.cooldown_remaining_ms, cooldown_ms);
        }
        let ranger = flow
            .world_projection()
            .entities
            .into_iter()
            .find(|entity| entity.descriptor.entity_id == "village-hunter-3")
            .unwrap();
        // Exact skill-to-animation bindings are unresolved; activation keeps a
        // neutral recovered Hunter clip rather than inventing an H3 mapping.
        assert_eq!(ranger.animation, "hunter_stay");
        assert_eq!(ranger.attack_effect_key, None);

        flow.refresh_skill_cooldowns(16_000);
        assert!(flow
            .hunter_roster
            .hunters
            .iter()
            .take(5)
            .all(|hunter| hunter.profile.skills[0].ready));
    }

    #[test]
    fn hunter_automatically_casts_the_first_ready_skill_on_an_acquired_target() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.state.screen = OriginalScreen::Village;
        flow.monster_world.tick(&mut flow.hunter_roster);
        let target = &flow.monster_world.fields[0].monsters[0];
        let target_id = target.entity_id.clone();
        let target_position = (target.x, target.y);
        let agent = flow
            .monster_world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        agent.region_id = Some(flow.monster_world.fields[0].map_id.clone());
        agent.x = target_position.0;
        agent.y = target_position.1;
        agent.target_monster_id = Some(target_id);
        agent.active_skill_id = None;
        agent.action_state = HunterActionState::Attacking;

        flow.auto_cast_ready_hunter_skills();

        let skill = &flow.hunter_roster.hunters[0].profile.skills[0];
        assert_eq!(skill.skill_id, "skill_h1_01");
        assert!(!skill.ready);
        assert_eq!(skill.cooldown_remaining_ms, 15_000);
        let agent = flow
            .monster_world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        assert_eq!(agent.active_skill_id.as_deref(), Some("skill_h1_01"));
        assert_eq!(agent.recovery_ticks, 3);
        assert_eq!(agent.skill_attack_percent, 10);
        assert_eq!(agent.skill_attack_speed_milli, 2_380);

        flow.advance_simulation_tick().expect("active village tick");
        assert_eq!(
            flow.hunter_roster.hunters[0].profile.skills[0].cooldown_remaining_ms,
            14_900
        );
    }

    #[test]
    fn hunter_does_not_attempt_auto_cast_while_chasing_an_out_of_range_target() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.state.screen = OriginalScreen::Village;
        flow.monster_world.tick(&mut flow.hunter_roster);
        let target_id = flow.monster_world.fields[0].monsters[0].entity_id.clone();
        let agent = flow
            .monster_world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        agent.target_monster_id = Some(target_id);
        agent.action_state = HunterActionState::Chasing;

        flow.auto_cast_ready_hunter_skills();

        let skill = &flow.hunter_roster.hunters[0].profile.skills[0];
        assert!(skill.ready);
        assert_eq!(skill.cooldown_remaining_ms, 0);
        assert!(flow.monster_world.hunters[0].active_skill_id.is_none());
    }

    #[test]
    fn rejected_targeted_skill_does_not_mutate_combat_presentation() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.monster_world.tick(&mut flow.hunter_roster);
        let agent = flow
            .monster_world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 3)
            .unwrap();
        agent.target_monster_id = None;
        let before = agent.clone();

        let result = flow
            .handle_command_with_id(
                ClientCommand::UseHunterSkill {
                    hunter_id: 3,
                    skill_id: "skill_h3_01".to_owned(),
                    target_entity_id: None,
                },
                Uuid::from_u128(301),
            )
            .unwrap();

        assert!(matches!(
            result.message,
            ServerMessage::IntentResult {
                accepted: false,
                ..
            }
        ));
        assert_eq!(
            flow.monster_world
                .hunters
                .iter()
                .find(|agent| agent.hunter_id == 3)
                .unwrap(),
            &before
        );
    }

    #[test]
    fn durable_aggregate_restores_hunter_runtime_but_excludes_monsters_and_drops() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.zone_id = Some("background_11".to_owned());
        roster.hunters[0].hunt.status = "hunting".to_owned();
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Field,
                    boot_completed: true,
                },
                hunter_roster: roster,
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.monster_world.enter_map("background_11").unwrap();
        flow.monster_world.set_density(3).unwrap();
        flow.monster_world.tick = 99;
        flow.monster_world.tick(&mut flow.hunter_roster);
        let target_id = flow.monster_world.fields[2].monsters[0].entity_id.clone();
        let agent = flow
            .monster_world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        agent.x = 411;
        agent.y = 733;
        agent.facing_left = true;
        agent.action_state = HunterActionState::Attacking;
        agent.animation = "hunter_atk".to_owned();
        agent.target_monster_id = Some(target_id.clone());
        agent.recovery_ticks = 4;
        flow.monster_world
            .current_field_mut()
            .drops
            .push(crate::simulation::MonsterDrop {
                drop_id: "drop-monster-background_11-0-test".to_owned(),
                monster_entity_id: "monster-background_11-0".to_owned(),
                item_id: "material:32".to_owned(),
                quantity: 1,
                x: 0,
                y: 0,
                owner_hunter_id: 1,
                gold: 0,
                experience: 0,
            });

        let durable = flow.durable_state();
        let json = serde_json::to_value(&durable).unwrap();
        assert!(json.get("monster_world").is_none());
        assert_eq!(json["hunter_world_runtime"][0]["x"], 411);
        assert!(json["monster_field_config"].get("tier_id").is_none());
        assert!(json["monster_field_config"].get("density_level").is_none());
        assert_eq!(
            json["monster_field_config"]["densities"][2]["map_id"],
            "background_11"
        );
        assert_eq!(
            json["monster_field_config"]["densities"][2]["density_level"],
            3
        );

        let restored = OriginalFlowSession::from_aggregate(durable, 7);
        assert_eq!(restored.monster_world.current_map_id, "map_new01");
        restored.monster_world.fields.iter().for_each(|field| {
            let expected = if field.map_id == "background_11" {
                3
            } else {
                1
            };
            assert_eq!(field.density_level, expected);
        });
        assert_eq!(restored.monster_world.tick, 0);
        assert!(restored
            .monster_world
            .fields
            .iter()
            .all(|field| field.drops.is_empty()));
        let restored_agent = restored
            .monster_world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        assert_eq!((restored_agent.x, restored_agent.y), (411, 733));
        assert!(restored_agent.facing_left);
        assert_eq!(restored_agent.action_state, HunterActionState::Attacking);
        assert_eq!(restored_agent.animation, "hunter_atk");
        assert_eq!(
            restored_agent.target_monster_id.as_deref(),
            Some(target_id.as_str())
        );
        assert_eq!(restored_agent.recovery_ticks, 4);
    }

    #[test]
    fn reconnect_drops_an_unrestorable_loot_action_without_resetting_position() {
        let mut roster = operational_migration_roster();
        roster.hunters[0].hunt.zone_id = Some("background_08".to_owned());
        roster.hunters[0].hunt.status = "hunting".to_owned();
        let runtime = HunterAgentState {
            hunter_id: 1,
            region_id: Some("background_08".to_owned()),
            x: 123,
            y: 456,
            facing_left: false,
            action_state: HunterActionState::CollectingLoot,
            animation: "hunter_stay".to_owned(),
            target_monster_id: None,
            target_drop_id: Some("expired-drop".to_owned()),
            recovery_ticks: 12,
            respawn_ticks: None,
            attack_sequence: 3,
            loot_sequence: 8,
            loot_item_id: Some("material:32".to_owned()),
            loot_quantity: 2,
            active_skill_id: None,
            skill_buff_ticks: 0,
            skill_attack_percent: 0,
            skill_defense_percent: 0,
            skill_evasion_percent: 0,
            skill_critical_percent: 0,
            skill_attack_speed_milli: 0,
            ice_armor_active: false,
            entry_stage: 2,
        };
        let restored = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: roster,
                hunter_world_runtime: vec![runtime],
                ..DurablePlayerAggregate::default()
            },
            7,
        );

        let agent = restored
            .monster_world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        assert_eq!((agent.x, agent.y), (123, 456));
        assert_eq!(agent.action_state, HunterActionState::AcquiringTarget);
        assert!(agent.target_drop_id.is_none());
        assert!(agent.loot_item_id.is_none());
        assert_eq!(agent.loot_quantity, 0);
        assert_eq!(agent.recovery_ticks, 0);
    }

    #[test]
    fn world_projection_includes_the_collected_gold_quantity() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.state.screen = OriginalScreen::Village;
        flow.monster_world.tick(&mut flow.hunter_roster);
        let agent = flow
            .monster_world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        agent.loot_sequence = 1;
        agent.loot_item_id = Some("gold".to_owned());
        agent.loot_quantity = 39;

        let hunter = flow
            .world_entities()
            .into_iter()
            .find(|entity| entity.descriptor.entity_id == "village-hunter-1")
            .unwrap();

        assert_eq!(hunter.loot_label.as_deref(), Some("Gold +39"));
    }

    #[test]
    fn village_density_board_updates_only_the_selected_hunting_region() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        });

        let result = flow
            .handle_command(ClientCommand::SetMonsterRegionDensity {
                region_id: "background_08".to_owned(),
                level: 3,
            })
            .expect("density board result");

        assert!(matches!(
            result.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.monster_world.current_map_id, "map_new01");
        assert_eq!(flow.monster_world.fields[0].density_level, 1);
        assert_eq!(flow.monster_world.fields[1].density_level, 3);
        assert_eq!(flow.monster_world.fields[2].density_level, 1);
    }

    #[test]
    fn legacy_single_map_density_migrates_without_persisting_the_visited_map() {
        let aggregate = DurablePlayerAggregate {
            monster_field_config: serde_json::from_value(serde_json::json!({
                "tier_id": "background_08",
                "density_level": 3
            }))
            .unwrap(),
            ..DurablePlayerAggregate::default()
        };

        let restored = OriginalFlowSession::from_aggregate(aggregate, 7);
        assert_eq!(restored.monster_world.current_map_id, "map_new01");
        assert_eq!(
            restored
                .monster_world
                .fields
                .iter()
                .find(|field| field.map_id == "background_08")
                .unwrap()
                .density_level,
            3
        );
    }

    #[test]
    fn simulation_outcome_is_invariant_to_scheduler_tick_rate() {
        let state = OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        };
        let mut ten_hz = OriginalFlowSession::from_state(state.clone());
        let mut twenty_hz = OriginalFlowSession::from_state(state);

        let mut ten_hz_result = None;
        for _ in 0..10 {
            ten_hz_result = ten_hz.advance_simulation_step(100_000_000);
        }
        let mut twenty_hz_result = None;
        for _ in 0..20 {
            if let Some(result) = twenty_hz.advance_simulation_step(50_000_000) {
                twenty_hz_result = Some(result);
            }
        }

        let ten_hz_result = ten_hz_result.expect("10 Hz produces a domain frame");
        let twenty_hz_result = twenty_hz_result.expect("20 Hz produces a domain frame");
        assert_eq!(
            ten_hz_result.simulation_tick,
            twenty_hz_result.simulation_tick
        );
        assert_eq!(ten_hz_result.world, twenty_hz_result.world);
        assert_eq!(ten_hz.durable_state(), twenty_hz.durable_state());
    }

    fn accepted_snapshot(message: &ServerMessage) -> &OriginalFlowSnapshot {
        match message {
            ServerMessage::IntentResult { snapshot, .. }
            | ServerMessage::BindingBlocked { snapshot, .. }
            | ServerMessage::Resync { snapshot }
            | ServerMessage::WorldUpdate { snapshot }
            | ServerMessage::Welcome { snapshot, .. } => snapshot,
            ServerMessage::WorldFrame { .. } | ServerMessage::FarmReportQueued { .. } => {
                panic!("lightweight transport messages do not carry domain snapshots")
            }
        }
    }
}
