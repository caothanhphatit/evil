use super::{
    can_pay_costs, capacity_for_level, gear_product_route, material_difficulty_rating, pay_costs,
    product_sale_building_id, BaseBuildingId, DurableMaterialStock, DurableProductStock,
    EconomyAmount, OriginalFlowSession, ServerMessage, ServiceEffectKind, Uuid,
    MAX_PRODUCTION_QUANTITY,
};

impl OriginalFlowSession {
    pub(super) fn set_material_request(
        &mut self,
        instance_id: &str,
        material_id: &str,
        quantity: u32,
    ) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("set_material_request", "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("set_material_request", "building_instance_unknown");
        };
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("set_material_request", "building_unknown");
        };
        if !self
            .building_content
            .gameplay
            .capabilities_for(&building_id)
            .any(|capability| capability.kind == "loot-purchase-reservations")
        {
            return self.rejected("set_material_request", "building_capability_mismatch");
        }
        let Some(difficulty) =
            material_difficulty_rating(&self.building_content.gameplay, material_id)
        else {
            return self.rejected("set_material_request", "material_difficulty_unresolved");
        };
        if difficulty >= building.level {
            return self.rejected("set_material_request", "material_difficulty_locked");
        }
        if quantity == 0 {
            return self.rejected("set_material_request", "material_quantity_invalid");
        }
        let Some(authoritative_price) = self
            .building_content
            .gameplay
            .item(material_id)
            .and_then(|item| item.town_pays_hunter_gold_per_unit)
        else {
            return self.rejected("set_material_request", "material_price_unresolved");
        };
        if let Some(stock) = self
            .buildings
            .material_stocks
            .iter_mut()
            .find(|stock| stock.id == material_id)
        {
            stock.requested = quantity;
            stock.unit_price = authoritative_price;
        } else {
            self.buildings.material_stocks.push(DurableMaterialStock {
                id: material_id.to_owned(),
                town_quantity: 0,
                hunter_quantity: 0,
                requested: quantity,
                unit_price: authoritative_price,
            });
            self.buildings
                .material_stocks
                .sort_by(|left, right| left.id.cmp(&right.id));
        }
        self.accepted("set_material_request")
    }

    pub(super) fn cancel_material_request(
        &mut self,
        instance_id: &str,
        material_id: &str,
    ) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("cancel_material_request", "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("cancel_material_request", "building_instance_unknown");
        };
        let capability_matches = BaseBuildingId::parse(&building.id).ok().is_some_and(|id| {
            self.building_content
                .gameplay
                .capabilities_for(&id)
                .any(|capability| capability.kind == "loot-purchase-reservations")
        });
        if !capability_matches {
            return self.rejected("cancel_material_request", "building_capability_mismatch");
        }
        let Some(stock) = self
            .buildings
            .material_stocks
            .iter_mut()
            .find(|stock| stock.id == material_id)
        else {
            return self.rejected("cancel_material_request", "material_request_unknown");
        };
        stock.requested = 0;
        self.accepted("cancel_material_request")
    }

    pub(super) fn craft_shop_item(
        &mut self,
        command_id: Uuid,
        instance_id: &str,
        recipe_id: &str,
        material_id: Option<&str>,
        quantity: u32,
    ) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("craft_shop_item", "village_unavailable");
        }
        let command_key = format!(
            "craft_shop_item:{instance_id}:{recipe_id}:{}:{quantity}",
            material_id.unwrap_or("")
        );
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &command_key {
                    self.accepted("craft_shop_item")
                } else {
                    self.rejected("craft_shop_item", "command_id_conflict")
                };
            }
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("craft_shop_item", "building_instance_unknown");
        };
        if !(1..=MAX_PRODUCTION_QUANTITY).contains(&quantity) {
            return self.rejected("craft_shop_item", "quantity_invalid");
        }
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("craft_shop_item", "building_unknown");
        };
        let Some(product) = self.building_content.gameplay.product(recipe_id) else {
            return self.rejected("craft_shop_item", "recipe_unknown");
        };
        if product.building_id.as_ref() != Some(&building_id) {
            return self.rejected("craft_shop_item", "recipe_building_mismatch");
        }
        // A partially migrated service row may have conversion options but no
        // decoded service payload. The building route still identifies it as a
        // service product, which must remain uncapped.
        let service_product = product.service.is_some()
            || (ServiceEffectKind::for_building(building_id.as_str()).is_some()
                && !product.conversion_options.is_empty());
        let gear_route = gear_product_route(&self.building_content.gameplay, product);
        if gear_route
            .as_ref()
            .is_some_and(|route| route.difficulty_group > u16::from(building.level))
        {
            return self.rejected("craft_shop_item", "product_level_locked");
        }
        if gear_route.is_some() {
            // The captured original writer exposes several option arrays and
            // a per-instance buyGold field, but its pool/order/price semantics
            // are not yet proven. Never debit materials for an unverifiable
            // gear result.
            return self.rejected("craft_shop_item", "gear_creation_evidence_unresolved");
        }
        let crafting_capability = self
            .building_content
            .gameplay
            .capabilities_for(&building_id)
            .any(|capability| {
                capability.kind == "weapon-and-armor-crafting"
                    || capability.kind == "potion-crafting"
                    || capability.kind == "accessory-crafting"
            });
        if !crafting_capability && !service_product {
            return self.rejected("craft_shop_item", "building_capability_mismatch");
        }
        let sale_building_id = product_sale_building_id(&self.building_content.gameplay, product);
        let stock_building = if let Some(sale_building_id) = &sale_building_id {
            let Some(sale_building) = self
                .buildings
                .buildings
                .iter()
                .find(|candidate| candidate.id == sale_building_id.as_str())
            else {
                return self.rejected("craft_shop_item", "sale_building_instance_unknown");
            };
            sale_building
        } else {
            building
        };
        let stock_building_id = BaseBuildingId::parse(&stock_building.id)
            .expect("validated building state references a canonical base id");
        let stock_building_instance_id = stock_building.instance_id.clone();
        let capacity = self
            .building_content
            .catalog
            .level(&stock_building_id, u16::from(stock_building.level))
            .and_then(|level| level.production_slots)
            .or_else(|| {
                capacity_for_level(stock_building_id.as_str(), u16::from(stock_building.level))
            })
            .map(u32::from)
            .unwrap_or(0);
        let stocked = self
            .buildings
            .product_stocks
            .iter()
            .filter(|stock| stock.building_instance_id == stock_building_instance_id)
            .fold(0_u32, |total, stock| total.saturating_add(stock.quantity));
        // Service products are consumed by the service flow and are not
        // constrained by the building's display-stock cap. Crafted gear and
        // sale inventory still use the authoritative capacity check.
        if !service_product && capacity > 0 && stocked.saturating_add(quantity) > capacity {
            return self.rejected("craft_shop_item", "product_capacity_exceeded");
        }
        let costs = if service_product {
            let Some(material_id) = material_id else {
                return self.rejected("craft_shop_item", "material_selection_required");
            };
            let Some(option) = product
                .conversion_options
                .iter()
                .find(|option| option.input_resource_id == material_id)
            else {
                return self.rejected("craft_shop_item", "material_selection_invalid");
            };
            let batches = u64::from(quantity)
                .saturating_add(option.output_stock_quantity.saturating_sub(1))
                / option.output_stock_quantity.max(1);
            vec![EconomyAmount {
                resource_id: option.input_resource_id.clone(),
                quantity: option.input_quantity.saturating_mul(batches),
            }]
        } else {
            product
                .inputs
                .iter()
                .map(|cost| EconomyAmount {
                    resource_id: cost.resource_id.clone(),
                    quantity: cost.quantity.saturating_mul(u64::from(quantity)),
                })
                .collect::<Vec<_>>()
        };
        if !can_pay_costs(&self.buildings, &costs) {
            return self.rejected("craft_shop_item", "insufficient_materials");
        }
        pay_costs(&mut self.buildings, &costs);
        if let Some(stock) = self.buildings.product_stocks.iter_mut().find(|stock| {
            stock.building_instance_id == stock_building_instance_id
                && stock.product_id == recipe_id
        }) {
            stock.quantity = stock.quantity.saturating_add(quantity);
        } else {
            self.buildings.product_stocks.push(DurableProductStock {
                building_instance_id: stock_building_instance_id,
                product_id: recipe_id.to_owned(),
                quantity,
            });
        }
        if command_id != Uuid::nil() {
            self.hunter_roster
                .hunt_commands
                .insert(command_id, command_key);
        }
        self.accepted("craft_shop_item")
    }
}
