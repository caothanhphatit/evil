use super::{
    building_definition_snapshot, building_grid_size, capacity_for_level,
    consumable_purchase_price, gear_product_route, gear_purchase_price, gold_cost,
    material_catalog_stocks, material_difficulty_rating, material_icon_path, mutation_condition,
    product_display_name, product_icon_path, product_sale_building_id, service_effect_kind,
    BaseBuildingId, BuildingInstanceSnapshot, BuildingStateSnapshot, BuildingSystemSnapshot,
    HashMap, HashSet, MaterialStockSnapshot, OriginalFlowSession, RecipeMaterialCostSnapshot,
    ServiceEffectKind, ShopRecipeSnapshot,
};

impl OriginalFlowSession {
    pub(super) fn building_snapshot(&self) -> BuildingSystemSnapshot {
        let content = &self.building_content;
        let definitions = content
            .catalog
            .bases
            .iter()
            .map(|building| building_definition_snapshot(building, content))
            .collect::<Vec<_>>();
        let states = content
            .catalog
            .bases
            .iter()
            .map(|definition| {
                let built = self
                    .buildings
                    .buildings
                    .iter()
                    .find(|item| item.id == definition.id.as_str());
                let constructed = built.is_some();
                let level = built.map_or(0, |item| item.level);
                let target_level = if constructed {
                    u16::from(level).saturating_add(1)
                } else {
                    1
                };
                let target_row = content.catalog.level(&definition.id, target_level);
                let gold_cost = target_row.and_then(gold_cost);
                let condition = mutation_condition(self, target_row);
                BuildingStateSnapshot {
                    id: definition.id.to_string(),
                    constructed,
                    level,
                    upgrade_cost: constructed.then_some(gold_cost).flatten(),
                    can_construct: !constructed && target_row.is_some() && condition.is_none(),
                    can_upgrade: constructed && target_row.is_some() && condition.is_none(),
                    condition: if target_row.is_none() && constructed {
                        Some("maximum_level".to_owned())
                    } else {
                        condition
                    },
                    uses: built.map_or(0, |item| item.uses),
                }
            })
            .collect();
        let instances = self
            .buildings
            .buildings
            .iter()
            .filter_map(|instance| {
                let building_id = BaseBuildingId::parse(&instance.id).ok()?;
                let definition = content.catalog.base(&building_id)?;
                let (grid_width, grid_height) = building_grid_size(definition)?;
                let target_row = content
                    .catalog
                    .level(&building_id, u16::from(instance.level).saturating_add(1));
                let condition = mutation_condition(self, target_row);
                Some(BuildingInstanceSnapshot {
                    instance_id: instance.instance_id.clone(),
                    building_id: instance.id.clone(),
                    level: instance.level,
                    grid_x: instance.grid_x,
                    grid_y: instance.grid_y,
                    grid_width,
                    grid_height,
                    sprite_asset_id: definition.base_sprite_asset_id.clone(),
                    upgrade_cost: target_row.and_then(gold_cost),
                    can_upgrade: target_row.is_some() && condition.is_none(),
                    condition,
                    uses: instance.uses,
                    seeded_by: instance.seeded_by.clone(),
                })
            })
            .collect();
        BuildingSystemSnapshot {
            evidence_label: "evil-hunter-1.411-postgresql-authoritative-content",
            town_gold: self.buildings.town_gold,
            definitions,
            states,
            instances,
            hunter_materials: self.buildings.hunter_materials,
            materials: self.buildings.materials,
            runes: self.buildings.runes,
            weapons: self.buildings.weapons,
            armor: self.buildings.armor,
            hunter_equipment_purchases: self.buildings.hunter_equipment_purchases,
            material_stocks: material_catalog_stocks(
                &content.gameplay,
                &self.buildings.material_stocks,
            )
            .iter()
            .map(|stock| MaterialStockSnapshot {
                id: stock.id.clone(),
                display_name: content
                    .gameplay
                    .item(&stock.id)
                    .and_then(|item| item.localized_names.get("en").cloned())
                    .unwrap_or_else(|| stock.id.clone()),
                icon: material_icon_path(&stock.id).unwrap_or_default(),
                town_quantity: stock.town_quantity,
                hunter_quantity: stock.hunter_quantity,
                requested: stock.requested,
                unit_price: stock.unit_price,
                difficulty: material_difficulty_rating(&content.gameplay, &stock.id)
                    .unwrap_or(u8::MAX),
            })
            .collect(),
            recipes: {
                let mut recipes = Vec::new();
                let mut products_by_building =
                    HashMap::<String, Vec<&crate::buildings::EconomyProductDefinition>>::new();
                for product in content.gameplay.products.values() {
                    if let Some(producer) = &product.building_id {
                        products_by_building
                            .entry(producer.to_string())
                            .or_default()
                            .push(product);
                    }
                    if let Some(sale) = product_sale_building_id(&content.gameplay, product) {
                        if product.building_id.as_ref() != Some(&sale) {
                            products_by_building
                                .entry(sale.to_string())
                                .or_default()
                                .push(product);
                        }
                    }
                }
                let product_stock_by_key = self
                    .buildings
                    .product_stocks
                    .iter()
                    .map(|stock| {
                        (
                            (
                                stock.building_instance_id.as_str(),
                                stock.product_id.as_str(),
                            ),
                            stock.quantity,
                        )
                    })
                    .collect::<HashMap<_, _>>();
                for definition in &content.catalog.bases {
                    let Some(building_id) = BaseBuildingId::parse(definition.id.as_str()).ok()
                    else {
                        continue;
                    };
                    let building = self
                        .buildings
                        .buildings
                        .iter()
                        .find(|building| building.id == definition.id.as_str());
                    let mut seen = HashSet::new();
                    let mut gear_buckets = HashMap::new();
                    for product in products_by_building
                        .get(definition.id.as_str())
                        .into_iter()
                        .flatten()
                    {
                        let gear_route = gear_product_route(&content.gameplay, product);
                        let sale_building_id = product_sale_building_id(&content.gameplay, product);
                        let is_native_product = product.building_id.as_ref() == Some(&building_id);
                        let is_sale_product = sale_building_id.as_ref() == Some(&building_id);
                        if !is_native_product && !is_sale_product {
                            continue;
                        }
                        if is_native_product {
                            if let Some(route) = &gear_route {
                                let count = gear_buckets
                                    .entry((route.kind, route.rating))
                                    .or_insert(0_u8);
                                if *count >= 6 {
                                    continue;
                                }
                                *count += 1;
                            } else if seen.len() >= 24 {
                                continue;
                            }
                        }

                        let stock_building = sale_building_id
                            .as_ref()
                            .and_then(|sale_building_id| {
                                self.buildings
                                    .buildings
                                    .iter()
                                    .find(|candidate| candidate.id == sale_building_id.as_str())
                            })
                            .or(building);
                        let stored_stock = stock_building.map_or(0, |stock_building| {
                            product_stock_by_key
                                .get(&(
                                    stock_building.instance_id.as_str(),
                                    product.product_id.as_str(),
                                ))
                                .copied()
                                .unwrap_or(0)
                        });
                        if is_sale_product && stored_stock == 0 {
                            continue;
                        }
                        let stock = stored_stock;
                        let product_name = product_display_name(&product.product_id)
                            .map(str::to_owned)
                            .or_else(|| {
                                product
                                    .outputs
                                    .first()
                                    .and_then(|output| content.gameplay.item(&output.resource_id))
                                    .and_then(|item| item.localized_names.get("en").cloned())
                            })
                            .unwrap_or_else(|| "Unresolved product".to_owned());
                        let dedupe_key = if gear_route.is_some() {
                            product.product_id.clone()
                        } else {
                            product_name.clone()
                        };
                        if !seen.insert(dedupe_key) {
                            continue;
                        }
                        let stock_level =
                            stock_building.map_or(1, |building| u16::from(building.level));
                        let stock_building_id = stock_building
                            .and_then(|building| BaseBuildingId::parse(&building.id).ok())
                            .unwrap_or_else(|| building_id.clone());
                        let capacity = content
                            .catalog
                            .level(&stock_building_id, stock_level)
                            .and_then(|level| level.production_slots)
                            .or_else(|| capacity_for_level(stock_building_id.as_str(), stock_level))
                            .unwrap_or(0);
                        // Preserve the service route when a legacy DB row has the
                        // optional service payload missing but still has recovered
                        // conversion inputs under a service building.
                        let service_product = product.service.is_some()
                            || (ServiceEffectKind::for_building(stock_building_id.as_str())
                                .is_some()
                                && !product.conversion_options.is_empty());
                        recipes.push(ShopRecipeSnapshot {
                            id: product.product_id.clone(),
                            shop_id: definition.id.to_string(),
                            icon: product_icon_path(&product.product_id)
                                .map(str::to_owned)
                                .unwrap_or_default(),
                            product_name,
                            material_costs: product
                                .inputs
                                .iter()
                                .map(|cost| (cost.resource_id.clone(), cost.quantity, 1))
                                .chain(
                                    service_product
                                        .then(|| {
                                            product.conversion_options.iter().map(|option| {
                                                (
                                                    option.input_resource_id.clone(),
                                                    option.input_quantity,
                                                    option.output_stock_quantity,
                                                )
                                            })
                                        })
                                        .into_iter()
                                        .flatten(),
                                )
                                .filter_map(|(material_id, amount, output)| {
                                    u32::try_from(amount)
                                        .ok()
                                        .zip(u32::try_from(output).ok())
                                        .map(|(quantity, output_quantity)| {
                                            let display_name = content
                                                .gameplay
                                                .item(&material_id)
                                                .and_then(|item| {
                                                    item.localized_names.get("en").cloned()
                                                })
                                                .or_else(|| match material_id.as_str() {
                                                    "currency:gem" => Some("Gem".to_owned()),
                                                    "currency:elemental" => {
                                                        Some("Elemental".to_owned())
                                                    }
                                                    _ => None,
                                                })
                                                .unwrap_or_else(|| {
                                                    "Unresolved material".to_owned()
                                                });
                                            RecipeMaterialCostSnapshot {
                                                material_id,
                                                display_name,
                                                quantity,
                                                output_quantity,
                                            }
                                        })
                                })
                                .collect(),
                            stock,
                            sale_price: product.service.as_ref().map_or_else(
                                || {
                                    product
                                        .sale_price
                                        .first()
                                        .map(|price| price.quantity)
                                        .or_else(|| {
                                            consumable_purchase_price(&content.gameplay, product)
                                        })
                                        .or_else(|| gear_purchase_price(&content.gameplay, product))
                                        .unwrap_or(0)
                                },
                                |service| service.use_money,
                            ),
                            kind: if service_product { "service" } else { "craft" },
                            required_level: gear_route.as_ref().map_or_else(
                                || {
                                    product
                                        .service
                                        .as_ref()
                                        .map_or(0, |service| service.required_level)
                                },
                                |route| route.rating,
                            ),
                            duration_ms: product.duration_ms.unwrap_or(0),
                            effect_value: product
                                .service
                                .as_ref()
                                .map_or(0, |service| service.effect_value),
                            effect_kind: service_effect_kind(definition.id.as_str()),
                            capacity,
                        });
                    }
                }
                recipes
            },
        }
    }
}
