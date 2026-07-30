use super::{
    face_toward_x, hunter_attack_range, hunter_attack_recovery_ticks, hunter_move_step, map_config,
    move_toward_avoiding, original_level_scaled_attack, set_hunter_presentation, squared_distance,
    town_roam_anchor_index, town_roam_idle_ticks, DurableHunterRosterState,
    GearEnhancementTaskStatus, HashMap, HunterActionState, HunterAttackSource, MonsterWorldState,
    NavigationObstacle, PendingOperation, HUNTER_MOVE_MAX_PX_PER_TICK, TOWN_ARRIVAL_INSIDE,
    TOWN_ARRIVAL_OUTSIDE, TOWN_RESPAWN_POINT, TOWN_ROAM_ANCHORS, TOWN_ROAM_BOUNDS,
};

impl MonsterWorldState {
    pub(super) fn tick_hunters(
        &mut self,
        roster: &mut DurableHunterRosterState,
        obstacles: &[NavigationObstacle],
        revival_point: Option<(i32, i32)>,
        town_destinations: &HashMap<u32, (i32, i32)>,
    ) -> Vec<PendingOperation> {
        let mut operations = Vec::new();
        for agent_index in 0..self.hunters.len() {
            let hunter_id = self.hunters[agent_index].hunter_id;
            self.hunters[agent_index].skill_buff_ticks =
                self.hunters[agent_index].skill_buff_ticks.saturating_sub(1);
            if self.hunters[agent_index].skill_buff_ticks == 0 {
                self.hunters[agent_index].skill_attack_percent = 0;
                self.hunters[agent_index].skill_defense_percent = 0;
                self.hunters[agent_index].skill_evasion_percent = 0;
                self.hunters[agent_index].skill_critical_percent = 0;
                self.hunters[agent_index].skill_attack_speed_milli = 0;
                self.hunters[agent_index].ice_armor_active = false;
            }
            let move_step = hunter_move_step(self.tick);
            if self.tick_dead_hunter(agent_index, roster, revival_point) {
                continue;
            }
            let Some(region_id) = self.hunters[agent_index].region_id.clone() else {
                if self.hunters[agent_index].entry_stage >= 3 {
                    let target = if self.hunters[agent_index].entry_stage == 3 {
                        TOWN_ARRIVAL_OUTSIDE
                    } else {
                        TOWN_ARRIVAL_INSIDE
                    };
                    let agent = &mut self.hunters[agent_index];
                    if squared_distance(agent.x, agent.y, target.0, target.1)
                        <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                    {
                        agent.x = target.0;
                        agent.y = target.1;
                        agent.entry_stage = if agent.entry_stage == 3 { 4 } else { 0 };
                    } else {
                        set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_walk");
                        move_toward_avoiding(
                            &mut agent.x,
                            &mut agent.y,
                            target.0,
                            target.1,
                            move_step,
                            &mut agent.facing_left,
                            obstacles,
                        );
                    }
                    continue;
                }
                let enhancement_destination = roster
                    .hunters
                    .iter()
                    .find(|hunter| hunter.hunter_id == hunter_id)
                    .and_then(|hunter| hunter.hunt.gear_enhancement.as_ref())
                    .map(|task| (task.status, task.interaction_x, task.interaction_y));
                if let Some((status, target_x, target_y)) = enhancement_destination {
                    let agent = &mut self.hunters[agent_index];
                    if status == GearEnhancementTaskStatus::Traveling
                        && squared_distance(agent.x, agent.y, target_x, target_y)
                            > i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                    {
                        set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_walk");
                        move_toward_avoiding(
                            &mut agent.x,
                            &mut agent.y,
                            target_x,
                            target_y,
                            move_step,
                            &mut agent.facing_left,
                            obstacles,
                        );
                        continue;
                    }
                    set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_stay");
                    if status == GearEnhancementTaskStatus::Traveling {
                        if let Ok(hunter) = roster.active_mut(hunter_id) {
                            if let Some(task) = hunter.hunt.gear_enhancement.as_mut() {
                                task.status = GearEnhancementTaskStatus::WaitingForInteraction;
                            }
                            hunter.profile.action_state =
                                "waiting_for_enhancement_interaction".to_owned();
                            hunter.profile.animation_name = "hunter_stay".to_owned();
                        }
                    }
                    continue;
                }
                if let Some(&(target_x, target_y)) = town_destinations.get(&hunter_id) {
                    let agent = &mut self.hunters[agent_index];
                    if squared_distance(agent.x, agent.y, target_x, target_y)
                        <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                    {
                        agent.x = target_x;
                        agent.y = target_y;
                        set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_stay");
                    } else {
                        set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_walk");
                        move_toward_avoiding(
                            &mut agent.x,
                            &mut agent.y,
                            target_x,
                            target_y,
                            move_step,
                            &mut agent.facing_left,
                            obstacles,
                        );
                    }
                    continue;
                }
                let agent = &mut self.hunters[agent_index];
                if agent.town_roam_idle_ticks > 0 {
                    agent.town_roam_idle_ticks = agent.town_roam_idle_ticks.saturating_sub(1);
                    set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_stay");
                    continue;
                }
                let anchor_index =
                    town_roam_anchor_index(agent.hunter_id, agent.town_roam_sequence);
                let (target_x, target_y) = TOWN_ROAM_ANCHORS[anchor_index];
                if squared_distance(agent.x, agent.y, target_x, target_y)
                    <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                {
                    agent.x = target_x;
                    agent.y = target_y;
                    agent.town_roam_sequence = agent.town_roam_sequence.wrapping_add(1);
                    agent.town_roam_idle_ticks =
                        town_roam_idle_ticks(agent.hunter_id, agent.town_roam_sequence);
                    set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_stay");
                    continue;
                }
                set_hunter_presentation(agent, HunterActionState::TownIdle, "hunter_walk");
                move_toward_avoiding(
                    &mut agent.x,
                    &mut agent.y,
                    target_x,
                    target_y,
                    move_step,
                    &mut agent.facing_left,
                    obstacles,
                );
                agent.x = agent
                    .x
                    .clamp(TOWN_ROAM_BOUNDS.min_x, TOWN_ROAM_BOUNDS.max_x);
                agent.y = agent
                    .y
                    .clamp(TOWN_ROAM_BOUNDS.min_y, TOWN_ROAM_BOUNDS.max_y);
                continue;
            };
            let Some(config) = map_config(&region_id) else {
                continue;
            };
            if !config
                .bounds
                .contains(self.hunters[agent_index].x, self.hunters[agent_index].y)
            {
                let agent = &mut self.hunters[agent_index];
                let (target_x, target_y) = if let Some(waypoint) =
                    config.entry_waypoints.get(usize::from(agent.entry_stage))
                {
                    *waypoint
                } else {
                    let final_approach = config.entry_waypoints[config.entry_waypoints.len() - 1];
                    config
                        .bounds
                        .closest_point(final_approach.0, final_approach.1, 48)
                };
                set_hunter_presentation(agent, HunterActionState::EnteringRegion, "hunter_walk");
                move_toward_avoiding(
                    &mut agent.x,
                    &mut agent.y,
                    target_x,
                    target_y,
                    move_step,
                    &mut agent.facing_left,
                    obstacles,
                );
                if squared_distance(agent.x, agent.y, target_x, target_y)
                    <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                {
                    agent.entry_stage = agent.entry_stage.saturating_add(1);
                }
                continue;
            }
            if let Ok(hunter) = roster.active_mut(hunter_id) {
                if hunter.profile.action_state == "entering_region" {
                    hunter.profile.action_state = "hunting".to_owned();
                    hunter.profile.animation_name = "hunter_stay".to_owned();
                }
            }
            self.hunters[agent_index].recovery_ticks =
                self.hunters[agent_index].recovery_ticks.saturating_sub(1);
            if self.hunters[agent_index].active_skill_id.is_some() {
                if self.hunters[agent_index].recovery_ticks > 0 {
                    continue;
                }
                self.hunters[agent_index].active_skill_id = None;
            }
            // Finish a pickup already in progress before responding to a new
            // aggro target, otherwise combat can reset the same pickup forever.
            if self.hunters[agent_index].target_drop_id.is_some()
                && self.try_collect_drop(agent_index, roster, &mut operations)
            {
                continue;
            }
            let current_target_id = self.valid_monster_target(agent_index, &region_id);
            // Keep an already engaged survivor ahead of loot, but give a
            // defeated target's drops one pickup pass before acquiring a new
            // unrelated monster. Otherwise continuous respawns can starve
            // even a single kill's gold/material drops indefinitely.
            let target_id = current_target_id
                .or_else(|| self.nearest_engaged_monster_id(agent_index, &region_id));
            if target_id.is_none() && self.try_collect_drop(agent_index, roster, &mut operations) {
                self.hunters[agent_index].target_monster_id = None;
                continue;
            }
            let target_id = target_id.or_else(|| self.nearest_monster_id(agent_index, &region_id));
            self.hunters[agent_index].target_monster_id = target_id.clone();
            let Some(target_id) = target_id else {
                set_hunter_presentation(
                    &mut self.hunters[agent_index],
                    HunterActionState::AcquiringTarget,
                    "hunter_stay",
                );
                continue;
            };
            let Some((target_x, target_y)) = self.monster_position(&target_id) else {
                continue;
            };
            let class_family = roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == hunter_id)
                .map(|hunter| hunter.profile.visual_family.as_str())
                .unwrap_or("H1");
            let attack_range = hunter_attack_range(class_family);
            let distance = squared_distance(
                self.hunters[agent_index].x,
                self.hunters[agent_index].y,
                target_x,
                target_y,
            );
            if distance > i64::from(attack_range).pow(2) {
                let agent = &mut self.hunters[agent_index];
                set_hunter_presentation(agent, HunterActionState::Chasing, "hunter_walk");
                move_toward_avoiding(
                    &mut agent.x,
                    &mut agent.y,
                    target_x,
                    target_y,
                    move_step,
                    &mut agent.facing_left,
                    obstacles,
                );
                continue;
            }
            let hunter_x = self.hunters[agent_index].x;
            face_toward_x(
                &mut self.hunters[agent_index].facing_left,
                hunter_x,
                target_x,
            );
            let attack_animation = format!("{}_hit", class_family.to_ascii_lowercase());
            set_hunter_presentation(
                &mut self.hunters[agent_index],
                HunterActionState::Attacking,
                &attack_animation,
            );
            if self.hunters[agent_index].recovery_ticks > 0 {
                continue;
            }
            self.hunters[agent_index].recovery_ticks = hunter_attack_recovery_ticks(
                roster
                    .hunters
                    .iter()
                    .find(|hunter| hunter.hunter_id == hunter_id)
                    .and_then(|hunter| hunter.profile.attack_speed_milli),
                self.hunters[agent_index].skill_attack_speed_milli,
            );
            self.hunters[agent_index].attack_sequence =
                self.hunters[agent_index].attack_sequence.wrapping_add(1);
            let Some(hunter) = roster
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == hunter_id)
            else {
                continue;
            };
            let Some(mut calculated_damage) =
                original_level_scaled_attack(hunter.profile.attack, hunter.profile.level)
            else {
                continue;
            };
            calculated_damage = calculated_damage.saturating_mul(i64::from(
                100 + self.hunters[agent_index].skill_attack_percent,
            )) / 100;
            let calculated_critical_percent = hunter
                .runtime
                .status
                .as_ref()
                .map(|status| status.critical)
                .or_else(|| {
                    hunter
                        .profile
                        .critical_rate_bps
                        .and_then(|value| i32::try_from(value / 100).ok())
                })
                .unwrap_or(0)
                .saturating_add(self.hunters[agent_index].skill_critical_percent);
            let hunter_feel = hunter
                .runtime
                .status
                .as_ref()
                .map(|status| status.feel)
                .unwrap_or(hunter.mood.maximum as f32);
            let hunter_now_feel = hunter
                .runtime
                .status
                .as_ref()
                .map(|status| status.now_feel)
                .unwrap_or(hunter.mood.current as f32);
            let attack_sequence = self.hunters[agent_index].attack_sequence;
            self.resolve_hunter_attack(
                &target_id,
                HunterAttackSource {
                    hunter_id,
                    calculated_damage,
                    calculated_critical_percent,
                    hunter_feel,
                    hunter_now_feel,
                    attack_sequence,
                },
            );
        }
        operations
    }

    pub(super) fn tick_dead_hunter(
        &mut self,
        agent_index: usize,
        roster: &mut DurableHunterRosterState,
        revival_point: Option<(i32, i32)>,
    ) -> bool {
        let Some(respawn) = self.hunters[agent_index].respawn_ticks.as_mut() else {
            return false;
        };
        *respawn = respawn.saturating_sub(1);
        if *respawn > 0 {
            return true;
        }
        let hunter_id = self.hunters[agent_index].hunter_id;
        if let Ok(hunter) = roster.active_mut(hunter_id) {
            hunter.current_hp = hunter.max_hp;
            hunter.hunt.status = if hunter.hunt.zone_id.is_some() {
                "hunting".to_owned()
            } else {
                "idle".to_owned()
            };
            hunter.profile.action_state = hunter.hunt.status.clone();
            hunter.profile.animation_name = "hunter_walk".to_owned();
        }
        let agent = &mut self.hunters[agent_index];
        let (revival_x, revival_y) = revival_point.unwrap_or(TOWN_RESPAWN_POINT);
        agent.x = revival_x;
        agent.y = revival_y;
        agent.action_state = if agent.region_id.is_some() {
            HunterActionState::EnteringRegion
        } else {
            HunterActionState::TownIdle
        };
        agent.animation = if agent.region_id.is_some() {
            "hunter_walk".to_owned()
        } else {
            "hunter_stay".to_owned()
        };
        agent.respawn_ticks = None;
        agent.entry_stage = 0;
        true
    }
}
