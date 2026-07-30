use super::{
    capacity_for_level, hunter_service_gauge, restore_hunter_service_gauge, BaseBuildingId,
    DurableProductServiceVisit, HashSet, InfirmaryHunterSnapshot, InfirmaryServiceSnapshot,
    InfirmaryTreatmentSnapshot, OriginalFlowSession, ProductServiceHunterSnapshot,
    ProductServiceSnapshot, ProductServiceVisitSnapshot, ServerMessage, ServiceEffectKind,
};

impl OriginalFlowSession {
    pub(super) fn start_product_service(
        &mut self,
        instance_id: &str,
        hunter_id: u32,
        product_id: &str,
    ) -> ServerMessage {
        const INTENT: &str = "start_building_service";
        if !self.shared_world_active() {
            return self.rejected(INTENT, "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected(INTENT, "service_instance_unknown");
        };
        let building_id = building.id.clone();
        let level = u16::from(building.level);
        let Some(effect_kind) = ServiceEffectKind::for_building(&building_id) else {
            return self.rejected(INTENT, "service_building_unsupported");
        };
        if !self.product_service_roster_resolved(effect_kind) {
            return self.binding_blocked(
                INTENT,
                &[
                    "hunter_roster_binding",
                    effect_kind.state_binding(),
                    "hunter_wallet_state_binding",
                ],
            );
        }
        let slots = capacity_for_level(&building_id, level).unwrap_or(0);
        let occupied_slots = self
            .product_services
            .visits
            .iter()
            .filter(|visit| visit.building_instance_id == instance_id)
            .count();
        if slots == 0 || occupied_slots >= usize::from(slots) {
            return self.rejected(INTENT, "service_slots_full");
        }
        if self
            .product_services
            .visits
            .iter()
            .any(|visit| visit.hunter_id == hunter_id)
            || self
                .hunter_roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == hunter_id)
                .is_some_and(|hunter| {
                    hunter.hunt.pending_trade.is_some() || hunter.hunt.gear_enhancement.is_some()
                })
        {
            return self.rejected(INTENT, "hunter_already_in_service");
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(INTENT, "hunter_unknown");
        };
        if !hunter_service_gauge(hunter, effect_kind).needs_service() {
            return self.rejected(INTENT, "service_not_required");
        }
        let Some(product) = self.building_content.gameplay.product(product_id) else {
            return self.rejected(INTENT, "recipe_unknown");
        };
        if product.building_id.as_ref().map(BaseBuildingId::as_str) != Some(building_id.as_str()) {
            return self.rejected(INTENT, "recipe_building_mismatch");
        }
        let Some(service) = product.service.as_ref() else {
            return self.rejected(INTENT, "service_recipe_required");
        };
        if service.required_level >= level {
            return self.rejected(INTENT, "product_level_locked");
        }
        let Some(stock) = self.buildings.product_stocks.iter_mut().find(|stock| {
            stock.building_instance_id == instance_id && stock.product_id == product_id
        }) else {
            return self.rejected(INTENT, "product_out_of_stock");
        };
        if stock.quantity == 0 {
            return self.rejected(INTENT, "product_out_of_stock");
        }
        let hunter = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .expect("validated service hunter");
        if hunter.gold < service.use_money {
            return self.rejected(INTENT, "insufficient_hunter_gold");
        }
        hunter.gold -= service.use_money;
        hunter.profile.action_state = "serving".to_owned();
        hunter.profile.animation_name = "hunter_stay".to_owned();
        stock.quantity -= 1;
        self.product_services
            .visits
            .push(DurableProductServiceVisit {
                hunter_id,
                building_instance_id: instance_id.to_owned(),
                building_id,
                product_id: product_id.to_owned(),
                effect_kind,
                remaining_ms: service.service_time_ms,
                effect_value: service.effect_value,
                payment_gold: service.use_money,
            });
        self.accepted(INTENT)
    }

