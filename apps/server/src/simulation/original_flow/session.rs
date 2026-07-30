use super::{
    binding, command_dispatch, hunter_roster_member, monster_world_snapshot, BindingConfidence,
    BottomMenuIntent, ClientCommand, DurableMonsterFieldConfig, DurableMonsterMapDensity,
    DurablePlayerAggregate, FieldSnapshot, HunterRosterSnapshot, MigrationFixtureCombatProjection,
    OriginalFlowCommandResult, OriginalFlowPlayerState, OriginalFlowSession, OriginalFlowSnapshot,
    OriginalScreen, Uuid, VillageSnapshot, DURABLE_PLAYER_SCHEMA_VERSION, MAX_ACTIVE_TOWN_HUNTERS,
    MIGRATION_FIXTURE_CONTENT_ID,
};

impl OriginalFlowSession {
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
        let message = command_dispatch::CommandDispatcher::dispatch(self, command, command_id)?;
        let operations = self.simulation.drain_operations();
        Some(OriginalFlowCommandResult {
            message,
            durable_state_changed: self.durable_state() != previous_state,
            operations,
        })
    }
}
