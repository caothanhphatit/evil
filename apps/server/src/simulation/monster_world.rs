#[path = "monster_world/catalog.rs"]
mod catalog;
#[path = "monster_world/combat_runtime.rs"]
mod combat_runtime;
use catalog::monster_pools;
pub(crate) use catalog::{install_map_configs, install_monster_pools};
pub use catalog::{map_config, map_configs};
#[path = "monster_world/control.rs"]
mod control;
#[path = "monster_world/hunter_tick.rs"]
mod hunter_tick;
#[path = "monster_world/monster_tick.rs"]
mod monster_tick;
#[path = "monster_world/navigation.rs"]
mod navigation;
#[path = "monster_world/runtime_tick.rs"]
mod runtime_tick;
#[path = "monster_world/skills.rs"]
mod skills;
#[path = "monster_world/targeting.rs"]
mod targeting;
use navigation::*;
#[path = "monster_world/presentation.rs"]
mod presentation;
use presentation::*;
#[path = "monster_world/rewards.rs"]
mod rewards;
use rewards::*;
#[path = "monster_world/spawn.rs"]
mod spawn;
#[allow(unused_imports)]
use spawn::*;
#[cfg(test)]
#[path = "monster_world/tests.rs"]
mod tests;

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::combat_core::runtime::{
    resolve_original_neutral_hunter_attack, resolve_original_neutral_monster_attack,
    OriginalHitPresentation, OriginalHunterAttackInputs, OriginalMonsterAttackInputs,
};
use super::combat_core::status_damage::original_status_calc_level;
use super::hunter_roster::{
    DurableHunterLoot, DurableHunterRosterState, GearEnhancementTaskStatus,
};
use super::original_combat::OriginalDamageMultiplierStream;
use super::original_rewards::original_material_slot_grants;
use super::PendingOperation;

pub const MONSTER_RULESET: &str = "evil-hunter-1.411-catalog-with-temporary-runtime-tuning";

// Native evidence confirms the FSM boundaries but not these numeric timings yet.
const HUNTER_MOVE_PX_PER_TWO_TICKS: i32 = 15;
const HUNTER_MOVE_MAX_PX_PER_TICK: i32 = 8;
const MONSTER_MOVE_PX_PER_TICK: i32 = 5;
const HUNTER_MELEE_ATTACK_RANGE_PX: i32 = 42;
const HUNTER_RANGED_ATTACK_RANGE_PX: i32 = 150;
const MONSTER_ATTACK_RANGE_PX: i32 = 34;
const MONSTER_DETECTION_RANGE_PX: i32 = 150;
// Skill-specific clips are unresolved. Bound the neutral cast presentation to
// the recovered 0.3333-second ordinary Hunter clip instead of freezing AI for
// the longer rebuild basic-attack cadence.
const HUNTER_SKILL_PRESENTATION_TICKS: u16 = 3;
// The original attack-factor writer remains unresolved. Keep cadence isolated
// as rebuild tuning; animation follows authoritative attack events below.
const MONSTER_ATTACK_RECOVERY_TICKS: u16 = 8;
// Keep pickup visible without letting per-item pauses build an unbounded loot
// backlog during continuous combat.
const HUNTER_LOOT_PICKUP_TICKS: u16 = 3;
const MONSTER_RESPAWN_TICKS: u16 = 30;
const HUNTER_RESPAWN_TICKS: u16 = 25;
// FsmMoveEnd confirms waypoint completion, but the native roam pause/radius
// remain unresolved. These two product values are isolated temporary tuning.
// Arrival itself is the first idle frame. A 24-tick countdown makes the next
// patrol step occur 25 fixed intervals (2.5 seconds) after that frame.
const MONSTER_PATROL_IDLE_TICKS: u16 = 24;
const MONSTER_PATROL_RADIUS_PX: i32 = 64;
// No native town-roam waypoint table has been recovered. These anchors and
// cadence are an explicit presentation fixture so idle Hunters do not freeze
// in the town while preserving server-owned positions and obstacle routing.
const TOWN_ROAM_MIN_IDLE_TICKS: u16 = 12;
const TOWN_ROAM_IDLE_VARIANCE_TICKS: u16 = 27;
pub(super) const TOWN_ROAM_BOUNDS: RegionBounds = RegionBounds {
    min_x: 1400,
    max_x: 1850,
    min_y: 560,
    max_y: 740,
};
// The order deliberately alternates rows and distances. Each Hunter walks a
// deterministic per-id permutation rather than marching through the same row.
pub(super) const TOWN_ROAM_ANCHORS: [(i32, i32); 8] = [
    (1410, 618),
    (1600, 690),
    (1505, 618),
    (1695, 690),
    (1410, 690),
    (1600, 618),
    (1505, 690),
    (1695, 618),
];
// Bridge C is the recovered tunnel route used for newly arriving town Hunters.
// The exact original arrival FSM coordinates remain unresolved.
const TOWN_ARRIVAL_OUTSIDE: (i32, i32) = (1356, 800);
const TOWN_ARRIVAL_INSIDE: (i32, i32) = TOWN_ROAM_ANCHORS[0];
// The native Evil-to-Hunter caller multiplies catalog damage by a selected
// runtime factor whose writer is still unresolved. Keep the existing rebuild
// projection isolated here; everything after this boundary uses recovered
// original arithmetic.
const MONSTER_INCOMING_DAMAGE_FIXTURE_DIVISOR: u64 = 250;
const TOWN_RESPAWN_POINT: (i32, i32) = (1627, 700);
// Persisted runtimes from older bridge routes may stop exactly beside a
// recovered waypoint. Treat that checkpoint as reached instead of routing the
// Hunter back to stage zero after a new player assignment.
const ENTRY_CHECKPOINT_TOLERANCE_PX: i32 = 64;

