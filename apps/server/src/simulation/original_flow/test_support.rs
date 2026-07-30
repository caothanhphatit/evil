use super::{
    Arc, AuthoritativeBuildingContent, BaseBuildingDefinition, BaseBuildingId,
    BuildingCapabilityDefinition, BuildingCatalog, BuildingGameplayCatalog,
    BuildingLevelDefinition, BuildingLevelPrerequisite, DurableBuilding, DurableBuildingState,
    EconomyAmount, EconomyItemDefinition, Uuid,
};

#[cfg(test)]
pub(crate) fn test_authoritative_building_content() -> Arc<AuthoritativeBuildingContent> {
    use std::collections::{BTreeMap, HashSet};
    use std::sync::OnceLock;

    use crate::content::building_registry::canonical_building_content;

    static CONTENT: OnceLock<Arc<AuthoritativeBuildingContent>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let embedded = canonical_building_content().expect("test building registry");
            let mut bases = Vec::with_capacity(embedded.buildings.len());
            let mut levels = Vec::new();
            for building in &embedded.buildings {
                let building_id = BaseBuildingId::parse(&building.id).expect("base building id");
                let [grid_width, grid_height] = building.source_data.grid_size.as_slice() else {
                    panic!("test building grid size");
                };
                bases.push(BaseBuildingDefinition {
                    id: building_id.clone(),
                    registry_id: embedded.registry_id.clone(),
                    display_name: building.display_name.clone(),
                    category: building.category.clone(),
                    source_type: building.source_data.source_type,
                    max_instances: u32::try_from(building.source_data.max_build)
                        .expect("max instances"),
                    grid_width: u16::try_from(*grid_width).expect("grid width"),
                    grid_height: u16::try_from(*grid_height).expect("grid height"),
                    movable: Some(building.source_data.movable != 0),
                    constructible: None,
                    base_sprite_asset_id: building.base_sprite_asset_id.clone(),
                });
                let mut seen_levels = HashSet::new();
                for row in building.build_rows.iter().chain(&building.levels) {
                    if !seen_levels.insert(row.level) {
                        continue;
                    }
                    levels.push(BuildingLevelDefinition {
                        building_id: building_id.clone(),
                        level: u16::from(row.level),
                        upgrade_duration_ms: None,
                        inventory_capacity: None,
                        production_slots: None,
                        costs: row
                            .costs
                            .iter()
                            .map(|cost| EconomyAmount {
                                resource_id: cost.item_id.clone(),
                                quantity: cost.quantity,
                            })
                            .collect(),
                        prerequisites: row
                            .required_town_hall_level
                            .map(|required_level| BuildingLevelPrerequisite {
                                building_id: BaseBuildingId::parse("build_1")
                                    .expect("town hall id"),
                                required_level: u16::from(required_level),
                            })
                            .into_iter()
                            .collect(),
                    });
                }
            }
            let catalog = BuildingCatalog {
                registry_id: embedded.registry_id.clone(),
                bases,
                levels,
                skins: Vec::new(),
            };
            let capabilities = embedded
                .capabilities
                .iter()
                .enumerate()
                .map(|(index, capability)| BuildingCapabilityDefinition {
                    capability_id: format!("test-capability-{index}"),
                    building_id: BaseBuildingId::parse(&capability.building_id)
                        .expect("capability building id"),
                    kind: capability.kind.clone(),
                    static_data_ready: capability.static_data_ready,
                    runnable: capability.runnable,
                })
                .collect();
            let items = embedded
                .items
                .iter()
                .map(|(item_id, item)| {
                    (
                        item_id.clone(),
                        EconomyItemDefinition {
                            item_id: item_id.clone(),
                            internal_name: item.internal_name.clone(),
                            item_type: item.item_type.clone(),
                            stack_limit: item.stack_limit,
                            town_pays_hunter_gold_per_unit: item.town_pays_hunter_gold_per_unit,
                            localized_names: item
                                .display_name
                                .as_ref()
                                .map(|name| ("en".to_owned(), name.clone()))
                                .into_iter()
                                .collect::<BTreeMap<_, _>>(),
                            buy_price: item
                                .buy_price
                                .as_deref()
                                .unwrap_or_default()
                                .iter()
                                .map(|amount| EconomyAmount {
                                    resource_id: amount.item_id.clone(),
                                    quantity: amount.quantity,
                                })
                                .collect(),
                            sell_price: item
                                .sell_price
                                .as_deref()
                                .unwrap_or_default()
                                .iter()
                                .map(|amount| EconomyAmount {
                                    resource_id: amount.item_id.clone(),
                                    quantity: amount.quantity,
                                })
                                .collect(),
                            hunter_pays_town_gold_by_tier: item
                                .hunter_pays_town_gold_by_tier
                                .clone()
                                .unwrap_or_default(),
                        },
                    )
                })
                .collect();
            Arc::new(
                AuthoritativeBuildingContent::new(
                    catalog,
                    BuildingGameplayCatalog {
                        registry_id: embedded.registry_id.clone(),
                        capabilities,
                        items,
                        products: BTreeMap::new(),
                    },
                )
                .expect("test authoritative building content"),
            )
        })
        .clone()
}

#[cfg(test)]
pub(super) fn test_town_building_state() -> DurableBuildingState {
    let mut state = DurableBuildingState {
        town_seed_version: 2,
        ..DurableBuildingState::default()
    };
    for id in 1_u128..=28 {
        let slot = i32::try_from(id - 1).unwrap();
        state.buildings.push(DurableBuilding {
            instance_id: Uuid::from_u128(id).to_string(),
            id: format!("build_{id}"),
            equipped_skin_id: None,
            level: 1,
            uses: 0,
            grid_x: ((slot % 7) - 3) * 4,
            grid_y: ((slot / 7) * 4) - 6,
            seeded_by: Some("town-template:default-town-v2".to_owned()),
        });
    }
    state
}
