use super::{
    gear_product_route, AuthoritativeBuildingContent, BaseBuildingDefinition, BaseBuildingId,
    BuildingDefinitionSnapshot, BuildingGameplayCatalog, BuildingLevelDefinition, DurableBuilding,
    DurableBuildingState, DurableHunterState, EconomyAmount, HunterServiceGauge,
    NavigationObstacle, OriginalFlowSession, ServiceEffectKind, TOWN_GRID_MAX, TOWN_GRID_MIN,
    TOWN_NAV_CELL_HEIGHT, TOWN_NAV_CELL_WIDTH, TOWN_NAV_ORIGIN_X, TOWN_NAV_ORIGIN_Y,
    TOWN_ROAM_ANCHORS,
};
pub(super) fn drop_icon_path(item_id: &str) -> String {
    if item_id == "gold" {
        return "/content/releases/original-flow-v1/sprites/top_ic_01_gold_24__4677.png".to_owned();
    }
    material_icon_path(item_id).unwrap_or_default()
}

pub(super) fn building_definition_snapshot(
    building: &BaseBuildingDefinition,
    content: &AuthoritativeBuildingContent,
) -> BuildingDefinitionSnapshot {
    let construction = content.catalog.level(&building.id, 1);
    let prerequisite = construction.and_then(|level| level.prerequisites.first());
    BuildingDefinitionSnapshot {
        id: building.id.to_string(),
        name: building.display_name.clone(),
        feature: building
            .category
            .clone()
            .or_else(|| {
                content
                    .gameplay
                    .capabilities_for(&building.id)
                    .find(|capability| capability.static_data_ready)
                    .map(|capability| capability.kind.clone())
            })
            .unwrap_or_else(|| "unresolved".to_owned()),
        max_level: content
            .catalog
            .levels
            .iter()
            .filter(|level| level.building_id == building.id)
            .filter_map(|level| u8::try_from(level.level).ok())
            .max()
            .unwrap_or(0),
        construct_cost: construction.and_then(gold_cost).unwrap_or(0),
        prerequisite_id: prerequisite.map(|value| value.building_id.to_string()),
        prerequisite_level: prerequisite
            .and_then(|value| u8::try_from(value.required_level).ok())
            .unwrap_or(0),
        max_build: building.max_instances,
        grid_width: building_grid_size(building).map_or(0, |size| size.0),
        grid_height: building_grid_size(building).map_or(0, |size| size.1),
        sprite_asset_id: building.base_sprite_asset_id.clone(),
    }
}

pub(super) fn building_grid_size(building: &BaseBuildingDefinition) -> Option<(u32, u32)> {
    Some((
        u32::from(building.grid_width),
        u32::from(building.grid_height),
    ))
}

pub(super) fn mutation_condition(
    flow: &OriginalFlowSession,
    row: Option<&BuildingLevelDefinition>,
) -> Option<String> {
    let row = row?;
    row.prerequisites.iter().find_map(|prerequisite| {
        let current_level = flow
            .buildings
            .buildings
            .iter()
            .filter(|building| building.id == prerequisite.building_id.as_str())
            .map(|building| u16::from(building.level))
            .max()
            .unwrap_or(0);
        (current_level < prerequisite.required_level).then(|| {
            format!(
                "building_prerequisite_required:{}",
                prerequisite.building_id
            )
        })
    })
}

pub(super) fn can_pay_costs(state: &DurableBuildingState, costs: &[EconomyAmount]) -> bool {
    costs.iter().all(|cost| {
        if cost.resource_id == "currency:gold" {
            state.town_gold >= cost.quantity
        } else {
            state
                .material_stocks
                .iter()
                .find(|stock| stock.id == cost.resource_id)
                .is_some_and(|stock| u64::from(stock.town_quantity) >= cost.quantity)
        }
    })
}

/// Resolves the recovered consumable price table for a crafted potion row.
/// Product `salePrice` is intentionally absent for these rows in the catalog.
pub(super) fn product_sale_building_id(
    gameplay: &BuildingGameplayCatalog,
    product: &crate::buildings::EconomyProductDefinition,
) -> Option<BaseBuildingId> {
    if let Some(route) = gear_product_route(gameplay, product) {
        return Some(route.sale_building_id);
    }
    let is_potion = product
        .building_id
        .as_ref()
        .is_some_and(|id| id.as_str() == "build_14")
        && product.outputs.len() == 1
        && product.outputs[0].resource_id.starts_with("consumable:");
    is_potion.then(|| BaseBuildingId::parse("build_11").expect("potion shop id is canonical"))
}

