use super::{
    basic_hunter_skill_definition, town_navigation_obstacles, DurableHunterRosterState,
    DurableProductStock, HunterRosterError, OriginalFlowSession, ServerMessage, Uuid,
};

impl OriginalFlowSession {
    pub(super) fn apply_hunter_command<F>(
        &mut self,
        command_id: Uuid,
        key: &str,
        intent: &str,
        apply: F,
    ) -> ServerMessage
    where
        F: FnOnce(&mut DurableHunterRosterState) -> Result<(), HunterRosterError>,
    {
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == key {
                    self.accepted(intent)
                } else {
                    self.rejected(
                        intent,
                        "command id was already used for a different hunter action",
                    )
                };
            }
        }
        match apply(&mut self.hunter_roster) {
            Ok(()) => {
                if command_id != Uuid::nil() {
                    self.hunter_roster
                        .hunt_commands
                        .insert(command_id, key.to_owned());
                }
                self.accepted(intent)
            }
            Err(error) => self.rejected(intent, &error.to_string()),
        }
    }

    pub(super) fn assign_hunter_hunt(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        zone_id: &str,
    ) -> ServerMessage {
        const INTENT: &str = "assign_hunter_hunt";
        let key = format!("{INTENT}:{hunter_id}:{zone_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted(INTENT)
                } else {
                    self.rejected(
                        INTENT,
                        "command id was already used for a different hunter action",
                    )
                };
            }
        }

        if let Err(error) = self.hunter_roster.assign_hunt(hunter_id, zone_id) {
            return self.rejected(INTENT, &error.to_string());
        }

        // Movement is the explicit highest-priority player task. A paid service
        // has not credited the town until completion, so canceling it restores
        // both the Hunter payment and the consumed product stock atomically.
        let cancelled_visits = self
            .product_services
            .visits
            .iter()
            .filter(|visit| visit.hunter_id == hunter_id)
            .cloned()
            .collect::<Vec<_>>();
        self.product_services
            .visits
            .retain(|visit| visit.hunter_id != hunter_id);
        if !cancelled_visits.is_empty() {
            let hunter = self
                .hunter_roster
                .hunters
                .iter_mut()
                .find(|hunter| hunter.hunter_id == hunter_id)
                .expect("hunt assignment validated the active Hunter");
            for visit in cancelled_visits {
                hunter.gold = hunter.gold.saturating_add(visit.payment_gold);
                if let Some(stock) = self.buildings.product_stocks.iter_mut().find(|stock| {
                    stock.building_instance_id == visit.building_instance_id
                        && stock.product_id == visit.product_id
                }) {
                    stock.quantity = stock.quantity.saturating_add(1);
                } else {
                    self.buildings.product_stocks.push(DurableProductStock {
                        building_instance_id: visit.building_instance_id,
                        product_id: visit.product_id,
                        quantity: 1,
                    });
                }
            }
        }

        let navigation_obstacles =
            town_navigation_obstacles(&self.buildings.buildings, &self.building_content.catalog);
        self.monster_world
            .prioritize_hunt_assignment(hunter_id, zone_id, &navigation_obstacles);
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted(INTENT)
    }

    pub(super) fn learn_hunter_skill(
        &mut self,
        command_id: Uuid,
        hunter_id: u32,
        skill_id: &str,
    ) -> ServerMessage {
        let key = format!("learn_hunter_skill:{hunter_id}:{skill_id}");
        if command_id != Uuid::nil() {
            if let Some(previous) = self.hunter_roster.hunt_commands.get(&command_id) {
                return if previous == &key {
                    self.accepted("learn_hunter_skill")
                } else {
                    self.rejected(
                        "learn_hunter_skill",
                        "command id was already used for a different hunter action",
                    )
                };
            }
        }
        let Some(hunter) = self
            .hunter_roster
            .hunters
            .iter_mut()
            .find(|hunter| hunter.hunter_id == hunter_id)
        else {
            return self.rejected(
                "learn_hunter_skill",
                "hunter is not in the active town roster",
            );
        };
        let Some(definition) = basic_hunter_skill_definition(skill_id) else {
            return self.rejected("learn_hunter_skill", "skill definition is unavailable");
        };
        if hunter.profile.class_id != definition.class_id
            || hunter.profile.visual_family != definition.class_family
        {
            return self.rejected("learn_hunter_skill", "skill is unavailable for hunter job");
        }
        if hunter
            .profile
            .skills
            .iter()
            .any(|skill| skill.skill_id == skill_id)
        {
            return self.rejected("learn_hunter_skill", "skill is already learned");
        }
        // Job ownership and cooldown come from the packaged basic-skill catalog.
        // Only the two H1 icon bindings are independently confirmed.
        hunter
            .profile
            .skills
            .push(super::super::hunter_roster::DurableHunterSkill {
                skill_id: definition.skill_id.to_owned(),
                display_name: definition.display_name.to_owned(),
                icon_path: definition.confirmed_icon_path.map(str::to_owned),
                animation_name: None,
                skill_level: 1,
                equipped_slot: None,
                ready: true,
                cooldown_remaining_ms: 0,
            });
        if command_id != Uuid::nil() {
            self.hunter_roster.hunt_commands.insert(command_id, key);
        }
        self.accepted("learn_hunter_skill")
    }
}
