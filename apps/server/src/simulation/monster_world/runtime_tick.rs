use super::{
    initial_town_roam_idle_ticks, map_config, nearest_clear_town_anchor, DurableHunterRosterState,
    GearEnhancementTaskStatus, HashMap, HunterActionState, HunterAgentState, MonsterWorldState,
    NavigationObstacle, PendingOperation, HUNTER_RESPAWN_TICKS, TOWN_ARRIVAL_OUTSIDE,
    TOWN_RESPAWN_POINT, TOWN_ROAM_ANCHORS, TOWN_ROAM_BOUNDS,
};

impl MonsterWorldState {
    pub fn tick(&mut self, roster: &mut DurableHunterRosterState) -> Vec<PendingOperation> {
        self.tick_with_obstacles(roster, &[], None, &HashMap::new())
    }

    pub fn tick_with_obstacles(
        &mut self,
        roster: &mut DurableHunterRosterState,
        obstacles: &[NavigationObstacle],
        revival_point: Option<(i32, i32)>,
        town_destinations: &HashMap<u32, (i32, i32)>,
    ) -> Vec<PendingOperation> {
        self.tick = self.tick.saturating_add(1);
        self.combat_presentations.clear();
        self.reconcile_hunters(roster, obstacles);
        self.tick_monsters(roster);
        self.tick_hunters(roster, obstacles, revival_point, town_destinations)
    }

    pub(super) fn reconcile_hunters(
        &mut self,
        roster: &DurableHunterRosterState,
        obstacles: &[NavigationObstacle],
    ) {
        self.hunters.retain(|agent| {
            roster
                .hunters
                .iter()
                .any(|hunter| hunter.hunter_id == agent.hunter_id)
        });
        let initializing_world = self.hunters.is_empty();
        for (slot, hunter) in roster.hunters.iter().enumerate() {
            if self
                .hunters
                .iter()
                .any(|agent| agent.hunter_id == hunter.hunter_id)
            {
                continue;
            }
            let region_id = hunter
                .hunt
                .zone_id
                .clone()
                .filter(|id| map_config(id).is_some());
            let arriving_in_town =
                !initializing_world && region_id.is_none() && hunter.current_hp > 0;
            let spawn = if region_id.is_some() {
                TOWN_RESPAWN_POINT
            } else if arriving_in_town {
                TOWN_ARRIVAL_OUTSIDE
            } else {
                TOWN_ROAM_ANCHORS[slot % TOWN_ROAM_ANCHORS.len()]
            };
            self.hunters.push(HunterAgentState {
                hunter_id: hunter.hunter_id,
                region_id,
                x: spawn.0,
                y: spawn.1,
                facing_left: false,
                action_state: if hunter.current_hp == 0 {
                    HunterActionState::Dead
                } else {
                    HunterActionState::TownIdle
                },
                animation: if hunter.current_hp == 0 {
                    "hunter_die".to_owned()
                } else {
                    "hunter_stay".to_owned()
                },
                target_monster_id: None,
                target_drop_id: None,
                recovery_ticks: 0,
                respawn_ticks: (hunter.current_hp == 0).then_some(HUNTER_RESPAWN_TICKS),
                attack_sequence: 0,
                loot_sequence: 0,
                loot_item_id: None,
                loot_quantity: 0,
                active_skill_id: None,
                skill_buff_ticks: 0,
                skill_attack_percent: 0,
                skill_defense_percent: 0,
                skill_evasion_percent: 0,
                skill_critical_percent: 0,
                skill_attack_speed_milli: 0,
                ice_armor_active: false,
                entry_stage: if arriving_in_town { 3 } else { 0 },
                town_roam_sequence: 0,
                town_roam_idle_ticks: initial_town_roam_idle_ticks(hunter.hunter_id),
                trade_sequence: 0,
                trade_gold: 0,
                trade_materials: Vec::new(),
            });
        }
        for agent in &mut self.hunters {
            let mut has_town_destination = false;
            if let Some(hunter) = roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == agent.hunter_id)
            {
                has_town_destination = hunter
                    .hunt
                    .gear_enhancement
                    .as_ref()
                    .is_some_and(|task| task.status == GearEnhancementTaskStatus::Traveling)
                    || hunter.hunt.pending_trade.is_some();
                let assigned = hunter
                    .hunt
                    .zone_id
                    .clone()
                    .filter(|id| map_config(id).is_some());
                if agent.region_id != assigned {
                    let returning_to_town = agent.region_id.is_some() && assigned.is_none();
                    agent.region_id = assigned;
                    agent.target_monster_id = None;
                    agent.target_drop_id = None;
                    agent.action_state = if agent.region_id.is_some() {
                        HunterActionState::EnteringRegion
                    } else {
                        HunterActionState::TownIdle
                    };
                    // Preserve the field position and walk back through the
                    // town-arrival corridor. Resetting to stage zero makes the
                    // out-of-town sanitizer teleport a returning Hunter home.
                    agent.entry_stage = if returning_to_town { 3 } else { 0 };
                }
            }
            if agent.region_id.is_none()
                && agent.entry_stage == 0
                && !has_town_destination
                && (!TOWN_ROAM_BOUNDS.contains(agent.x, agent.y)
                    || obstacles
                        .iter()
                        .any(|obstacle| obstacle.expanded(14).contains(agent.x, agent.y)))
            {
                if let Some((x, y)) = nearest_clear_town_anchor(agent.x, agent.y, obstacles) {
                    agent.x = x;
                    agent.y = y;
                }
            }
        }
    }
}