    pub(super) fn advance_product_services(&mut self, elapsed_ms: u64) {
        for visit in &mut self.product_services.visits {
            visit.remaining_ms = visit.remaining_ms.saturating_sub(elapsed_ms);
        }
        let completed_visits = self
            .product_services
            .visits
            .iter()
            .filter(|visit| visit.remaining_ms == 0)
            .cloned()
            .collect::<Vec<_>>();
        for visit in &completed_visits {
            if let Some(hunter) = self
                .hunter_roster
                .hunters
                .iter_mut()
                .find(|hunter| hunter.hunter_id == visit.hunter_id)
            {
                restore_hunter_service_gauge(hunter, visit.effect_kind, visit.effect_value);
                hunter.profile.action_state = "idle".to_owned();
                hunter.profile.animation_name = "hunter_stay".to_owned();
                self.buildings.town_gold =
                    self.buildings.town_gold.saturating_add(visit.payment_gold);
            }
        }
        self.product_services
            .visits
            .retain(|visit| visit.remaining_ms > 0);
    }

    pub(super) fn product_service_roster_resolved(&self, effect_kind: ServiceEffectKind) -> bool {
        if !self.hunter_roster.roster_resolved || !self.hunter_roster.wallets_resolved {
            return false;
        }
        let mut hunter_ids = HashSet::with_capacity(self.hunter_roster.hunters.len());
        self.hunter_roster.hunters.iter().all(|hunter| {
            hunter_service_gauge(hunter, effect_kind).is_resolved()
                && hunter_ids.insert(hunter.hunter_id)
        })
    }

    pub(super) fn infirmary_snapshot(&self) -> InfirmaryServiceSnapshot {
        let service = self
            .product_service_snapshot("build_12")
            .expect("static service building");
        InfirmaryServiceSnapshot {
            roster_resolved: service.roster_resolved,
            slots: service.slots,
            available_slots: service.available_slots,
            hunters: service
                .hunters
                .into_iter()
                .map(|hunter| InfirmaryHunterSnapshot {
                    hunter_id: hunter.hunter_id,
                    current_hp: hunter.current_value,
                    max_hp: hunter.maximum_value,
                    treatment_state: if hunter.service_state == "serving" {
                        "treating"
                    } else {
                        "idle"
                    },
                })
                .collect(),
            active: service
                .active
                .into_iter()
                .map(|visit| InfirmaryTreatmentSnapshot {
                    hunter_id: visit.hunter_id,
                    building_instance_id: visit.building_instance_id,
                    product_id: visit.product_id,
                    remaining_ms: visit.remaining_ms,
                    effect_value: visit.effect_value,
                    payment_gold: visit.payment_gold,
                })
                .collect(),
            blockers: service.blockers,
        }
    }

    pub(super) fn product_service_snapshot(
        &self,
        building_id: &'static str,
    ) -> Option<ProductServiceSnapshot> {
        let effect_kind = ServiceEffectKind::for_building(building_id)?;
        let roster_resolved = self.product_service_roster_resolved(effect_kind);
        let slots = self
            .buildings
            .buildings
            .iter()
            .filter(|building| building.id == building_id)
            .filter_map(|building| capacity_for_level(building_id, u16::from(building.level)))
            .fold(0_u16, u16::saturating_add);
        let active = self
            .product_services
            .visits
            .iter()
            .filter(|visit| visit.building_id == building_id)
            .map(|visit| ProductServiceVisitSnapshot {
                hunter_id: visit.hunter_id,
                building_instance_id: visit.building_instance_id.clone(),
                product_id: visit.product_id.clone(),
                remaining_ms: visit.remaining_ms,
                effect_value: visit.effect_value,
                payment_gold: visit.payment_gold,
            })
            .collect::<Vec<_>>();
        let occupied_slots = u16::try_from(active.len()).unwrap_or(u16::MAX);
        Some(ProductServiceSnapshot {
            building_id,
            effect_kind: effect_kind.as_str(),
            roster_resolved,
            slots,
            available_slots: slots.saturating_sub(occupied_slots),
            hunters: self
                .hunter_roster
                .hunters
                .iter()
                .map(|hunter| {
                    let gauge = hunter_service_gauge(hunter, effect_kind);
                    ProductServiceHunterSnapshot {
                        hunter_id: hunter.hunter_id,
                        gold: hunter.gold,
                        current_value: gauge.current,
                        maximum_value: gauge.maximum,
                        service_state: if self
                            .product_services
                            .visits
                            .iter()
                            .any(|visit| visit.hunter_id == hunter.hunter_id)
                        {
                            "serving"
                        } else {
                            "idle"
                        },
                    }
                })
                .collect(),
            active,
            blockers: if roster_resolved {
                Vec::new()
            } else {
                vec![
                    "hunter_roster_binding",
                    effect_kind.state_binding(),
                    "hunter_wallet_state_binding",
                ]
            },
        })
    }
}
