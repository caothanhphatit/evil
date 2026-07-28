use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::combat_core::runtime::{
    resolve_original_neutral_hunter_attack, resolve_original_neutral_monster_attack,
    OriginalHitPresentation, OriginalHunterAttackInputs, OriginalMonsterAttackInputs,
};
use super::hunter_roster::{DurableHunterLoot, DurableHunterRosterState};
use super::original_combat::OriginalDamageMultiplierStream;
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
const HUNTER_ATTACK_RECOVERY_TICKS: u16 = 5;
const MONSTER_ATTACK_RECOVERY_TICKS: u16 = 8;
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
const TOWN_ROAM_CYCLE_TICKS: u64 = 80;
const TOWN_ROAM_MOVE_TICKS: u64 = 56;
const TOWN_ROAM_BOUNDS: RegionBounds = RegionBounds {
    min_x: 1320,
    max_x: 1990,
    min_y: 500,
    max_y: 920,
};
const TOWN_ROAM_ANCHORS: [(i32, i32); 6] = [
    (1450, 610),
    (1580, 540),
    (1780, 570),
    (1900, 700),
    (1760, 840),
    (1480, 820),
];
// The native Evil-to-Hunter caller multiplies catalog damage by a selected
// runtime factor whose writer is still unresolved. Keep the existing rebuild
// projection isolated here; everything after this boundary uses recovered
// original arithmetic.
const MONSTER_INCOMING_DAMAGE_PROJECTION_DIVISOR: u64 = 250;
const TOWN_RESPAWN_POINT: (i32, i32) = (1627, 700);

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
    pub active_skill_id: Option<String>,
    #[serde(default)]
    pub entry_stage: u8,
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
    pub entry_waypoints: [(i32, i32); 2],
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

pub const MAP_CONFIGS: [MonsterMapConfig; 3] = [
    MonsterMapConfig {
        map_id: "map_new01",
        area: 0,
        monster_tier: 1,
        map_asset_id: "/content/releases/visible-world-v1/maps/map_new01.png",
        density_counts: [3, 6, 9],
        bounds: RegionBounds {
            min_x: 320,
            max_x: 1030,
            min_y: 500,
            max_y: 1000,
        },
        // Village_Bridge_C -> sign_01, projected from exact scene transforms.
        entry_waypoints: [(1356, 800), (1233, 786)],
    },
    MonsterMapConfig {
        map_id: "background_08",
        area: 1,
        monster_tier: 2,
        map_asset_id:
            "/content/releases/visible-world-v1/village/background/background_08__1530.png",
        density_counts: [3, 6, 9],
        bounds: RegionBounds {
            min_x: 1080,
            max_x: 1760,
            min_y: 1080,
            max_y: 1430,
        },
        // Village_Bridge_C -> sign_02, projected from exact scene transforms.
        entry_waypoints: [(1356, 800), (1416, 873)],
    },
    MonsterMapConfig {
        map_id: "background_11",
        area: 2,
        monster_tier: 3,
        map_asset_id:
            "/content/releases/visible-world-v1/village/background/background_11__1508.png",
        density_counts: [3, 6, 9],
        bounds: RegionBounds {
            min_x: 2220,
            max_x: 2860,
            min_y: 500,
            max_y: 1030,
        },
        // sign_03 -> Village_Bridge_B follows the town-to-eastern-field geometry.
        entry_waypoints: [(1957, 809), (2043, 724)],
    },
];

#[derive(Deserialize)]
struct OrdinaryMonsterMap {
    regions: Vec<OrdinaryRegion>,
}

