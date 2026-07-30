use super::{BaseBuildingId, BuildingGameplayCatalog, EconomyProductDefinition};

const GEAR_CRAFTING_CAPABILITY: &str = "weapon-and-armor-crafting";
const ACCESSORY_CRAFTING_CAPABILITY: &str = "accessory-crafting";
const WEAPON_SALE_CAPABILITY: &str = "weapon-display-and-sale";
const ARMOR_SALE_CAPABILITY: &str = "armor-display-and-sale";
const ACCESSORY_SALE_CAPABILITY: &str = "accessory-display-and-sale";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearProductFamily {
    Weapon,
    Armor,
    Accessory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GearProductKind {
    Weapon,
    Armor,
    Helmet,
    Gloves,
    Boots,
    Ring,
    Necklace,
    Belt,
}

impl GearProductKind {
    pub fn family(self) -> GearProductFamily {
        match self {
            Self::Weapon => GearProductFamily::Weapon,
            Self::Armor | Self::Helmet | Self::Gloves | Self::Boots => GearProductFamily::Armor,
            Self::Ring | Self::Necklace | Self::Belt => GearProductFamily::Accessory,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GearProductRoute {
    pub producer_building_id: BaseBuildingId,
    pub sale_building_id: BaseBuildingId,
    pub family: GearProductFamily,
    pub kind: GearProductKind,
    pub rating: u16,
    pub difficulty_group: u16,
}

/// Resolves the Blacksmith -> display shop route from recovered capability
/// bindings instead of coupling simulation code to concrete building IDs.
pub fn gear_product_route(
    gameplay: &BuildingGameplayCatalog,
    product: &EconomyProductDefinition,
) -> Option<GearProductRoute> {
    let producer_building_id = product.building_id.clone()?;
    #[cfg(not(test))]
    let gear = gameplay.gear_product(&product.product_id)?;
    #[cfg(test)]
    let test_gear;
    #[cfg(test)]
    let gear = match gameplay.gear_product(&product.product_id) {
        Some(gear) => gear,
        None => {
            test_gear = test_gear_product(&product.product_id)?;
            &test_gear
        }
    };
    let kind = parse_gear_kind(&gear.gear_kind)?;
    let rating = gear.rating;
    let family = kind.family();
    let producer_capability = match family {
        GearProductFamily::Weapon | GearProductFamily::Armor => GEAR_CRAFTING_CAPABILITY,
        GearProductFamily::Accessory => ACCESSORY_CRAFTING_CAPABILITY,
    };
    if !gameplay
        .capabilities_for(&producer_building_id)
        .any(|capability| capability.kind == producer_capability)
    {
        return None;
    }
    let sale_capability = match family {
        GearProductFamily::Weapon => WEAPON_SALE_CAPABILITY,
        GearProductFamily::Armor => ARMOR_SALE_CAPABILITY,
        GearProductFamily::Accessory => ACCESSORY_SALE_CAPABILITY,
    };
    let mut sale_buildings = gameplay
        .capabilities
        .iter()
        .filter(|capability| capability.kind == sale_capability)
        .map(|capability| capability.building_id.clone());
    let sale_building_id = sale_buildings.next()?;
    if sale_buildings.next().is_some() {
        return None;
    }

    Some(GearProductRoute {
        producer_building_id,
        sale_building_id,
        family,
        kind,
        rating,
        difficulty_group: gear.difficulty_group,
    })
}

#[cfg(test)]
fn test_gear_product(product_id: &str) -> Option<crate::buildings::GearProductDefinition> {
    let mut parts = product_id.split(':');
    (parts.next()? == "recipe").then_some(())?;
    let gear_kind = parts.next()?.to_owned();
    let gear_index = parts.next()?.parse::<u32>().ok()?;
    (parts.next()? == "rating").then_some(())?;
    let rating = parts.next()?.parse::<u16>().ok()?;
    parts.next().is_none().then_some(())?;
    let payload: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../packages/content/releases/evil-hunter-1.411/gear-catalog.json"
    ))
    .expect("test gear fixture must decode");
    let difficulty_group = payload["rows"].as_array()?.iter().find_map(|row| {
        (row["kind"] == gear_kind && row["index"] == gear_index)
            .then(|| u16::try_from(row["group"].as_u64()?).ok())
            .flatten()
    })?;
    Some(crate::buildings::GearProductDefinition {
        product_id: product_id.to_owned(),
        gear_kind,
        gear_index,
        rating,
        difficulty_group,
    })
}

fn parse_gear_kind(kind: &str) -> Option<GearProductKind> {
    Some(match kind {
        "weapon" => GearProductKind::Weapon,
        "armor" => GearProductKind::Armor,
        "helmet" => GearProductKind::Helmet,
        "gloves" => GearProductKind::Gloves,
        "boots" => GearProductKind::Boots,
        "ring" => GearProductKind::Ring,
        "necklace" => GearProductKind::Necklace,
        "belt" => GearProductKind::Belt,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::buildings::{BuildingCapabilityDefinition, EconomyProductDefinition};

    fn gameplay() -> BuildingGameplayCatalog {
        BuildingGameplayCatalog {
            registry_id: "test".to_owned(),
            capabilities: vec![
                capability("build_10", GEAR_CRAFTING_CAPABILITY),
                capability("build_7", WEAPON_SALE_CAPABILITY),
                capability("build_8", ARMOR_SALE_CAPABILITY),
                capability("build_21", ACCESSORY_CRAFTING_CAPABILITY),
                capability("build_20", ACCESSORY_SALE_CAPABILITY),
            ],
            items: BTreeMap::new(),
            products: BTreeMap::new(),
            gear_products: [
                ("recipe:weapon:12:rating:3", "weapon", 12, 3, 0),
                ("recipe:weapon:0:rating:0", "weapon", 0, 0, 0),
                ("recipe:armor:2:rating:1", "armor", 2, 1, 0),
                ("recipe:helmet:2:rating:1", "helmet", 2, 1, 0),
                ("recipe:gloves:2:rating:1", "gloves", 2, 1, 0),
                ("recipe:boots:2:rating:1", "boots", 2, 1, 0),
                ("recipe:ring:2:rating:1", "ring", 2, 1, 0),
                ("recipe:necklace:2:rating:1", "necklace", 2, 1, 0),
                ("recipe:belt:2:rating:1", "belt", 2, 1, 0),
            ]
            .into_iter()
            .map(
                |(product_id, gear_kind, gear_index, rating, difficulty_group)| {
                    (
                        product_id.to_owned(),
                        crate::buildings::GearProductDefinition {
                            product_id: product_id.to_owned(),
                            gear_kind: gear_kind.to_owned(),
                            gear_index,
                            rating,
                            difficulty_group,
                        },
                    )
                },
            )
            .collect(),
            consumable_products: BTreeMap::new(),
        }
    }

    fn capability(building_id: &str, kind: &str) -> BuildingCapabilityDefinition {
        BuildingCapabilityDefinition {
            capability_id: format!("capability:{kind}"),
            building_id: BaseBuildingId::parse(building_id).unwrap(),
            kind: kind.to_owned(),
            static_data_ready: true,
            runnable: false,
        }
    }

    fn product(product_id: &str) -> EconomyProductDefinition {
        product_for_building(product_id, "build_10")
    }

    fn product_for_building(product_id: &str, building_id: &str) -> EconomyProductDefinition {
        EconomyProductDefinition {
            product_id: product_id.to_owned(),
            building_id: Some(BaseBuildingId::parse(building_id).unwrap()),
            duration_ms: None,
            exact_mutation_ready: false,
            inputs: Vec::new(),
            outputs: Vec::new(),
            sale_price: Vec::new(),
            service: None,
            conversion_options: Vec::new(),
            random_output: None,
        }
    }

    #[test]
    fn routes_accessories_from_jeweler_to_accessory_shop() {
        let gameplay = gameplay();
        for kind in ["ring", "necklace", "belt"] {
            let route = gear_product_route(
                &gameplay,
                &product_for_building(&format!("recipe:{kind}:2:rating:1"), "build_21"),
            )
            .unwrap();
            assert_eq!(route.family, GearProductFamily::Accessory);
            assert_eq!(route.producer_building_id.as_str(), "build_21");
            assert_eq!(route.sale_building_id.as_str(), "build_20");
        }
    }

    #[test]
    fn routes_weapons_and_wearable_armor_to_their_display_capabilities() {
        let gameplay = gameplay();
        let weapon = gear_product_route(&gameplay, &product("recipe:weapon:12:rating:3")).unwrap();
        assert_eq!(weapon.family, GearProductFamily::Weapon);
        assert_eq!(weapon.sale_building_id.as_str(), "build_7");
        assert_eq!(weapon.rating, 3);

        for kind in ["armor", "helmet", "gloves", "boots"] {
            let route =
                gear_product_route(&gameplay, &product(&format!("recipe:{kind}:2:rating:1")))
                    .unwrap();
            assert_eq!(route.family, GearProductFamily::Armor);
            assert_eq!(route.sale_building_id.as_str(), "build_8");
        }
    }

    #[test]
    fn fails_closed_for_non_gear_products_or_ambiguous_sale_capabilities() {
        let mut gameplay = gameplay();
        assert!(gear_product_route(&gameplay, &product("product:5")).is_none());
        gameplay
            .capabilities
            .push(capability("build_20", WEAPON_SALE_CAPABILITY));
        assert!(gear_product_route(&gameplay, &product("recipe:weapon:0:rating:0")).is_none());
    }
}
