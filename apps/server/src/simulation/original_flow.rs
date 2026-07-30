#[path = "original_flow/building_commands.rs"]
mod building_commands;
#[path = "original_flow/building_domain.rs"]
mod building_domain;
#[path = "original_flow/building_projection.rs"]
mod building_projection;
#[path = "original_flow/command_dispatch.rs"]
mod command_dispatch;
#[path = "original_flow/crafting_commands.rs"]
mod crafting_commands;
#[path = "original_flow/durable_contracts.rs"]
mod durable_contracts;
#[path = "original_flow/enhancement_commands.rs"]
mod enhancement_commands;
#[path = "original_flow/hunt_commands.rs"]
mod hunt_commands;
#[path = "original_flow/hunter_domain.rs"]
mod hunter_domain;
#[path = "original_flow/hunter_trade_workflow.rs"]
mod hunter_trade_workflow;
#[path = "original_flow/navigation_commands.rs"]
mod navigation_commands;
#[path = "original_flow/product_services.rs"]
mod product_services;
#[path = "original_flow/projection.rs"]
mod projection;
#[path = "original_flow/runtime.rs"]
mod runtime;
#[path = "original_flow/session.rs"]
mod session;
#[path = "original_flow/session_restore.rs"]
mod session_restore;
#[path = "original_flow/shop_commands.rs"]
mod shop_commands;
#[path = "original_flow/skill_commands.rs"]
mod skill_commands;
#[path = "original_flow/snapshot_contracts.rs"]
mod snapshot_contracts;
#[cfg(test)]
#[path = "original_flow/test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "original_flow/tests.rs"]
mod tests;
#[path = "original_flow/trade_commands.rs"]
mod trade_commands;
#[path = "original_flow/world_projection_support.rs"]
mod world_projection_support;
#[path = "original_flow/world_snapshot.rs"]
mod world_snapshot;

use building_domain::*;
use durable_contracts::*;
pub use durable_contracts::{
    BottomMenuIntent, DurableBuilding, DurableBuildingState, DurableMaterialStock,
    DurableMonsterFieldConfig, DurableMonsterMapDensity, DurablePlayerAggregate,
    DurableProductServiceState, DurableProductServiceVisit, DurableProductStock,
    DurableTradeSettlement, Facing, OriginalFlowPlayerState, OriginalScreen,
    WorldEntityActionState, WorldEntityKind, WorldMode, DURABLE_PLAYER_SCHEMA_VERSION,
    MAX_GEAR_ENHANCEMENT_LEVEL, MIGRATION_FIXTURE_CONTENT_ID,
};
use hunter_domain::*;
pub use snapshot_contracts::*;
use world_projection_support::*;
use world_snapshot::*;

#[cfg(test)]
pub(crate) use test_support::test_authoritative_building_content;
#[cfg(test)]
use test_support::test_town_building_state;

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

use super::basic_hunter_skills::definition as basic_hunter_skill_definition;
use super::evidence_policy::{BUILDING_CAPABILITY_BLOCKERS, GEAR_ENHANCEMENT_BLOCKERS};
#[cfg(test)]
use super::hunter_roster::operational_migration_roster;
#[cfg(test)]
use super::hunter_roster::DurableHunterProfile;
use super::hunter_roster::{
    DurableGearEnhancementTask, DurableHunterRosterState, DurableHunterState,
    DurableHunterTradeTask, GearEnhancementTaskStatus, HunterRosterError,
    GEAR_ENHANCEMENT_WORKFLOW_VERSION, HUNTER_TRADE_WORKFLOW_VERSION, HUNT_TICKS_TO_RETURN,
    MAX_ACTIVE_TOWN_HUNTERS,
};
use super::monster_world::{TOWN_ROAM_ANCHORS, TOWN_ROAM_BOUNDS};
use super::product_service::{capacity_for_level, HunterServiceGauge, ServiceEffectKind};
#[cfg(test)]
use super::trading_post::ACTIVE_MATERIAL_REQUEST;
use super::trading_post::{
    material_catalog_stocks, material_difficulty_rating, settle_returning_hunters,
};
use super::{
    map_config, map_configs, ClientCommand, DurablePlayerState, FixtureCommand, HunterActionState,
    HunterAgentState, HunterEvidenceState, MonsterActionState, MonsterState, MonsterWorldState,
    NavigationObstacle, PendingOperation, ServerMessage, Simulation, WorldSnapshot,
    MONSTER_RULESET,
};

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
