use std::collections::HashSet;

use super::{BaseBuildingId, BuildingRepositoryError, BuildingSkinId, TownBuildingInstanceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownBuildingInstance {
    pub instance_id: TownBuildingInstanceId,
    pub building_id: BaseBuildingId,
    pub equipped_skin_id: Option<BuildingSkinId>,
    pub level: u16,
    pub uses: u32,
    pub grid_x: i32,
    pub grid_y: i32,
    pub seeded_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownBuildingState {
    pub release_id: String,
    pub town_gold: u64,
    pub seed_version: u16,
    pub next_building_sequence: u64,
    pub buildings: Vec<TownBuildingInstance>,
    pub hunter_materials: u32,
    pub materials: u32,
    pub runes: u32,
    pub weapons: u32,
    pub armor: u32,
    pub hunter_equipment_purchases: u32,
    pub field_trip_id: u64,
    pub settled_field_trip_id: u64,
    pub material_stocks: Vec<TownMaterialStock>,
    pub product_stocks: Vec<TownProductStock>,
    pub crafted_gear_stocks: Vec<TownCraftedGearStock>,
    pub trade_settlements: Vec<TownTradeSettlement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownMaterialStock {
    pub id: String,
    pub town_quantity: u32,
    pub hunter_quantity: u32,
    pub requested: u32,
    pub unit_price: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownProductStock {
    pub building_instance_id: TownBuildingInstanceId,
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownCraftedGearStock {
    pub building_instance_id: TownBuildingInstanceId,
    pub gear_instance_id: uuid::Uuid,
    pub product_id: String,
    pub gear_kind: String,
    pub rating: u16,
    pub quality: u8,
    pub primary_stat: u32,
    pub option_type: u8,
    pub option_value: u16,
    pub icon_path: String,
    pub ruleset: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownTradeSettlement {
    pub settlement_id: String,
    pub field_trip_id: u64,
    pub material_id: String,
    pub quantity: u32,
    pub unit_price: u64,
    pub total_gold: u64,
}

impl TownBuildingState {
    pub fn validate(&self) -> Result<(), BuildingRepositoryError> {
        if self.release_id.trim().is_empty() {
            return Err(BuildingRepositoryError::InvalidTown(
                "content release is required",
            ));
        }
        if self.settled_field_trip_id > self.field_trip_id {
            return Err(BuildingRepositoryError::InvalidTown(
                "settled field trip cannot exceed the latest trip",
            ));
        }
        let mut instance_ids = HashSet::with_capacity(self.buildings.len());
        for building in &self.buildings {
            if building.level == 0 {
                return Err(BuildingRepositoryError::InvalidTown(
                    "building level must be positive",
                ));
            }
            if !instance_ids.insert(building.instance_id) {
                return Err(BuildingRepositoryError::DuplicateInstance(
                    building.instance_id,
                ));
            }
        }
        let mut material_ids = HashSet::with_capacity(self.material_stocks.len());
        for stock in &self.material_stocks {
            if stock.id.trim().is_empty() || !material_ids.insert(stock.id.as_str()) {
                return Err(BuildingRepositoryError::InvalidTown(
                    "material stock ids must be non-empty and unique",
                ));
            }
        }
        let mut gear_instance_ids = HashSet::with_capacity(self.crafted_gear_stocks.len());
        for gear in &self.crafted_gear_stocks {
            if gear.product_id.trim().is_empty()
                || gear.gear_kind.trim().is_empty()
                || gear.ruleset.trim().is_empty()
                || !instance_ids.contains(&gear.building_instance_id)
                || !gear_instance_ids.insert(gear.gear_instance_id)
            {
                return Err(BuildingRepositoryError::InvalidTown(
                    "crafted gear stock contains an invalid or duplicate row",
                ));
            }
        }
        let building_ids = self
            .buildings
            .iter()
            .map(|building| building.instance_id)
            .collect::<HashSet<_>>();
        let mut product_keys = HashSet::with_capacity(self.product_stocks.len());
        for stock in &self.product_stocks {
            if stock.product_id.trim().is_empty()
                || !building_ids.contains(&stock.building_instance_id)
                || !product_keys.insert((stock.building_instance_id, stock.product_id.as_str()))
            {
                return Err(BuildingRepositoryError::InvalidTown(
                    "product stocks must reference a building and have unique non-empty products",
                ));
            }
        }
        let mut settlement_ids = HashSet::with_capacity(self.trade_settlements.len());
        for settlement in &self.trade_settlements {
            if settlement.settlement_id.trim().is_empty()
                || settlement.material_id.trim().is_empty()
                || settlement.field_trip_id == 0
                || settlement.quantity == 0
                || !settlement_ids.insert(settlement.settlement_id.as_str())
            {
                return Err(BuildingRepositoryError::InvalidTown(
                    "trade settlements must have valid unique identities",
                ));
            }
        }
        Ok(())
    }
}
