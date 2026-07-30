use super::{
    BottomMenuIntent, Facing, HunterEvidenceState, OriginalScreen, Serialize, Uuid,
    WorldEntityActionState, WorldEntityKind, WorldMode, WorldSnapshot,
};

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
    pub trade_sequence: u64,
    pub trade_gold: u64,
    pub trade_materials: Vec<TradeMaterialPresentationSnapshot>,
    pub attack_effect_key: Option<&'static str>,
    pub skill_presentation_key: Option<String>,
    pub current_hp: Option<u64>,
    pub maximum_hp: Option<u64>,
    pub interaction_prompt_key: Option<&'static str>,
    pub selectable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TradeMaterialPresentationSnapshot {
    pub material_id: String,
    pub display_name: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatPresentationSnapshot {
    pub sequence: u64,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub kind: crate::simulation::CombatPresentationKind,
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