// CalcAttackSpeed is the authoritative delay in seconds. The fixture profile
// stores that value in milli-seconds; keep the 100 ms simulation cadence while
// preserving the recovered floor and Fury's speed multiplier.
fn hunter_attack_recovery_ticks(attack_speed_milli: Option<u32>, skill_speed_milli: u32) -> u16 {
    let base_delay_milli = attack_speed_milli.unwrap_or(1_000).max(250);
    let effective_delay_milli = if skill_speed_milli > 1_000 {
        base_delay_milli
            .saturating_mul(1_000)
            .checked_div(skill_speed_milli)
            .unwrap_or(250)
            .max(250)
    } else {
        base_delay_milli
    };
    u16::try_from(effective_delay_milli.div_ceil(100).max(1)).unwrap_or(u16::MAX)
}

// Exact sign transforms are recovered from the Unity scene. The field extents
// remain isolated temporary tuning, extending outward from each town gate.
#[cfg(test)]
const TOWN_EXCLUSION_BOUNDS: RegionBounds = RegionBounds {
    min_x: 1070,
    max_x: 2190,
    min_y: 280,
    max_y: 1030,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HunterActionState {
    TownIdle,
    EnteringRegion,
    AcquiringTarget,
    Chasing,
    Attacking,
    CollectingLoot,
    Dead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonsterActionState {
    Idle,
    Patrolling,
    Chasing,
    Attacking,
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MonsterWorldState {
    #[serde(alias = "tier_id", alias = "map_id")]
    pub current_map_id: String,
    #[serde(alias = "difficulty")]
    pub world_difficulty: u8,
    pub tick: u64,
    pub fields: Vec<MonsterFieldState>,
    pub hunters: Vec<HunterAgentState>,
    reward_sequence: u64,
    #[serde(default)]
    presentation_sequence: u64,
    #[serde(default)]
    damage_multiplier_stream: OriginalDamageMultiplierStream,
    #[serde(skip)]
    pub combat_presentations: Vec<CombatPresentation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPresentationKind {
    IncomingDamage,
    NormalDamage,
    CriticalDamage,
    Experience,
    Evade,
    Miss,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatPresentation {
    pub sequence: u64,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub kind: CombatPresentationKind,
    pub amount: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MonsterFieldState {
    pub map_id: String,
    pub density_level: u8,
    pub spawn_count: u32,
    pub monsters: Vec<MonsterState>,
    pub drops: Vec<MonsterDrop>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunterAgentState {
    pub hunter_id: u32,
    pub region_id: Option<String>,
    pub x: i32,
    pub y: i32,
    pub facing_left: bool,
    pub action_state: HunterActionState,
    pub animation: String,
    pub target_monster_id: Option<String>,
    pub target_drop_id: Option<String>,
    pub recovery_ticks: u16,
    pub respawn_ticks: Option<u16>,
    #[serde(default)]
    pub attack_sequence: u64,
    #[serde(default)]
    pub loot_sequence: u64,
    #[serde(default)]
    pub loot_item_id: Option<String>,
    #[serde(default)]
    pub loot_quantity: u32,
    #[serde(default)]
    pub active_skill_id: Option<String>,
    #[serde(default)]
    pub skill_buff_ticks: u16,
    #[serde(default)]
    pub skill_attack_percent: i32,
    #[serde(default)]
    pub skill_defense_percent: i32,
    #[serde(default)]
    pub skill_evasion_percent: i32,
    #[serde(default)]
    pub skill_critical_percent: i32,
    #[serde(default)]
    pub skill_attack_speed_milli: u32,
    #[serde(default)]
    pub ice_armor_active: bool,
    #[serde(default)]
    pub entry_stage: u8,
    #[serde(default)]
    pub town_roam_sequence: u32,
    #[serde(default)]
    pub town_roam_idle_ticks: u16,
    #[serde(default)]
    pub trade_sequence: u64,
    #[serde(default)]
    pub trade_gold: u64,
    #[serde(default)]
    pub trade_materials: Vec<TradeMaterialPresentation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeMaterialPresentation {
    pub material_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterState {
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
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub patrol_phase: u16,
    #[serde(default)]
    pub patrol_idle_ticks: u16,
    pub action_state: MonsterActionState,
    pub animation: String,
    pub facing_left: bool,
    pub target_hunter_id: Option<u32>,
    pub recovery_ticks: u16,
    pub respawn_ticks: Option<u16>,
    #[serde(default)]
    pub attack_sequence: u64,
    #[serde(default)]
    pub stun_ticks: u16,
    #[serde(default)]
    pub slow_ticks: u16,
    pub materials: Vec<MonsterMaterialDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterMaterialDefinition {
    pub source_index: u32,
    pub count: u32,
    pub raw_percent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterDrop {
    pub drop_id: String,
    pub monster_entity_id: String,
    pub item_id: String,
    pub quantity: u32,
    pub x: i32,
    pub y: i32,
    pub owner_hunter_id: u32,
    pub gold: u64,
    pub experience: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HunterAttackSource {
    hunter_id: u32,
    calculated_damage: i64,
    calculated_critical_percent: i32,
    hunter_feel: f32,
    hunter_now_feel: f32,
    attack_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonsterMapConfig {
    pub map_id: &'static str,
    pub area: u8,
    pub monster_tier: u8,
    pub map_asset_id: &'static str,
    pub density_counts: [u32; 3],
    pub bounds: RegionBounds,
    pub entry_waypoints: [(i32, i32); 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationObstacle {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl Default for MonsterWorldState {
    fn default() -> Self {
        Self::with_densities(std::iter::empty())
    }
}

impl Default for MonsterFieldState {
    fn default() -> Self {
        Self {
            map_id: map_configs()[0].map_id.to_owned(),
            density_level: 1,
            spawn_count: 0,
            monsters: Vec::new(),
            drops: Vec::new(),
        }
    }
}