pub(super) fn consumable_purchase_price(
    gameplay: &BuildingGameplayCatalog,
    product: &crate::buildings::EconomyProductDefinition,
) -> Option<u64> {
    let output = product.outputs.first()?;
    if !output.resource_id.starts_with("consumable:") || output.quantity == 0 {
        return None;
    }
    let level = product
        .product_id
        .split_once(":level:")?
        .1
        .parse::<usize>()
        .ok()?;
    gameplay
        .item(&output.resource_id)?
        .hunter_pays_town_gold_by_tier
        .get(level)
        .copied()
}

/// Resolves the recovered `buyMoneyByRating` value from the output gear item.
pub(super) fn gear_purchase_price(
    gameplay: &BuildingGameplayCatalog,
    product: &crate::buildings::EconomyProductDefinition,
) -> Option<u64> {
    let route = gear_product_route(gameplay, product)?;
    let output = product.outputs.first()?;
    if output.quantity != 1 || !output.resource_id.starts_with("gear:") {
        return None;
    }
    gameplay
        .item(&output.resource_id)?
        .hunter_pays_town_gold_by_tier
        .get(usize::from(route.rating))
        .copied()
}

pub(super) fn product_display_name(product_id: &str) -> Option<&'static str> {
    Some(match product_id {
        "product:0" => "Small Room",
        "product:1" => "Standard Room",
        "product:2" => "Superior Room",
        "product:3" => "Deluxe Room",
        "product:4" => "Suite Room",
        "product:5" => "Linen Bandage",
        "product:6" => "Wool Bandage",
        "product:7" => "Silk Bandage",
        "product:8" => "Magic Bandage",
        "product:9" => "Hell Bandage",
        "product:10" => "Cake",
        "product:11" => "Parfait",
        "product:12" => "Handmade Burger",
        "product:13" => "Tomato Pasta",
        "product:14" => "Tenderloin Steak",
        "product:15" => "Orange Juice",
        "product:16" => "Beer",
        "product:17" => "Red Wine",
        "product:18" => "Cocktail",
        "product:19" => "Whiskey",
        "product:29" => "Luxury Room",
        "product:30" => "Shiny Bandage",
        "product:31" => "Three Course Meal",
        "product:32" => "Vodka",
        "product:48" => "Special Room",
        "product:49" => "Pink Silk Bandage",
        "product:50" => "Afternoon Meal",
        "product:51" => "Tequila",
        _ => return None,
    })
}

pub(super) fn product_icon_path(product_id: &str) -> Option<&'static str> {
    Some(match product_id {
        "product:0" => "/content/releases/original-flow-v1/sprites/product_00__3523.png",
        "product:1" => "/content/releases/original-flow-v1/sprites/product_01__4988.png",
        "product:2" => "/content/releases/original-flow-v1/sprites/product_02__4912.png",
        "product:3" => "/content/releases/original-flow-v1/sprites/product_03__2634.png",
        "product:4" => "/content/releases/original-flow-v1/sprites/product_04__7168.png",
        "product:5" => "/content/releases/original-flow-v1/sprites/product_05__2957.png",
        "product:6" => "/content/releases/original-flow-v1/sprites/product_06__3994.png",
        "product:7" => "/content/releases/original-flow-v1/sprites/product_07__2037.png",
        "product:8" => "/content/releases/original-flow-v1/sprites/product_08__6490.png",
        "product:9" => "/content/releases/original-flow-v1/sprites/product_09__1935.png",
        "product:10" => "/content/releases/original-flow-v1/sprites/product_10__6271.png",
        "product:11" => "/content/releases/original-flow-v1/sprites/product_11__2026.png",
        "product:12" => "/content/releases/original-flow-v1/sprites/product_12__1368.png",
        "product:13" => "/content/releases/original-flow-v1/sprites/product_13__3637.png",
        "product:14" => "/content/releases/original-flow-v1/sprites/product_14__1604.png",
        "product:15" => "/content/releases/original-flow-v1/sprites/product_15__6488.png",
        "product:16" => "/content/releases/original-flow-v1/sprites/product_16__3707.png",
        "product:17" => "/content/releases/original-flow-v1/sprites/product_17__6592.png",
        "product:18" => "/content/releases/original-flow-v1/sprites/product_18__6216.png",
        "product:19" => "/content/releases/original-flow-v1/sprites/product_19__5193.png",
        "product:29" => "/content/releases/original-flow-v1/sprites/product_29__6396.png",
        "product:30" => "/content/releases/original-flow-v1/sprites/product_30__3026.png",
        "product:31" => "/content/releases/original-flow-v1/sprites/product_31__1771.png",
        "product:32" => "/content/releases/original-flow-v1/sprites/product_32__7065.png",
        "product:48" => "/content/releases/original-flow-v1/sprites/product_48__4411.png",
        "product:49" => "/content/releases/original-flow-v1/sprites/product_49__4142.png",
        "product:50" => "/content/releases/original-flow-v1/sprites/product_50__4905.png",
        "product:51" => "/content/releases/original-flow-v1/sprites/product_51__6664.png",
        _ => return None,
    })
}

