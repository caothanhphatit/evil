use std::collections::BTreeMap;

use crate::buildings::BuildingGameplayCatalog;

use super::original_flow::{DurableBuildingState, DurableMaterialStock};

#[cfg(test)]
pub(super) const ACTIVE_MATERIAL_REQUEST: u32 = 1;

// Generated from core-economy-tables-v1.json materials[index].rating. The six decoded ratings
// map directly to Trading Post levels Easy through Torment.
const MATERIAL_DIFFICULTY_RATINGS: [u8; 369] = [
    0, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0,
    0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0,
    1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1,
    2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2,
    3, 3, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 0, 0, 1, 2, 3,
    4, 0, 1, 2, 3, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0, 0, 5,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 4, 5, 5, 5,
    4, 5, 5, 5, 4, 5, 5, 5, 4, 5, 5, 5, 4, 5, 5, 5, 4, 5, 5, 5, 4, 5, 5, 5, 4, 5, 5, 5, 4, 5, 5, 5,
    4, 5, 5, 5, 5, 5, 5, 5, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 0, 0, 1, 2,
    3, 4, 5, 5, 5, 5, 5, 5, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 4, 5, 5, 5, 5, 5, 5, 4, 5, 5, 5, 5, 5,
];

pub(super) fn material_difficulty_rating(material_id: &str) -> Option<u8> {
    let index = material_id
        .strip_prefix("material:")?
        .parse::<usize>()
        .ok()?;
    MATERIAL_DIFFICULTY_RATINGS.get(index).copied()
}

/// Projects the decoded material price table together with the town's durable stock state.
/// This keeps zero-stock materials requestable without manufacturing inventory or hunter loot.
pub(super) fn material_catalog_stocks(
    gameplay: &BuildingGameplayCatalog,
    durable: &[DurableMaterialStock],
) -> Vec<DurableMaterialStock> {
    let mut stocks = gameplay
        .items
        .values()
        .filter_map(|item| {
            let unit_price = item.town_pays_hunter_gold_per_unit?;
            let display_name = item.localized_names.get("en")?.trim();
            if display_name.is_empty() || display_name.ends_with("---") {
                return None;
            }
            let existing = durable.iter().find(|stock| stock.id == item.item_id);
            Some((
                item.item_id.clone(),
                DurableMaterialStock {
                    id: item.item_id.clone(),
                    town_quantity: existing.map_or(0, |stock| stock.town_quantity),
                    hunter_quantity: existing.map_or(0, |stock| stock.hunter_quantity),
                    requested: existing.map_or(0, |stock| stock.requested),
                    unit_price,
                },
            ))
        })
        .collect::<BTreeMap<_, _>>();

    // Preserve persisted rows that are no longer present in the current release so no stock is
    // silently hidden during a content migration.
    for stock in durable {
        if gameplay.items.contains_key(&stock.id) {
            continue;
        }
        stocks
            .entry(stock.id.clone())
            .or_insert_with(|| stock.clone());
    }
    let mut stocks = stocks.into_values().collect::<Vec<_>>();
    stocks.sort_by_key(|stock| {
        stock
            .id
            .strip_prefix("material:")
            .and_then(|index| index.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    });
    stocks
}

/// Marks an empty return as observed, but fails closed when a sale needs settlement. The decoded
/// durable state currently pools hunter stock and cannot identify the seller whose wallet must be
/// credited; deducting town gold here would violate economy conservation.
pub(super) fn settle_returning_hunters(state: &mut DurableBuildingState) -> bool {
    let trip_id = state.field_trip_id;
    if trip_id == 0 || state.settled_field_trip_id >= trip_id {
        return false;
    }

    if state
        .material_stocks
        .iter()
        .any(|stock| stock.requested > 0 && stock.hunter_quantity > 0 && stock.unit_price > 0)
    {
        return false;
    }
    state.settled_field_trip_id = trip_id;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_preserves_economy_until_seller_attribution_is_available() {
        let mut state = DurableBuildingState {
            town_gold: 25,
            field_trip_id: 4,
            material_stocks: vec![DurableMaterialStock {
                id: "material:1".to_owned(),
                town_quantity: 0,
                hunter_quantity: 5,
                requested: ACTIVE_MATERIAL_REQUEST,
                unit_price: 10,
            }],
            ..DurableBuildingState::default()
        };

        assert!(!settle_returning_hunters(&mut state));
        assert_eq!(state.town_gold, 25);
        assert_eq!(state.material_stocks[0].town_quantity, 0);
        assert_eq!(state.material_stocks[0].hunter_quantity, 5);
        assert_eq!(state.material_stocks[0].requested, ACTIVE_MATERIAL_REQUEST);
        assert!(state.trade_settlements.is_empty());
        assert!(!settle_returning_hunters(&mut state));
        assert!(state.trade_settlements.is_empty());
    }

    #[test]
    fn decoded_material_ratings_cover_all_six_trading_post_levels() {
        assert_eq!(material_difficulty_rating("material:1"), Some(0));
        assert_eq!(material_difficulty_rating("material:2"), Some(1));
        assert_eq!(material_difficulty_rating("material:3"), Some(2));
        assert_eq!(material_difficulty_rating("material:4"), Some(3));
        assert_eq!(material_difficulty_rating("material:5"), Some(4));
        assert_eq!(material_difficulty_rating("material:171"), Some(5));
        assert_eq!(material_difficulty_rating("currency:gold"), None);
    }

    #[test]
    fn material_catalog_excludes_unresolved_placeholder_rows() {
        let gameplay = BuildingGameplayCatalog {
            registry_id: "test".to_owned(),
            capabilities: Vec::new(),
            items: [
                ("material:1", "Linen Cloth", 10),
                ("material:132", "B---", 0),
            ]
            .into_iter()
            .map(|(id, name, price)| {
                (
                    id.to_owned(),
                    crate::buildings::EconomyItemDefinition {
                        item_id: id.to_owned(),
                        internal_name: None,
                        item_type: Some("material".to_owned()),
                        stack_limit: None,
                        town_pays_hunter_gold_per_unit: Some(price),
                        localized_names: [("en".to_owned(), name.to_owned())].into_iter().collect(),
                        buy_price: Vec::new(),
                        sell_price: Vec::new(),
                        hunter_pays_town_gold_by_tier: Vec::new(),
                    },
                )
            })
            .collect(),
            products: BTreeMap::new(),
        };

        let projected = material_catalog_stocks(&gameplay, &[]);

        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].id, "material:1");
    }
}
