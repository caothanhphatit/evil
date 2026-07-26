use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::product_service::HunterServiceGauge;

pub const MAX_ACTIVE_TOWN_HUNTERS: usize = 8;
pub const MIGRATION_HUNTER_RELEASE_ID: &str = "migration.hunter-demo-v1";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterProfile {
    pub content_release_id: String,
    pub display_name: String,
    pub portrait_asset_id: Option<String>,
    pub class_id: String,
    pub class_name: String,
    pub visual_family: String,
    pub rarity_id: String,
    pub rarity_name: String,
    pub level: u32,
    pub xp: u64,
    pub xp_to_next_level: Option<u64>,
    pub attack: u64,
    pub defense: u64,
    pub dps_milli: Option<u64>,
    pub critical_rate_bps: Option<u32>,
    pub attack_speed_milli: Option<u32>,
    pub evasion_rate_bps: Option<u32>,
    pub awakening: Option<DurableHunterProgress>,
    pub reincarnation: Option<DurableHunterProgress>,
    pub is_locked: Option<bool>,
    pub characteristic_name: Option<String>,
    pub riding_pet_state_resolved: bool,
    pub action_state: String,
    pub animation_name: String,
    pub traits: Vec<DurableHunterTrait>,
    pub skills: Vec<DurableHunterSkill>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableHunterProgress {
    pub current: u32,
    pub maximum: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterTrait {
    pub trait_id: String,
    pub display_name: String,
    pub icon_path: String,
    pub unlocked_rank: u8,
    pub equipped: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterSkill {
    pub skill_id: String,
    pub display_name: String,
    pub icon_path: Option<String>,
    pub animation_name: Option<String>,
    pub skill_level: u8,
    pub equipped_slot: Option<u8>,
    pub ready: bool,
}

impl DurableHunterProfile {
    pub fn migration_default(hunter_id: u32) -> Self {
        Self {
            content_release_id: MIGRATION_HUNTER_RELEASE_ID.to_owned(),
            display_name: format!("Hunter {hunter_id}"),
            class_id: "h1".to_owned(),
            class_name: "Berserker".to_owned(),
            visual_family: "H1".to_owned(),
            rarity_id: "normal".to_owned(),
            rarity_name: "Normal".to_owned(),
            level: 1,
            attack: 10,
            defense: 10,
            action_state: "idle".to_owned(),
            animation_name: "hunter_stay".to_owned(),
            ..Self::default()
        }
    }
}

/// Provides an operational roster while the original starter composition remains unresolved.
/// The snapshot keeps that evidence flag false; these values are not legacy balance data.
pub fn operational_migration_roster() -> DurableHunterRosterState {
    let mut roster = DurableHunterRosterState {
        roster_resolved: true,
        wallets_resolved: true,
        ..DurableHunterRosterState::default()
    };
    for hunter_id in 1..=9 {
        let hunter = DurableHunterState {
            hunter_id,
            gold: 1_000,
            current_hp: 72 + u64::from((hunter_id * 3) % 29),
            max_hp: 100,
            stamina: HunterServiceGauge {
                current: 65 + u64::from((hunter_id * 5) % 31),
                maximum: 100,
            },
            satiety: HunterServiceGauge {
                current: 60 + u64::from((hunter_id * 7) % 36),
                maximum: 100,
            },
            mood: HunterServiceGauge {
                current: 70 + u64::from((hunter_id * 4) % 27),
                maximum: 100,
            },
            profile: DurableHunterProfile::migration_default(hunter_id),
        };
        roster
            .arrive(hunter)
            .expect("fixed migration roster satisfies capacity invariants");
    }
    roster
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterRosterState {
    pub roster_resolved: bool,
    pub wallets_resolved: bool,
    /// Active town roster in stable slot order. Slots are always compact after a banishment.
    pub hunters: Vec<DurableHunterState>,
    pub waiting_queue: Vec<DurableWaitingHunter>,
    pub next_arrival_sequence: u64,
    pub banish_commands: BTreeMap<Uuid, HunterBanishment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableHunterState {
    pub hunter_id: u32,
    pub gold: u64,
    pub current_hp: u64,
    pub max_hp: u64,
    #[serde(default)]
    pub stamina: HunterServiceGauge,
    #[serde(default)]
    pub satiety: HunterServiceGauge,
    #[serde(default)]
    pub mood: HunterServiceGauge,
    #[serde(default)]
    pub profile: DurableHunterProfile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableWaitingHunter {
    pub arrival_sequence: u64,
    pub hunter: DurableHunterState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HunterArrivalDisposition {
    Active { slot: usize },
    Waiting { position: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunterBanishment {
    pub banished_hunter_id: u32,
    pub promoted_hunter_id: Option<u32>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum HunterRosterError {
    #[error("hunter already belongs to this town")]
    DuplicateHunter,
    #[error("hunter is not in the active town roster")]
    ActiveHunterUnknown,
    #[error("hunter roster invariant violated: {0}")]
    InvalidState(&'static str),
    #[error("command id was already used for a different hunter")]
    CommandConflict,
}

impl DurableHunterRosterState {
    pub fn arrive(
        &mut self,
        hunter: DurableHunterState,
    ) -> Result<HunterArrivalDisposition, HunterRosterError> {
        self.validate()?;
        if self.contains(hunter.hunter_id) {
            return Err(HunterRosterError::DuplicateHunter);
        }
        if self.hunters.len() < MAX_ACTIVE_TOWN_HUNTERS {
            self.hunters.push(hunter);
            return Ok(HunterArrivalDisposition::Active {
                slot: self.hunters.len() - 1,
            });
        }

        let arrival_sequence = self.allocate_arrival_sequence();
        self.waiting_queue.push(DurableWaitingHunter {
            arrival_sequence,
            hunter,
        });
        Ok(HunterArrivalDisposition::Waiting {
            position: self.waiting_queue.len() - 1,
        })
    }

    pub fn banish_active(&mut self, hunter_id: u32) -> Result<HunterBanishment, HunterRosterError> {
        self.validate()?;
        let Some(index) = self
            .hunters
            .iter()
            .position(|hunter| hunter.hunter_id == hunter_id)
        else {
            return Err(HunterRosterError::ActiveHunterUnknown);
        };
        self.hunters.remove(index);

        self.waiting_queue
            .sort_by_key(|waiting| waiting.arrival_sequence);
        let promoted_hunter_id =
            if self.hunters.len() < MAX_ACTIVE_TOWN_HUNTERS && !self.waiting_queue.is_empty() {
                let promoted = self.waiting_queue.remove(0).hunter;
                let promoted_hunter_id = promoted.hunter_id;
                self.hunters.push(promoted);
                Some(promoted_hunter_id)
            } else {
                None
            };
        self.validate()?;
        Ok(HunterBanishment {
            banished_hunter_id: hunter_id,
            promoted_hunter_id,
        })
    }

    pub fn banish_active_idempotent(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
    ) -> Result<HunterBanishment, HunterRosterError> {
        self.validate()?;
        if let Some(previous) = self.banish_commands.get(&command_id) {
            return if previous.banished_hunter_id == hunter_id {
                Ok(previous.clone())
            } else {
                Err(HunterRosterError::CommandConflict)
            };
        }
        let result = self.banish_active(hunter_id)?;
        self.banish_commands.insert(command_id, result.clone());
        Ok(result)
    }

    pub fn upgrade_legacy_capacity(&mut self) {
        if self.hunters.len() > MAX_ACTIVE_TOWN_HUNTERS {
            let overflow = self.hunters.split_off(MAX_ACTIVE_TOWN_HUNTERS);
            for hunter in overflow {
                let arrival_sequence = self.allocate_arrival_sequence();
                self.waiting_queue.push(DurableWaitingHunter {
                    arrival_sequence,
                    hunter,
                });
            }
        }
        self.waiting_queue
            .sort_by_key(|waiting| waiting.arrival_sequence);
        let highest = self
            .waiting_queue
            .last()
            .map_or(0, |waiting| waiting.arrival_sequence);
        self.next_arrival_sequence = self
            .next_arrival_sequence
            .max(highest.saturating_add(1))
            .max(1);
    }

    pub fn validate(&self) -> Result<(), HunterRosterError> {
        if self.hunters.len() > MAX_ACTIVE_TOWN_HUNTERS {
            return Err(HunterRosterError::InvalidState(
                "active roster exceeds town capacity",
            ));
        }
        let mut hunter_ids =
            HashSet::with_capacity(self.hunters.len().saturating_add(self.waiting_queue.len()));
        if !self
            .hunters
            .iter()
            .all(|hunter| hunter_ids.insert(hunter.hunter_id))
            || !self
                .waiting_queue
                .iter()
                .all(|waiting| hunter_ids.insert(waiting.hunter.hunter_id))
        {
            return Err(HunterRosterError::InvalidState(
                "hunter id appears more than once",
            ));
        }
        if self
            .waiting_queue
            .windows(2)
            .any(|pair| pair[0].arrival_sequence >= pair[1].arrival_sequence)
        {
            return Err(HunterRosterError::InvalidState(
                "waiting queue is not strict FIFO order",
            ));
        }
        Ok(())
    }

    fn contains(&self, hunter_id: u32) -> bool {
        self.hunters
            .iter()
            .any(|hunter| hunter.hunter_id == hunter_id)
            || self
                .waiting_queue
                .iter()
                .any(|waiting| waiting.hunter.hunter_id == hunter_id)
    }

    fn allocate_arrival_sequence(&mut self) -> u64 {
        let highest = self
            .waiting_queue
            .last()
            .map_or(0, |waiting| waiting.arrival_sequence);
        let sequence = self
            .next_arrival_sequence
            .max(highest.saturating_add(1))
            .max(1);
        self.next_arrival_sequence = sequence.saturating_add(1);
        sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hunter(hunter_id: u32) -> DurableHunterState {
        DurableHunterState {
            hunter_id,
            gold: 100,
            current_hp: 10,
            max_hp: 10,
            stamina: HunterServiceGauge::default(),
            satiety: HunterServiceGauge::default(),
            mood: HunterServiceGauge::default(),
            profile: DurableHunterProfile::migration_default(hunter_id),
        }
    }

    #[test]
    fn ninth_arrival_waits_and_banishment_promotes_fifo() {
        let mut roster = DurableHunterRosterState::default();
        for hunter_id in 1..=MAX_ACTIVE_TOWN_HUNTERS as u32 {
            assert!(matches!(
                roster.arrive(hunter(hunter_id)),
                Ok(HunterArrivalDisposition::Active { .. })
            ));
        }
        assert_eq!(
            roster.arrive(hunter(9)),
            Ok(HunterArrivalDisposition::Waiting { position: 0 })
        );
        assert_eq!(
            roster.arrive(hunter(10)),
            Ok(HunterArrivalDisposition::Waiting { position: 1 })
        );

        let result = roster.banish_active(4).unwrap();
        assert_eq!(result.promoted_hunter_id, Some(9));
        assert_eq!(roster.hunters.len(), MAX_ACTIVE_TOWN_HUNTERS);
        assert_eq!(roster.hunters.last().unwrap().hunter_id, 9);
        assert_eq!(roster.waiting_queue[0].hunter.hunter_id, 10);
        roster.validate().unwrap();
    }

    #[test]
    fn operational_roster_exercises_capacity_and_waiting_queue() {
        let roster = operational_migration_roster();
        assert!(roster.roster_resolved);
        assert!(roster.wallets_resolved);
        assert_eq!(roster.hunters.len(), MAX_ACTIVE_TOWN_HUNTERS);
        assert_eq!(roster.waiting_queue.len(), 1);
        assert_eq!(roster.waiting_queue[0].hunter.hunter_id, 9);
        roster.validate().unwrap();
    }

    #[test]
    fn duplicate_and_waiting_banishment_are_rejected_without_mutation() {
        let mut roster = DurableHunterRosterState::default();
        for hunter_id in 1..=9 {
            roster.arrive(hunter(hunter_id)).unwrap();
        }
        let before = roster.clone();
        assert_eq!(
            roster.arrive(hunter(1)),
            Err(HunterRosterError::DuplicateHunter)
        );
        assert_eq!(
            roster.banish_active(9),
            Err(HunterRosterError::ActiveHunterUnknown)
        );
        assert_eq!(roster, before);
    }

    #[test]
    fn invalid_over_capacity_or_non_fifo_state_fails_closed() {
        let mut over_capacity = DurableHunterRosterState {
            hunters: (1..=9).map(hunter).collect(),
            ..DurableHunterRosterState::default()
        };
        assert!(over_capacity.validate().is_err());
        assert!(over_capacity.arrive(hunter(10)).is_err());

        over_capacity.hunters.truncate(8);
        over_capacity.waiting_queue = vec![
            DurableWaitingHunter {
                arrival_sequence: 2,
                hunter: hunter(9),
            },
            DurableWaitingHunter {
                arrival_sequence: 1,
                hunter: hunter(10),
            },
        ];
        assert!(over_capacity.validate().is_err());
    }

    #[test]
    fn banishment_is_idempotent_by_command_id_and_conflicts_fail_closed() {
        let mut roster = DurableHunterRosterState::default();
        for hunter_id in 1..=9 {
            roster.arrive(hunter(hunter_id)).unwrap();
        }
        let command_id = Uuid::new_v4();
        let first = roster.banish_active_idempotent(command_id, 3).unwrap();
        let after_first = roster.clone();
        assert_eq!(roster.banish_active_idempotent(command_id, 3), Ok(first));
        assert_eq!(roster, after_first);
        assert_eq!(
            roster.banish_active_idempotent(command_id, 4),
            Err(HunterRosterError::CommandConflict)
        );
    }

    #[test]
    fn legacy_overflow_is_moved_to_fifo_without_dropping_hunters() {
        let mut roster = DurableHunterRosterState {
            hunters: (1..=10).map(hunter).collect(),
            next_arrival_sequence: 0,
            ..DurableHunterRosterState::default()
        };
        roster.upgrade_legacy_capacity();
        assert_eq!(roster.hunters.len(), 8);
        assert_eq!(
            roster
                .waiting_queue
                .iter()
                .map(|entry| entry.hunter.hunter_id)
                .collect::<Vec<_>>(),
            vec![9, 10]
        );
        assert_eq!(roster.next_arrival_sequence, 3);
        roster.validate().unwrap();
    }
}
