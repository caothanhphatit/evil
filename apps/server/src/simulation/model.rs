use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::rng::DeterministicRng;

/// Internal-only command set for the deterministic combat fixture tests.
/// It is intentionally not part of the browser protocol or the original-flow session.
#[derive(Debug)]
pub enum FixtureCommand {
    StartBattle { monster_id: u32 },
    StopBattle,
    RespawnHunter,
    EquipItem { command_id: Uuid, item_id: u32 },
    RequestResync,
}

const HUNTER_ID: u32 = 1;
const MONSTER_ID: u32 = 1001;
const LOOT_ITEM_ID: u32 = 2001;
const HUNTER_ATTACK_INTERVAL: u64 = 5;
const MONSTER_ATTACK_INTERVAL: u64 = 10;
const REVIVAL_TICKS: u64 = 30;
const RESPAWN_MONSTER_TICKS: u64 = 20;
const ATTACK_RANGE: i32 = 55;
const PICKUP_RANGE: i32 = 12;
const HUNTER_SPEED: i32 = 18;
const MONSTER_SPEED: i32 = 8;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Combatant {
    id: u32,
    hp: i32,
    max_hp: i32,
    attack: i32,
    x: i32,
    y: i32,
    next_attack_tick: u64,
    state: EntityState,
}