#[derive(Deserialize)]
struct OrdinaryRegion {
    area: u8,
    difficulties: Vec<OrdinaryDifficulty>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryDifficulty {
    global_difficulty: u8,
    monster_pool: Vec<OrdinaryMonster>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryMonster {
    source_index: u32,
    hp: u64,
    damage: u64,
    armor: u64,
    experience: u64,
    gold: u64,
    materials: OrdinaryMaterials,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryMaterials {
    indices: Vec<u32>,
    counts: Vec<u32>,
    percent_values: Vec<u32>,
}

pub fn map_config(map_id: &str) -> Option<&'static MonsterMapConfig> {
    MAP_CONFIGS.iter().find(|config| config.map_id == map_id)
}

impl Default for MonsterWorldState {
    fn default() -> Self {
        Self::with_densities(std::iter::empty())
    }
}

impl Default for MonsterFieldState {
    fn default() -> Self {
        Self {
            map_id: MAP_CONFIGS[0].map_id.to_owned(),
            density_level: 1,
            spawn_count: 0,
            monsters: Vec::new(),
            drops: Vec::new(),
        }
    }
}

impl MonsterWorldState {
    /// Starts a server-authoritative Hunter skill presentation. Exact target
    /// requirements, effect formulas and animation bindings remain unresolved,
    /// so activation validates an optional target without inventing outcomes.
    pub fn trigger_hunter_skill(
        &mut self,
        hunter_id: u32,
        target_entity_id: Option<&str>,
        class_family: &str,
        skill_id: &str,
    ) -> Result<(), &'static str> {
        let agent_index = self
            .hunters
            .iter()
            .position(|agent| agent.hunter_id == hunter_id)
            .ok_or("hunter is not in the world")?;
        let target_id = target_entity_id.map(str::to_owned);
        if let Some(target_id) = target_id {
            let region_id = self.hunters[agent_index]
                .region_id
                .as_deref()
                .ok_or("hunter is not assigned to a hunting region")?;
            let Some((target_x, target_y)) = self.monster_position_in_region(region_id, &target_id)
            else {
                return Err("skill target is unavailable");
            };
            let distance = squared_distance(
                self.hunters[agent_index].x,
                self.hunters[agent_index].y,
                target_x,
                target_y,
            );
            if distance > i64::from(hunter_attack_range(class_family)).pow(2) {
                return Err("skill target is out of range");
            }
            let hunter_x = self.hunters[agent_index].x;
            face_toward_x(
                &mut self.hunters[agent_index].facing_left,
                hunter_x,
                target_x,
            );
            self.hunters[agent_index].target_monster_id = Some(target_id);
        }
        // Skill-to-animation/effect/projectile bindings are unresolved. Keep
        // the exact skill identity as an event key and leave presentation on a
        // neutral recovered Hunter clip instead of inventing a mapping.
        set_hunter_presentation(
            &mut self.hunters[agent_index],
            HunterActionState::Attacking,
            "hunter_stay",
        );
        self.hunters[agent_index].active_skill_id = Some(skill_id.to_owned());
        self.hunters[agent_index].recovery_ticks = HUNTER_ATTACK_RECOVERY_TICKS;
        self.hunters[agent_index].attack_sequence =
            self.hunters[agent_index].attack_sequence.wrapping_add(1);
        Ok(())
    }

    pub fn with_densities<'a>(densities: impl IntoIterator<Item = (&'a str, u8)>) -> Self {
        let configured = densities.into_iter().collect::<Vec<_>>();
        let world_difficulty = 0;
        let fields = MAP_CONFIGS
            .iter()
            .map(|config| {
                let density = configured
                    .iter()
                    .find_map(|(map_id, level)| (*map_id == config.map_id).then_some(*level))
                    .filter(|level| (1..=3).contains(level))
                    .unwrap_or(1);
                MonsterFieldState::spawned(config, density, world_difficulty)
            })
            .collect();
        Self {
            current_map_id: MAP_CONFIGS[0].map_id.to_owned(),
            world_difficulty,
            tick: 0,
            fields,
            hunters: Vec::new(),
            reward_sequence: 0,
            presentation_sequence: 0,
            damage_multiplier_stream: OriginalDamageMultiplierStream::default(),
            combat_presentations: Vec::new(),
        }
    }

    pub fn enter_map(&mut self, map_id: &str) -> Result<(), &'static str> {
        map_config(map_id).ok_or("monster map unavailable")?;
        self.current_map_id = map_id.to_owned();
        Ok(())
    }

    pub fn current_field(&self) -> &MonsterFieldState {
        self.fields
            .iter()
            .find(|field| field.map_id == self.current_map_id)
            .unwrap_or(&self.fields[0])
    }

    pub fn current_field_mut(&mut self) -> &mut MonsterFieldState {
        let map_id = self.current_map_id.clone();
        let index = self
            .fields
            .iter()
            .position(|field| field.map_id == map_id)
            .unwrap_or(0);
        &mut self.fields[index]
    }

    pub fn set_density(&mut self, level: u8) -> Result<(), &'static str> {
        let region_id = self.current_map_id.clone();
        self.set_region_density(&region_id, level)
    }

    pub fn set_region_density(&mut self, region_id: &str, level: u8) -> Result<(), &'static str> {
        if !(1..=3).contains(&level) {
            return Err("monster density unavailable");
        }
        let world_difficulty = self.world_difficulty;
        let field = self
            .fields
            .iter_mut()
            .find(|field| field.map_id == region_id)
            .ok_or("monster region unavailable")?;
        let config = map_config(region_id).ok_or("monster map unavailable")?;
        field.density_level = level;
        field.spawn_count = config.density_counts[usize::from(level - 1)];
        field.reconcile_spawn_count(config, world_difficulty);
        Ok(())
    }

    pub fn select_target(&mut self, monster_id: &str, hunter_id: u32) -> Result<(), &'static str> {
        let monster = self
            .fields
            .iter_mut()
            .flat_map(|field| field.monsters.iter_mut())
            .find(|monster| monster.entity_id == monster_id)
            .ok_or("monster unavailable")?;
        if monster.hp == 0 {
            return Err("monster is dead");
        }
        monster.target_hunter_id = Some(hunter_id);
        Ok(())
    }

    pub fn tick(&mut self, roster: &mut DurableHunterRosterState) -> Vec<PendingOperation> {
        self.tick_with_obstacles(roster, &[])
    }

    pub fn tick_with_obstacles(
        &mut self,
        roster: &mut DurableHunterRosterState,
        obstacles: &[NavigationObstacle],
    ) -> Vec<PendingOperation> {
        self.tick = self.tick.saturating_add(1);
        self.combat_presentations.clear();
        self.reconcile_hunters(roster);
        self.tick_monsters(roster);
        self.tick_hunters(roster, obstacles)
    }

    fn reconcile_hunters(&mut self, roster: &DurableHunterRosterState) {
        self.hunters.retain(|agent| {
            roster
                .hunters
                .iter()
                .any(|hunter| hunter.hunter_id == agent.hunter_id)
        });
        for (slot, hunter) in roster.hunters.iter().enumerate() {
            if self
                .hunters
                .iter()
                .any(|agent| agent.hunter_id == hunter.hunter_id)
            {
                continue;
            }
            self.hunters.push(HunterAgentState {
                hunter_id: hunter.hunter_id,
                region_id: hunter
                    .hunt
                    .zone_id
                    .clone()
                    .filter(|id| map_config(id).is_some()),
                x: TOWN_RESPAWN_POINT.0 + i32::try_from(slot % 4).unwrap_or(0) * 24,
                y: TOWN_RESPAWN_POINT.1 + i32::try_from(slot / 4).unwrap_or(0) * 28,
                facing_left: false,
                action_state: if hunter.current_hp == 0 {
                    HunterActionState::Dead
                } else {
                    HunterActionState::TownIdle
                },
                animation: if hunter.current_hp == 0 {
                    "hunter_die".to_owned()
                } else {
                    "hunter_stay".to_owned()
                },
                target_monster_id: None,
                target_drop_id: None,
                recovery_ticks: 0,
                respawn_ticks: (hunter.current_hp == 0).then_some(HUNTER_RESPAWN_TICKS),
                attack_sequence: 0,
                active_skill_id: None,
                entry_stage: 0,
            });
        }
        for agent in &mut self.hunters {
            if let Some(hunter) = roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == agent.hunter_id)
            {
                let assigned = hunter
                    .hunt
                    .zone_id
                    .clone()
                    .filter(|id| map_config(id).is_some());
                if agent.region_id != assigned {
                    agent.region_id = assigned;
                    agent.target_monster_id = None;
                    agent.target_drop_id = None;
                    agent.action_state = if agent.region_id.is_some() {
                        HunterActionState::EnteringRegion
                    } else {
                        HunterActionState::TownIdle
                    };
                    agent.entry_stage = 0;
                }
            }
        }
    }

    fn tick_monsters(&mut self, roster: &mut DurableHunterRosterState) {
        let tick = self.tick;
        let presentations = &mut self.combat_presentations;
        let presentation_sequence = &mut self.presentation_sequence;
        let damage_multiplier_stream = &mut self.damage_multiplier_stream;
        for field in &mut self.fields {
            let Some(config) = map_config(&field.map_id) else {
                continue;
            };
            for monster in &mut field.monsters {
                if let Some(respawn) = monster.respawn_ticks.as_mut() {
                    *respawn = respawn.saturating_sub(1);
                    if *respawn == 0 {
                        monster.hp = monster.max_hp;
                        monster.x = monster.spawn_x;
                        monster.y = monster.spawn_y;
                        monster.action_state = MonsterActionState::Idle;
                        monster.animation = "stay".to_owned();
                        monster.target_hunter_id = None;
                        monster.patrol_idle_ticks = MONSTER_PATROL_IDLE_TICKS;
                        monster.respawn_ticks = None;
                    }
                    continue;
                }
                monster.recovery_ticks = monster.recovery_ticks.saturating_sub(1);
                let target = valid_hunter_target(
                    &self.hunters,
                    roster,
                    &field.map_id,
                    monster.target_hunter_id,
                )
                .or_else(|| {
                    nearest_hunter(
                        &self.hunters,
                        roster,
                        &field.map_id,
                        monster.x,
                        monster.y,
                        MONSTER_DETECTION_RANGE_PX,
                    )
                });
                monster.target_hunter_id = target.map(|target| target.hunter_id);
                let Some(target) = target else {
                    patrol(monster, config.bounds);
                    continue;
                };
                let distance = squared_distance(monster.x, monster.y, target.x, target.y);
                if distance > i64::from(MONSTER_ATTACK_RANGE_PX).pow(2) {
                    monster.action_state = MonsterActionState::Chasing;
                    monster.animation = monster_directional_animation("walk", monster.y, target.y);
                    let chase_target = config.bounds.closest_point(target.x, target.y, 24);
                    move_toward(
                        &mut monster.x,
                        &mut monster.y,
                        chase_target.0,
                        chase_target.1,
                        MONSTER_MOVE_PX_PER_TICK,
                        &mut monster.facing_left,
                    );
                    continue;
                }
                face_toward_x(&mut monster.facing_left, monster.x, target.x);
                monster.action_state = MonsterActionState::Attacking;
                monster.animation = monster_directional_animation("atk", monster.y, target.y);
                if monster.recovery_ticks > 0 {
                    continue;
                }
                monster.recovery_ticks = MONSTER_ATTACK_RECOVERY_TICKS;
                if let Ok(hunter) = roster.active_mut(target.hunter_id) {
                    let projected_incoming_damage =
                        (monster.damage / MONSTER_INCOMING_DAMAGE_PROJECTION_DIVISOR).max(1);
                    let Some(incoming_damage) = i64::try_from(projected_incoming_damage).ok()
                    else {
                        continue;
                    };
                    let Some(hunter_hp) = i64::try_from(hunter.current_hp).ok() else {
                        continue;
                    };
                    let runtime_status = hunter.runtime.status.as_ref();
                    let hunter_armor = runtime_status
                        .map(|status| status.armor)
                        .or_else(|| i64::try_from(hunter.profile.defense).ok())
                        .unwrap_or(0);
                    let hunter_feel = runtime_status
                        .map(|status| status.feel)
                        .unwrap_or(hunter.mood.maximum as f32);
                    let hunter_now_feel = runtime_status
                        .map(|status| status.now_feel)
                        .unwrap_or(hunter.mood.current as f32);
                    let multiplier =
                        f32::from(damage_multiplier_stream.next_hundredths()) * 0.01_f32;
                    let dodge_roll = deterministic_combat_percent_roll(
                        tick,
                        target.hunter_id,
                        tick,
                        monster.source_index,
                    );
                    let pet_dodge_roll = i32::try_from(
                        (deterministic_roll(
                            tick,
                            tick,
                            monster.source_index,
                            u64::from(target.hunter_id).wrapping_add(1),
                        ) - 1)
                            % 1000,
                    )
                    .unwrap_or(0);
                    let Ok(result) =
                        resolve_original_neutral_monster_attack(OriginalMonsterAttackInputs {
                            incoming_damage,
                            rand_damage_multiplier: multiplier,
                            // No live effect-54 producer is modeled yet. Zero is
                            // the exact disabled state, not a synthesized miss roll.
                            attacker_effect_54_value: 0,
                            effect_54_roll_zero_to_ninety_nine: 0,
                            hunter_armor,
                            hunter_feel,
                            hunter_now_feel,
                            hunter_shield: 0,
                            hunter_hp,
                            hunter_calc_dodge: hunter.profile.calc_dodge(),
                            hunter_dodge_primary_roll_zero_to_ninety_nine: dodge_roll,
                            // Riding-pet dodge is still unresolved per Hunter.
                            hunter_riding_pet_dodge: 0,
                            hunter_riding_pet_roll_zero_to_nine_ninety_nine: pet_dodge_roll,
                        })
                    else {
                        continue;
                    };
                    hunter.current_hp = u64::try_from(result.hunter_hp).unwrap_or(0);
                    let (kind, amount) = match result.presentation {
                        OriginalHitPresentation::Normal => (
                            CombatPresentationKind::IncomingDamage,
                            u64::try_from(result.final_damage).ok(),
                        ),
                        OriginalHitPresentation::Miss => (CombatPresentationKind::Miss, None),
                        OriginalHitPresentation::Evade => (CombatPresentationKind::Evade, None),
                        OriginalHitPresentation::Critical => continue,
                    };
                    push_combat_presentation(
                        presentations,
                        presentation_sequence,
                        monster.entity_id.clone(),
                        village_hunter_entity_id(hunter.hunter_id),
                        kind,
                        amount,
                    );
                    if hunter.current_hp == 0 {
                        hunter.hunt.status = "dead".to_owned();
                        hunter.profile.action_state = "dead".to_owned();
                        hunter.profile.animation_name = "hunter_die".to_owned();
                        if let Some(agent) = self
                            .hunters
                            .iter_mut()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)
                        {
                            agent.action_state = HunterActionState::Dead;
                            agent.animation = "hunter_die".to_owned();
                            agent.respawn_ticks = Some(HUNTER_RESPAWN_TICKS);
                            agent.target_monster_id = None;
                        }
                    }
                }
            }
        }
    }

    fn tick_hunters(
        &mut self,
        roster: &mut DurableHunterRosterState,
        obstacles: &[NavigationObstacle],
    ) -> Vec<PendingOperation> {
        let mut operations = Vec::new();
        for agent_index in 0..self.hunters.len() {
            let hunter_id = self.hunters[agent_index].hunter_id;
            let move_step = hunter_move_step(self.tick);
            if self.tick_dead_hunter(agent_index, roster) {
                continue;
            }
            let Some(region_id) = self.hunters[agent_index].region_id.clone() else {
                let hunter_id = self.hunters[agent_index].hunter_id;
                let cycle_tick = self
                    .tick
                    .wrapping_add(u64::from(hunter_id).wrapping_mul(13))
                    % TOWN_ROAM_CYCLE_TICKS;
                if cycle_tick >= TOWN_ROAM_MOVE_TICKS {
                    set_hunter_presentation(
                        &mut self.hunters[agent_index],
                        HunterActionState::TownIdle,
                        "hunter_stay",
                    );
                    continue;
                }
                let anchor_index = usize::try_from(
                    (self.tick / TOWN_ROAM_CYCLE_TICKS).wrapping_add(u64::from(hunter_id)),
                )
                .unwrap_or(0)
                    % TOWN_ROAM_ANCHORS.len();
                let (target_x, target_y) = TOWN_ROAM_ANCHORS[anchor_index];
                let agent = &mut self.hunters[agent_index];
                if squared_distance(agent.x, agent.y, target_x, target_y)
                    <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                {
                    set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_stay");
                    continue;
                }
                set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_walk");
                move_toward_avoiding(
                    &mut agent.x,
                    &mut agent.y,
                    target_x,
                    target_y,
                    move_step,
                    &mut agent.facing_left,
                    obstacles,
                );
                agent.x = agent
                    .x
                    .clamp(TOWN_ROAM_BOUNDS.min_x, TOWN_ROAM_BOUNDS.max_x);
                agent.y = agent
                    .y
                    .clamp(TOWN_ROAM_BOUNDS.min_y, TOWN_ROAM_BOUNDS.max_y);
                continue;
            };
            let Some(config) = map_config(&region_id) else {
                continue;
            };
            if !config
                .bounds
                .contains(self.hunters[agent_index].x, self.hunters[agent_index].y)
            {
                let agent = &mut self.hunters[agent_index];
                let (target_x, target_y) = if let Some(waypoint) =
                    config.entry_waypoints.get(usize::from(agent.entry_stage))
                {
                    *waypoint
                } else {
                    let final_approach = config.entry_waypoints[1];
                    config
                        .bounds
                        .closest_point(final_approach.0, final_approach.1, 48)
                };
                set_hunter_presentation(agent, HunterActionState::EnteringRegion, "hunter_walk");
                move_toward_avoiding(
                    &mut agent.x,
                    &mut agent.y,
                    target_x,
                    target_y,
                    move_step,
                    &mut agent.facing_left,
                    obstacles,
                );
                if squared_distance(agent.x, agent.y, target_x, target_y)
                    <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                {
                    agent.entry_stage = agent.entry_stage.saturating_add(1);
                }
                continue;
            }
            self.hunters[agent_index].recovery_ticks =
                self.hunters[agent_index].recovery_ticks.saturating_sub(1);
            if self.hunters[agent_index].active_skill_id.is_some() {
                if self.hunters[agent_index].recovery_ticks > 0 {
                    continue;
                }
                self.hunters[agent_index].active_skill_id = None;
            }
            if self.try_collect_drop(agent_index, roster, &mut operations) {
                continue;
            }
            let target_id = self
                .valid_monster_target(agent_index, &region_id)
                .or_else(|| self.nearest_monster_id(agent_index, &region_id));
            self.hunters[agent_index].target_monster_id = target_id.clone();
            let Some(target_id) = target_id else {
                set_hunter_presentation(
                    &mut self.hunters[agent_index],
                    HunterActionState::AcquiringTarget,
                    "hunter_stay",
                );
                continue;
            };
            let Some((target_x, target_y)) = self.monster_position(&target_id) else {
                continue;
            };
            let class_family = roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == hunter_id)
                .map(|hunter| hunter.profile.visual_family.as_str())
                .unwrap_or("H1");
            let attack_range = hunter_attack_range(class_family);
            let distance = squared_distance(
                self.hunters[agent_index].x,
                self.hunters[agent_index].y,
                target_x,
                target_y,
            );
            if distance > i64::from(attack_range).pow(2) {
                let agent = &mut self.hunters[agent_index];
                set_hunter_presentation(agent, HunterActionState::Chasing, "hunter_walk");
                move_toward_avoiding(
                    &mut agent.x,
                    &mut agent.y,
                    target_x,
                    target_y,
                    move_step,
                    &mut agent.facing_left,
                    obstacles,
                );
                continue;
            }
            let hunter_x = self.hunters[agent_index].x;
            face_toward_x(
                &mut self.hunters[agent_index].facing_left,
                hunter_x,
                target_x,
            );
            let attack_animation = format!("{}_hit", class_family.to_ascii_lowercase());
            set_hunter_presentation(
                &mut self.hunters[agent_index],
                HunterActionState::Attacking,
                &attack_animation,
            );
            if self.hunters[agent_index].recovery_ticks > 0 {
                continue;
            }
            self.hunters[agent_index].recovery_ticks = HUNTER_ATTACK_RECOVERY_TICKS;
            self.hunters[agent_index].attack_sequence =
                self.hunters[agent_index].attack_sequence.wrapping_add(1);
            let Some(hunter) = roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == hunter_id)
            else {
                continue;
            };
            let Some(calculated_damage) = i64::try_from(hunter.profile.attack).ok() else {
                continue;
            };
            let calculated_critical_percent = hunter
                .runtime
                .status
                .as_ref()
                .map(|status| status.critical)
                .or_else(|| {
                    hunter
                        .profile
                        .critical_rate_bps
                        .and_then(|value| i32::try_from(value / 100).ok())
                })
                .unwrap_or(0);
            let hunter_feel = hunter
                .runtime
                .status
                .as_ref()
                .map(|status| status.feel)
                .unwrap_or(hunter.mood.maximum as f32);
            let hunter_now_feel = hunter
                .runtime
                .status
                .as_ref()
                .map(|status| status.now_feel)
                .unwrap_or(hunter.mood.current as f32);
            let attack_sequence = self.hunters[agent_index].attack_sequence;
            self.resolve_hunter_attack(
                &target_id,
                HunterAttackSource {
                    hunter_id,
                    calculated_damage,
                    calculated_critical_percent,
                    hunter_feel,
                    hunter_now_feel,
                    attack_sequence,
                },
            );
        }
        operations
    }

    fn tick_dead_hunter(
        &mut self,
        agent_index: usize,
        roster: &mut DurableHunterRosterState,
    ) -> bool {
        let Some(respawn) = self.hunters[agent_index].respawn_ticks.as_mut() else {
            return false;
        };
        *respawn = respawn.saturating_sub(1);
        if *respawn > 0 {
            return true;
        }
        let hunter_id = self.hunters[agent_index].hunter_id;
        if let Ok(hunter) = roster.active_mut(hunter_id) {
            hunter.current_hp = hunter.max_hp;
            hunter.hunt.status = if hunter.hunt.zone_id.is_some() {
                "hunting".to_owned()
            } else {
                "idle".to_owned()
            };
            hunter.profile.action_state = hunter.hunt.status.clone();
            hunter.profile.animation_name = "hunter_walk".to_owned();
        }
        let agent = &mut self.hunters[agent_index];
        agent.x = TOWN_RESPAWN_POINT.0;
        agent.y = TOWN_RESPAWN_POINT.1;
        agent.action_state = if agent.region_id.is_some() {
            HunterActionState::EnteringRegion
        } else {
            HunterActionState::TownIdle
        };
        agent.animation = if agent.region_id.is_some() {
            "hunter_walk".to_owned()
        } else {
            "hunter_stay".to_owned()
        };
        agent.respawn_ticks = None;
        agent.entry_stage = 0;
        true
    }

    fn valid_monster_target(&self, agent_index: usize, region_id: &str) -> Option<String> {
        let target_id = self.hunters[agent_index].target_monster_id.as_ref()?;
        self.fields
            .iter()
            .find(|field| field.map_id == region_id)?
            .monsters
            .iter()
            .find(|monster| monster.entity_id == *target_id && monster.hp > 0)
            .map(|monster| monster.entity_id.clone())
    }

    fn nearest_monster_id(&self, agent_index: usize, region_id: &str) -> Option<String> {
        let agent = &self.hunters[agent_index];
        self.fields
            .iter()
            .find(|field| field.map_id == region_id)?
            .monsters
            .iter()
            .filter(|monster| monster.hp > 0)
            .min_by_key(|monster| squared_distance(agent.x, agent.y, monster.x, monster.y))
            .map(|monster| monster.entity_id.clone())
    }

    fn monster_position(&self, target_id: &str) -> Option<(i32, i32)> {
        self.fields
            .iter()
            .flat_map(|field| &field.monsters)
            .find(|monster| monster.entity_id == target_id && monster.hp > 0)
            .map(|monster| (monster.x, monster.y))
    }

    fn monster_position_in_region(&self, region_id: &str, target_id: &str) -> Option<(i32, i32)> {
        self.fields
            .iter()
            .find(|field| field.map_id == region_id)?
            .monsters
            .iter()
            .find(|monster| monster.entity_id == target_id && monster.hp > 0)
            .map(|monster| (monster.x, monster.y))
    }

    fn resolve_hunter_attack(&mut self, target_id: &str, source: HunterAttackSource) {
        let multiplier = f32::from(self.damage_multiplier_stream.next_hundredths()) * 0.01_f32;
        let Some(monster) = self
            .fields
            .iter()
            .flat_map(|field| &field.monsters)
            .find(|monster| monster.entity_id == target_id)
        else {
            return;
        };
        let Some(target_armor) = i64::try_from(monster.armor).ok() else {
            return;
        };
        let Some(target_hp) = i64::try_from(monster.hp).ok() else {
            return;
        };
        let critical_roll = deterministic_combat_percent_roll(
            self.tick,
            source.hunter_id,
            source.attack_sequence,
            monster.source_index,
        );
        let Ok(result) = resolve_original_neutral_hunter_attack(OriginalHunterAttackInputs {
            calculated_damage: source.calculated_damage,
            calculated_critical_percent: source.calculated_critical_percent,
            critical_roll_zero_to_ninety_nine: critical_roll,
            conditional_critical_bonus_enabled: false,
            conditional_critical_bonus_percent: 0,
            target_armor,
            target_hp,
            hunter_feel: source.hunter_feel,
            hunter_now_feel: source.hunter_now_feel,
            rand_damage_multiplier: multiplier,
        }) else {
            return;
        };
        let Some(damage) = u64::try_from(result.final_damage).ok() else {
            return;
        };
        let kind = match result.presentation {
            OriginalHitPresentation::Normal => CombatPresentationKind::NormalDamage,
            OriginalHitPresentation::Critical => CombatPresentationKind::CriticalDamage,
            OriginalHitPresentation::Miss | OriginalHitPresentation::Evade => return,
        };
        self.apply_damage_to_monster(target_id, source.hunter_id, damage, kind);
    }

    fn apply_damage_to_monster(
        &mut self,
        target_id: &str,
        hunter_id: u32,
        damage: u64,
        presentation_kind: CombatPresentationKind,
    ) {
        let Some(field) = self.fields.iter_mut().find(|field| {
            field
                .monsters
                .iter()
                .any(|monster| monster.entity_id == target_id)
        }) else {
            return;
        };
        let Some(monster) = field
            .monsters
            .iter_mut()
            .find(|monster| monster.entity_id == target_id)
        else {
            return;
        };
        monster.hp = monster.hp.saturating_sub(damage);
        monster.target_hunter_id = Some(hunter_id);
        push_combat_presentation(
            &mut self.combat_presentations,
            &mut self.presentation_sequence,
            village_hunter_entity_id(hunter_id),
            monster.entity_id.clone(),
            presentation_kind,
            Some(damage),
        );
        if monster.hp > 0 {
            return;
        }
        monster.action_state = MonsterActionState::Dead;
        monster.animation = "die".to_owned();
        monster.respawn_ticks = Some(MONSTER_RESPAWN_TICKS);
        monster.target_hunter_id = None;
        self.reward_sequence = self.reward_sequence.saturating_add(1);
        let drop_id = format!("drop-{}-{}", monster.entity_id, self.reward_sequence);
        let mut material_drops = monster
            .materials
            .iter()
            .enumerate()
            .filter_map(|(slot, material)| {
                let roll = deterministic_roll(
                    self.tick,
                    self.reward_sequence,
                    monster.source_index,
                    slot as u64,
                );
                (material.raw_percent.saturating_mul(10) >= roll)
                    .then_some((material.source_index, material.count))
            })
            .collect::<Vec<_>>();
        if material_drops.is_empty() {
            field.drops.push(MonsterDrop {
                drop_id,
                monster_entity_id: monster.entity_id.clone(),
                item_id: "gold".to_owned(),
                quantity: 0,
                x: monster.x,
                y: monster.y,
                owner_hunter_id: hunter_id,
                gold: monster.gold,
                experience: monster.experience,
            });
        } else {
            for (index, (item_index, quantity)) in material_drops.drain(..).enumerate() {
                field.drops.push(MonsterDrop {
                    drop_id: format!("{drop_id}-{index}"),
                    monster_entity_id: monster.entity_id.clone(),
                    item_id: format!("material:{item_index}"),
                    quantity,
                    x: monster.x + i32::try_from(index).unwrap_or(0) * 8,
                    y: monster.y,
                    owner_hunter_id: hunter_id,
                    gold: if index == 0 { monster.gold } else { 0 },
                    experience: if index == 0 { monster.experience } else { 0 },
                });
            }
        }
    }

    fn try_collect_drop(
        &mut self,
        agent_index: usize,
        roster: &mut DurableHunterRosterState,
        operations: &mut Vec<PendingOperation>,
    ) -> bool {
        let hunter_id = self.hunters[agent_index].hunter_id;
        let Some(region_id) = self.hunters[agent_index].region_id.clone() else {
            return false;
        };
        let Some(field_index) = self
            .fields
            .iter()
            .position(|field| field.map_id == region_id)
        else {
            return false;
        };
        let candidate = self.fields[field_index]
            .drops
            .iter()
            .enumerate()
            .filter(|(_, drop)| drop.owner_hunter_id == hunter_id)
            .min_by_key(|(_, drop)| {
                squared_distance(
                    self.hunters[agent_index].x,
                    self.hunters[agent_index].y,
                    drop.x,
                    drop.y,
                )
            })
            .map(|(index, drop)| (index, drop.clone()));
        let Some((drop_index, drop)) = candidate else {
            return false;
        };
        if squared_distance(
            self.hunters[agent_index].x,
            self.hunters[agent_index].y,
            drop.x,
            drop.y,
        ) > i64::from(HUNTER_MELEE_ATTACK_RANGE_PX).pow(2)
        {
            let agent = &mut self.hunters[agent_index];
            agent.target_drop_id = Some(drop.drop_id.clone());
            set_hunter_presentation(agent, HunterActionState::CollectingLoot, "hunter_walk");
            move_toward_avoiding(
                &mut agent.x,
                &mut agent.y,
                drop.x,
                drop.y,
                hunter_move_step(self.tick),
                &mut agent.facing_left,
                &[],
            );
            return true;
        }
        self.fields[field_index].drops.remove(drop_index);
        let Ok(hunter) = roster.active_mut(hunter_id) else {
            return true;
        };
        hunter.gold = hunter.gold.saturating_add(drop.gold);
        add_experience(hunter, drop.experience);
        if drop.quantity > 0 {
            if let Some(existing) = hunter
                .hunt
                .loot
                .iter_mut()
                .find(|loot| loot.item_id == drop.item_id)
            {
                existing.quantity = existing.quantity.saturating_add(drop.quantity);
            } else {
                hunter.hunt.loot.push(DurableHunterLoot {
                    item_id: drop.item_id.clone(),
                    quantity: drop.quantity,
                });
            }
        }
        let operation_id = reward_operation_id(self.tick, hunter_id, &drop.drop_id);
        let item_id = drop
            .item_id
            .strip_prefix("material:")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|item_id| *item_id > 0);
        if let Some(item_id) = item_id.filter(|_| drop.quantity > 0) {
            operations.push(PendingOperation::Reward {
                operation_id,
                gold: drop.gold,
                item_id,
                quantity: drop.quantity,
            });
        }
        self.hunters[agent_index].target_drop_id = None;
        true
    }
}

