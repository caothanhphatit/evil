use super::{
    building_grid_size, can_pay_costs, mutation_condition, pay_costs, placement_is_valid,
    BaseBuildingId, DurableBuilding, OriginalFlowSession, ServerMessage, Uuid,
};

impl OriginalFlowSession {
    pub(super) fn construct_building(&mut self, building_id: &str) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("construct_building", "village_unavailable");
        }
        let Ok(building_id) = BaseBuildingId::parse(building_id) else {
            return self.rejected("construct_building", "building_unknown");
        };
        if self.building_content.catalog.base(&building_id).is_none() {
            return self.rejected("construct_building", "building_unknown");
        }
        self.rejected("construct_building", "placement_required")
    }

    pub(super) fn construct_building_at(
        &mut self,
        building_id: &str,
        grid_x: i32,
        grid_y: i32,
    ) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("construct_building_at", "village_unavailable");
        }
        let Ok(base_id) = BaseBuildingId::parse(building_id) else {
            return self.rejected("construct_building_at", "building_unknown");
        };
        let Some(definition) = self.building_content.catalog.base(&base_id) else {
            return self.rejected("construct_building_at", "building_unknown");
        };
        let max_build = definition.max_instances;
        if max_build == 0
            || self
                .buildings
                .buildings
                .iter()
                .filter(|building| building.id == building_id)
                .count()
                >= max_build as usize
        {
            return self.rejected("construct_building_at", "max_build_reached");
        }
        let Some((grid_width, grid_height)) = building_grid_size(definition) else {
            return self.rejected("construct_building_at", "grid_size_unresolved");
        };
        if !placement_is_valid(
            &self.buildings.buildings,
            &self.building_content.catalog,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
            None,
        ) {
            return self.rejected("construct_building_at", "placement_blocked");
        }
        let Some(row) = self.building_content.catalog.level(&base_id, 1) else {
            return self.rejected("construct_building_at", "build_row_unresolved");
        };
        if let Some(reason) = mutation_condition(self, Some(row)) {
            return self.rejected("construct_building_at", &reason);
        }
        if !can_pay_costs(&self.buildings, &row.costs) {
            return self.rejected("construct_building_at", "insufficient_building_cost");
        }
        pay_costs(&mut self.buildings, &row.costs);
        let instance_id = Uuid::new_v4().to_string();
        self.buildings.next_building_instance_id += 1;
        self.buildings.buildings.push(DurableBuilding {
            instance_id,
            id: building_id.to_owned(),
            equipped_skin_id: None,
            level: 1,
            uses: 0,
            grid_x,
            grid_y,
            seeded_by: None,
        });
        self.accepted("construct_building_at")
    }

    pub(super) fn upgrade_building(&mut self, instance_id: &str) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("upgrade_building", "village_unavailable");
        }
        let Some((index, building)) = self
            .buildings
            .buildings
            .iter()
            .enumerate()
            .find(|(_, building)| building.instance_id == instance_id)
        else {
            return self.rejected("upgrade_building", "building_instance_unknown");
        };
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("upgrade_building", "building_unknown");
        };
        let Some(row) = self
            .building_content
            .catalog
            .level(&building_id, u16::from(building.level).saturating_add(1))
        else {
            return self.rejected("upgrade_building", "maximum_level");
        };
        if let Some(reason) = mutation_condition(self, Some(row)) {
            return self.rejected("upgrade_building", &reason);
        }
        if !can_pay_costs(&self.buildings, &row.costs) {
            return self.rejected("upgrade_building", "insufficient_building_cost");
        }
        pay_costs(&mut self.buildings, &row.costs);
        let Ok(level) = u8::try_from(row.level) else {
            return self.rejected("upgrade_building", "building_level_out_of_range");
        };
        self.buildings.buildings[index].level = level;
        self.accepted("upgrade_building")
    }

    pub(super) fn move_building(
        &mut self,
        instance_id: &str,
        grid_x: i32,
        grid_y: i32,
    ) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("move_building", "village_unavailable");
        }
        let Some((index, building)) = self
            .buildings
            .buildings
            .iter()
            .enumerate()
            .find(|(_, building)| building.instance_id == instance_id)
        else {
            return self.rejected("move_building", "building_instance_unknown");
        };
        if self.hunter_roster.hunters.iter().any(|hunter| {
            hunter
                .hunt
                .pending_trade
                .as_ref()
                .is_some_and(|task| task.building_instance_id == instance_id)
        }) {
            return self.rejected("move_building", "building_has_incoming_hunter");
        }
        let Ok(building_id) = BaseBuildingId::parse(&building.id) else {
            return self.rejected("move_building", "building_unknown");
        };
        let Some(definition) = self.building_content.catalog.base(&building_id) else {
            return self.rejected("move_building", "building_unknown");
        };
        let Some((grid_width, grid_height)) = building_grid_size(definition) else {
            return self.rejected("move_building", "grid_size_unresolved");
        };
        if !placement_is_valid(
            &self.buildings.buildings,
            &self.building_content.catalog,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
            Some(index),
        ) {
            return self.rejected("move_building", "placement_blocked");
        }
        self.buildings.buildings[index].grid_x = grid_x;
        self.buildings.buildings[index].grid_y = grid_y;
        self.accepted("move_building")
    }

    pub(super) fn use_building(&mut self, instance_id: &str) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("use_building", "village_unavailable");
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.instance_id == instance_id)
        else {
            return self.rejected("use_building", "building_instance_unknown");
        };
        self.capability_blocked("use_building", &building.id, &[])
    }
}