impl Combatant {
    fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityState {
    Idle,
    Moving,
    Attacking,
    Dead,
    Reviving,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryStack {
    pub item_id: u32,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundDrop {
    pub drop_id: String,
    pub item_id: u32,
    pub quantity: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DurablePlayerState {
    pub tick: u64,
    pub gold: u64,
    pub autonomous: bool,
    pub hunter_hp: i32,
    pub hunter_x: i32,
    pub hunter_y: i32,
    pub hunter_next_attack_tick: u64,
    pub hunter_state: EntityState,
    pub hunter_revival_tick: Option<u64>,
    pub monster_hp: i32,
    pub monster_x: i32,
    pub monster_y: i32,
    pub monster_next_attack_tick: u64,
    pub monster_state: EntityState,
    pub monster_respawn_tick: Option<u64>,
    pub inventory: BTreeMap<u32, u32>,
    pub equipped_item_id: Option<u32>,
    pub ground_drops: Vec<GroundDrop>,
    pub rng_state: u64,
}

impl Default for DurablePlayerState {
    fn default() -> Self {
        Self {
            tick: 0,
            gold: 0,
            autonomous: true,
            hunter_hp: 100,
            hunter_x: 120,
            hunter_y: 320,
            hunter_next_attack_tick: 0,
            hunter_state: EntityState::Idle,
            hunter_revival_tick: None,
            monster_hp: 50,
            monster_x: 640,
            monster_y: 320,
            monster_next_attack_tick: 0,
            monster_state: EntityState::Idle,
            monster_respawn_tick: None,
            inventory: BTreeMap::new(),
            equipped_item_id: None,
            ground_drops: Vec::new(),
            rng_state: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingOperation {
    Reward {
        operation_id: Uuid,
        gold: u64,
        item_id: u32,
        quantity: u32,
    },
    Equip {
        command_id: Uuid,
        item_id: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutcome {
    pub command_id: Uuid,
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub struct Simulation {
    tick: u64,
    gold: u64,
    autonomous: bool,
    hunter: Combatant,
    monster: Combatant,
    monster_respawn_tick: Option<u64>,
    hunter_revival_tick: Option<u64>,
    inventory: BTreeMap<u32, u32>,
    equipped_item_id: Option<u32>,
    ground_drops: Vec<GroundDrop>,
    rng: DeterministicRng,
    events: Vec<CombatEvent>,
    operations: Vec<PendingOperation>,
    command_results: HashMap<Uuid, CommandOutcome>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub fighting: bool,
    pub gold: u64,
    pub hunter: EntitySnapshot,
    pub monster: EntitySnapshot,
    pub inventory: Vec<InventoryStack>,
    pub equipped_item_id: Option<u32>,
    pub ground_drops: Vec<GroundDrop>,
    pub events: Vec<CombatEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EntitySnapshot {
    pub id: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub x: i32,
    pub y: i32,
    pub state: EntityState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CombatEvent {
    BattleStarted,
    BattleStopped,
    Damage {
        source_id: u32,
        target_id: u32,
        amount: i32,
    },
    MonsterDefeated {
        monster_id: u32,
        gold_reward: u64,
    },
    DropCreated {
        drop_id: String,
        item_id: u32,
        quantity: u32,
    },
    DropCollected {
        drop_id: String,
        item_id: u32,
        quantity: u32,
    },
    ItemEquipped {
        item_id: u32,
    },
    HunterDefeated,
    HunterRespawned,
    MonsterRespawned,
    CommandRejected {
        reason: &'static str,
    },
}

impl Simulation {
    pub fn new(seed: u64) -> Self {
        Self::from_state(seed, DurablePlayerState::default())
    }

    pub fn from_state(seed: u64, state: DurablePlayerState) -> Self {
        let rng_seed = if state.rng_state == 0 {
            seed
        } else {
            state.rng_state
        };
        Self {
            tick: state.tick,
            gold: state.gold,
            autonomous: state.autonomous,
            hunter: Combatant {
                id: HUNTER_ID,
                hp: state.hunter_hp.clamp(0, 100),
                max_hp: 100,
                attack: 12,
                x: state.hunter_x,
                y: state.hunter_y,
                next_attack_tick: state.hunter_next_attack_tick,
                state: state.hunter_state,
            },
            monster: Combatant {
                id: MONSTER_ID,
                hp: state.monster_hp.clamp(0, 50),
                max_hp: 50,
                attack: 5,
                x: state.monster_x,
                y: state.monster_y,
                next_attack_tick: state.monster_next_attack_tick,
                state: state.monster_state,
            },
            monster_respawn_tick: state.monster_respawn_tick,
            hunter_revival_tick: state.hunter_revival_tick,
            inventory: state.inventory,
            equipped_item_id: state.equipped_item_id,
            ground_drops: state.ground_drops,
            rng: DeterministicRng::new(rng_seed),
            events: Vec::new(),
            operations: Vec::new(),
            command_results: HashMap::new(),
        }
    }

    pub fn handle_command(&mut self, command: FixtureCommand) -> Option<CommandOutcome> {
        match command {
            FixtureCommand::StartBattle { monster_id } if monster_id == self.monster.id => {
                self.autonomous = true;
                self.events.push(CombatEvent::BattleStarted);
                None
            }
            FixtureCommand::StartBattle { .. } => {
                self.events.push(CombatEvent::CommandRejected {
                    reason: "target_unavailable",
                });
                None
            }
            FixtureCommand::StopBattle => {
                self.autonomous = false;
                self.events.push(CombatEvent::BattleStopped);
                None
            }
            FixtureCommand::RespawnHunter => {
                self.events.push(CombatEvent::CommandRejected {
                    reason: "revival_server_managed",
                });
                None
            }
            FixtureCommand::EquipItem {
                command_id,
                item_id,
            } => Some(self.equip(command_id, item_id)),
            FixtureCommand::RequestResync => None,
        }
    }

    pub fn step(&mut self) -> WorldSnapshot {
        self.tick += 1;
        self.process_respawns();

        if self.hunter.is_alive() {
            if !self.ground_drops.is_empty() {
                self.move_to_and_collect_drop();
            } else if self.autonomous && self.monster.is_alive() {
                self.move_and_fight();
            } else {
                self.hunter.state = EntityState::Idle;
            }
        }

        let snapshot = self.snapshot();
        self.events.clear();
        snapshot
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            tick: self.tick,
            fighting: self.hunter.state == EntityState::Attacking && self.monster.is_alive(),
            gold: self.gold,
            hunter: entity_snapshot(&self.hunter),
            monster: entity_snapshot(&self.monster),
            inventory: self
                .inventory
                .iter()
                .map(|(&item_id, &quantity)| InventoryStack { item_id, quantity })
                .collect(),
            equipped_item_id: self.equipped_item_id,
            ground_drops: self.ground_drops.clone(),
            events: self.events.clone(),
        }
    }

    pub fn durable_state(&self) -> DurablePlayerState {
        DurablePlayerState {
            tick: self.tick,
            gold: self.gold,
            autonomous: self.autonomous,
            hunter_hp: self.hunter.hp,
            hunter_x: self.hunter.x,
            hunter_y: self.hunter.y,
            hunter_next_attack_tick: self.hunter.next_attack_tick,
            hunter_state: self.hunter.state,
            hunter_revival_tick: self.hunter_revival_tick,
            monster_hp: self.monster.hp,
            monster_x: self.monster.x,
            monster_y: self.monster.y,
            monster_next_attack_tick: self.monster.next_attack_tick,
            monster_state: self.monster.state,
            monster_respawn_tick: self.monster_respawn_tick,
            inventory: self.inventory.clone(),
            equipped_item_id: self.equipped_item_id,
            ground_drops: self.ground_drops.clone(),
            rng_state: self.rng.state(),
        }
    }

    pub fn drain_operations(&mut self) -> Vec<PendingOperation> {
        std::mem::take(&mut self.operations)
    }

    fn equip(&mut self, command_id: Uuid, item_id: u32) -> CommandOutcome {
        if let Some(result) = self.command_results.get(&command_id) {
            return result.clone();
        }
        let accepted = self.inventory.get(&item_id).copied().unwrap_or(0) > 0;
        let outcome = CommandOutcome {
            command_id,
            accepted,
            reason: (!accepted).then(|| "item_not_owned".to_owned()),
        };
        if accepted {
            self.equipped_item_id = Some(item_id);
            self.events.push(CombatEvent::ItemEquipped { item_id });
            self.operations.push(PendingOperation::Equip {
                command_id,
                item_id,
            });
        }
        self.command_results.insert(command_id, outcome.clone());
        outcome
    }

    fn move_and_fight(&mut self) {
        let distance = (self.monster.x - self.hunter.x).abs();
        if distance > ATTACK_RANGE {
            self.hunter.state = EntityState::Moving;
            self.monster.state = EntityState::Moving;
            move_towards(&mut self.hunter.x, self.monster.x, HUNTER_SPEED);
            move_towards(&mut self.monster.x, self.hunter.x, MONSTER_SPEED);
            return;
        }

        self.hunter.state = EntityState::Attacking;
        self.monster.state = EntityState::Attacking;
        self.resolve_combat_tick();
    }

    fn resolve_combat_tick(&mut self) {
        if self.tick >= self.hunter.next_attack_tick {
            let bonus = self.equipped_item_id.map_or(0, |_| 4);
            let damage = (self.hunter.attack + bonus + self.rng.range_inclusive(-2, 2)).max(1);
            self.monster.hp = (self.monster.hp - damage).max(0);
            self.hunter.next_attack_tick = self.tick + HUNTER_ATTACK_INTERVAL;
            self.events.push(CombatEvent::Damage {
                source_id: self.hunter.id,
                target_id: self.monster.id,
                amount: damage,
            });
            if !self.monster.is_alive() {
                self.defeat_monster();
                return;
            }
        }

        if self.tick >= self.monster.next_attack_tick {
            let damage = (self.monster.attack + self.rng.range_inclusive(-1, 1)).max(1);
            self.hunter.hp = (self.hunter.hp - damage).max(0);
            self.monster.next_attack_tick = self.tick + MONSTER_ATTACK_INTERVAL;
            self.events.push(CombatEvent::Damage {
                source_id: self.monster.id,
                target_id: self.hunter.id,
                amount: damage,
            });
            if !self.hunter.is_alive() {
                self.hunter.state = EntityState::Dead;
                self.monster.state = EntityState::Idle;
                self.hunter_revival_tick = Some(self.tick + REVIVAL_TICKS);
                self.events.push(CombatEvent::HunterDefeated);
            }
        }
    }

    fn defeat_monster(&mut self) {
        self.monster.state = EntityState::Dead;
        self.hunter.state = EntityState::Idle;
        self.monster_respawn_tick = Some(self.tick + RESPAWN_MONSTER_TICKS);
        let operation_id = deterministic_uuid(self.rng.next_u64(), self.tick);
        let drop_id = operation_id.to_string();
        self.ground_drops.push(GroundDrop {
            drop_id: drop_id.clone(),
            item_id: LOOT_ITEM_ID,
            quantity: 1,
            x: self.monster.x,
            y: self.monster.y,
        });
        self.events.push(CombatEvent::MonsterDefeated {
            monster_id: self.monster.id,
            gold_reward: 10,
        });
        self.events.push(CombatEvent::DropCreated {
            drop_id,
            item_id: LOOT_ITEM_ID,
            quantity: 1,
        });
    }

    fn move_to_and_collect_drop(&mut self) {
        let target_x = self.ground_drops[0].x;
        if (target_x - self.hunter.x).abs() > PICKUP_RANGE {
            self.hunter.state = EntityState::Moving;
            move_towards(&mut self.hunter.x, target_x, HUNTER_SPEED);
            return;
        }
        self.hunter.state = EntityState::Idle;
        let drop = self.ground_drops.remove(0);
        *self.inventory.entry(drop.item_id).or_default() += drop.quantity;
        self.gold += 10;
        let operation_id = Uuid::parse_str(&drop.drop_id)
            .unwrap_or_else(|_| deterministic_uuid(self.tick, u64::from(drop.item_id)));
        self.operations.push(PendingOperation::Reward {
            operation_id,
            gold: 10,
            item_id: drop.item_id,
            quantity: drop.quantity,
        });
        self.events.push(CombatEvent::DropCollected {
            drop_id: drop.drop_id,
            item_id: drop.item_id,
            quantity: drop.quantity,
        });
    }

    fn process_respawns(&mut self) {
        if self
            .hunter_revival_tick
            .is_some_and(|tick| self.tick >= tick)
        {
            self.revive_hunter();
        }
        if self
            .monster_respawn_tick
            .is_some_and(|tick| self.tick >= tick)
        {
            self.monster.hp = self.monster.max_hp;
            self.monster.x = 640;
            self.monster.state = EntityState::Idle;
            self.monster.next_attack_tick = self.tick + 1;
            self.monster_respawn_tick = None;
            self.events.push(CombatEvent::MonsterRespawned);
        }
    }

    fn revive_hunter(&mut self) {
        self.hunter.hp = self.hunter.max_hp;
        self.hunter.x = 120;
        self.hunter.state = EntityState::Reviving;
        self.hunter.next_attack_tick = self.tick + 1;
        self.hunter_revival_tick = None;
        self.events.push(CombatEvent::HunterRespawned);
    }
}

fn move_towards(position: &mut i32, target: i32, speed: i32) {
    *position += (target - *position).clamp(-speed, speed);
}

fn deterministic_uuid(high: u64, low: u64) -> Uuid {
    Uuid::from_u128((u128::from(high) << 64) | u128::from(low))
}

fn entity_snapshot(entity: &Combatant) -> EntitySnapshot {
    EntitySnapshot {
        id: entity.id,
        hp: entity.hp,
        max_hp: entity.max_hp,
        alive: entity.is_alive(),
        x: entity.x,
        y: entity.y,
        state: entity.state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(seed: u64, ticks: usize) -> Vec<String> {
        let mut simulation = Simulation::new(seed);
        (0..ticks)
            .map(|_| serde_json::to_string(&simulation.step()).expect("snapshot serializes"))
            .collect()
    }

    #[test]
    fn deterministic_golden_trace_is_stable() {
        let golden_trace = trace(7, 60);
        assert_eq!(golden_trace, trace(7, 60));
        assert_eq!(
            golden_trace[0],
            r#"{"tick":1,"fighting":false,"gold":0,"hunter":{"id":1,"hp":100,"max_hp":100,"alive":true,"x":138,"y":320,"state":"moving"},"monster":{"id":1001,"hp":50,"max_hp":50,"alive":true,"x":632,"y":320,"state":"moving"},"inventory":[],"equipped_item_id":null,"ground_drops":[],"events":[]}"#
        );
        assert!(golden_trace
            .iter()
            .any(|line| line.contains("monster_defeated")));
        assert!(golden_trace
            .iter()
            .any(|line| line.contains("drop_collected")));
    }

    #[test]
    fn server_autonomously_moves_fights_and_collects_drop() {
        let mut simulation = Simulation::new(7);
        for _ in 0..80 {
            simulation.step();
        }
        let snapshot = simulation.snapshot();
        assert!(snapshot.gold >= 10);
        assert!(snapshot
            .inventory
            .iter()
            .any(|stack| stack.item_id == LOOT_ITEM_ID));
        assert!(simulation
            .drain_operations()
            .iter()
            .any(|operation| matches!(operation, PendingOperation::Reward { .. })));
    }

    #[test]
    fn duplicate_equip_command_returns_original_result_once() {
        let mut state = DurablePlayerState::default();
        state.inventory.insert(LOOT_ITEM_ID, 1);
        let mut simulation = Simulation::from_state(7, state);
        let command_id = Uuid::from_u128(9);
        let first = simulation.handle_command(FixtureCommand::EquipItem {
            command_id,
            item_id: LOOT_ITEM_ID,
        });
        let second = simulation.handle_command(FixtureCommand::EquipItem {
            command_id,
            item_id: LOOT_ITEM_ID,
        });
        assert_eq!(first, second);
        assert_eq!(simulation.drain_operations().len(), 1);
    }

    #[test]
    fn hunter_death_is_followed_by_server_owned_revival() {
        let mut simulation = Simulation::new(91);
        let mut defeated_at = None;
        let mut revived_after_defeat = false;
        for _ in 0..800 {
            let snapshot = simulation.step();
            if snapshot.events.contains(&CombatEvent::HunterDefeated) {
                defeated_at = Some(snapshot.tick);
            }
            if defeated_at.is_some() && snapshot.events.contains(&CombatEvent::HunterRespawned) {
                revived_after_defeat = true;
                break;
            }
        }
        assert!(
            defeated_at.is_some(),
            "combat must eventually defeat the hunter"
        );
        assert!(
            revived_after_defeat,
            "server must revive the hunter without client outcome input"
        );
    }

    #[test]
    fn reconnect_preserves_ground_drop_and_monster_respawn_deadline() {
        let mut simulation = Simulation::new(7);
        for _ in 0..100 {
            simulation.step();
            if !simulation.ground_drops.is_empty() {
                break;
            }
        }
        assert!(
            !simulation.ground_drops.is_empty(),
            "fixture must create a drop"
        );

        let expected_drop = simulation.ground_drops.clone();
        let expected_respawn = simulation.monster_respawn_tick;
        let restored = Simulation::from_state(7, simulation.durable_state());

        assert_eq!(restored.ground_drops, expected_drop);
        assert_eq!(restored.monster_respawn_tick, expected_respawn);
        assert!(!restored.monster.is_alive());
    }

    #[test]
    fn reconnect_preserves_server_owned_revival_deadline() {
        let mut simulation = Simulation::new(91);
        for _ in 0..800 {
            simulation.step();
            if !simulation.hunter.is_alive() {
                break;
            }
        }
        assert!(
            !simulation.hunter.is_alive(),
            "fixture must defeat the hunter"
        );
        let deadline = simulation.hunter_revival_tick;
        let mut restored = Simulation::from_state(91, simulation.durable_state());
        assert_eq!(restored.hunter_revival_tick, deadline);

        restored.handle_command(FixtureCommand::RespawnHunter);
        assert!(
            !restored.hunter.is_alive(),
            "client cannot bypass revival time"
        );

        let mut revived = false;
        for _ in 0..=REVIVAL_TICKS {
            let snapshot = restored.step();
            if snapshot.events.contains(&CombatEvent::HunterRespawned) {
                revived = true;
                break;
            }
        }
        assert!(revived, "restored timer must revive the hunter");
    }

    #[test]
    fn equipped_fixture_item_improves_authoritative_damage() {
        let base_state = DurablePlayerState {
            hunter_x: 600,
            monster_x: 640,
            ..DurablePlayerState::default()
        };
        let equipped_state = DurablePlayerState {
            inventory: BTreeMap::from([(LOOT_ITEM_ID, 1)]),
            equipped_item_id: Some(LOOT_ITEM_ID),
            ..base_state.clone()
        };

        let mut base = Simulation::from_state(123, base_state);
        let mut equipped = Simulation::from_state(123, equipped_state);
        base.step();
        equipped.step();

        assert!(equipped.monster.hp < base.monster.hp);
    }
}
