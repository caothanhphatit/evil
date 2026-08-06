use super::{
    village_hunter_entity_id, BottomMenuIntent, FixtureCommand, HunterRosterError,
    OriginalFlowSession, OriginalScreen, ServerMessage, Uuid,
};

impl OriginalFlowSession {
    pub(super) fn equip_owned_rebuild_weapon(
        &mut self,
        hunter_index: usize,
        gear_instance_id: Uuid,
    ) -> Result<(), &'static str> {
        let Some(item) = self.hunter_roster.hunters[hunter_index]
            .owned_items
            .iter()
            .find(|item| item.gear_instance_id == Some(gear_instance_id))
        else {
            return Err("gear_instance_unknown");
        };
        let Some(definition) =
            super::super::web_rebuild_gear::rebuild_weapon_definition(&item.product_id)
        else {
            return Err("weapon_definition_unresolved");
        };
        if definition.visual_family
            != self.hunter_roster.hunters[hunter_index]
                .profile
                .visual_family
        {
            return Err("weapon_class_mismatch");
        }
        let required_class_id = self.hunter_roster.hunters[hunter_index]
            .profile
            .class_id
            .clone();
        let Some(slot) = self.hunter_roster.hunters[hunter_index]
            .profile
            .equipment_slots
            .iter_mut()
            .find(|slot| slot.slot_id == "weapon")
        else {
            return Err("weapon_slot_unavailable");
        };
        slot.catalog_kind = format!("rebuild_weapon_instance:{gear_instance_id}");
        slot.catalog_index = definition.gear_index;
        slot.display_name = definition.display_name_vi;
        slot.icon_path = definition.icon_path;
        slot.required_class_id = Some(required_class_id);
        slot.evidence_state = "web_rebuild_weapon_v1".to_owned();
        Ok(())
    }

    pub(super) fn equip_rebuild_weapon(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        gear_instance_id: Uuid,
    ) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("equip_hunter_weapon", "world_unavailable");
        }
        let key = format!("equip_hunter_weapon:{hunter_id}:{gear_instance_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted("equip_hunter_weapon")
                } else {
                    self.rejected("equip_hunter_weapon", "command_id_conflict")
                };
            }
        }
        let Some(hunter_index) = self
            .hunter_roster
            .hunters
            .iter()
            .position(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected("equip_hunter_weapon", "hunter_unknown");
        };
        if let Err(reason) = self.equip_owned_rebuild_weapon(hunter_index, gear_instance_id) {
            return self.rejected("equip_hunter_weapon", reason);
        }
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted("equip_hunter_weapon")
    }

    pub(super) fn equip_fixture_item(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        item_id: u32,
    ) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field || hunter_id != 1 {
            return self.rejected("equip_hunter_item", "fixture_hunter_unavailable");
        }
        let outcome = self
            .simulation
            .handle_command(FixtureCommand::EquipItem {
                command_id,
                item_id,
            })
            .expect("fixture equip always returns a command outcome");
        self.combat_snapshot = self.simulation.snapshot();
        ServerMessage::IntentResult {
            intent: "equip_hunter_item".to_owned(),
            accepted: outcome.accepted,
            reason: outcome.reason,
            snapshot: self.snapshot(),
        }
    }

    pub(super) fn banish_hunter(&mut self, command_id: Uuid, hunter_id: u32) -> ServerMessage {
        if !matches!(
            self.state.screen,
            OriginalScreen::Village | OriginalScreen::HunterRoster
        ) {
            return self.rejected("banish_hunter", "hunter_roster_unavailable");
        }
        if !self.hunter_roster.roster_resolved {
            return self.rejected("banish_hunter", "hunter_roster_unresolved");
        }
        if self
            .product_services
            .visits
            .iter()
            .any(|visit| visit.hunter_id == hunter_id)
            || self
                .hunter_roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == hunter_id)
                .is_some_and(|hunter| {
                    hunter.hunt.pending_trade.is_some() || hunter.hunt.gear_enhancement.is_some()
                })
        {
            return self.rejected("banish_hunter", "hunter_busy");
        }
        match self
            .hunter_roster
            .banish_active_idempotent(command_id, hunter_id)
        {
            Ok(_) => {
                let banished_entity_id = village_hunter_entity_id(hunter_id);
                if self.selected_entity_id.as_deref() == Some(banished_entity_id.as_str()) {
                    self.selected_entity_id = None;
                }
                self.accepted("banish_hunter")
            }
            Err(HunterRosterError::ActiveHunterUnknown) => {
                self.rejected("banish_hunter", "active_hunter_unknown")
            }
            Err(HunterRosterError::CommandConflict) => {
                self.rejected("banish_hunter", "banish_command_conflict")
            }
            Err(HunterRosterError::DuplicateHunter | HunterRosterError::InvalidState(_)) => {
                self.rejected("banish_hunter", "hunter_roster_invalid")
            }
        }
    }

    pub(super) fn select_bottom_menu(&mut self, menu: BottomMenuIntent) -> ServerMessage {
        if !self.shared_world_active() {
            return self.rejected("select_bottom_menu", "bottom_menu_unavailable");
        }
        match menu {
            // Hunter List is a client-presentational overlay in the shared
            // world. Keep the authoritative navigation focus unchanged.
            BottomMenuIntent::Character => self.accepted("select_bottom_menu.character"),
            BottomMenuIntent::Build => self.accepted("select_bottom_menu.build"),
            BottomMenuIntent::Archive => {
                self.binding_blocked("select_bottom_menu.archive", &["archive_rules_binding"])
            }
            BottomMenuIntent::Store => {
                self.binding_blocked("select_bottom_menu.store", &["store_catalog_binding"])
            }
            BottomMenuIntent::Raid => {
                self.binding_blocked("select_bottom_menu.raid", &["raid_rules_binding"])
            }
        }
    }

    pub(super) fn navigate_back(&mut self) -> ServerMessage {
        match self.state.screen {
            OriginalScreen::HunterRoster | OriginalScreen::Field => {
                if self.state.screen == OriginalScreen::Field {
                    self.settle_returning_hunters();
                }
                self.state.screen = OriginalScreen::Village;
                self.selected_entity_id = None;
                self.accepted("navigate_back")
            }
            _ => self.rejected("navigate_back", "navigation_unavailable"),
        }
    }

    pub(super) fn enter_field(&mut self) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("enter_field", "field_entry_unavailable");
        }
        self.state.screen = OriginalScreen::Field;
        self.buildings.field_trip_id = self.buildings.field_trip_id.saturating_add(1);
        self.selected_entity_id = None;
        self.accepted("enter_field")
    }

    pub(super) fn enter_monster_map(&mut self, map_id: &str) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field {
            return self.rejected("enter_monster_map", "field_required");
        }
        match self.monster_world.enter_map(map_id) {
            Ok(()) => {
                self.selected_entity_id = None;
                self.accepted("enter_monster_map")
            }
            Err(reason) => self.rejected("enter_monster_map", reason),
        }
    }

    pub(super) fn set_monster_density(&mut self, level: u8) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field {
            return self.rejected("set_monster_density", "field_required");
        }
        match self.monster_world.set_density(level) {
            Ok(()) => self.accepted("set_monster_density"),
            Err(reason) => self.rejected("set_monster_density", reason),
        }
    }

    pub(super) fn set_monster_region_density(
        &mut self,
        region_id: &str,
        level: u8,
    ) -> ServerMessage {
        if !matches!(
            self.state.screen,
            OriginalScreen::Village | OriginalScreen::Field
        ) {
            return self.rejected("set_monster_region_density", "world_required");
        }
        match self.monster_world.set_region_density(region_id, level) {
            Ok(()) => self.accepted("set_monster_region_density"),
            Err(reason) => self.rejected("set_monster_region_density", reason),
        }
    }

    pub(super) fn select_monster_target(
        &mut self,
        monster_id: &str,
        hunter_id: u32,
    ) -> ServerMessage {
        if self.state.screen != OriginalScreen::Field {
            return self.rejected("select_monster_target", "field_required");
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected("select_monster_target", "hunter_unavailable");
        };
        if hunter.current_hp == 0 || hunter.profile.action_state == "dead" {
            return self.rejected("select_monster_target", "hunter_unavailable");
        }
        match self.monster_world.select_target(monster_id, hunter_id) {
            Ok(()) => self.accepted("select_monster_target"),
            Err(reason) => self.rejected("select_monster_target", reason),
        }
    }

    pub(super) fn select_entity(&mut self, entity_id: &str) -> ServerMessage {
        let selected = self
            .world_entities()
            .into_iter()
            .find(|entity| entity.descriptor.entity_id == entity_id && entity.selectable)
            .map(|entity| entity.descriptor.entity_id);
        let Some(selected) = selected else {
            return self.rejected("select_entity", "entity_unavailable");
        };
        self.selected_entity_id = Some(selected);
        self.accepted("select_entity")
    }
}
