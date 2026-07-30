use super::{
    hunter_trade_workflow, release_hunter_from_trade, town_building_interaction_point,
    town_navigation_obstacles, BTreeMap, DurableHunterState, DurableHunterTradeTask,
    DurableMaterialStock, DurableTradeSettlement, HashMap, OriginalFlowSession, ServerMessage,
    Uuid, HUNTER_TRADE_WORKFLOW_VERSION,
};

impl OriginalFlowSession {
    pub(super) fn sell_hunter_loot(&mut self, command_id: Uuid, hunter_id: u32) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("sell_hunter_loot", "village_unavailable");
        }
        let requested_only = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .is_some_and(|hunter| {
                hunter.hunt.status == "hunting"
                    && hunter.hunt.zone_id.as_deref().is_some_and(|zone_id| {
                        super::super::hunter_roster::is_ordinary_hunt_region(zone_id)
                    })
            });
        self.schedule_hunter_loot_sale(command_id, hunter_id, requested_only)
    }

    pub(super) fn schedule_hunter_loot_sale(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        requested_only: bool,
    ) -> ServerMessage {
        let key = format!("sell_hunter_loot:{hunter_id}");
        if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
            return if previous == &key {
                self.accepted("sell_hunter_loot")
            } else {
                self.rejected(
                    "sell_hunter_loot",
                    "command id was already used for a different hunter action",
                )
            };
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(
                "sell_hunter_loot",
                "hunter is not in the active town roster",
            );
        };
        if let Some(task) = &hunter.hunt.pending_trade {
            return if task.command_id == command_id {
                self.accepted("sell_hunter_loot")
            } else {
                self.rejected("sell_hunter_loot", "hunter is already traveling to trade")
            };
        }
        let ordinary_field_sale = requested_only
            && hunter.hunt.status == "hunting"
            && hunter.hunt.zone_id.as_deref().is_some_and(|zone_id| {
                super::super::hunter_roster::is_ordinary_hunt_region(zone_id)
            });
        let active_service = self
            .product_services
            .visits
            .iter()
            .any(|visit| visit.hunter_id == hunter_id);
        if hunter.current_hp == 0
            || hunter.hunt.gear_enhancement.is_some()
            || active_service
            || (!hunter.hunt.is_idle() && !ordinary_field_sale)
        {
            return self.rejected("sell_hunter_loot", "hunter is unavailable for trade");
        }
        if !self.has_affordable_sale(hunter, requested_only) {
            return self.rejected(
                "sell_hunter_loot",
                "hunter has no affordable requested loot",
            );
        }
        let obstacles =
            town_navigation_obstacles(&self.buildings.buildings, &self.building_content.catalog);
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_3")
        else {
            return self.rejected("sell_hunter_loot", "trading post is unavailable");
        };
        let Some((interaction_x, interaction_y)) =
            town_building_interaction_point(building, &self.building_content.catalog, &obstacles)
        else {
            return self.rejected("sell_hunter_loot", "trading post path is unavailable");
        };
        let building_instance_id = building.instance_id.clone();
        let hunter = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .expect("validated Hunter remains in roster");
        hunter_trade_workflow::HunterTradeWorkflow::begin(
            hunter,
            DurableHunterTradeTask {
                workflow_version: HUNTER_TRADE_WORKFLOW_VERSION,
                command_id,
                requested_only,
                building_instance_id,
                interaction_x,
                interaction_y,
            },
        );
        self.accepted("sell_hunter_loot")
    }

    pub(super) fn settle_hunter_loot_internal(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        requested_only: bool,
    ) -> ServerMessage {
        let key = format!("sell_hunter_loot:{hunter_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted("sell_hunter_loot")
                } else {
                    self.rejected(
                        "sell_hunter_loot",
                        "command id was already used for a different hunter action",
                    )
                };
            }
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(
                "sell_hunter_loot",
                "hunter is not in the active town roster",
            );
        };
        let ordinary_field_auto_sale = requested_only
            && hunter.hunt.status == "hunting"
            && hunter.hunt.zone_id.as_deref().is_some_and(|zone_id| {
                super::super::hunter_roster::is_ordinary_hunt_region(zone_id)
            });
        let pending_trade = hunter
            .hunt
            .pending_trade
            .as_ref()
            .is_some_and(|task| task.command_id == command_id);
        if !hunter.hunt.is_idle() && !ordinary_field_auto_sale && !pending_trade {
            return self.rejected("sell_hunter_loot", "hunter must be idle to sell loot");
        }
        let mut sale_lines = BTreeMap::<String, u32>::new();
        for loot in &hunter.hunt.loot {
            if loot.quantity == 0 {
                continue;
            }
            if loot.item_id == "gold" {
                continue;
            }
            if !loot.item_id.starts_with("material:") {
                return self.rejected("sell_hunter_loot", "loot definition is unavailable");
            }
            let Some(item) = self.building_content.gameplay.item(&loot.item_id) else {
                return self.rejected("sell_hunter_loot", "loot definition is unavailable");
            };
            let already_selected = sale_lines.get(&loot.item_id).copied().unwrap_or(0);
            let sale_quantity = if requested_only {
                let remaining_request = self
                    .buildings
                    .material_stocks
                    .iter()
                    .find(|stock| stock.id == loot.item_id)
                    .map_or(0, |stock| stock.requested)
                    .saturating_sub(already_selected);
                loot.quantity.min(remaining_request)
            } else {
                loot.quantity
            };
            if sale_quantity == 0 {
                continue;
            }
            if item.item_type.as_deref() != Some("material")
                || item.town_pays_hunter_gold_per_unit.is_none()
            {
                return self.rejected("sell_hunter_loot", "loot price is unavailable");
            }
            let quantity = sale_lines.entry(loot.item_id.clone()).or_default();
            let Some(total) = quantity.checked_add(sale_quantity) else {
                return self.rejected("sell_hunter_loot", "loot quantity overflow");
            };
            *quantity = total;
        }
        if sale_lines.is_empty() {
            return self.rejected("sell_hunter_loot", "hunter has no hunt loot");
        }
        let mut priced_lines = Vec::with_capacity(sale_lines.len());
        let mut total_gold = 0_u64;
        let mut available_town_gold = self.buildings.town_gold;
        for (material_id, quantity) in sale_lines {
            let Some(unit_price) = self
                .building_content
                .gameplay
                .item(&material_id)
                .and_then(|item| item.town_pays_hunter_gold_per_unit)
            else {
                return self.rejected("sell_hunter_loot", "loot price is unavailable");
            };
            if unit_price == 0 {
                return self.rejected("sell_hunter_loot", "loot price is unavailable");
            }
            let affordable_quantity = available_town_gold / unit_price;
            let quantity = quantity.min(u32::try_from(affordable_quantity).unwrap_or(u32::MAX));
            if quantity == 0 {
                continue;
            }
            let Some(line_gold) = u64::from(quantity).checked_mul(unit_price) else {
                return self.rejected("sell_hunter_loot", "loot price overflow");
            };
            let Some(next_total) = total_gold.checked_add(line_gold) else {
                return self.rejected("sell_hunter_loot", "loot price overflow");
            };
            total_gold = next_total;
            available_town_gold -= line_gold;
            priced_lines.push((material_id, quantity, unit_price, line_gold));
        }
        if priced_lines.is_empty() {
            return self.rejected("sell_hunter_loot", "town wallet cannot afford loot");
        }
        let settlement_field_trip_id = self.buildings.field_trip_id.max(1);
        self.buildings.field_trip_id = settlement_field_trip_id;
        self.buildings.town_gold -= total_gold;
        for (material_id, quantity, unit_price, _) in &priced_lines {
            if let Some(stock) = self
                .buildings
                .material_stocks
                .iter_mut()
                .find(|stock| stock.id == *material_id)
            {
                stock.town_quantity = stock.town_quantity.saturating_add(*quantity);
                stock.requested = stock.requested.saturating_sub(*quantity);
                stock.unit_price = *unit_price;
            } else {
                self.buildings.material_stocks.push(DurableMaterialStock {
                    id: material_id.clone(),
                    town_quantity: *quantity,
                    hunter_quantity: 0,
                    requested: 0,
                    unit_price: *unit_price,
                });
            }
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            unreachable!()
        };
        hunter.gold = hunter.gold.saturating_add(total_gold);
        release_hunter_from_trade(hunter);
        let mut sold_quantities = priced_lines
            .iter()
            .map(|(material_id, quantity, _, _)| (material_id.clone(), *quantity))
            .collect::<BTreeMap<_, _>>();
        for loot in &mut hunter.hunt.loot {
            if let Some(sold) = sold_quantities.get_mut(&loot.item_id) {
                let deducted = loot.quantity.min(*sold);
                loot.quantity -= deducted;
                *sold -= deducted;
            }
        }
        hunter.hunt.loot.retain(|loot| loot.quantity > 0);
        if let Some(agent) = self
            .monster_world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == hunter_id)
        {
            agent.trade_sequence = agent.trade_sequence.saturating_add(1);
            agent.trade_gold = total_gold;
            agent.trade_materials = priced_lines
                .iter()
                .map(|(material_id, quantity, _, _)| {
                    super::super::monster_world::TradeMaterialPresentation {
                        material_id: material_id.clone(),
                        quantity: *quantity,
                    }
                })
                .collect();
        }
        for (line_index, (material_id, quantity, unit_price, line_gold)) in
            priced_lines.into_iter().enumerate()
        {
            let settlement_id = if line_index == 0 {
                command_id.to_string()
            } else {
                format!("{command_id}:{line_index}")
            };
            self.buildings
                .trade_settlements
                .push(DurableTradeSettlement {
                    settlement_id,
                    field_trip_id: settlement_field_trip_id,
                    material_id,
                    quantity,
                    unit_price,
                    total_gold: line_gold,
                });
        }
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted("sell_hunter_loot")
    }

    pub(super) fn auto_sell_requested_hunter_loot(&mut self) {
        let hunter_ids = self
            .hunter_roster
            .hunters
            .iter()
            .filter(|hunter| {
                let can_settle_requested_loot = hunter.hunt.pending_trade.is_none()
                    && (hunter.hunt.is_idle()
                        || (hunter.hunt.status == "hunting"
                            && hunter.hunt.zone_id.as_deref().is_some_and(|zone_id| {
                                super::super::hunter_roster::is_ordinary_hunt_region(zone_id)
                            })));
                can_settle_requested_loot && self.has_affordable_auto_sale(hunter)
            })
            .map(|hunter| hunter.hunter_id)
            .collect::<Vec<_>>();
        for hunter_id in hunter_ids {
            let settlement_id = self.next_auto_trade_settlement_id(hunter_id);
            let _ = self.schedule_hunter_loot_sale(settlement_id, hunter_id, true);
        }
    }

    pub(super) fn pending_trade_destinations(&self) -> HashMap<u32, (i32, i32)> {
        self.hunter_roster
            .hunters
            .iter()
            .filter_map(|hunter| {
                hunter
                    .hunt
                    .pending_trade
                    .as_ref()
                    .map(|task| (hunter.hunter_id, (task.interaction_x, task.interaction_y)))
            })
            .collect()
    }

    pub(super) fn settle_arrived_hunter_trades(&mut self) {
        let arrived = self
            .hunter_roster
            .hunters
            .iter()
            .filter_map(|hunter| {
                let task = hunter.hunt.pending_trade.as_ref()?;
                self.monster_world
                    .hunters
                    .iter()
                    .any(|agent| {
                        agent.hunter_id == hunter.hunter_id
                            && agent.region_id.is_none()
                            && agent.entry_stage == 0
                            && (agent.x, agent.y) == (task.interaction_x, task.interaction_y)
                    })
                    .then_some((hunter.hunter_id, task.command_id, task.requested_only))
            })
            .collect::<Vec<_>>();
        for (hunter_id, command_id, requested_only) in arrived {
            let result = self.settle_hunter_loot_internal(command_id, hunter_id, requested_only);
            if matches!(
                result,
                ServerMessage::IntentResult {
                    accepted: false,
                    ..
                }
            ) {
                if let Some(hunter) = self
                    .hunter_roster
                    .hunters
                    .iter_mut()
                    .find(|hunter| hunter.hunter_id == hunter_id)
                {
                    release_hunter_from_trade(hunter);
                }
            }
        }
    }

    /// Avoid invoking the rejection path on every simulation tick when a
    /// requested sale cannot succeed. Rejections build a complete snapshot,
    /// which is too expensive for the 10 Hz movement loop.
    pub(super) fn has_affordable_auto_sale(&self, hunter: &DurableHunterState) -> bool {
        self.has_affordable_sale(hunter, true)
    }

    pub(super) fn has_affordable_sale(
        &self,
        hunter: &DurableHunterState,
        requested_only: bool,
    ) -> bool {
        for loot in &hunter.hunt.loot {
            if loot.quantity == 0 || loot.item_id == "gold" {
                continue;
            }
            let Some(item) = self.building_content.gameplay.item(&loot.item_id) else {
                return false;
            };
            let Some(unit_price) = item.town_pays_hunter_gold_per_unit else {
                return false;
            };
            if unit_price == 0 {
                return false;
            }
            let eligible_quantity = if requested_only {
                self.buildings
                    .material_stocks
                    .iter()
                    .find(|stock| stock.id == loot.item_id)
                    .map_or(0, |stock| loot.quantity.min(stock.requested))
            } else {
                loot.quantity
            };
            if eligible_quantity > 0 && self.buildings.town_gold >= unit_price {
                return true;
            }
        }
        false
    }

    pub(super) fn next_auto_trade_settlement_id(&self, hunter_id: u32) -> Uuid {
        let mut sequence = u32::try_from(self.buildings.trade_settlements.len())
            .unwrap_or(u32::MAX)
            .saturating_add(1);
        loop {
            let id = Uuid::from_u128(
                (u128::from(self.buildings.field_trip_id.max(1)) << 64)
                    | (u128::from(hunter_id) << 32)
                    | u128::from(sequence),
            );
            let id_text = id.to_string();
            let settlement_exists = self.buildings.trade_settlements.iter().any(|settlement| {
                settlement.settlement_id == id_text
                    || settlement.settlement_id.starts_with(&format!("{id_text}:"))
            });
            if !settlement_exists && !self.hunter_roster.hunt_commands.contains_key(&id) {
                return id;
            }
            sequence = sequence.wrapping_add(1);
        }
    }
}
