use super::{
    default_material_stocks, enhancement_task_terminal, hunter_trade_workflow,
    is_enhancement_action_state, release_hunter_from_enhancement, Arc,
    AuthoritativeBuildingContent, DurablePlayerAggregate, DurableProductServiceVisit, HashSet,
    MonsterWorldState, OriginalFlowSession, OriginalScreen, ServiceEffectKind, Simulation,
    GEAR_ENHANCEMENT_WORKFLOW_VERSION, HUNTER_TRADE_WORKFLOW_VERSION,
};

#[cfg(test)]
use super::{
    test_authoritative_building_content, test_town_building_state, OriginalFlowPlayerState,
};

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
        // Hunter roster is now a client-presentational overlay. Persisted
        // sessions from the removed server screen resume in the village.
        if state.navigation.screen == OriginalScreen::HunterRoster {
            state.navigation.screen = OriginalScreen::Village;
        }
        if state.schema_version < 3 && state.buildings.hunter_materials == 0 {
            state.buildings.hunter_materials = 20;
        }
        if state.schema_version < 4 || state.buildings.material_stocks.is_empty() {
            state.buildings.material_stocks = default_material_stocks();
        }
        if state.schema_version < 11 {
            state.hunter_roster.upgrade_legacy_capacity();
        }
        let trading_post_instances = state
            .buildings
            .buildings
            .iter()
            .filter(|building| building.id == "build_3")
            .map(|building| building.instance_id.clone())
            .collect::<HashSet<_>>();
        // Older runtime builds accidentally copied collected gold drops into the
        // material inventory after already crediting the Hunter wallet.
        for hunter in state.hunter_roster.hunters.iter_mut().chain(
            state
                .hunter_roster
                .waiting_queue
                .iter_mut()
                .map(|waiting| &mut waiting.hunter),
        ) {
            hunter.hunt.loot.retain(|loot| loot.item_id != "gold");
            if hunter.hunt.status == "returning_for_infirmary" && hunter.hunt.zone_id.is_none() {
                hunter.hunt.status = "idle".to_owned();
                hunter.profile.action_state = "idle".to_owned();
                hunter.profile.animation_name = "hunter_stay".to_owned();
            }
            let incompatible_or_terminal_enhancement =
                hunter.hunt.gear_enhancement.as_ref().is_some_and(|task| {
                    task.workflow_version != GEAR_ENHANCEMENT_WORKFLOW_VERSION
                        || enhancement_task_terminal(task)
                });
            let orphaned_enhancement_action = hunter.hunt.gear_enhancement.is_none()
                && is_enhancement_action_state(&hunter.profile.action_state);
            if incompatible_or_terminal_enhancement || orphaned_enhancement_action {
                release_hunter_from_enhancement(hunter);
            }
            hunter_trade_workflow::HunterTradeWorkflow::normalize_restored(hunter, |task| {
                task.workflow_version == HUNTER_TRADE_WORKFLOW_VERSION
                    && !task.command_id.is_nil()
                    && !task.building_instance_id.is_empty()
                    && trading_post_instances.contains(task.building_instance_id.as_str())
            });
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
        let monster_densities = state.monster_field_config.normalized_densities();
        let mut monster_world = MonsterWorldState::with_densities(
            monster_densities
                .iter()
                .map(|density| (density.map_id.as_str(), density.density_level)),
        );
        monster_world.restore_hunter_runtime(&state.hunter_roster, state.hunter_world_runtime);
        let simulation = Simulation::from_state(seed, state.migration_fixture_combat);
        let combat_snapshot = simulation.snapshot();
        Self {
            state: state.navigation,
            simulation,
            combat_snapshot,
            selected_entity_id: None,
            visual_tick: 0,
            simulation_remainder_ns: 0,
            buildings: state.buildings,
            hunter_roster: state.hunter_roster,
            product_services: state.product_services,
            monster_world,
            building_content,
        }
    }
}
