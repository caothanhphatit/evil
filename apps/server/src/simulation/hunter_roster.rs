use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::combat_core::hit_resolution::OriginalEvasionCalculator;
use super::product_service::HunterServiceGauge;
use super::rng::DeterministicRng;
use super::DurableHunterRuntimeState;

pub const MAX_ACTIVE_TOWN_HUNTERS: usize = 8;
pub const MIGRATION_HUNTER_RELEASE_ID: &str = "migration.hunter-demo-v1";
pub const HUNT_TICKS_TO_RETURN: u32 = 10;
pub const FIXTURE_HUNT_ZONE_ID: &str = "migration-zone-1";
pub const ORDINARY_HUNT_REGION_IDS: [&str; 3] = ["map_new01", "background_08", "background_11"];
pub const GEAR_ENHANCEMENT_WORKFLOW_VERSION: u16 = 1;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterLoot {
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterOwnedItem {
    pub product_id: String,
    pub quantity: u32,
    #[serde(default)]
    pub enhancement_level: Option<u8>,
    #[serde(default)]
    pub gear_instance_id: Option<Uuid>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GearEnhancementTaskStatus {
    Traveling,
    WaitingForInteraction,
    Configuring,
    Processing,
    Result,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableGearEnhancementAttempt {
    pub attempt: u32,
    pub starting_level: u8,
    pub resulting_level: u8,
    pub succeeded: bool,
    pub gold_spent: u64,
    pub materials_spent: Vec<DurableHunterLoot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableGearEnhancementTask {
    /// Zero identifies rows written before enhancement workflow compatibility
    /// was tracked. Such tasks are released during session restore.
    pub workflow_version: u16,
    pub building_instance_id: String,
    pub status: GearEnhancementTaskStatus,
    pub interaction_x: i32,
    pub interaction_y: i32,
    pub selected_gear_instance_id: Option<Uuid>,
    pub selected_product_id: Option<String>,
    pub mode: Option<String>,
    pub target_level: Option<u8>,
    pub optional_material_ids: Vec<String>,
    pub attempts: Vec<DurableGearEnhancementAttempt>,
    pub spent_gold: u64,
    pub spent_materials: Vec<DurableHunterLoot>,
    pub final_level: Option<u8>,
    pub stop_reason: Option<String>,
    pub blockers: Vec<String>,
}

impl Default for DurableGearEnhancementTask {
    fn default() -> Self {
        Self {
            workflow_version: 0,
            building_instance_id: String::new(),
            status: GearEnhancementTaskStatus::Traveling,
            interaction_x: 0,
            interaction_y: 0,
            selected_gear_instance_id: None,
            selected_product_id: None,
            mode: None,
            target_level: None,
            optional_material_ids: Vec::new(),
            attempts: Vec::new(),
            spent_gold: 0,
            spent_materials: Vec::new(),
            final_level: None,
            stop_reason: None,
            blockers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterHuntState {
    /// This is an explicit web-rebuild-v1 fixture state, not a recovered legacy rule.
    pub status: String,
    pub zone_id: Option<String>,
    pub progress_ticks: u32,
    pub loot: Vec<DurableHunterLoot>,
    #[serde(default)]
    pub healing_potion_cooldown_ms: u64,
    #[serde(default)]
    pub gear_enhancement: Option<DurableGearEnhancementTask>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
    pub equipment_slots: Vec<DurableHunterEquipmentSlot>,
    pub action_state: String,
    pub animation_name: String,
    pub traits: Vec<DurableHunterTrait>,
    pub skills: Vec<DurableHunterSkill>,
}

impl DurableHunterProfile {
    /// `evasion_rate_bps` is the persisted total display value. Until the
    /// remaining native producer inputs are captured, the total is projected
    /// as the HunterData dodge component and the other native layers are zero.
    pub(crate) fn calc_dodge(&self) -> i32 {
        self.evasion_calculator().calc_dodge().unwrap_or(0)
    }

    pub(crate) fn evasion_calculator(&self) -> OriginalEvasionCalculator {
        let mut calculator = OriginalEvasionCalculator::default();
        if let Some(basis_points) = self.evasion_rate_bps {
            calculator
                .set_additive_source("profile_total_evasion", basis_points as f32 / 100.0_f32);
        }
        calculator
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterEquipmentSlot {
    pub slot_id: String,
    pub catalog_kind: String,
    pub catalog_index: u32,
    pub display_name: String,
    pub icon_path: String,
    pub presentation_gender: String,
    pub required_class_id: Option<String>,
    pub locked: bool,
    /// Operational test data only; never promoted into runtime evidence.
    pub evidence_state: String,
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
    #[serde(default)]
    pub cooldown_remaining_ms: u64,
}

impl DurableHunterProfile {
    pub fn migration_default(hunter_id: u32) -> Self {
        const CLASSES: [(&str, &str, &str); 5] = [
            ("h1", "Berserker", "H1"),
            ("h2", "Paladin", "H2"),
            ("h3", "Ranger", "H3"),
            ("h4", "Sorcerer", "H4"),
            ("h5", "DarkKnight", "H5"),
        ];
        const RARITIES: [(&str, &str); 5] = [
            ("normal", "Normal"),
            ("rare", "Rare"),
            ("superior", "Superior"),
            ("heroic", "Heroic"),
            ("legendary", "Legendary"),
        ];
        const PERSONALITIES: [&str; 33] = [
            "Strong",
            "Fast Runner",
            "Swift",
            "Fragile",
            "Sluggish",
            "Thickheaded",
            "Careless",
            "Stingy",
            "Charismatic",
            "Dead Weight",
            "Baggy Eyes",
            "Energetic",
            "Overweight",
            "Skinny",
            "Optimistic",
            "Pessimistic",
            "Coward",
            "Fearless",
            "Addict",
            "Scared of Hospital",
            "Heroic",
            "Rich",
            "Gambler",
            "Man of Steel",
            "Nimble",
            "Laggard",
            "Sharp",
            "Dull",
            "Ordinary",
            "YOLO",
            "Internet Troll",
            "Naughty",
            "Rude",
        ];
        let class = CLASSES[(hunter_id.saturating_sub(1) as usize) % CLASSES.len()];
        let rarity = RARITIES[(fixture_roll(hunter_id, 1) as usize) % RARITIES.len()];
        let level = 8 + fixture_roll(hunter_id, 2) % 23;
        let xp_to_next_level = 180 + u64::from(fixture_roll(hunter_id, 3) % 360);
        let xp = u64::from(fixture_roll(hunter_id, 4)) % xp_to_next_level;
        let attack = 38 + u64::from(fixture_roll(hunter_id, 5) % 115);
        let defense = 25 + u64::from(fixture_roll(hunter_id, 6) % 90);
        Self {
            content_release_id: MIGRATION_HUNTER_RELEASE_ID.to_owned(),
            display_name: format!("Hunter {hunter_id}"),
            class_id: class.0.to_owned(),
            class_name: class.1.to_owned(),
            visual_family: class.2.to_owned(),
            rarity_id: rarity.0.to_owned(),
            rarity_name: rarity.1.to_owned(),
            level,
            xp,
            xp_to_next_level: Some(xp_to_next_level),
            attack,
            defense,
            dps_milli: Some((attack * 1_000) + u64::from(fixture_roll(hunter_id, 7) % 900)),
            critical_rate_bps: Some(300 + fixture_roll(hunter_id, 8) % 1_200),
            attack_speed_milli: Some(1_200 + fixture_roll(hunter_id, 9) % 1_500),
            evasion_rate_bps: Some(100 + fixture_roll(hunter_id, 10) % 900),
            awakening: Some(DurableHunterProgress {
                current: fixture_roll(hunter_id, 11) % 3,
                maximum: 4,
            }),
            reincarnation: Some(DurableHunterProgress {
                current: fixture_roll(hunter_id, 12) % 3,
                maximum: 5,
            }),
            is_locked: Some(hunter_id % 4 == 0),
            characteristic_name: Some(
                PERSONALITIES[(fixture_roll(hunter_id, 13) as usize) % PERSONALITIES.len()]
                    .to_owned(),
            ),
            riding_pet_state_resolved: hunter_id % 3 == 0,
            equipment_slots: fixture_equipment(class.0, hunter_id),
            skills: fixture_basic_skills(class.0),
            action_state: "idle".to_owned(),
            animation_name: "hunter_stay".to_owned(),
            ..Self::default()
        }
    }
}

fn fixture_basic_skills(class_id: &str) -> Vec<DurableHunterSkill> {
    let rows = match class_id {
        "h1" => [("skill_h1_01", "Fury"), ("skill_h1_02", "War Cry")],
        "h2" => [("skill_h2_01", "Holy Light"), ("skill_h2_02", "Barrier")],
        "h3" => [("skill_h3_01", "Multishot"), ("skill_h3_02", "Dodge")],
        "h4" => [("skill_h4_01", "Thunderbolt"), ("skill_h4_02", "Ice Armor")],
        "h5" => [
            ("skill_h5_01", "Round Slash"),
            ("skill_h5_02", "Concentrate"),
        ],
        _ => return Vec::new(),
    };
    rows.into_iter()
        .map(|(skill_id, display_name)| DurableHunterSkill {
            skill_id: skill_id.to_owned(),
            display_name: display_name.to_owned(),
            icon_path: match skill_id {
                "skill_h1_01" => Some("sprites/skill_h1_01__1395.png".to_owned()),
                "skill_h1_02" => Some("sprites/skill_h1_02__5620.png".to_owned()),
                _ => None,
            },
            skill_level: 1,
            ready: true,
            ..DurableHunterSkill::default()
        })
        .collect()
}

fn fixture_equipment(class_id: &str, hunter_id: u32) -> Vec<DurableHunterEquipmentSlot> {
    let gender = if hunter_id % 2 == 0 { "male" } else { "female" };
    let weapon = match class_id {
        "h1" => (0, "Junk Sword", "weapon-0.png"),
        "h2" => (9, "Junk Hammer", "weapon-9.png"),
        "h3" => (18, "Junk Bow", "weapon-18.png"),
        "h4" => (27, "Junk Staff", "weapon-27.png"),
        "h5" => (252, "Rusty Spear", "weapon-252.png"),
        _ => return Vec::new(),
    };
    let row = |slot_id: &str,
               kind: &str,
               index: u32,
               name: &str,
               icon: &str,
               required_class_id: Option<&str>| DurableHunterEquipmentSlot {
        slot_id: slot_id.to_owned(),
        catalog_kind: kind.to_owned(),
        catalog_index: index,
        display_name: name.to_owned(),
        icon_path: format!("/content/releases/evil-hunter-1.411/gear-icons/{icon}"),
        presentation_gender: gender.to_owned(),
        required_class_id: required_class_id.map(str::to_owned),
        locked: false,
        evidence_state: "web_rebuild_test_fixture".to_owned(),
    };
    vec![
        row(
            "gloves",
            "gloves",
            0,
            "Tattered Gloves",
            "gloves-0.png",
            None,
        ),
        row("boots", "boots", 0, "Tattered Shoes", "boots-0.png", None),
        row(
            "weapon",
            "weapon",
            weapon.0,
            weapon.1,
            weapon.2,
            Some(class_id),
        ),
        row("armor", "armor", 0, "Tattered Armor", "armor-0.png", None),
    ]
}

/// Deterministic server-side fixture RNG. It is not a recovered original-game roll.
fn fixture_roll(hunter_id: u32, stream: u32) -> u32 {
    let mut value = u64::from(hunter_id) ^ (u64::from(stream) << 32) ^ 0x9e37_79b9_7f4a_7c15;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (value ^ (value >> 31)) as u32
}

pub fn upgrade_operational_fixture_roster(roster: &mut DurableHunterRosterState) -> bool {
    let mut upgraded = false;
    for hunter in roster
        .hunters
        .iter_mut()
        .chain(roster.waiting_queue.iter_mut().map(|row| &mut row.hunter))
    {
        let is_migration_fixture = hunter.profile.content_release_id == MIGRATION_HUNTER_RELEASE_ID;
        let old_fixture = is_migration_fixture
            && hunter.profile.class_id == "h1"
            && hunter.profile.visual_family == "H1"
            && hunter.profile.level == 1
            && hunter.profile.attack == 10
            && hunter.profile.defense == 10;
        if !old_fixture {
            if is_migration_fixture && hunter.profile.equipment_slots.is_empty() {
                hunter.profile.equipment_slots =
                    fixture_equipment(&hunter.profile.class_id, hunter.hunter_id);
                upgraded = true;
            }
            if is_migration_fixture && hunter.profile.skills.is_empty() {
                hunter.profile.skills = fixture_basic_skills(&hunter.profile.class_id);
                upgraded = true;
            }
            continue;
        }
        let skills = hunter.profile.skills.clone();
        let action_state = hunter.profile.action_state.clone();
        let animation_name = hunter.profile.animation_name.clone();
        hunter.profile = DurableHunterProfile::migration_default(hunter.hunter_id);
        if !skills.is_empty() {
            hunter.profile.skills = skills;
        }
        hunter.profile.action_state = action_state;
        hunter.profile.animation_name = animation_name;
        upgraded = true;
    }
    upgraded
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
        let class_id = ["h1", "h2", "h3", "h4", "h5"][(hunter_id as usize - 1) % 5];
        let (max_hp, current_hp) = fixture_hp_values(hunter_id, class_id);
        let (stamina_max, satiety_max, mood_max) = fixture_gauge_maxima(hunter_id);
        let hunter = DurableHunterState {
            hunter_id,
            gold: 1_000,
            current_hp,
            max_hp,
            stamina: HunterServiceGauge {
                current: stamina_max.saturating_sub(10 + u64::from((hunter_id * 5) % 31)),
                maximum: stamina_max,
            },
            satiety: HunterServiceGauge {
                current: satiety_max.saturating_sub(12 + u64::from((hunter_id * 7) % 36)),
                maximum: satiety_max,
            },
            mood: HunterServiceGauge {
                current: mood_max.saturating_sub(8 + u64::from((hunter_id * 4) % 27)),
                maximum: mood_max,
            },
            profile: DurableHunterProfile::migration_default(hunter_id),
            runtime: DurableHunterRuntimeState::default(),
            hunt: DurableHunterHuntState::default(),
            owned_items: Vec::new(),
        };
        roster
            .arrive(hunter)
            .expect("fixed migration roster satisfies capacity invariants");
    }
    roster
}

/// Seeds the starter roster for a newly registered local account. This is a
/// rebuild rule: the original starter roll is unresolved, so the server owns a
/// deterministic RNG stream derived from the account UUID and never rerolls it
/// on reconnect.
pub fn new_account_roster(player_token: Uuid) -> DurableHunterRosterState {
    const CLASSES: [(&str, &str, &str); 5] = [
        ("h1", "Berserker", "H1"),
        ("h2", "Paladin", "H2"),
        ("h3", "Ranger", "H3"),
        ("h4", "Sorcerer", "H4"),
        ("h5", "DarkKnight", "H5"),
    ];
    const RARITIES: [(&str, &str); 5] = [
        ("normal", "Normal"),
        ("rare", "Rare"),
        ("superior", "Superior"),
        ("heroic", "Heroic"),
        ("legendary", "Legendary"),
    ];
    let seed = u64::from_le_bytes(player_token.as_bytes()[..8].try_into().unwrap());
    let mut rng = DeterministicRng::new(seed);
    let mut roster = DurableHunterRosterState {
        roster_resolved: true,
        wallets_resolved: true,
        ..DurableHunterRosterState::default()
    };
    for hunter_id in 1..=5 {
        let class = CLASSES[rng.range_inclusive(0, 4) as usize];
        let rarity = RARITIES[rng.range_inclusive(0, 4) as usize];
        let base_hp = if matches!(class.0, "h3" | "h4") {
            5_600
        } else {
            6_000
        };
        let max_hp = base_hp + rng.range_inclusive(0, 200) as u64;
        let current_hp = max_hp.saturating_sub(rng.range_inclusive(80, 500) as u64);
        let stamina_max = rng.range_inclusive(90, 150) as u64;
        let satiety_max = rng.range_inclusive(95, 160) as u64;
        let mood_max = rng.range_inclusive(85, 155) as u64;
        let mut profile = DurableHunterProfile::migration_default(hunter_id);
        profile.class_id = class.0.to_owned();
        profile.class_name = class.1.to_owned();
        profile.visual_family = class.2.to_owned();
        profile.rarity_id = rarity.0.to_owned();
        profile.rarity_name = rarity.1.to_owned();
        profile.level = 1 + rng.range_inclusive(0, 4) as u32;
        profile.xp = 0;
        profile.equipment_slots = fixture_equipment(class.0, hunter_id);
        profile.skills = fixture_basic_skills(class.0);
        let hunter = DurableHunterState {
            hunter_id,
            gold: 0,
            current_hp,
            max_hp,
            stamina: HunterServiceGauge {
                current: stamina_max.saturating_sub(rng.range_inclusive(0, 20) as u64),
                maximum: stamina_max,
            },
            satiety: HunterServiceGauge {
                current: satiety_max.saturating_sub(rng.range_inclusive(0, 20) as u64),
                maximum: satiety_max,
            },
            mood: HunterServiceGauge {
                current: mood_max.saturating_sub(rng.range_inclusive(0, 20) as u64),
                maximum: mood_max,
            },
            profile,
            runtime: DurableHunterRuntimeState::default(),
            hunt: DurableHunterHuntState::default(),
            owned_items: Vec::new(),
        };
        roster
            .arrive(hunter)
            .expect("new-account starter roster satisfies capacity invariants");
    }
    roster
}

// These are deterministic rebuild fixtures, not claims about the unresolved
// original constructor RNG. HP stays inside the recovered class bounds while
// the other three native current/max pairs remain visibly non-percent gauges.
fn fixture_hp_values(hunter_id: u32, class_id: &str) -> (u64, u64) {
    let base = if matches!(class_id, "h3" | "h4") {
        5_600
    } else {
        6_000
    };
    let maximum = base + u64::from((hunter_id * 37) % 201);
    let current = maximum.saturating_sub(180 + u64::from((hunter_id * 53) % 420));
    (current, maximum)
}

fn fixture_gauge_maxima(hunter_id: u32) -> (u64, u64, u64) {
    (
        90 + u64::from((hunter_id * 17) % 61),
        95 + u64::from((hunter_id * 23) % 66),
        85 + u64::from((hunter_id * 29) % 71),
    )
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterRosterState {
    pub roster_resolved: bool,
    pub wallets_resolved: bool,
    /// Active town roster in stable slot order. Slots are always compact after a banishment.
    pub hunters: Vec<DurableHunterState>,
    pub waiting_queue: Vec<DurableWaitingHunter>,
    pub next_arrival_sequence: u64,
    pub banish_commands: BTreeMap<Uuid, HunterBanishment>,
    #[serde(default)]
    pub hunt_commands: BTreeMap<Uuid, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub runtime: DurableHunterRuntimeState,
    #[serde(default)]
    pub hunt: DurableHunterHuntState,
    #[serde(default)]
    pub owned_items: Vec<DurableHunterOwnedItem>,
}

impl DurableHunterHuntState {
    pub fn is_idle(&self) -> bool {
        self.status.is_empty() || self.status == "idle"
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub fn active_mut(
        &mut self,
        hunter_id: u32,
    ) -> Result<&mut DurableHunterState, HunterRosterError> {
        self.hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .ok_or(HunterRosterError::ActiveHunterUnknown)
    }

    pub fn assign_hunt(&mut self, hunter_id: u32, zone_id: &str) -> Result<(), HunterRosterError> {
        if zone_id.trim().is_empty() {
            return Err(HunterRosterError::InvalidState("hunt zone is empty"));
        }
        if zone_id != FIXTURE_HUNT_ZONE_ID && !ORDINARY_HUNT_REGION_IDS.contains(&zone_id) {
            return Err(HunterRosterError::InvalidState("hunt zone is unavailable"));
        }
        let hunter = self.active_mut(hunter_id)?;
        if hunter.hunt.gear_enhancement.is_some() {
            return Err(HunterRosterError::InvalidState(
                "hunter has an active enhancement task",
            ));
        }
        if !hunter.hunt.is_idle() && hunter.hunt.status != "hunting" {
            return Err(HunterRosterError::InvalidState("hunter is not idle"));
        }
        let existing_loot = std::mem::take(&mut hunter.hunt.loot);
        let healing_potion_cooldown_ms = hunter.hunt.healing_potion_cooldown_ms;
        hunter.hunt = DurableHunterHuntState {
            status: "hunting".to_owned(),
            zone_id: Some(zone_id.to_owned()),
            progress_ticks: 0,
            loot: existing_loot,
            healing_potion_cooldown_ms,
            gear_enhancement: None,
        };
        hunter.profile.action_state = "hunting".to_owned();
        hunter.profile.animation_name = "hunter_walk".to_owned();
        Ok(())
    }

    pub fn advance_hunt(&mut self, hunter_id: u32, ticks: u32) -> Result<(), HunterRosterError> {
        let hunter = self.active_mut(hunter_id)?;
        if hunter.hunt.status != "hunting" {
            return Err(HunterRosterError::InvalidState("hunter is not hunting"));
        }
        let remaining = HUNT_TICKS_TO_RETURN.saturating_sub(hunter.hunt.progress_ticks);
        let advanced = ticks.min(remaining);
        hunter.hunt.progress_ticks = hunter.hunt.progress_ticks.saturating_add(advanced);
        if hunter.hunt.progress_ticks >= HUNT_TICKS_TO_RETURN {
            hunter.hunt.status = "returning".to_owned();
            hunter.hunt.loot.push(DurableHunterLoot {
                item_id: "material:1".to_owned(),
                quantity: 1,
            });
            hunter.profile.action_state = "returning".to_owned();
            hunter.profile.animation_name = "hunter_walk".to_owned();
        }
        Ok(())
    }

    pub fn return_from_hunt(&mut self, hunter_id: u32) -> Result<(), HunterRosterError> {
        let hunter = self.active_mut(hunter_id)?;
        if hunter.hunt.status != "returning" {
            return Err(HunterRosterError::InvalidState(
                "hunt is not ready to return",
            ));
        }
        hunter.hunt.status = "idle".to_owned();
        hunter.profile.action_state = "idle".to_owned();
        hunter.profile.animation_name = "hunter_stay".to_owned();
        Ok(())
    }

    pub fn defeat_hunter(&mut self, hunter_id: u32) -> Result<(), HunterRosterError> {
        let hunter = self.active_mut(hunter_id)?;
        hunter.current_hp = 0;
        hunter.hunt.status = "dead".to_owned();
        hunter.profile.action_state = "dead".to_owned();
        hunter.profile.animation_name = "hunter_die".to_owned();
        Ok(())
    }

    pub fn revive_hunter(&mut self, hunter_id: u32) -> Result<(), HunterRosterError> {
        let hunter = self.active_mut(hunter_id)?;
        if hunter.hunt.status != "dead" {
            return Err(HunterRosterError::InvalidState("hunter is not dead"));
        }
        hunter.current_hp = hunter.max_hp;
        hunter.hunt = DurableHunterHuntState {
            status: "idle".to_owned(),
            ..DurableHunterHuntState::default()
        };
        hunter.profile.action_state = "idle".to_owned();
        hunter.profile.animation_name = "hunter_stay".to_owned();
        Ok(())
    }
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
        for hunter in self
            .hunters
            .iter()
            .chain(self.waiting_queue.iter().map(|waiting| &waiting.hunter))
        {
            let mut slots = HashSet::new();
            for equipment in &hunter.profile.equipment_slots {
                if !slots.insert(equipment.slot_id.as_str()) {
                    return Err(HunterRosterError::InvalidState(
                        "equipment slot appears more than once",
                    ));
                }
                if equipment.evidence_state != "web_rebuild_test_fixture"
                    || !matches!(equipment.presentation_gender.as_str(), "female" | "male")
                {
                    return Err(HunterRosterError::InvalidState(
                        "equipment fixture evidence is invalid",
                    ));
                }
                if equipment.catalog_kind == "weapon"
                    && equipment.required_class_id.as_deref()
                        != Some(hunter.profile.class_id.as_str())
                {
                    return Err(HunterRosterError::InvalidState(
                        "fixture weapon does not match hunter class",
                    ));
                }
            }
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

    #[test]
    fn new_account_seed_has_five_rng_hunters_and_is_replayable() {
        let player = Uuid::from_u128(0x1234);
        let first = new_account_roster(player);
        let second = new_account_roster(player);
        assert_eq!(first, second);
        assert_eq!(first.hunters.len(), 5);
        assert!(first.waiting_queue.is_empty());
        assert!(first.validate().is_ok());
        assert!(first.hunters.iter().all(|hunter| hunter.gold == 0));
        assert!(first
            .hunters
            .iter()
            .any(|hunter| hunter.profile.class_id != "h1"));
    }

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
            runtime: DurableHunterRuntimeState::default(),
            hunt: DurableHunterHuntState::default(),
            owned_items: Vec::new(),
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
    fn operational_roster_uses_deterministic_diverse_server_fixture_rolls() {
        let first = operational_migration_roster();
        let second = operational_migration_roster();
        assert_eq!(first, second);
        let families = first
            .hunters
            .iter()
            .map(|hunter| hunter.profile.visual_family.as_str())
            .collect::<HashSet<_>>();
        let personalities = first
            .hunters
            .iter()
            .filter_map(|hunter| hunter.profile.characteristic_name.as_deref())
            .collect::<HashSet<_>>();
        assert_eq!(families.len(), 5);
        assert!(personalities.len() >= 4);
        assert!(first
            .hunters
            .iter()
            .all(|hunter| hunter.profile.xp_to_next_level.is_some()));
        for hunter in &first.hunters {
            assert_eq!(hunter.profile.equipment_slots.len(), 4);
            let weapon = hunter
                .profile
                .equipment_slots
                .iter()
                .find(|equipment| equipment.slot_id == "weapon")
                .unwrap();
            assert_eq!(
                weapon.required_class_id.as_deref(),
                Some(hunter.profile.class_id.as_str())
            );
            assert_eq!(weapon.evidence_state, "web_rebuild_test_fixture");
            assert_eq!(
                weapon.presentation_gender,
                if hunter.hunter_id % 2 == 0 {
                    "male"
                } else {
                    "female"
                }
            );
        }
    }

    #[test]
    fn fixture_weapons_follow_the_packaged_base_job_catalog() {
        let expected = [
            ("h1", 0, "weapon-0.png"),
            ("h2", 9, "weapon-9.png"),
            ("h3", 18, "weapon-18.png"),
            ("h4", 27, "weapon-27.png"),
            ("h5", 252, "weapon-252.png"),
        ];
        for hunter_id in 1..=5 {
            let profile = DurableHunterProfile::migration_default(hunter_id);
            let weapon = profile
                .equipment_slots
                .iter()
                .find(|equipment| equipment.slot_id == "weapon")
                .unwrap();
            let expected = expected[(hunter_id - 1) as usize];
            assert_eq!(
                (profile.class_id.as_str(), weapon.catalog_index),
                (expected.0, expected.1)
            );
            assert!(weapon.icon_path.ends_with(expected.2));
        }
    }

    #[test]
    fn fixture_equipment_references_existing_packaged_catalog_rows() {
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/content/releases/evil-hunter-1.411/gear-catalog.json"
        ))
        .unwrap();
        let rows = catalog["rows"].as_array().unwrap();

        for hunter_id in 1..=5 {
            for equipment in DurableHunterProfile::migration_default(hunter_id).equipment_slots {
                assert!(rows.iter().any(|row| {
                    row["kind"] == equipment.catalog_kind
                        && row["index"] == equipment.catalog_index
                        && row["name"] == equipment.display_name
                        && row["iconPath"] == equipment.icon_path
                }));
            }
        }
    }

    #[test]
    fn fixture_equipment_validation_rejects_class_mismatches() {
        let mut roster = operational_migration_roster();
        let weapon = roster.hunters[0]
            .profile
            .equipment_slots
            .iter_mut()
            .find(|equipment| equipment.slot_id == "weapon")
            .unwrap();
        weapon.required_class_id = Some("h5".to_owned());

        assert_eq!(
            roster.validate(),
            Err(HunterRosterError::InvalidState(
                "fixture weapon does not match hunter class"
            ))
        );
    }

    #[test]
    fn old_uniform_fixture_profiles_upgrade_once_without_losing_learned_skills() {
        let mut roster = operational_migration_roster();
        for hunter in &mut roster.hunters {
            hunter.profile.class_id = "h1".to_owned();
            hunter.profile.class_name = "Berserker".to_owned();
            hunter.profile.visual_family = "H1".to_owned();
            hunter.profile.level = 1;
            hunter.profile.attack = 10;
            hunter.profile.defense = 10;
        }
        roster.hunters[0].profile.skills.push(DurableHunterSkill {
            skill_id: "skill_h1_01".to_owned(),
            display_name: "Fury".to_owned(),
            skill_level: 1,
            ..DurableHunterSkill::default()
        });
        assert!(upgrade_operational_fixture_roster(&mut roster));
        assert_eq!(roster.hunters[0].profile.skills[0].skill_id, "skill_h1_01");
        assert_eq!(roster.hunters[1].profile.visual_family, "H2");
        assert!(!upgrade_operational_fixture_roster(&mut roster));
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

    #[test]
    fn hunt_flow_is_server_advanced_and_uses_the_fixture_whitelist() {
        let mut roster = operational_migration_roster();
        assert!(roster.assign_hunt(1, "arbitrary-zone").is_err());
        roster.assign_hunt(1, FIXTURE_HUNT_ZONE_ID).unwrap();
        roster.advance_hunt(1, HUNT_TICKS_TO_RETURN - 1).unwrap();
        assert_eq!(roster.hunters[0].hunt.status, "hunting");
        assert!(roster.hunters[0].hunt.loot.is_empty());
        roster.advance_hunt(1, 1).unwrap();
        assert_eq!(roster.hunters[0].hunt.status, "returning");
        assert_eq!(
            roster.hunters[0].hunt.loot,
            vec![DurableHunterLoot {
                item_id: "material:1".to_owned(),
                quantity: 1
            }]
        );
        roster.return_from_hunt(1).unwrap();
        assert!(roster.hunters[0].hunt.is_idle());
    }

    #[test]
    fn defeat_and_revive_are_authoritative_and_persistable() {
        let mut roster = operational_migration_roster();
        roster.defeat_hunter(1).unwrap();
        assert_eq!(roster.hunters[0].current_hp, 0);
        assert_eq!(roster.hunters[0].hunt.status, "dead");
        assert!(roster.revive_hunter(2).is_err());
        roster.revive_hunter(1).unwrap();
        assert_eq!(roster.hunters[0].current_hp, roster.hunters[0].max_hp);
        let encoded = serde_json::to_string(&roster).unwrap();
        let restored: DurableHunterRosterState = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, roster);
    }

    #[test]
    fn profile_total_evasion_drives_calc_dodge_and_dynamic_sources() {
        let mut profile = DurableHunterProfile::migration_default(1);
        profile.evasion_rate_bps = Some(760);
        assert_eq!(profile.calc_dodge(), 8);

        let mut calculator = profile.evasion_calculator();
        calculator.set_additive_source("temporary_buff", 2.5);
        assert_eq!(calculator.calc_dodge(), Ok(10));
        calculator.remove_additive_source("temporary_buff");
        assert_eq!(calculator.calc_dodge(), Ok(8));
    }
}
