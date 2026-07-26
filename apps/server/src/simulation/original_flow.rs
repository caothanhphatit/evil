use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::buildings::{
    gear_product_route, AuthoritativeBuildingContent, BaseBuildingDefinition, BaseBuildingId,
    BuildingLevelDefinition, EconomyAmount,
};
#[cfg(test)]
use crate::buildings::{
    BuildingCapabilityDefinition, BuildingCatalog, BuildingGameplayCatalog,
    BuildingLevelPrerequisite, EconomyItemDefinition, EconomyProductDefinition,
    EconomyProductService,
};

#[cfg(test)]
use super::hunter_roster::operational_migration_roster;
#[cfg(test)]
use super::hunter_roster::DurableHunterProfile;
use super::hunter_roster::{
    DurableHunterRosterState, DurableHunterState, HunterRosterError, MAX_ACTIVE_TOWN_HUNTERS,
};
use super::product_service::{capacity_for_level, HunterServiceGauge, ServiceEffectKind};
use super::trading_post::{
    material_catalog_stocks, material_difficulty_rating, settle_returning_hunters,
    ACTIVE_MATERIAL_REQUEST,
};
use super::{
    ClientCommand, DurablePlayerState, FixtureCommand, PendingOperation, ServerMessage, Simulation,
    WorldSnapshot,
};

pub const DURABLE_PLAYER_SCHEMA_VERSION: u16 = 11;
pub const MIGRATION_FIXTURE_CONTENT_ID: &str = "migration-fixture.slice1-combat-v1";

const TOWN_GRID_MIN: i32 = -32;
const TOWN_GRID_MAX: i32 = 32;
const MAX_PRODUCTION_QUANTITY: u32 = 1_000;

