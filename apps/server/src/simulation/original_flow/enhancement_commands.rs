use super::{
    gear_product_route, release_hunter_from_enhancement, town_building_interaction_point,
    DurableGearEnhancementTask, GearEnhancementTaskStatus, HashSet, OriginalFlowSession,
    ServerMessage, Uuid, GEAR_ENHANCEMENT_BLOCKERS, GEAR_ENHANCEMENT_WORKFLOW_VERSION,
    MAX_GEAR_ENHANCEMENT_LEVEL,
};

impl OriginalFlowSession {
    pub(super) fn start_hunter_enhancement(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
    ) -> ServerMessage {
        const INTENT: &str = "start_hunter_enhancement";
        if !self.shared_world_active() {
            return self.rejected(INTENT, "village_unavailable");
        }
        let key = format!("{INTENT}:{hunter_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted(INTENT)
                } else {
                    self.rejected(INTENT, "command_id_conflict")
                };
            }
        }
        let Some(building) = self
            .buildings
            .buildings
            .iter()
            .find(|building| building.id == "build_15")
        else {
            return self.rejected(INTENT, "enhancement_forge_unavailable");
        };
        let obstacles = super::town_navigation_obstacles(
            &self.buildings.buildings,
            &self.building_content.catalog,
        );
        let Some((interaction_x, interaction_y)) =
            town_building_interaction_point(building, &self.building_content.catalog, &obstacles)
        else {
            return self.rejected(INTENT, "enhancement_forge_geometry_unavailable");
        };
        let building_instance_id = building.instance_id.clone();
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(INTENT, "hunter_unknown");
        };
        if !hunter.hunt.is_idle() {
            return self.rejected(INTENT, "hunter_not_in_town");
        }
        if hunter.current_hp == 0 {
            return self.rejected(INTENT, "hunter_unavailable");
        }
        if hunter.hunt.gear_enhancement.is_some() {
            return self.rejected(INTENT, "enhancement_task_already_active");
        }
        hunter.hunt.gear_enhancement = Some(DurableGearEnhancementTask {
            workflow_version: GEAR_ENHANCEMENT_WORKFLOW_VERSION,
            building_instance_id,
            status: GearEnhancementTaskStatus::Traveling,
            interaction_x,
            interaction_y,
            blockers: GEAR_ENHANCEMENT_BLOCKERS
                .iter()
                .map(|blocker| (*blocker).to_owned())
                .collect(),
            ..DurableGearEnhancementTask::default()
        });
        hunter.profile.action_state = "traveling_to_enhancement_forge".to_owned();
        hunter.profile.animation_name = "hunter_walk".to_owned();
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted(INTENT)
    }

    pub(super) fn enhance_hunter_gear(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        gear_instance_id: Uuid,
        mode: &str,
        optional_material_ids: &[String],
    ) -> ServerMessage {
        const INTENT: &str = "enhance_hunter_gear";
        if !self.shared_world_active() {
            return self.rejected(INTENT, "village_unavailable");
        }
        let target_level = match mode {
            "single" => None,
            "to_10" => Some(10),
            "to_15" => Some(15),
            "to_20" => Some(MAX_GEAR_ENHANCEMENT_LEVEL),
            _ => return self.rejected(INTENT, "enhancement_mode_invalid"),
        };
        let mut unique_materials = HashSet::new();
        for material_id in optional_material_ids {
            if !unique_materials.insert(material_id.as_str()) {
                return self.rejected(INTENT, "enhancement_optional_material_duplicated");
            }
            if self.building_content.gameplay.item(material_id).is_none() {
                return self.rejected(INTENT, "enhancement_optional_material_unknown");
            }
        }
        let key = format!(
            "{INTENT}:{hunter_id}:{gear_instance_id}:{mode}:{}",
            optional_material_ids.join(",")
        );
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.binding_blocked(INTENT, &GEAR_ENHANCEMENT_BLOCKERS)
                } else {
                    self.rejected(INTENT, "command_id_conflict")
                };
            }
        }
        let Some(hunter_index) = self
            .hunter_roster
            .hunters
            .iter()
            .position(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(INTENT, "hunter_unknown");
        };
        let hunter = &self.hunter_roster.hunters[hunter_index];
        if !hunter.hunt.is_idle() {
            return self.rejected(INTENT, "hunter_not_in_town");
        }
        let Some(task) = hunter.hunt.gear_enhancement.as_ref() else {
            return self.rejected(INTENT, "enhancement_visit_not_started");
        };
        if !matches!(
            task.status,
            GearEnhancementTaskStatus::WaitingForInteraction
                | GearEnhancementTaskStatus::Configuring
        ) {
            return self.rejected(INTENT, "hunter_not_ready_at_enhancement_forge");
        }
        let Some(owned) = hunter
            .owned_items
            .iter()
            .find(|owned| owned.gear_instance_id == Some(gear_instance_id) && owned.quantity > 0)
        else {
            return self.rejected(INTENT, "gear_instance_not_owned");
        };
        let Some(product) = self.building_content.gameplay.product(&owned.product_id) else {
            return self.rejected(INTENT, "gear_definition_unavailable");
        };
        if gear_product_route(&self.building_content.gameplay, product).is_none() {
            return self.rejected(INTENT, "product_is_not_gear");
        }
        if owned
            .enhancement_level
            .is_some_and(|level| level >= MAX_GEAR_ENHANCEMENT_LEVEL)
        {
            return self.rejected(INTENT, "gear_enhancement_cap_reached");
        }
        let product_id = owned.product_id.clone();
        let current_level = owned.enhancement_level;
        let hunter = &mut self.hunter_roster.hunters[hunter_index];
        let task = hunter
            .hunt
            .gear_enhancement
            .as_mut()
            .expect("enhancement task was validated above");
        task.status = GearEnhancementTaskStatus::Configuring;
        task.selected_gear_instance_id = Some(gear_instance_id);
        task.selected_product_id = Some(product_id);
        task.mode = Some(mode.to_owned());
        task.target_level = target_level;
        task.optional_material_ids = optional_material_ids.to_vec();
        task.attempts.clear();
        task.spent_gold = 0;
        task.spent_materials.clear();
        task.final_level = current_level;
        task.stop_reason = Some("evidence_disabled".to_owned());
        task.blockers = GEAR_ENHANCEMENT_BLOCKERS
            .iter()
            .map(|blocker| (*blocker).to_owned())
            .collect();
        hunter.profile.action_state = "configuring_enhancement".to_owned();
        hunter.profile.animation_name = "hunter_stay".to_owned();
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        // This is a terminal fail-closed result: no economy mutation occurred,
        // so the Hunter must be released instead of remaining pinned to the forge.
        release_hunter_from_enhancement(hunter);
        self.binding_blocked(INTENT, &GEAR_ENHANCEMENT_BLOCKERS)
    }
}
