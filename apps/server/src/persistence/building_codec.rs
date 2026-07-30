use super::{
    BaseBuildingId, BuildingRepositoryError, BuildingSkinId, DurableBuilding, DurableBuildingState,
    DurableHunterProgress, DurableMaterialStock, DurableProductStock, DurableTradeSettlement,
    RepositoryError, Row, TownBuildingInstance, TownBuildingInstanceId, TownBuildingState,
    TownMaterialStock, TownProductStock, TownTradeSettlement, Uuid, ACTIVE_BUILDING_RELEASE_ID,
};

pub(super) fn nonempty_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

pub(super) fn fixture_equipment_slot_order(slot_id: &str) -> Result<i16, RepositoryError> {
    match slot_id {
        "gloves" => Ok(0),
        "helmet" => Ok(1),
        "necklace" => Ok(2),
        "boots" => Ok(3),
        "ring" => Ok(4),
        "weapon" => Ok(5),
        "armor" => Ok(6),
        "belt" => Ok(7),
        _ => Err(RepositoryError::InvalidOperation),
    }
}

pub(super) fn db_i64(value: u64) -> Result<i64, RepositoryError> {
    i64::try_from(value).map_err(|_| RepositoryError::InvalidOperation)
}

pub(super) fn db_u64(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, RepositoryError> {
    u64::try_from(row.try_get::<i64, _>(column)?).map_err(|_| RepositoryError::InvalidOperation)
}

pub(super) fn optional_db_u64(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<u64>, RepositoryError> {
    row.try_get::<Option<i64>, _>(column)?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| RepositoryError::InvalidOperation)
}

pub(super) fn optional_db_u32(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<u32>, RepositoryError> {
    row.try_get::<Option<i32>, _>(column)?
        .map(u32::try_from)
        .transpose()
        .map_err(|_| RepositoryError::InvalidOperation)
}

pub(super) fn optional_progress(
    row: &sqlx::postgres::PgRow,
    current_column: &str,
    maximum_column: &str,
) -> Result<Option<DurableHunterProgress>, RepositoryError> {
    match (
        optional_db_u32(row, current_column)?,
        optional_db_u32(row, maximum_column)?,
    ) {
        (None, None) => Ok(None),
        (Some(current), Some(maximum)) => Ok(Some(DurableHunterProgress { current, maximum })),
        _ => Err(RepositoryError::InvalidOperation),
    }
}

pub(super) fn town_from_durable_buildings(
    state: &DurableBuildingState,
) -> Result<TownBuildingState, RepositoryError> {
    let buildings = state
        .buildings
        .iter()
        .map(|building| {
            Ok(TownBuildingInstance {
                instance_id: TownBuildingInstanceId::new(
                    Uuid::parse_str(&building.instance_id).map_err(|_| {
                        BuildingRepositoryError::InvalidTown("instance id must be UUID")
                    })?,
                ),
                building_id: BaseBuildingId::parse(building.id.clone())?,
                equipped_skin_id: building
                    .equipped_skin_id
                    .map(BuildingSkinId::new)
                    .transpose()?,
                level: u16::from(building.level),
                uses: building.uses,
                grid_x: building.grid_x,
                grid_y: building.grid_y,
                seeded_by: building.seeded_by.clone(),
            })
        })
        .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
    Ok(TownBuildingState {
        release_id: ACTIVE_BUILDING_RELEASE_ID.to_owned(),
        town_gold: state.town_gold,
        seed_version: state.town_seed_version,
        next_building_sequence: state.next_building_instance_id,
        buildings,
        hunter_materials: state.hunter_materials,
        materials: state.materials,
        runes: state.runes,
        weapons: state.weapons,
        armor: state.armor,
        hunter_equipment_purchases: state.hunter_equipment_purchases,
        field_trip_id: state.field_trip_id,
        settled_field_trip_id: state.settled_field_trip_id,
        material_stocks: state
            .material_stocks
            .iter()
            .map(|stock| TownMaterialStock {
                id: stock.id.clone(),
                town_quantity: stock.town_quantity,
                hunter_quantity: stock.hunter_quantity,
                requested: stock.requested,
                unit_price: stock.unit_price,
            })
            .collect(),
        product_stocks: state
            .product_stocks
            .iter()
            .map(|stock| {
                Ok(TownProductStock {
                    building_instance_id: TownBuildingInstanceId::new(
                        Uuid::parse_str(&stock.building_instance_id).map_err(|_| {
                            BuildingRepositoryError::InvalidTown(
                                "product stock building instance id must be UUID",
                            )
                        })?,
                    ),
                    product_id: stock.product_id.clone(),
                    quantity: stock.quantity,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?,
        trade_settlements: state
            .trade_settlements
            .iter()
            .map(|settlement| TownTradeSettlement {
                settlement_id: settlement.settlement_id.clone(),
                field_trip_id: settlement.field_trip_id,
                material_id: settlement.material_id.clone(),
                quantity: settlement.quantity,
                unit_price: settlement.unit_price,
                total_gold: settlement.total_gold,
            })
            .collect(),
    })
}

pub(super) fn durable_buildings_from_town(
    state: TownBuildingState,
) -> Result<DurableBuildingState, RepositoryError> {
    let buildings = state
        .buildings
        .into_iter()
        .map(|building| {
            Ok(DurableBuilding {
                instance_id: building.instance_id.get().to_string(),
                id: building.building_id.to_string(),
                equipped_skin_id: building.equipped_skin_id.map(BuildingSkinId::get),
                level: u8::try_from(building.level)
                    .map_err(|_| RepositoryError::InvalidOperation)?,
                uses: building.uses,
                grid_x: building.grid_x,
                grid_y: building.grid_y,
                seeded_by: building.seeded_by,
            })
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;
    Ok(DurableBuildingState {
        town_gold: state.town_gold,
        buildings,
        hunter_materials: state.hunter_materials,
        materials: state.materials,
        runes: state.runes,
        weapons: state.weapons,
        armor: state.armor,
        material_stocks: state
            .material_stocks
            .into_iter()
            .map(|stock| DurableMaterialStock {
                id: stock.id,
                town_quantity: stock.town_quantity,
                hunter_quantity: stock.hunter_quantity,
                requested: stock.requested,
                unit_price: stock.unit_price,
            })
            .collect(),
        product_stocks: state
            .product_stocks
            .into_iter()
            .map(|stock| DurableProductStock {
                building_instance_id: stock.building_instance_id.get().to_string(),
                product_id: stock.product_id,
                quantity: stock.quantity,
            })
            .collect(),
        hunter_equipment_purchases: state.hunter_equipment_purchases,
        town_seed_version: state.seed_version,
        next_building_instance_id: state.next_building_sequence,
        field_trip_id: state.field_trip_id,
        settled_field_trip_id: state.settled_field_trip_id,
        trade_settlements: state
            .trade_settlements
            .into_iter()
            .map(|settlement| DurableTradeSettlement {
                settlement_id: settlement.settlement_id,
                field_trip_id: settlement.field_trip_id,
                material_id: settlement.material_id,
                quantity: settlement.quantity,
                unit_price: settlement.unit_price,
                total_gold: settlement.total_gold,
            })
            .collect(),
    })
}