fn village_hunter_entity_id(hunter_id: u32) -> String {
    format!("village-hunter-{hunter_id}")
}

fn push_combat_presentation(
    presentations: &mut Vec<CombatPresentation>,
    sequence: &mut u64,
    source_entity_id: String,
    target_entity_id: String,
    kind: CombatPresentationKind,
    amount: Option<u64>,
) {
    *sequence = sequence.wrapping_add(1);
    presentations.push(CombatPresentation {
        sequence: *sequence,
        source_entity_id,
        target_entity_id,
        kind,
        amount,
    });
}

impl MonsterFieldState {
    fn spawned(config: &MonsterMapConfig, density_level: u8, world_difficulty: u8) -> Self {
        let spawn_count = config.density_counts[usize::from(density_level - 1)];
        let mut field = Self {
            map_id: config.map_id.to_owned(),
            density_level,
            spawn_count,
            monsters: Vec::new(),
            drops: Vec::new(),
        };
        field.reconcile_spawn_count(config, world_difficulty);
        field
    }

    fn reconcile_spawn_count(&mut self, config: &MonsterMapConfig, world_difficulty: u8) {
        let target = usize::try_from(self.spawn_count).unwrap_or(0);
        if self.monsters.len() > target {
            self.monsters.truncate(target);
        }
        for (index, monster) in self.monsters.iter_mut().enumerate() {
            if !config.bounds.contains(monster.spawn_x, monster.spawn_y) {
                let (x, y) = spawn_point(config.bounds, index);
                monster.x = x;
                monster.y = y;
                monster.spawn_x = x;
                monster.spawn_y = y;
                monster.target_hunter_id = None;
                monster.action_state = MonsterActionState::Idle;
                monster.animation = "stay".to_owned();
                monster.patrol_idle_ticks = 0;
            }
        }
        while self.monsters.len() < target {
            let index = self.monsters.len();
            self.monsters
                .push(spawn_monster(config, world_difficulty, index));
        }
    }
}