pub(super) fn material_icon_path(material_id: &str) -> Option<String> {
    if let Some(index) = material_id
        .strip_prefix("material:")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|index| *index < 369)
    {
        return Some(format!(
            "/content/releases/evil-hunter-1.411/material-icons/material-{index}.png"
        ));
    }
    Some(
        match material_id {
            "currency:gem" => "/content/releases/original-flow-v1/sprites/top_ic_02_gem__6963.png",
            "currency:elemental" => {
                "/content/releases/original-flow-v1/sprites/top_ic_03_element__4250.png"
            }
            _ => return None,
        }
        .to_owned(),
    )
}

pub(super) fn service_effect_kind(building_id: &str) -> &'static str {
    match building_id {
        "build_9" => "stamina",
        "build_12" => "HP",
        "build_13" => "satiety",
        "build_19" => "mood",
        _ => "service",
    }
}

pub(super) fn hunter_service_gauge(
    hunter: &DurableHunterState,
    effect_kind: ServiceEffectKind,
) -> HunterServiceGauge {
    match effect_kind {
        ServiceEffectKind::Hp => HunterServiceGauge {
            current: hunter.current_hp,
            maximum: hunter.max_hp,
        },
        ServiceEffectKind::Stamina => hunter.stamina,
        ServiceEffectKind::Satiety => hunter.satiety,
        ServiceEffectKind::Mood => hunter.mood,
    }
}

pub(super) fn pay_costs(state: &mut DurableBuildingState, costs: &[EconomyAmount]) {
    for cost in costs {
        if cost.resource_id == "currency:gold" {
            state.town_gold -= cost.quantity;
        } else if let Some(stock) = state
            .material_stocks
            .iter_mut()
            .find(|stock| stock.id == cost.resource_id)
        {
            stock.town_quantity -= u32::try_from(cost.quantity)
                .expect("validated building cost fits available u32 stock");
        }
    }
}

pub(super) fn placement_is_valid(
    buildings: &[DurableBuilding],
    catalog: &crate::buildings::BuildingCatalog,
    grid_x: i32,
    grid_y: i32,
    grid_width: u32,
    grid_height: u32,
    ignored_index: Option<usize>,
) -> bool {
    let Ok(width) = i32::try_from(grid_width) else {
        return false;
    };
    let Ok(height) = i32::try_from(grid_height) else {
        return false;
    };
    let Some(right) = grid_x.checked_add(width) else {
        return false;
    };
    let Some(bottom) = grid_y.checked_add(height) else {
        return false;
    };
    if grid_x < TOWN_GRID_MIN
        || grid_y < TOWN_GRID_MIN
        || right > TOWN_GRID_MAX
        || bottom > TOWN_GRID_MAX
    {
        return false;
    }
    buildings.iter().enumerate().all(|(index, building)| {
        if ignored_index == Some(index) {
            return true;
        }
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return false;
        };
        let Some(definition) = catalog.base(&building_id) else {
            return false;
        };
        let Some((other_width, other_height)) = building_grid_size(definition) else {
            return false;
        };
        let Ok(other_width) = i32::try_from(other_width) else {
            return false;
        };
        let Ok(other_height) = i32::try_from(other_height) else {
            return false;
        };
        right <= building.grid_x
            || grid_x >= building.grid_x + other_width
            || bottom <= building.grid_y
            || grid_y >= building.grid_y + other_height
    })
}