const FIELD_GAMEPLAY_BLOCKERS: [&str; 3] = [
    "field_map_exact_binding",
    "field_monster_gameplay_binding",
    "combat_rules_binding",
];
const HUNTER_PROGRESSION_BLOCKERS: [&str; 3] = [
    "hunter_catalog_binding",
    "starter_stats_binding",
    "progression_rules_binding",
];
const QUEST_BLOCKERS: [&str; 2] = ["quest_catalog_binding", "quest_reward_binding"];
const SHOP_BLOCKERS: [&str; 2] = ["shop_catalog_binding", "shop_price_binding"];
const GEAR_SALE_BLOCKERS: [&str; 4] = [
    "hunter_roster_binding",
    "hunter_wallet_binding",
    "hunter_equipment_inventory_binding",
    "shop_visit_binding",
];
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurablePlayerAggregate {
    pub schema_version: u16,
    pub navigation: OriginalFlowPlayerState,
    pub migration_fixture_combat: DurablePlayerState,
    pub buildings: DurableBuildingState,
    pub hunter_roster: DurableHunterRosterState,
    pub product_services: DurableProductServiceState,
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
            legacy_infirmary: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableProductServiceState {
    pub visits: Vec<DurableProductServiceVisit>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
    pub hunter_info: HunterInfoSnapshot,
    pub roster_state: &'static str,
    pub position: usize,
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
    pub icon_path: Option<String>,
    pub placeholder_icon_path: Option<String>,
    pub locked: Option<bool>,
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
    pub animation: &'static str,
    pub class_family: Option<String>,
    pub selectable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldProjection {
    pub mode: WorldMode,
    pub visual_tick: u64,
    pub coordinate_space: &'static str,
    pub authority_scope: &'static str,
    pub entities: Vec<WorldEntityProjection>,
    pub selected_entity_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OriginalFlowSnapshot {
    pub screen: OriginalScreen,
    pub content_release_id: &'static str,
    pub content_release_runnable: bool,
    pub flow_order: Vec<OriginalScreen>,
    pub village: VillageSnapshot,
    pub hunter_roster: HunterRosterSnapshot,
    pub field: FieldSnapshot,
    pub world: WorldProjection,
    pub migration_fixture_combat: MigrationFixtureCombatProjection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationFixtureCombatProjection {
    pub content_id: &'static str,
    pub evidence_label: &'static str,
    pub active: bool,
    pub world: WorldSnapshot,
}

#[derive(Debug)]
pub struct OriginalFlowSession {
    state: OriginalFlowPlayerState,
    simulation: Simulation,
    combat_snapshot: WorldSnapshot,
    selected_entity_id: Option<String>,
    visual_tick: u64,
    buildings: DurableBuildingState,
    hunter_roster: DurableHunterRosterState,
    product_services: DurableProductServiceState,
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
    pub snapshot: OriginalFlowSnapshot,
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
        let simulation = Simulation::from_state(seed, state.migration_fixture_combat);
        let combat_snapshot = simulation.snapshot();
        Self {
            state: state.navigation,
            simulation,
            combat_snapshot,
            selected_entity_id: None,
            visual_tick: 0,
            buildings: state.buildings,
            hunter_roster: state.hunter_roster,
            product_services: state.product_services,
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
            legacy_infirmary: None,
        }
    }

    pub fn advance_simulation_tick(&mut self) -> Option<OriginalFlowTickResult> {
        if self.state.screen != OriginalScreen::Field {
            return None;
        }
        self.combat_snapshot = self.simulation.step();
        Some(OriginalFlowTickResult {
            snapshot: self.snapshot(),
            operations: self.simulation.drain_operations(),
        })
    }

    pub fn advance_visual_tick(&mut self) -> Option<OriginalFlowSnapshot> {
        if self.state.screen != OriginalScreen::Village {
            return None;
        }
        self.visual_tick = self.visual_tick.wrapping_add(1);
        self.advance_product_services(200);
        Some(self.snapshot())
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
                gameplay_runnable: false,
                blockers: FIELD_GAMEPLAY_BLOCKERS.to_vec(),
            },
            world: self.world_projection(),
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
            ClientCommand::OpenHunterProgression { .. } => {
                self.binding_blocked("open_hunter_progression", &HUNTER_PROGRESSION_BLOCKERS)
            }
            ClientCommand::BanishHunter { hunter_id } => self.banish_hunter(command_id, hunter_id),
            ClientCommand::EquipHunterItem { hunter_id, item_id } => {
                self.equip_fixture_item(command_id, hunter_id, item_id)
            }
            ClientCommand::ClaimQuestReward { .. } => {
                self.binding_blocked("claim_quest_reward", &QUEST_BLOCKERS)
            }
            ClientCommand::OpenShop { .. } => self.binding_blocked("open_shop", &SHOP_BLOCKERS),
            ClientCommand::PurchaseShopItem {
                shop_id,
                product_id,
            } => self.purchase_shop_item(&shop_id, &product_id),
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
                icon: material_icon_path(&stock.id).unwrap_or_default().to_owned(),
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
                        let is_native_product = product.building_id.as_ref() == Some(&building_id);
                        let is_sale_product = gear_route
                            .as_ref()
                            .is_some_and(|route| route.sale_building_id == building_id);
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

                        let stock_building = gear_route
                            .as_ref()
                            .and_then(|route| {
                                self.buildings.buildings.iter().find(|candidate| {
                                    candidate.id == route.sale_building_id.as_str()
                                })
                            })
                            .or(building);
                        let stock = stock_building.map_or(0, |stock_building| {
                            self.buildings
                                .product_stocks
                                .iter()
                                .find(|stock| {
                                    stock.building_instance_id == stock_building.instance_id
                                        && stock.product_id == product.product_id
                                })
                                .map_or(0, |stock| stock.quantity)
                        });
                        if is_sale_product && stock == 0 {
                            continue;
                        }
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
                                    product
                                        .service
                                        .is_some()
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
                                || product.sale_price.first().map_or(0, |price| price.quantity),
                                |service| service.use_money,
                            ),
                            kind: if product.service.is_some() {
                                "service"
                            } else {
                                "craft"
                            },
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
        // The recovered BuildingPop exposes a per-item Request/Cancel toggle, not a requested
        // quantity editor. Keep the wire field for protocol compatibility but accept only the
        // active-reservation sentinel.
        if quantity != ACTIVE_MATERIAL_REQUEST {
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
            stock.requested = ACTIVE_MATERIAL_REQUEST;
            stock.unit_price = authoritative_price;
        } else {
            self.buildings.material_stocks.push(DurableMaterialStock {
                id: material_id.to_owned(),
                town_quantity: 0,
                hunter_quantity: 0,
                requested: ACTIVE_MATERIAL_REQUEST,
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
        let gear_route = gear_product_route(&self.building_content.gameplay, product);
        if gear_route
            .as_ref()
            .is_some_and(|route| route.rating >= u16::from(building.level))
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
        if !crafting_capability && product.service.is_none() {
            return self.rejected("craft_shop_item", "building_capability_mismatch");
        }
        let stock_building = if let Some(route) = &gear_route {
            let Some(sale_building) = self
                .buildings
                .buildings
                .iter()
                .find(|candidate| candidate.id == route.sale_building_id.as_str())
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
        if capacity > 0 && stocked.saturating_add(quantity) > capacity {
            return self.rejected("craft_shop_item", "product_capacity_exceeded");
        }
        let costs = if product.service.is_some() {
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

    fn purchase_shop_item(&mut self, shop_id: &str, product_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("purchase_shop_item", "village_unavailable");
        }
        let Ok(building_id) = BaseBuildingId::parse(shop_id) else {
            return self.rejected("purchase_shop_item", "building_unknown");
        };
        let Some(product) = self.building_content.gameplay.product(product_id) else {
            return self.rejected("purchase_shop_item", "recipe_unknown");
        };
        let Some(route) = gear_product_route(&self.building_content.gameplay, product) else {
            return self.rejected("purchase_shop_item", "recipe_building_mismatch");
        };
        if route.sale_building_id != building_id {
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
        if route.rating >= u16::from(building.level) {
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
        let price = product
            .sale_price
            .first()
            .map_or(0, |amount| amount.quantity);
        if price == 0 {
            return self.rejected("purchase_shop_item", "sale_price_unresolved");
        }
        let _ = stock;
        // A hunter sale must debit a concrete hunter wallet and hand an owned
        // gear instance to that hunter atomically. The current command carries
        // neither a buyer nor authoritative hunter economy state, so settling
        // town gold here would create value from nothing.
        self.binding_blocked("purchase_shop_item", &GEAR_SALE_BLOCKERS)
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

    fn world_projection(&self) -> WorldProjection {
        WorldProjection {
            mode: match self.state.screen {
                OriginalScreen::Village => WorldMode::Village,
                OriginalScreen::Field => WorldMode::Field,
                OriginalScreen::Boot | OriginalScreen::HunterRoster => WorldMode::Inactive,
            },
            visual_tick: self.visual_tick,
            coordinate_space: "normalized_viewport_1000",
            authority_scope: "visual_roaming_only",
            entities: self.world_entities(),
            selected_entity_id: self.selected_entity_id.clone(),
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
                        let motion = village_hunter_motion(self.visual_tick, slot);
                        let mut entity = visual_entity(
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
                        );
                        entity.class_family = Some(hunter.profile.visual_family.clone());
                        entity
                    })
                    .collect::<Vec<_>>();
                entities.push(visual_entity(
                    "village-npc-01",
                    WorldEntityKind::Npc,
                    "npc",
                    "Npc",
                    BindingConfidence::Confirmed,
                    625,
                    625,
                    Facing::Left,
                    WorldEntityActionState::Idle,
                    "npc_stay",
                ));
                entities
            }
            OriginalScreen::Field => vec![
                visual_entity(
                    "field-hunter-01",
                    WorldEntityKind::Hunter,
                    "hunter",
                    "hunter",
                    BindingConfidence::Confirmed,
                    roam(self.visual_tick, 235, 390, 90).0,
                    650,
                    roam(self.visual_tick, 235, 390, 90).1,
                    WorldEntityActionState::Walking,
                    "hunter_walk",
                ),
                visual_entity(
                    "field-monster-candidate-01",
                    WorldEntityKind::Monster,
                    "mon_goldblin",
                    "mon_goldblin",
                    BindingConfidence::Confirmed,
                    roam(self.visual_tick + 37, 610, 780, 110).0,
                    650,
                    roam(self.visual_tick + 37, 610, 780, 110).1,
                    WorldEntityActionState::Walking,
                    "walk",
                ),
            ],
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

fn material_icon_path(material_id: &str) -> Option<&'static str> {
    Some(match material_id {
        "material:1" => "/content/releases/original-flow-v1/sprites/shop_product_26__6294.png",
        "material:16" => "/content/releases/original-flow-v1/sprites/shop_product_251__3130.png",
        "currency:gem" => "/content/releases/original-flow-v1/sprites/top_ic_02_gem__6963.png",
        "currency:elemental" => {
            "/content/releases/original-flow-v1/sprites/top_ic_03_element__4250.png"
        }
        _ => return None,
    })
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
            })
            .collect(),
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
            equipment_slots: None,
            skills: None,
            growth: None,
            riding_pet: hunter.profile.riding_pet_state_resolved.then_some(
                HunterRidingPetSnapshot::Empty {
                    mounted: false,
                    can_move_to_ranch: false,
                },
            ),
            materials: None,
        },
        roster_state,
        position,
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
    animation: &'static str,
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
        animation,
        class_family: None,
        selectable: true,
    }
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
    let min_x = 235 + i32::try_from(slot % 4).unwrap_or(0) * 135;
    let max_x = min_x + 72;
    // Separate lanes guarantee that active Hunters never share a world position.
    let y = 555 + i32::try_from(slot).unwrap_or(0) * 22;
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

fn roam(tick: u64, min: i32, max: i32, period: u64) -> (i32, Facing) {
    let span = (max - min).max(1) as u64;
    let phase = tick % (period * 2);
    let offset = if phase <= period {
        phase * span / period
    } else {
        (period * 2 - phase) * span / period
    };
    let facing = if phase < period {
        Facing::Right
    } else {
        Facing::Left
    };
    (min + offset as i32, facing)
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
    fn blacksmith_crafts_into_weapon_shop_stock_without_minting_unbound_hunter_gold() {
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
        let blocked_sale = flow
            .handle_command(ClientCommand::PurchaseShopItem {
                shop_id: "build_7".to_owned(),
                product_id: product_id.to_owned(),
            })
            .unwrap();
        assert!(matches!(
            blocked_sale.message,
            ServerMessage::BindingBlocked { ref blockers, .. }
                if blockers == &GEAR_SALE_BLOCKERS
        ));
        assert_eq!(flow.buildings.product_stocks[0].quantity, 2);
        assert_eq!(flow.buildings.town_gold, gold_before);
        assert_eq!(flow.buildings.hunter_equipment_purchases, 0);

        let recipes = flow.snapshot().village.building_system.recipes;
        assert!(recipes
            .iter()
            .any(|recipe| recipe.id == product_id && recipe.shop_id == "build_10"));
        assert!(recipes.iter().any(|recipe| {
            recipe.id == product_id && recipe.shop_id == "build_7" && recipe.stock == 2
        }));
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
    fn blacksmith_routes_wearable_armor_to_armor_shop_and_enforces_tier_levels() {
        let product_id = "recipe:helmet:0:rating:1";
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
            .any(|stock| stock.id == "material:0" && stock.town_quantity == 0));
        let wrong_building = flow
            .handle_command(ClientCommand::SetMaterialRequest {
                instance_id: town_hall_instance_id,
                material_id: "material:0".to_owned(),
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
        let invalid_quantity = flow
            .handle_command(ClientCommand::SetMaterialRequest {
                instance_id: trading_post_instance_id.clone(),
                material_id: "material:0".to_owned(),
                quantity: 2,
            })
            .unwrap();
        assert!(matches!(
            invalid_quantity.message,
            ServerMessage::IntentResult {
                accepted: false,
                ref reason,
                ..
            } if reason.as_deref() == Some("material_quantity_invalid")
        ));
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
            material_id: "material:0".to_owned(),
            quantity: ACTIVE_MATERIAL_REQUEST,
        });
        flow.handle_command(ClientCommand::CancelMaterialRequest {
            instance_id: trading_post_instance_id.clone(),
            material_id: "material:0".to_owned(),
        });
        assert_eq!(flow.buildings.material_stocks[0].requested, 0);
        flow.handle_command(ClientCommand::SetMaterialRequest {
            instance_id: trading_post_instance_id,
            material_id: "material:0".to_owned(),
            quantity: ACTIVE_MATERIAL_REQUEST,
        });
        flow.buildings.material_stocks[0].hunter_quantity = 5;
        flow.handle_command(ClientCommand::EnterField);
        flow.handle_command(ClientCommand::NavigateBack);
        assert_eq!(flow.buildings.town_gold, 1_500);
        assert_eq!(flow.buildings.material_stocks[0].town_quantity, 0);
        assert_eq!(flow.buildings.material_stocks[0].hunter_quantity, 5);
        assert_eq!(
            flow.buildings.material_stocks[0].requested,
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
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        });
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
        assert_eq!(snapshot.world.authority_scope, "visual_roaming_only");
        assert_eq!(snapshot.world.entities.len(), 2);
        assert!(snapshot.field.visual_projection_runnable);
        assert!(!snapshot.field.gameplay_runnable);
        assert_eq!(snapshot.field.blockers, FIELD_GAMEPLAY_BLOCKERS);
        assert!(snapshot.world.entities.iter().all(|entity| {
            !matches!(entity.animation, "atk" | "die" | "dying")
                && !entity.descriptor.placement_binding.resolved
        }));
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
    fn visual_tick_moves_entities_without_changing_durable_state() {
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
        let after = flow.advance_visual_tick().expect("active world tick");
        assert_eq!(after.world.visual_tick, before.world.visual_tick + 1);
        assert_ne!(after.world.entities, before.world.entities);
        assert_eq!(flow.state(), &state);
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
            (entity.action_state, entity.animation),
            (WorldEntityActionState::Idle, "hunter_stay")
                | (WorldEntityActionState::Walking, "hunter_walk")
        )));
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
            assert!(matches!(
                result.message,
                ServerMessage::BindingBlocked { .. }
                    | ServerMessage::IntentResult {
                        accepted: false,
                        ..
                    }
            ));
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
        assert_eq!(durable.schema_version, 11);
        assert_eq!(durable.hunter_roster.hunters.len(), 8);
        assert_eq!(durable.hunter_roster.waiting_queue.len(), 2);
        assert_eq!(durable.hunter_roster.waiting_queue[0].hunter.hunter_id, 9);
        assert_eq!(durable.hunter_roster.waiting_queue[1].hunter.hunter_id, 10);
    }

    fn accepted_snapshot(message: &ServerMessage) -> &OriginalFlowSnapshot {
        match message {
            ServerMessage::IntentResult { snapshot, .. }
            | ServerMessage::BindingBlocked { snapshot, .. }
            | ServerMessage::Resync { snapshot }
            | ServerMessage::WorldUpdate { snapshot }
            | ServerMessage::Welcome { snapshot, .. } => snapshot,
        }
    }
}
