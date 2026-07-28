use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HunterEvidenceState {
    #[default]
    Unresolved,
    SchemaConfirmed,
    ValueCaptured,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterRuntimeState {
    pub source_dictionary_key: Option<String>,
    pub source_index: Option<i32>,
    pub source_job: Option<i32>,
    pub source_sub_job: Option<i32>,
    pub source_third_job: Option<i32>,
    pub source_fourth_job: Option<i32>,
    pub source_personality: Option<i32>,
    pub source_grade_rank_up: Option<i32>,
    pub source_dark_soul: Option<i64>,
    pub source_used_dark_soul: Option<i64>,
    pub source_used_job_trait: Option<i64>,
    pub appearance: Option<DurableHunterRuntimeAppearance>,
    pub status: Option<DurableHunterRuntimeStatus>,
    pub skills: Option<Vec<DurableHunterRuntimeSkill>>,
    pub inventory: Option<DurableHunterRuntimeInventory>,
    pub growth: Option<Vec<DurableHunterRuntimeGrowth>>,
    pub riding_pet: Option<DurableHunterRuntimeRidingPet>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableHunterRuntimeAppearance {
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
    pub ramble_pet_index: i32,
    pub ramble_pet_hidden: bool,
    pub hat_hidden: bool,
    pub costume_hat_hidden: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterRuntimeStatus {
    pub hp: i64,
    pub now_hp: i64,
    pub feel: f32,
    pub now_feel: f32,
    pub hungry: f32,
    pub now_hungry: f32,
    pub tire: f32,
    pub now_tire: f32,
    pub damage: i64,
    pub armor: i64,
    pub critical: i32,
    pub attack_speed: f32,
    pub dodge: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DurableHunterRuntimeSkill {
    pub dictionary_key: String,
    pub source_index: i32,
    pub skill_index: i32,
    pub cool_time: f64,
    pub level: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DurableHunterRuntimeInventory {
    pub items: Vec<DurableHunterRuntimeItem>,
    pub gear: Vec<DurableHunterRuntimeGear>,
    pub consumables: Vec<DurableHunterRuntimeConsumable>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DurableHunterRuntimeItem {
    pub dictionary_key: String,
    pub new_check: bool,
    pub source_index: i32,
    pub count: i64,
    pub reservation: i64,
    pub infinity_check: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DurableHunterRuntimeGear {
    pub dictionary_key: String,
    pub source_index: i32,
    pub gear_index: i32,
    pub inventory_index: i32,
    pub quality: i32,
    pub new_check: bool,
    pub level: i32,
    pub rating: i32,
    pub group: i32,
    pub plus_type: Vec<i32>,
    pub plus_value: Vec<i32>,
    pub minus_type: Vec<i32>,
    pub minus_value: Vec<i32>,
    pub additional_plus_type: Vec<i32>,
    pub additional_plus_value: Vec<i32>,
    pub additional_minus_type: Vec<i32>,
    pub additional_minus_value: Vec<i32>,
    pub buy_gold: i32,
    pub buy_date: String,
    pub buy_date_value: i64,
    pub quality_count: i32,
    pub option_count: i32,
    pub lock_count: i32,
    pub potential: i32,
    pub runes_index: i32,
    pub runes_value: i32,
    pub skill_runes_index: i32,
    pub skill_runes_value: i32,
    pub delete_count: i32,
    pub unidentified_option_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableHunterRuntimeConsumable {
    pub dictionary_key: String,
    pub total_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableHunterRuntimeGrowth {
    pub source_order: i16,
    pub property_level: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableHunterRuntimeRidingPet {
    pub pasture_index: i32,
    pub source_index: i32,
    pub master_index: String,
    pub rating: i32,
    pub skill_index: i32,
    pub trait_index: i32,
    pub trait_level: i32,
    pub use_soul: i32,
    pub use_growth_stone: i32,
    pub locked: bool,
}