pub(super) fn town_navigation_obstacles(
    buildings: &[DurableBuilding],
    catalog: &crate::buildings::BuildingCatalog,
) -> Vec<NavigationObstacle> {
    buildings
        .iter()
        .filter_map(|building| building_navigation_obstacle(building, catalog))
        .collect()
}

pub(super) fn building_navigation_obstacle(
    building: &DurableBuilding,
    catalog: &crate::buildings::BuildingCatalog,
) -> Option<NavigationObstacle> {
    let building_id = BaseBuildingId::parse(&building.id).ok()?;
    let definition = catalog.base(&building_id)?;
    let (width, height) = building_grid_size(definition)?;
    let width = i32::try_from(width).ok()?;
    let height = i32::try_from(height).ok()?;
    Some(NavigationObstacle {
        min_x: TOWN_NAV_ORIGIN_X + building.grid_x * TOWN_NAV_CELL_WIDTH,
        max_x: TOWN_NAV_ORIGIN_X + (building.grid_x + width) * TOWN_NAV_CELL_WIDTH,
        min_y: TOWN_NAV_ORIGIN_Y + building.grid_y * TOWN_NAV_CELL_HEIGHT,
        max_y: TOWN_NAV_ORIGIN_Y + (building.grid_y + height) * TOWN_NAV_CELL_HEIGHT,
    })
}

pub(super) fn town_revival_point(
    buildings: &[DurableBuilding],
    catalog: &crate::buildings::BuildingCatalog,
    obstacles: &[NavigationObstacle],
) -> Option<(i32, i32)> {
    const CLEARANCE: i32 = 15;
    let sanctuary = buildings.iter().find(|building| building.id == "build_2")?;
    let footprint = building_navigation_obstacle(sanctuary, catalog)?;
    let center_x = footprint.min_x + (footprint.max_x - footprint.min_x) / 2;
    let center_y = footprint.min_y + (footprint.max_y - footprint.min_y) / 2;
    let candidates = [
        (center_x, footprint.max_y + CLEARANCE),
        (footprint.max_x + CLEARANCE, center_y),
        (footprint.min_x - CLEARANCE, center_y),
        (center_x, footprint.min_y - CLEARANCE),
    ];
    candidates
        .into_iter()
        .chain(TOWN_ROAM_ANCHORS)
        .find(|(x, y)| {
            obstacles.iter().all(|obstacle| {
                *x < obstacle.min_x - 14
                    || *x > obstacle.max_x + 14
                    || *y < obstacle.min_y - 14
                    || *y > obstacle.max_y + 14
            })
        })
}

pub(super) fn town_building_interaction_point(
    building: &DurableBuilding,
    catalog: &crate::buildings::BuildingCatalog,
    obstacles: &[NavigationObstacle],
) -> Option<(i32, i32)> {
    const CLEARANCE: i32 = 15;
    let footprint = building_navigation_obstacle(building, catalog)?;
    let center_x = footprint.min_x + (footprint.max_x - footprint.min_x) / 2;
    let center_y = footprint.min_y + (footprint.max_y - footprint.min_y) / 2;
    [
        (center_x, footprint.max_y + CLEARANCE),
        (footprint.max_x + CLEARANCE, center_y),
        (footprint.min_x - CLEARANCE, center_y),
        (center_x, footprint.min_y - CLEARANCE),
    ]
    .into_iter()
    .find(|(x, y)| {
        obstacles.iter().all(|obstacle| {
            *x < obstacle.min_x - 14
                || *x > obstacle.max_x + 14
                || *y < obstacle.min_y - 14
                || *y > obstacle.max_y + 14
        })
    })
}

pub(super) fn gold_cost(row: &BuildingLevelDefinition) -> Option<u64> {
    row.costs
        .iter()
        .find(|cost| cost.resource_id == "currency:gold")
        .map(|cost| cost.quantity)
}