impl RegionBounds {
    fn contains(self, x: i32, y: i32) -> bool {
        (self.min_x..=self.max_x).contains(&x) && (self.min_y..=self.max_y).contains(&y)
    }

    fn closest_point(self, x: i32, y: i32, inset: i32) -> (i32, i32) {
        (
            x.clamp(self.min_x + inset, self.max_x - inset),
            y.clamp(self.min_y + inset, self.max_y - inset),
        )
    }

    #[cfg(test)]
    fn intersects(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

fn spawn_monster(config: &MonsterMapConfig, world_difficulty: u8, index: usize) -> MonsterState {
    static DEFINITIONS: OnceLock<OrdinaryMonsterMap> = OnceLock::new();
    let definitions = DEFINITIONS.get_or_init(|| {
        match serde_json::from_str(include_str!("../../../../packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json")) {
            Ok(definitions) => definitions,
            Err(error) => panic!("validated ordinary monster mapping cannot be decoded: {error}"),
        }
    });
    let definition = definitions
        .regions
        .iter()
        .find(|region| region.area == config.area)
        .and_then(|region| {
            region
                .difficulties
                .iter()
                .find(|difficulty| difficulty.global_difficulty == world_difficulty)
        })
        .and_then(|difficulty| {
            difficulty
                .monster_pool
                .get(index % difficulty.monster_pool.len().max(1))
        });
    let Some(monster) = definition else {
        panic!(
            "validated ordinary monster pool is missing for area {} difficulty {}",
            config.area, world_difficulty
        );
    };
    let materials = monster
        .materials
        .indices
        .iter()
        .enumerate()
        .map(|(slot, source_index)| {
            let Some(count) = monster.materials.counts.get(slot) else {
                panic!("validated monster material count is missing at slot {slot}");
            };
            let Some(raw_percent) = monster.materials.percent_values.get(slot) else {
                panic!("validated monster material percentage is missing at slot {slot}");
            };
            MonsterMaterialDefinition {
                source_index: *source_index,
                count: *count,
                raw_percent: *raw_percent,
            }
        })
        .collect();
    let (source_index, hp, damage, armor, experience, gold) = (
        monster.source_index,
        monster.hp,
        monster.damage,
        monster.armor,
        monster.experience,
        monster.gold,
    );
    let (x, y) = spawn_point(config.bounds, index);
    MonsterState {
        entity_id: format!("monster-{}-{index}", config.map_id),
        monster_id: format!("monster:{source_index}"),
        source_index,
        asset_bundle_id: "mon_a_01_1".to_owned(),
        hp,
        max_hp: hp,
        damage,
        armor,
        experience,
        gold,
        x,
        y,
        spawn_x: x,
        spawn_y: y,
        patrol_phase: u16::try_from(index * 13).unwrap_or(0),
        patrol_idle_ticks: 0,
        action_state: MonsterActionState::Idle,
        animation: "stay".to_owned(),
        facing_left: index % 2 == 0,
        target_hunter_id: None,
        recovery_ticks: 0,
        respawn_ticks: None,
        materials,
    }
}

fn spawn_point(bounds: RegionBounds, index: usize) -> (i32, i32) {
    let columns = 3_i32;
    let column = i32::try_from(index).unwrap_or(0) % columns;
    let row = i32::try_from(index).unwrap_or(0) / columns;
    let horizontal_step = (bounds.max_x - bounds.min_x - 120) / 3;
    let vertical_step = (bounds.max_y - bounds.min_y - 120) / 3;
    (
        bounds.min_x + 60 + column * horizontal_step,
        bounds.min_y + 60 + row * vertical_step,
    )
}

fn valid_hunter_target<'a>(
    agents: &'a [HunterAgentState],
    roster: &DurableHunterRosterState,
    region_id: &str,
    hunter_id: Option<u32>,
) -> Option<&'a HunterAgentState> {
    let hunter_id = hunter_id?;
    let alive = roster
        .hunters
        .iter()
        .any(|hunter| hunter.hunter_id == hunter_id && hunter.current_hp > 0);
    alive
        .then(|| {
            agents.iter().find(|agent| {
                agent.hunter_id == hunter_id && agent.region_id.as_deref() == Some(region_id)
            })
        })
        .flatten()
}

