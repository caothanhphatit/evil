use super::{
    capacity_for_level, enhancement_task_terminal, release_hunter_from_enhancement,
    town_building_interaction_point, town_navigation_obstacles, town_revival_point, BaseBuildingId,
    HashMap, HashSet, NavigationObstacle, OriginalFlowSession, OriginalFlowSnapshot,
    OriginalFlowTickResult, OriginalScreen, PendingOperation, TOWN_ROAM_BOUNDS,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct AutonomousServicePlan {
    hunter_id: u32,
    building_instance_id: String,
    product_id: Option<String>,
    destination: (i32, i32),
}

impl OriginalFlowSession {
    pub fn advance_simulation_tick(&mut self) -> Option<OriginalFlowTickResult> {
        self.advance_simulation_step(100_000_000)
    }

    /// Advances the deterministic 10 Hz gameplay domain using scheduler time.
    /// Network cadence may vary without changing movement, combat, or cooldown speed.
    pub fn advance_simulation_step(&mut self, elapsed_ns: u64) -> Option<OriginalFlowTickResult> {
        if matches!(
            self.state.screen,
            OriginalScreen::Boot | OriginalScreen::HunterRoster
        ) {
            return None;
        }
        const DOMAIN_STEP_NS: u64 = 100_000_000;
        self.simulation_remainder_ns = self.simulation_remainder_ns.saturating_add(elapsed_ns);
        let step_count = self.simulation_remainder_ns / DOMAIN_STEP_NS;
        self.simulation_remainder_ns %= DOMAIN_STEP_NS;
        if step_count == 0 {
            return None;
        }

        let mut operations = Vec::new();
        for _ in 0..step_count {
            operations.extend(self.advance_domain_tick());
        }
        Some(OriginalFlowTickResult {
            world: self.world_projection(),
            simulation_tick: self.monster_world.tick.max(self.combat_snapshot.tick),
            operations,
        })
    }

    fn advance_domain_tick(&mut self) -> Vec<PendingOperation> {
        if self.state.screen == OriginalScreen::Field {
            self.combat_snapshot = self.simulation.step();
        }
        self.refresh_skill_cooldowns(100);
        for hunter in &mut self.hunter_roster.hunters {
            hunter.hunt.healing_potion_cooldown_ms =
                hunter.hunt.healing_potion_cooldown_ms.saturating_sub(100);
        }
        self.auto_cast_ready_hunter_skills();
        self.visual_tick = self.visual_tick.wrapping_add(1);
        self.decay_hunter_service_gauges();
        let navigation_obstacles =
            town_navigation_obstacles(&self.buildings.buildings, &self.building_content.catalog);
        let revival_point = town_revival_point(
            &self.buildings.buildings,
            &self.building_content.catalog,
            &navigation_obstacles,
        );
        self.apply_autonomous_hunter_healing_policy();
        self.prepare_autonomous_service_requests();
        let autonomous_service_plans = self.autonomous_service_plans(&navigation_obstacles);
        self.mark_unavailable_service_waiters(&autonomous_service_plans);
        let mut autonomous_town_destinations = autonomous_service_plans
            .iter()
            .map(|plan| (plan.hunter_id, plan.destination))
            .collect::<HashMap<_, _>>();
        autonomous_town_destinations.extend(self.pending_trade_destinations());
        for hunter in &mut self.hunter_roster.hunters {
            let terminal_enhancement = hunter
                .hunt
                .gear_enhancement
                .as_ref()
                .is_some_and(enhancement_task_terminal);
            if terminal_enhancement {
                release_hunter_from_enhancement(hunter);
            }
        }
        let mut operations = self.monster_world.tick_with_obstacles(
            &mut self.hunter_roster,
            &navigation_obstacles,
            revival_point,
            &autonomous_town_destinations,
        );
        self.start_arrived_autonomous_services(&autonomous_service_plans);
        self.advance_legacy_hunter_hunts(1);
        self.settle_arrived_hunter_trades();
        self.auto_sell_requested_hunter_loot();
        if self.state.screen == OriginalScreen::Field {
            let mut combined = self.simulation.drain_operations();
            combined.append(&mut operations);
            combined
        } else {
            operations
        }
    }

    /// Applies the explicit rebuild healing policy at the simulation boundary.
    ///
    /// The original package exposes potion slots and HP/service methods, but no
    /// captured threshold or autonomous decision body. Until that evidence is
    /// recovered, the product rule is: below 10% HP, consume a Healing Potion
    /// first; when none is owned, leave the hunting region and return to town for
    /// the Infirmary route. This mutation is server-owned and deterministic.
    pub(super) fn apply_autonomous_hunter_healing_policy(&mut self) {
        for hunter in &mut self.hunter_roster.hunters {
            if hunter.current_hp == 0
                || hunter.max_hp == 0
                || hunter.profile.action_state == "entering_region"
                || u128::from(hunter.current_hp) * 100 >= u128::from(hunter.max_hp) * 10
                || hunter.hunt.gear_enhancement.is_some()
                || hunter.hunt.pending_trade.is_some()
                || hunter.hunt.healing_potion_cooldown_ms > 0
                || self
                    .product_services
                    .visits
                    .iter()
                    .any(|visit| visit.hunter_id == hunter.hunter_id)
            {
                continue;
            }

            let potion = hunter
                .owned_items
                .iter_mut()
                .filter_map(|item| {
                    #[cfg(not(test))]
                    let definition = self
                        .building_content
                        .gameplay
                        .consumable_product(&item.product_id)?;
                    #[cfg(test)]
                    let definition = self
                        .building_content
                        .gameplay
                        .consumable_product(&item.product_id)
                        .or_else(|| test_consumable_products().get(&item.product_id))?;
                    (definition.consumable_index == 0 && item.quantity > 0).then_some((
                        definition.level,
                        definition.keep_value,
                        definition.cooldown_ms,
                        item,
                    ))
                })
                .max_by_key(|(level, _, _, _)| *level);
            if let Some((_, keep_value, cooldown_ms, item)) = potion {
                item.quantity = item.quantity.saturating_sub(1);
                hunter.current_hp = hunter
                    .current_hp
                    .saturating_add(keep_value)
                    .min(hunter.max_hp);
                hunter.profile.action_state = "using_healing_potion".to_owned();
                hunter.profile.animation_name = "hunter_stay".to_owned();
                hunter.hunt.healing_potion_cooldown_ms = cooldown_ms;
                continue;
            }

            // No consumable is available. Unassign the farm region so the world
            // agent returns to town; service stock/payment remains authoritative
            // and can be started by the Infirmary flow once it arrives.
            let leaving_field = hunter.hunt.zone_id.take().is_some();
            // The exact autonomous service-selection body is still unresolved.
            // Preserve an active return until the world actor reaches town.
            // Rewriting it to idle on the following tick made the service route
            // disappear before the Hunter could reach the Infirmary.
            if leaving_field {
                hunter.hunt.status = "idle".to_owned();
            }
            hunter.profile.action_state =
                if super::hunter_service_gauge(hunter, super::ServiceEffectKind::Hp)
                    .needs_autonomous_service()
                {
                    "returning_for_infirmary"
                } else {
                    "returning_for_service"
                }
                .to_owned();
            hunter.profile.animation_name = "hunter_walk".to_owned();
        }
    }

    fn prepare_autonomous_service_requests(&mut self) {
        for hunter in &mut self.hunter_roster.hunters {
            let critical = hunter.current_hp > 0
                && (super::hunter_service_gauge(hunter, super::ServiceEffectKind::Hp)
                    .needs_autonomous_service()
                    || hunter.stamina.needs_autonomous_service()
                    || hunter.satiety.needs_autonomous_service()
                    || hunter.mood.needs_autonomous_service());
            if !critical
                || hunter.profile.action_state == "serving"
                || hunter.hunt.pending_trade.is_some()
                || hunter.hunt.gear_enhancement.is_some()
            {
                continue;
            }
            hunter.hunt.zone_id = None;
            hunter.hunt.status = "idle".to_owned();
            hunter.profile.action_state =
                if super::hunter_service_gauge(hunter, super::ServiceEffectKind::Hp)
                    .needs_autonomous_service()
                {
                    "returning_for_infirmary"
                } else {
                    "returning_for_service"
                }
                .to_owned();
            hunter.profile.animation_name = "hunter_walk".to_owned();
        }
    }

    fn decay_hunter_service_gauges(&mut self) {
        for hunter in &mut self.hunter_roster.hunters {
            if hunter.hunt.status != "hunting" || hunter.profile.action_state == "serving" {
                continue;
            }
            if self.visual_tick % 10 == 0 {
                hunter.stamina.current = hunter.stamina.current.saturating_sub(1);
            }
            if self.visual_tick % 15 == 0 {
                hunter.satiety.current = hunter.satiety.current.saturating_sub(1);
            }
            if self.visual_tick % 20 == 0 {
                hunter.mood.current = hunter.mood.current.saturating_sub(1);
            }
        }
    }

    fn autonomous_service_plans(
        &self,
        obstacles: &[NavigationObstacle],
    ) -> Vec<AutonomousServicePlan> {
        let mut remaining_stock = self
            .buildings
            .product_stocks
            .iter()
            .map(|stock| {
                (
                    (stock.building_instance_id.clone(), stock.product_id.clone()),
                    stock.quantity,
                )
            })
            .collect::<HashMap<_, _>>();
        let mut remaining_slots = self
            .buildings
            .buildings
            .iter()
            .filter(|building| {
                matches!(
                    building.id.as_str(),
                    "build_9" | "build_12" | "build_13" | "build_19"
                )
            })
            .map(|building| {
                let occupied = self
                    .product_services
                    .visits
                    .iter()
                    .filter(|visit| visit.building_instance_id == building.instance_id)
                    .count();
                let capacity = capacity_for_level(building.id.as_str(), u16::from(building.level))
                    .map(usize::from)
                    .unwrap_or(0);
                (
                    building.instance_id.clone(),
                    capacity.saturating_sub(occupied),
                )
            })
            .collect::<HashMap<_, _>>();
        let mut plans = Vec::new();

        for hunter in self.hunter_roster.hunters.iter().filter(|hunter| {
            [
                (
                    super::ServiceEffectKind::Hp,
                    hunter.current_hp,
                    hunter.max_hp,
                ),
                (
                    super::ServiceEffectKind::Stamina,
                    hunter.stamina.current,
                    hunter.stamina.maximum,
                ),
                (
                    super::ServiceEffectKind::Satiety,
                    hunter.satiety.current,
                    hunter.satiety.maximum,
                ),
                (
                    super::ServiceEffectKind::Mood,
                    hunter.mood.current,
                    hunter.mood.maximum,
                ),
            ]
            .iter()
            .any(|(_, current, maximum)| {
                *maximum > 0 && u128::from(*current) * 100 < u128::from(*maximum) * 10
            }) && hunter.profile.action_state != "serving"
                && !self
                    .product_services
                    .visits
                    .iter()
                    .any(|visit| visit.hunter_id == hunter.hunter_id)
        }) {
            let effect_kind = [
                super::ServiceEffectKind::Hp,
                super::ServiceEffectKind::Stamina,
                super::ServiceEffectKind::Satiety,
                super::ServiceEffectKind::Mood,
            ]
            .into_iter()
            .find(|kind| super::hunter_service_gauge(hunter, *kind).needs_autonomous_service())
            .expect("critical gauge");
            let service_building_id = match effect_kind {
                super::ServiceEffectKind::Hp => "build_12",
                super::ServiceEffectKind::Stamina => "build_9",
                super::ServiceEffectKind::Satiety => "build_13",
                super::ServiceEffectKind::Mood => "build_19",
            };
            let mut candidates = Vec::new();
            for building in self
                .buildings
                .buildings
                .iter()
                .filter(|building| building.id == service_building_id)
            {
                if remaining_slots
                    .get(&building.instance_id)
                    .copied()
                    .unwrap_or(0)
                    == 0
                {
                    continue;
                }
                let Some(destination) = town_building_interaction_point(
                    building,
                    &self.building_content.catalog,
                    obstacles,
                ) else {
                    continue;
                };
                let level = u16::from(building.level);
                for ((instance_id, product_id), quantity) in &remaining_stock {
                    if instance_id != &building.instance_id || *quantity == 0 {
                        continue;
                    }
                    let Some(product) = self.building_content.gameplay.product(product_id) else {
                        continue;
                    };
                    if product.building_id.as_ref().map(BaseBuildingId::as_str)
                        != Some(service_building_id)
                    {
                        continue;
                    }
                    let Some(service) = product.service.as_ref() else {
                        continue;
                    };
                    if service.required_level >= level || service.use_money > hunter.gold {
                        continue;
                    }
                    candidates.push(AutonomousServicePlan {
                        hunter_id: hunter.hunter_id,
                        building_instance_id: building.instance_id.clone(),
                        product_id: Some(product_id.clone()),
                        destination,
                    });
                }
            }
            candidates.sort_by(|left, right| {
                left.product_id
                    .cmp(&right.product_id)
                    .then_with(|| left.building_instance_id.cmp(&right.building_instance_id))
            });
            let plan = if let Some(plan) = candidates.into_iter().next() {
                if let Some(product_id) = plan.product_id.as_ref() {
                    if let Some(quantity) = remaining_stock
                        .get_mut(&(plan.building_instance_id.clone(), product_id.clone()))
                    {
                        *quantity = quantity.saturating_sub(1);
                    }
                }
                if let Some(slots) = remaining_slots.get_mut(&plan.building_instance_id) {
                    *slots = slots.saturating_sub(1);
                }
                plan
            } else {
                let Some(building) = self
                    .buildings
                    .buildings
                    .iter()
                    .find(|building| building.id == service_building_id)
                else {
                    continue;
                };
                let Some(destination) = town_building_interaction_point(
                    building,
                    &self.building_content.catalog,
                    obstacles,
                ) else {
                    continue;
                };
                let linger_offsets = [(-18, 0), (0, 10), (18, 0), (0, -10)];
                let offset_index =
                    usize::try_from((self.visual_tick / 30 + u64::from(hunter.hunter_id)) % 4)
                        .unwrap_or(0);
                let offset = linger_offsets[offset_index];
                AutonomousServicePlan {
                    hunter_id: hunter.hunter_id,
                    building_instance_id: building.instance_id.clone(),
                    product_id: None,
                    destination: (destination.0 + offset.0, destination.1 + offset.1),
                }
            };
            plans.push(plan);
        }
        plans
    }

    fn start_arrived_autonomous_services(&mut self, plans: &[AutonomousServicePlan]) {
        let planned_hunters = plans
            .iter()
            .map(|plan| plan.hunter_id)
            .collect::<HashSet<_>>();
        for plan in plans {
            let arrived = self.monster_world.hunters.iter().any(|agent| {
                agent.hunter_id == plan.hunter_id
                    && agent.region_id.is_none()
                    && agent.entry_stage == 0
                    && (agent.x, agent.y) == plan.destination
            });
            if arrived {
                if let Some(product_id) = plan.product_id.as_deref() {
                    let _ = self.start_product_service(
                        &plan.building_instance_id,
                        plan.hunter_id,
                        product_id,
                    );
                } else if let Ok(hunter) = self.hunter_roster.active_mut(plan.hunter_id) {
                    hunter.profile.action_state = "waiting_for_service".to_owned();
                    hunter.profile.animation_name = "hunter_stay".to_owned();
                }
            }
        }

        for hunter in &mut self.hunter_roster.hunters {
            if !matches!(
                hunter.profile.action_state.as_str(),
                "returning_for_service" | "returning_for_infirmary"
            ) || planned_hunters.contains(&hunter.hunter_id)
            {
                continue;
            }
            let returned_without_service = self.monster_world.hunters.iter().any(|agent| {
                agent.hunter_id == hunter.hunter_id
                    && agent.region_id.is_none()
                    && agent.entry_stage == 0
                    && TOWN_ROAM_BOUNDS.contains(agent.x, agent.y)
            });
            if returned_without_service {
                hunter.profile.action_state = "waiting_for_service".to_owned();
                hunter.profile.animation_name = "hunter_stay".to_owned();
            }
        }
    }

    fn mark_unavailable_service_waiters(&mut self, plans: &[AutonomousServicePlan]) {
        for plan in plans.iter().filter(|plan| plan.product_id.is_none()) {
            if let Ok(hunter) = self.hunter_roster.active_mut(plan.hunter_id) {
                hunter.profile.action_state = "waiting_for_service".to_owned();
                hunter.profile.animation_name = "hunter_walk".to_owned();
            }
        }
    }

    pub fn advance_visual_tick(&mut self) -> Option<OriginalFlowSnapshot> {
        self.advance_visual_tick_by(200)
    }

    pub fn advance_visual_tick_by(&mut self, elapsed_ms: u64) -> Option<OriginalFlowSnapshot> {
        if !self.advance_visual_clock_by(elapsed_ms) {
            return None;
        }
        Some(self.snapshot())
    }

    pub fn advance_visual_clock_by(&mut self, elapsed_ms: u64) -> bool {
        if !self.shared_world_active() {
            return false;
        }
        self.advance_product_services(elapsed_ms);
        true
    }

    pub(super) fn shared_world_active(&self) -> bool {
        matches!(
            self.state.screen,
            OriginalScreen::Village | OriginalScreen::Field
        )
    }

    fn advance_legacy_hunter_hunts(&mut self, ticks: u32) {
        let ids = self
            .hunter_roster
            .hunters
            .iter()
            .filter(|hunter| {
                hunter.hunt.zone_id.as_deref()
                    == Some(crate::simulation::hunter_roster::FIXTURE_HUNT_ZONE_ID)
            })
            .map(|hunter| hunter.hunter_id)
            .collect::<Vec<_>>();
        for hunter_id in ids {
            let _ = self.hunter_roster.advance_hunt(hunter_id, ticks);
        }
    }
}

#[cfg(test)]
fn test_consumable_products(
) -> &'static std::collections::BTreeMap<String, crate::buildings::ConsumableProductDefinition> {
    static PRODUCTS: std::sync::OnceLock<
        std::collections::BTreeMap<String, crate::buildings::ConsumableProductDefinition>,
    > = std::sync::OnceLock::new();
    PRODUCTS.get_or_init(|| {
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../reverse-engineering/evidence/core-economy-tables-v1.json"
        ))
        .expect("test consumable fixture must decode");
        source["consumables"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|consumable| {
                let index = consumable["index"].as_u64().expect("consumable index") as u32;
                let cooldown_ms = consumable["coolTime"]
                    .as_f64()
                    .expect("consumable cooldown") as u64
                    * 1_000;
                let keep_times = consumable["keepTimeByLevel"]
                    .as_array()
                    .expect("consumable keep times");
                let prices = consumable["priceByLevel"]
                    .as_array()
                    .expect("consumable prices");
                consumable["keepValueByLevel"]
                    .as_array()
                    .expect("consumable keep values")
                    .iter()
                    .enumerate()
                    .map(move |(level, keep_value)| {
                        let product_id = format!("recipe:consumable:{index}:level:{level}");
                        (
                            product_id.clone(),
                            crate::buildings::ConsumableProductDefinition {
                                product_id,
                                consumable_index: index,
                                level: u16::try_from(level).expect("consumable level"),
                                keep_time_ms: keep_times[level]
                                    .as_f64()
                                    .expect("consumable keep time")
                                    as u64
                                    * 1_000,
                                keep_value: keep_value.as_f64().expect("consumable keep value")
                                    as u64,
                                cooldown_ms,
                                price: prices[level].as_f64().expect("consumable price") as u64,
                            },
                        )
                    })
            })
            .collect()
    })
}
