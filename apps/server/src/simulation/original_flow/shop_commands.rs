use super::{
    consumable_purchase_price, gear_product_route, gear_purchase_price,
    is_purchasable_crafted_gear, product_sale_building_id, settle_returning_hunters,
    BaseBuildingId, OriginalFlowSession, ServerMessage, Uuid,
};

impl OriginalFlowSession {
    pub(super) fn settle_returning_hunters(&mut self) {
        settle_returning_hunters(&mut self.buildings);
    }

    pub(super) fn purchase_shop_item(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        shop_id: &str,
        product_id: &str,
    ) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("purchase_shop_item", "village_unavailable");
        }
        let Ok(building_id) = BaseBuildingId::parse(shop_id) else {
            return self.rejected("purchase_shop_item", "building_unknown");
        };
        let Some(product) = self.building_content.gameplay.product(product_id) else {
            return self.rejected("purchase_shop_item", "recipe_unknown");
        };
        let route = gear_product_route(&self.building_content.gameplay, product);
        let sale_building_id = product_sale_building_id(&self.building_content.gameplay, product);
        if sale_building_id.as_ref() != Some(&building_id) {
            return self.rejected("purchase_shop_item", "recipe_building_mismatch");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == shop_id)
        else {
            return self.rejected("purchase_shop_item", "building_instance_unknown");
        };
        if route
            .as_ref()
            .is_some_and(|route| route.difficulty_group > u16::from(building.level))
        {
            return self.rejected("purchase_shop_item", "product_level_locked");
        }
        let building_instance_id = building.instance_id.clone();
        let Some(stock_index) = self.buildings.product_stocks.iter().position(|stock| {
            stock.building_instance_id == building_instance_id && stock.product_id == product_id
        }) else {
            return self.rejected("purchase_shop_item", "product_stock_empty");
        };
        if self.buildings.product_stocks[stock_index].quantity == 0 {
            return self.rejected("purchase_shop_item", "product_stock_empty");
        }
        let price = route
            .as_ref()
            .and_then(|_| product.sale_price.first().map(|amount| amount.quantity))
            .or_else(|| gear_purchase_price(&self.building_content.gameplay, product))
            .or_else(|| consumable_purchase_price(&self.building_content.gameplay, product))
            .unwrap_or(0);
        if price == 0 {
            return self.rejected("purchase_shop_item", "sale_price_unresolved");
        }
        let Some(hunter_index) = self
            .hunter_roster
            .hunters
            .iter()
            .position(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected("purchase_shop_item", "hunter_unknown");
        };
        let hunter = &self.hunter_roster.hunters[hunter_index];
        if !hunter.hunt.is_idle() {
            return self.rejected("purchase_shop_item", "hunter_not_in_town");
        }
        if hunter.gold < price {
            return self.rejected("purchase_shop_item", "insufficient_hunter_gold");
        }
        let key = format!("purchase_shop_item:{hunter_id}:{shop_id}:{product_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted("purchase_shop_item")
                } else {
                    self.rejected("purchase_shop_item", "command_id_conflict")
                };
            }
        }

        let weapon_definition =
            super::super::web_rebuild_gear::rebuild_weapon_definition(product_id);
        if weapon_definition.as_ref().is_some_and(|definition| {
            definition.visual_family
                != self.hunter_roster.hunters[hunter_index]
                    .profile
                    .visual_family
        }) {
            return self.rejected("purchase_shop_item", "weapon_class_incompatible");
        }
        let auto_equip_weapon = weapon_definition;
        if auto_equip_weapon.is_some()
            && !self.hunter_roster.hunters[hunter_index]
                .profile
                .equipment_slots
                .iter()
                .any(|slot| slot.slot_id == "weapon")
        {
            return self.rejected("purchase_shop_item", "weapon_slot_unavailable");
        }

        let crafted_gear = if route.is_some() {
            let Some(position) = self.buildings.crafted_gear_stocks.iter().position(|gear| {
                gear.building_instance_id == building_instance_id
                    && gear.product_id == product_id
                    && is_purchasable_crafted_gear(gear)
            }) else {
                return self.rejected("purchase_shop_item", "crafted_gear_stock_empty");
            };
            Some(self.buildings.crafted_gear_stocks.remove(position))
        } else {
            None
        };

        let hunter = &mut self.hunter_roster.hunters[hunter_index];
        hunter.gold -= price;
        self.buildings.product_stocks[stock_index].quantity -= 1;
        self.buildings.town_gold = self.buildings.town_gold.saturating_add(price);
        // A gear purchase is an individually rolled item and must never be
        // merged into a product stack. Consumables remain stackable.
        if route.is_none() {
            if let Some(owned) = hunter
                .owned_items
                .iter_mut()
                .find(|owned| owned.product_id == product_id && owned.gear_instance_id.is_none())
            {
                owned.quantity = owned.quantity.saturating_add(1);
            } else {
                hunter
                    .owned_items
                    .push(super::super::hunter_roster::DurableHunterOwnedItem {
                        product_id: product_id.to_owned(),
                        quantity: 1,
                        enhancement_level: None,
                        gear_instance_id: None,
                        ..Default::default()
                    });
            }
        } else {
            let crafted = crafted_gear.expect("gear stock was resolved before mutation");
            let gear_instance_id = crafted.gear_instance_id;
            hunter
                .owned_items
                .push(super::super::hunter_roster::DurableHunterOwnedItem {
                    product_id: product_id.to_owned(),
                    quantity: 1,
                    enhancement_level: Some(0),
                    gear_instance_id: Some(crafted.gear_instance_id),
                    quality: Some(crafted.quality),
                    primary_stat: Some(crafted.primary_stat),
                    option_type: Some(crafted.option_type),
                    option_value: Some(crafted.option_value),
                    ruleset: Some(crafted.ruleset),
                });
            if auto_equip_weapon.is_some() {
                self.equip_owned_rebuild_weapon(hunter_index, gear_instance_id)
                    .expect("compatible purchase was validated before settlement");
            }
        }
        if route.is_some() {
            self.buildings.hunter_equipment_purchases =
                self.buildings.hunter_equipment_purchases.saturating_add(1);
        }
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted("purchase_shop_item")
    }

    pub(super) fn sell_shop_item(&mut self, shop_id: &str, product_id: &str) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("sell_shop_item", "village_unavailable");
        }
        let _ = (shop_id, product_id);
        self.capability_blocked(
            "sell_shop_item",
            shop_id,
            &[
                "weapon-display-and-sale",
                "armor-display-and-sale",
                "potion-display-and-sale",
                "accessory-display-and-sale",
            ],
        )
    }
}