fn nearest_hunter<'a>(
    agents: &'a [HunterAgentState],
    roster: &DurableHunterRosterState,
    region_id: &str,
    x: i32,
    y: i32,
    range: i32,
) -> Option<&'a HunterAgentState> {
    agents
        .iter()
        .filter(|agent| {
            agent.region_id.as_deref() == Some(region_id)
                && roster
                    .hunters
                    .iter()
                    .any(|hunter| hunter.hunter_id == agent.hunter_id && hunter.current_hp > 0)
        })
        .filter(|agent| squared_distance(x, y, agent.x, agent.y) <= i64::from(range).pow(2))
        .min_by_key(|agent| squared_distance(x, y, agent.x, agent.y))
}

fn patrol(monster: &mut MonsterState, bounds: RegionBounds) {
    if monster.action_state == MonsterActionState::Idle && monster.patrol_idle_ticks > 0 {
        monster.patrol_idle_ticks = monster.patrol_idle_ticks.saturating_sub(1);
        monster.animation = "stay".to_owned();
        return;
    }

    let waypoint = patrol_waypoint(monster, bounds);
    monster.action_state = MonsterActionState::Patrolling;
    monster.animation = monster_directional_animation("walk", monster.y, waypoint.1);
    move_toward(
        &mut monster.x,
        &mut monster.y,
        waypoint.0,
        waypoint.1,
        MONSTER_MOVE_PX_PER_TICK,
        &mut monster.facing_left,
    );
    if monster.x == waypoint.0 && monster.y == waypoint.1 {
        monster.patrol_phase = monster.patrol_phase.wrapping_add(1);
        monster.patrol_idle_ticks = MONSTER_PATROL_IDLE_TICKS;
        monster.action_state = MonsterActionState::Idle;
        monster.animation = "stay".to_owned();
    }
}

fn patrol_waypoint(monster: &MonsterState, bounds: RegionBounds) -> (i32, i32) {
    const OFFSETS: [(i32, i32); 8] = [
        (MONSTER_PATROL_RADIUS_PX, 0),
        (45, 45),
        (0, MONSTER_PATROL_RADIUS_PX),
        (-45, 45),
        (-MONSTER_PATROL_RADIUS_PX, 0),
        (-45, -45),
        (0, -MONSTER_PATROL_RADIUS_PX),
        (45, -45),
    ];
    let offset = OFFSETS[usize::from(monster.patrol_phase) % OFFSETS.len()];
    bounds.closest_point(
        monster.spawn_x.saturating_add(offset.0),
        monster.spawn_y.saturating_add(offset.1),
        24,
    )
}

fn move_toward(
    x: &mut i32,
    y: &mut i32,
    target_x: i32,
    target_y: i32,
    step: i32,
    facing_left: &mut bool,
) {
    let dx = target_x - *x;
    let dy = target_y - *y;
    if dx != 0 {
        *facing_left = dx < 0;
    }
    let squared = u64::try_from(i64::from(dx).pow(2) + i64::from(dy).pow(2)).unwrap_or(u64::MAX);
    let distance = integer_sqrt(squared);
    if distance <= u64::try_from(step).unwrap_or(0) {
        *x = target_x;
        *y = target_y;
        return;
    }
    let distance = i64::try_from(distance).unwrap_or(i64::MAX).max(1);
    let step = i64::from(step);
    let step_x = (i64::from(dx) * step / distance).clamp(-step, step);
    let step_y = (i64::from(dy) * step / distance).clamp(-step, step);
    *x = x.saturating_add(i32::try_from(step_x).unwrap_or(0));
    *y = y.saturating_add(i32::try_from(step_y).unwrap_or(0));
}

fn hunter_move_step(tick: u64) -> i32 {
    // Preserve the requested 1.5x increase from 5 px/tick without rounding
    // away the half pixel in the deterministic integer simulation.
    HUNTER_MOVE_PX_PER_TWO_TICKS / 2 + i32::from(tick % 2 == 0)
}

fn hunter_attack_range(class_family: &str) -> i32 {
    match class_family {
        "H3" | "H4" => HUNTER_RANGED_ATTACK_RANGE_PX,
        _ => HUNTER_MELEE_ATTACK_RANGE_PX,
    }
}

fn face_toward_x(facing_left: &mut bool, current_x: i32, target_x: i32) {
    if current_x != target_x {
        *facing_left = target_x < current_x;
    }
}

// The packaged monster has explicit front/back Spine clips. The native axis
// comparator is unresolved; this rebuild policy treats a target above the
// actor in scene Y-down coordinates as the back clip.
fn monster_directional_animation(base: &str, actor_y: i32, target_y: i32) -> String {
    if target_y < actor_y {
        format!("{base}_b")
    } else {
        base.to_owned()
    }
}

fn move_toward_avoiding(
    x: &mut i32,
    y: &mut i32,
    target_x: i32,
    target_y: i32,
    step: i32,
    facing_left: &mut bool,
    obstacles: &[NavigationObstacle],
) {
    const CLEARANCE: i32 = 14;
    let (direct_x, direct_y) = next_step(*x, *y, target_x, target_y, step);
    let blocking = obstacles
        .iter()
        .find(|obstacle| obstacle.expanded(CLEARANCE).contains(direct_x, direct_y));
    let Some(obstacle) = blocking else {
        move_toward(x, y, target_x, target_y, step, facing_left);
        return;
    };
    let expanded = obstacle.expanded(CLEARANCE);
    let top = expanded.min_y.saturating_sub(1);
    let bottom = expanded.max_y.saturating_add(1);
    let left = expanded.min_x.saturating_sub(1);
    let right = expanded.max_x.saturating_add(1);
    let (waypoint_x, waypoint_y) = if *y <= expanded.min_y {
        (
            if target_x >= expanded.max_x {
                right
            } else {
                left
            },
            top,
        )
    } else if *y >= expanded.max_y {
        (
            if target_x >= expanded.max_x {
                right
            } else {
                left
            },
            bottom,
        )
    } else if *x <= expanded.min_x || *x >= expanded.max_x {
        let top_cost = i64::from((*y - top).abs()) + i64::from((target_y - top).abs());
        let bottom_cost = i64::from((*y - bottom).abs()) + i64::from((target_y - bottom).abs());
        (*x, if top_cost <= bottom_cost { top } else { bottom })
    } else {
        let candidates = [(left, *y), (right, *y), (*x, top), (*x, bottom)];
        candidates
            .into_iter()
            .min_by_key(|(candidate_x, candidate_y)| {
                squared_distance(*x, *y, *candidate_x, *candidate_y)
            })
            .unwrap_or((target_x, target_y))
    };
    move_toward(x, y, waypoint_x, waypoint_y, step, facing_left);
}

fn next_step(x: i32, y: i32, target_x: i32, target_y: i32, step: i32) -> (i32, i32) {
    let mut next_x = x;
    let mut next_y = y;
    let mut ignored_facing = false;
    move_toward(
        &mut next_x,
        &mut next_y,
        target_x,
        target_y,
        step,
        &mut ignored_facing,
    );
    (next_x, next_y)
}

impl NavigationObstacle {
    fn expanded(self, amount: i32) -> Self {
        Self {
            min_x: self.min_x.saturating_sub(amount),
            max_x: self.max_x.saturating_add(amount),
            min_y: self.min_y.saturating_sub(amount),
            max_y: self.max_y.saturating_add(amount),
        }
    }

    fn contains(self, x: i32, y: i32) -> bool {
        (self.min_x..=self.max_x).contains(&x) && (self.min_y..=self.max_y).contains(&y)
    }
}

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut left = 1_u64;
    let mut right = value.min(u64::from(u32::MAX));
    while left <= right {
        let middle = left + (right - left) / 2;
        if middle <= value / middle {
            left = middle.saturating_add(1);
        } else {
            right = middle.saturating_sub(1);
        }
    }
    right
}

fn squared_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i64 {
    i64::from(x2 - x1).pow(2) + i64::from(y2 - y1).pow(2)
}

fn set_hunter_presentation(
    agent: &mut HunterAgentState,
    state: HunterActionState,
    animation: &str,
) {
    agent.action_state = state;
    agent.animation = animation.to_owned();
}

fn deterministic_roll(tick: u64, reward_sequence: u64, source_index: u32, slot: u64) -> u32 {
    let mut value = tick
        ^ reward_sequence.rotate_left(17)
        ^ u64::from(source_index).rotate_left(31)
        ^ slot.rotate_left(47)
        ^ 0x9e37_79b9_7f4a_7c15;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    u32::try_from((value ^ (value >> 31)) % 10_000 + 1).unwrap_or(1)
}

fn deterministic_combat_percent_roll(
    tick: u64,
    hunter_id: u32,
    attack_sequence: u64,
    source_index: u32,
) -> i32 {
    // Unity's Random.Range(0,100) bounds are proven. Its global PRNG state is
    // not, so the authoritative rebuild supplies a deterministic uniform roll
    // while preserving the original threshold comparison exactly.
    let roll = deterministic_roll(tick, attack_sequence, source_index, u64::from(hunter_id));
    i32::try_from((roll - 1) % 100).unwrap_or(0)
}

fn reward_operation_id(tick: u64, hunter_id: u32, drop_id: &str) -> Uuid {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in drop_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    Uuid::from_u128((u128::from(tick) << 64) ^ (u128::from(hunter_id) << 32) ^ u128::from(hash))
}

fn add_experience(hunter: &mut super::hunter_roster::DurableHunterState, experience: u64) {
    // The native PlusExp cap applies to stored HunterData.level (display is +1).
    if hunter.profile.level >= super::original_progression::ORIGINAL_HUNTER_MAX_STORED_LEVEL {
        return;
    }
    hunter.profile.xp = hunter.profile.xp.saturating_add(experience);
    while let Some(required) = hunter
        .profile
        .xp_to_next_level
        // Native PlusExp carries only when the post-grant remainder is
        // positive; landing exactly on the threshold stays at the level.
        .filter(|required| *required > 0 && hunter.profile.xp > *required)
    {
        hunter.profile.xp -= required;
        hunter.profile.level = hunter.profile.level.saturating_add(1);
        // Exact lookup is recovered, but the fixture class-to-job column mapping is not yet bound.
        hunter.profile.xp_to_next_level = Some(required.saturating_add(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::hunter_roster::operational_migration_roster;

    #[test]
    fn live_progression_keeps_the_recovered_strict_experience_threshold() {
        let mut roster = operational_migration_roster();
        let hunter = &mut roster.hunters[0];
        hunter.profile.level = 0;
        hunter.profile.xp = 0;
        hunter.profile.xp_to_next_level = Some(240);

        add_experience(hunter, 240);
        assert_eq!((hunter.profile.level, hunter.profile.xp), (0, 240));

        add_experience(hunter, 1);
        assert_eq!((hunter.profile.level, hunter.profile.xp), (1, 1));
    }

    #[test]
    fn density_reconciles_only_the_selected_region() {
        let mut world = MonsterWorldState::default();
        let first_region = world.fields[0].monsters.len();
        world.set_region_density("background_08", 3).unwrap();
        assert_eq!(world.fields[0].monsters.len(), first_region);
        assert_eq!(world.fields[1].monsters.len(), 9);
    }

    #[test]
    fn ordinary_regions_never_overlap_the_town_building_zone() {
        for config in MAP_CONFIGS {
            assert!(!config.bounds.intersects(TOWN_EXCLUSION_BOUNDS));
            for index in 0..9 {
                let point = spawn_point(config.bounds, index);
                assert!(config.bounds.contains(point.0, point.1));
                assert!(!TOWN_EXCLUSION_BOUNDS.contains(point.0, point.1));
            }
        }
    }

    #[test]
    fn hunter_enters_each_field_through_recovered_bridge_and_sign_anchors() {
        for config in MAP_CONFIGS {
            let mut world = MonsterWorldState::default();
            let mut roster = operational_migration_roster();
            roster.assign_hunt(1, config.map_id).unwrap();
            for (stage, waypoint) in config.entry_waypoints.iter().enumerate() {
                let mut reached = false;
                for _ in 0..300 {
                    world.tick(&mut roster);
                    let agent = world
                        .hunters
                        .iter()
                        .find(|agent| agent.hunter_id == 1)
                        .unwrap();
                    if usize::from(agent.entry_stage) > stage {
                        reached = true;
                        assert!(
                            squared_distance(agent.x, agent.y, waypoint.0, waypoint.1)
                                <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                        );
                        break;
                    }
                }
                assert!(
                    reached,
                    "Hunter never reached entry waypoint {stage} for {}",
                    config.map_id
                );
            }

            let mut entered_field = false;
            for _ in 0..300 {
                world.tick(&mut roster);
                let agent = world
                    .hunters
                    .iter()
                    .find(|agent| agent.hunter_id == 1)
                    .unwrap();
                if config.bounds.contains(agent.x, agent.y) {
                    entered_field = true;
                    break;
                }
            }
            assert!(
                entered_field,
                "Hunter never entered field for {}",
                config.map_id
            );
        }
    }

    #[test]
    fn movement_uses_a_bounded_constant_length_step() {
        let (mut x, mut y, mut facing_left) = (0, 0, false);
        move_toward(&mut x, &mut y, -100, 100, 10, &mut facing_left);
        assert!(facing_left);
        assert!(squared_distance(0, 0, x, y) <= 100);
        assert!(x < 0 && y > 0);
    }

    #[test]
    fn monster_attack_uses_back_clip_when_target_is_above_actor() {
        assert_eq!(monster_directional_animation("atk", 500, 450), "atk_b");
        assert_eq!(monster_directional_animation("atk", 500, 550), "atk");
    }

    #[test]
    fn hunter_move_tuning_averages_exactly_seven_and_a_half_pixels_per_tick() {
        assert_eq!(hunter_move_step(1), 7);
        assert_eq!(hunter_move_step(2), 8);
        assert_eq!(hunter_move_step(1) + hunter_move_step(2), 15);
    }

    #[test]
    fn unassigned_hunters_roam_in_town_then_pause_without_leaving_town_bounds() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        world.tick(&mut roster);
        let initial = world.hunters[0].clone();
        for _ in 0..30 {
            world.tick(&mut roster);
        }
        let moved = world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == initial.hunter_id)
            .expect("hunter remains in world");
        assert_ne!((moved.x, moved.y), (initial.x, initial.y));
        assert_eq!(moved.region_id, None);
        assert!(TOWN_ROAM_BOUNDS.contains(moved.x, moved.y));
        assert_eq!(moved.action_state, HunterActionState::TownIdle);
        assert!(matches!(
            moved.animation.as_str(),
            "hunter_walk" | "hunter_stay"
        ));
    }

    #[test]
    fn field_target_acquisition_searches_the_entire_assigned_region() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        roster.assign_hunt(1, "map_new01").unwrap();
        world.tick(&mut roster);
        let field_monster = world.fields[0].monsters[0].clone();
        for monster in world.fields[0].monsters.iter_mut().skip(1) {
            monster.hp = 0;
        }
        let agent = world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        agent.region_id = Some("map_new01".to_owned());
        agent.entry_stage = 2;
        agent.x = MAP_CONFIGS[0].bounds.max_x;
        agent.y = MAP_CONFIGS[0].bounds.max_y;
        assert!(
            squared_distance(agent.x, agent.y, field_monster.x, field_monster.y)
                > i64::from(MONSTER_DETECTION_RANGE_PX).pow(2)
        );
        world.tick(&mut roster);
        let updated = world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        assert!(updated.target_monster_id.is_some());
        assert_eq!(updated.region_id.as_deref(), Some("map_new01"));
        assert_eq!(
            updated.target_monster_id.as_deref(),
            Some(field_monster.entity_id.as_str())
        );
    }

    #[test]
    fn ranger_and_sorcerer_attack_from_range_while_melee_families_close_in() {
        assert_eq!(hunter_attack_range("H1"), HUNTER_MELEE_ATTACK_RANGE_PX);
        assert_eq!(hunter_attack_range("H2"), HUNTER_MELEE_ATTACK_RANGE_PX);
        assert_eq!(hunter_attack_range("H3"), HUNTER_RANGED_ATTACK_RANGE_PX);
        assert_eq!(hunter_attack_range("H4"), HUNTER_RANGED_ATTACK_RANGE_PX);
        assert_eq!(hunter_attack_range("H5"), HUNTER_MELEE_ATTACK_RANGE_PX);

        for (hunter_id, expected_state) in [
            (1, HunterActionState::Chasing),
            (3, HunterActionState::Attacking),
            (4, HunterActionState::Attacking),
        ] {
            let mut world = MonsterWorldState::default();
            let mut roster = operational_migration_roster();
            roster.assign_hunt(hunter_id, "map_new01").unwrap();
            world.tick(&mut roster);
            let monster = world.fields[0].monsters[0].clone();
            let agent = world
                .hunters
                .iter_mut()
                .find(|agent| agent.hunter_id == hunter_id)
                .unwrap();
            agent.region_id = Some("map_new01".to_owned());
            agent.entry_stage = 2;
            agent.x = monster.x + 120;
            agent.y = monster.y;
            agent.target_monster_id = Some(monster.entity_id);
            world.tick(&mut roster);

            let agent = world
                .hunters
                .iter()
                .find(|agent| agent.hunter_id == hunter_id)
                .unwrap();
            assert_eq!(agent.action_state, expected_state);
            if expected_state == HunterActionState::Attacking {
                assert!(
                    agent.facing_left,
                    "ranged Hunter must face its target before firing"
                );
            }
        }
    }

    #[test]
    fn authoritative_damage_records_a_monotonic_target_bound_presentation() {
        let mut world = MonsterWorldState::default();
        let monster = world.fields[0].monsters[0].clone();

        world.apply_damage_to_monster(
            &monster.entity_id,
            1,
            17,
            CombatPresentationKind::NormalDamage,
        );

        assert_eq!(world.combat_presentations.len(), 1);
        assert_eq!(
            world.combat_presentations[0],
            CombatPresentation {
                sequence: 1,
                source_entity_id: "village-hunter-1".to_owned(),
                target_entity_id: monster.entity_id.clone(),
                kind: CombatPresentationKind::NormalDamage,
                amount: Some(17),
            }
        );

        world.apply_damage_to_monster(
            &monster.entity_id,
            1,
            3,
            CombatPresentationKind::NormalDamage,
        );
        assert_eq!(world.combat_presentations[1].sequence, 2);
    }

    #[test]
    fn connected_original_resolver_emits_critical_damage_from_the_server() {
        let mut world = MonsterWorldState::default();
        let monster = world.fields[0].monsters[0].clone();

        world.resolve_hunter_attack(
            &monster.entity_id,
            HunterAttackSource {
                hunter_id: 1,
                calculated_damage: 100,
                calculated_critical_percent: 100,
                hunter_feel: 100.0,
                hunter_now_feel: 100.0,
                attack_sequence: 1,
            },
        );

        assert_eq!(world.combat_presentations.len(), 1);
        assert_eq!(
            world.combat_presentations[0].kind,
            CombatPresentationKind::CriticalDamage
        );
        assert_eq!(world.combat_presentations[0].amount, Some(191));
        assert_eq!(world.fields[0].monsters[0].hp, monster.hp - 191);
    }

    #[test]
    fn authoritative_monster_hit_records_incoming_damage_for_the_hunter() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        roster.assign_hunt(1, "map_new01").unwrap();
        world.tick(&mut roster);

        let monster = &mut world.fields[0].monsters[0];
        let agent = world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == 1)
            .unwrap();
        agent.region_id = Some("map_new01".to_owned());
        agent.x = monster.x;
        agent.y = monster.y;
        monster.target_hunter_id = Some(1);
        monster.recovery_ticks = 0;
        let hunter = roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == 1)
            .unwrap();
        let expected_tick = world.tick.saturating_add(1);
        let expected_dodge_roll = deterministic_combat_percent_roll(
            expected_tick,
            hunter.hunter_id,
            expected_tick,
            monster.source_index,
        );
        let expected_pet_roll = i32::try_from(
            (deterministic_roll(
                expected_tick,
                expected_tick,
                monster.source_index,
                u64::from(hunter.hunter_id).wrapping_add(1),
            ) - 1)
                % 1000,
        )
        .unwrap_or(0);
        let expected = resolve_original_neutral_monster_attack(OriginalMonsterAttackInputs {
            incoming_damage: i64::try_from(
                (monster.damage / MONSTER_INCOMING_DAMAGE_PROJECTION_DIVISOR).max(1),
            )
            .unwrap(),
            rand_damage_multiplier: 0.91,
            hunter_armor: i64::try_from(hunter.profile.defense).unwrap(),
            hunter_feel: hunter.mood.maximum as f32,
            hunter_now_feel: hunter.mood.current as f32,
            hunter_hp: i64::try_from(hunter.current_hp).unwrap(),
            hunter_calc_dodge: hunter.profile.calc_dodge(),
            hunter_dodge_primary_roll_zero_to_ninety_nine: expected_dodge_roll,
            hunter_riding_pet_dodge: 0,
            hunter_riding_pet_roll_zero_to_nine_ninety_nine: expected_pet_roll,
            ..OriginalMonsterAttackInputs::default()
        })
        .unwrap();
        let monster_entity_id = monster.entity_id.clone();

        world.tick(&mut roster);

        let expected_kind = match expected.presentation {
            OriginalHitPresentation::Normal => CombatPresentationKind::IncomingDamage,
            OriginalHitPresentation::Miss => CombatPresentationKind::Miss,
            OriginalHitPresentation::Evade => CombatPresentationKind::Evade,
            OriginalHitPresentation::Critical => unreachable!(),
        };
        assert!(world.combat_presentations.iter().any(|presentation| {
            presentation.source_entity_id == monster_entity_id
                && presentation.target_entity_id == "village-hunter-1"
                && presentation.kind == expected_kind
                && presentation.amount
                    == u64::try_from(expected.final_damage)
                        .ok()
                        .filter(|_| expected.presentation == OriginalHitPresentation::Normal)
        }));
    }

    #[test]
    fn a_new_server_tick_expires_prior_combat_presentations() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        let monster_id = world.fields[0].monsters[0].entity_id.clone();
        world.apply_damage_to_monster(&monster_id, 1, 1, CombatPresentationKind::NormalDamage);
        assert_eq!(world.combat_presentations.len(), 1);

        world.tick(&mut roster);

        assert!(world.combat_presentations.is_empty());
    }

    #[test]
    fn gold_only_drop_never_emits_an_invalid_item_reward_operation() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        let config = &MAP_CONFIGS[0];
        let (x, y) = spawn_point(config.bounds, 0);
        world.hunters.push(HunterAgentState {
            hunter_id: 1,
            region_id: Some(config.map_id.to_owned()),
            x,
            y,
            facing_left: false,
            action_state: HunterActionState::CollectingLoot,
            animation: "hunter_walk".to_owned(),
            target_monster_id: None,
            target_drop_id: None,
            recovery_ticks: 0,
            respawn_ticks: None,
            attack_sequence: 0,
            active_skill_id: None,
            entry_stage: 1,
        });
        world.fields[0].drops.push(MonsterDrop {
            drop_id: "gold-only".to_owned(),
            monster_entity_id: "monster-1".to_owned(),
            item_id: "gold".to_owned(),
            quantity: 0,
            x,
            y,
            owner_hunter_id: 1,
            gold: 11,
            experience: 7,
        });
        let gold_before = roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == 1)
            .unwrap()
            .gold;
        let mut operations = Vec::new();

        assert!(world.try_collect_drop(0, &mut roster, &mut operations));
        assert_eq!(
            roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == 1)
                .unwrap()
                .gold,
            gold_before + 11
        );
        assert!(operations.is_empty());
    }

    #[test]
    fn hunter_navigation_detours_around_building_footprints() {
        let obstacle = NavigationObstacle {
            min_x: 40,
            max_x: 60,
            min_y: -20,
            max_y: 20,
        };
        let (mut x, mut y, mut facing_left) = (0, 0, false);
        for _ in 0..30 {
            move_toward_avoiding(&mut x, &mut y, 100, 0, 10, &mut facing_left, &[obstacle]);
            assert!(!obstacle.expanded(12).contains(x, y));
        }
        assert!(x > obstacle.max_x);
    }

    #[test]
    fn monster_patrol_uses_short_segments_then_idles_for_two_and_a_half_seconds() {
        let config = &MAP_CONFIGS[0];
        let mut monster = spawn_monster(config, 0, 0);
        let origin = (monster.spawn_x, monster.spawn_y);

        for _ in 0..40 {
            patrol(&mut monster, config.bounds);
            if monster.action_state == MonsterActionState::Idle
                && monster.patrol_idle_ticks == MONSTER_PATROL_IDLE_TICKS
            {
                break;
            }
        }

        assert_eq!(monster.action_state, MonsterActionState::Idle);
        assert_eq!(monster.animation, "stay");
        assert_eq!(monster.patrol_idle_ticks, MONSTER_PATROL_IDLE_TICKS);
        assert_eq!(MONSTER_PATROL_IDLE_TICKS + 1, 25);
        assert!(
            squared_distance(origin.0, origin.1, monster.x, monster.y)
                <= i64::from(MONSTER_PATROL_RADIUS_PX).pow(2)
        );

        let resting_at = (monster.x, monster.y);
        for _ in 0..MONSTER_PATROL_IDLE_TICKS {
            patrol(&mut monster, config.bounds);
            assert_eq!((monster.x, monster.y), resting_at);
            assert_eq!(monster.action_state, MonsterActionState::Idle);
            assert_eq!(monster.animation, "stay");
        }

        patrol(&mut monster, config.bounds);
        assert_eq!(monster.action_state, MonsterActionState::Patrolling);
        assert_eq!(monster.animation, "walk");
        assert_ne!((monster.x, monster.y), resting_at);
    }

    #[test]
    fn monster_patrol_waypoints_stay_inside_their_region() {
        for config in &MAP_CONFIGS {
            for index in 0..9 {
                let mut monster = spawn_monster(config, 0, index);
                for phase in 0..8 {
                    monster.patrol_phase = phase;
                    let waypoint = patrol_waypoint(&monster, config.bounds);
                    assert!(config.bounds.contains(waypoint.0, waypoint.1));
                    assert!(
                        squared_distance(monster.spawn_x, monster.spawn_y, waypoint.0, waypoint.1,)
                            <= i64::from(MONSTER_PATROL_RADIUS_PX).pow(2)
                    );
                }
            }
        }
    }

    #[test]
    fn assigned_hunter_enters_region_fights_collects_and_monster_respawns() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        roster.hunters[0].profile.attack = 29;
        roster.hunters[0].current_hp = 130;
        roster.hunters[0].max_hp = 130;
        let gold_before = roster.hunters[0].gold;
        let xp_before = roster.hunters[0].profile.xp;
        roster.assign_hunt(1, "map_new01").unwrap();
        for _ in 0..1_200 {
            world.tick(&mut roster);
        }
        let hunter = roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == 1)
            .unwrap();
        assert!(hunter.gold > gold_before);
        assert!(hunter.profile.xp != xp_before || hunter.profile.level > 1);
        assert!(!world.fields[0].monsters.is_empty());
    }

    #[test]
    fn ordinary_material_roll_is_inclusive_and_bounded() {
        for slot in 0..64 {
            assert!((1..=10_000).contains(&deterministic_roll(10, 2, 34, slot)));
        }
    }

    #[test]
    fn durable_dead_hunter_resumes_the_authoritative_respawn_clock_after_reconnect() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        roster.assign_hunt(1, "map_new01").unwrap();
        roster.defeat_hunter(1).unwrap();
        for _ in 0..=HUNTER_RESPAWN_TICKS {
            world.tick(&mut roster);
        }
        assert_eq!(roster.hunters[0].current_hp, roster.hunters[0].max_hp);
        assert_eq!(roster.hunters[0].hunt.status, "hunting");
    }
}
