use super::{
    entry_checkpoint_stage, map_config, map_configs, nearest_clear_town_anchor,
    set_hunter_presentation, HunterActionState, MonsterFieldState, MonsterWorldState,
    NavigationObstacle, OriginalDamageMultiplierStream,
};

impl MonsterWorldState {
    pub fn with_densities<'a>(densities: impl IntoIterator<Item = (&'a str, u8)>) -> Self {
        let configured = densities.into_iter().collect::<Vec<_>>();
        let world_difficulty = 0;
        let fields = map_configs()
            .iter()
            .map(|config| {
                let density = configured
                    .iter()
                    .find_map(|(map_id, level)| (*map_id == config.map_id).then_some(*level))
                    .filter(|level| (1..=3).contains(level))
                    .unwrap_or(1);
                MonsterFieldState::spawned(config, density, world_difficulty)
            })
            .collect();
        Self {
            current_map_id: map_configs()[0].map_id.to_owned(),
            world_difficulty,
            tick: 0,
            fields,
            hunters: Vec::new(),
            reward_sequence: 0,
            presentation_sequence: 0,
            damage_multiplier_stream: OriginalDamageMultiplierStream::default(),
            combat_presentations: Vec::new(),
        }
    }

    pub fn enter_map(&mut self, map_id: &str) -> Result<(), &'static str> {
        map_config(map_id).ok_or("monster map unavailable")?;
        self.current_map_id = map_id.to_owned();
        Ok(())
    }

    pub fn current_field(&self) -> &MonsterFieldState {
        self.fields
            .iter()
            .find(|field| field.map_id == self.current_map_id)
            .unwrap_or(&self.fields[0])
    }

    pub fn current_field_mut(&mut self) -> &mut MonsterFieldState {
        let map_id = self.current_map_id.clone();
        let index = self
            .fields
            .iter()
            .position(|field| field.map_id == map_id)
            .unwrap_or(0);
        &mut self.fields[index]
    }

    pub fn set_density(&mut self, level: u8) -> Result<(), &'static str> {
        let region_id = self.current_map_id.clone();
        self.set_region_density(&region_id, level)
    }

    pub fn set_region_density(&mut self, region_id: &str, level: u8) -> Result<(), &'static str> {
        if !(1..=3).contains(&level) {
            return Err("monster density unavailable");
        }
        let world_difficulty = self.world_difficulty;
        let field = self
            .fields
            .iter_mut()
            .find(|field| field.map_id == region_id)
            .ok_or("monster region unavailable")?;
        let config = map_config(region_id).ok_or("monster map unavailable")?;
        field.density_level = level;
        field.spawn_count = config.density_counts[usize::from(level - 1)];
        field.reconcile_spawn_count(config, world_difficulty);
        Ok(())
    }

    pub fn select_target(&mut self, monster_id: &str, hunter_id: u32) -> Result<(), &'static str> {
        let monster = self
            .fields
            .iter_mut()
            .flat_map(|field| field.monsters.iter_mut())
            .find(|monster| monster.entity_id == monster_id)
            .ok_or("monster unavailable")?;
        if monster.hp == 0 {
            return Err("monster is dead");
        }
        monster.target_hunter_id = Some(hunter_id);
        Ok(())
    }

    /// Applies a player-issued region assignment at the command boundary.
    /// The following simulation tick owns movement, but the accepted snapshot
    /// must already expose the preempted FSM instead of the previous town task.
    pub fn prioritize_hunt_assignment(
        &mut self,
        hunter_id: u32,
        region_id: &str,
        obstacles: &[NavigationObstacle],
    ) {
        let Some(config) = map_config(region_id) else {
            return;
        };
        let Some(agent) = self
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == hunter_id)
        else {
            return;
        };
        agent.region_id = Some(region_id.to_owned());
        agent.target_monster_id = None;
        agent.target_drop_id = None;
        agent.loot_item_id = None;
        agent.loot_quantity = 0;
        agent.recovery_ticks = 0;
        agent.active_skill_id = None;
        let checkpoint_stage = entry_checkpoint_stage(config, agent.x, agent.y);
        let position_blocked = obstacles
            .iter()
            .any(|obstacle| obstacle.expanded(14).contains(agent.x, agent.y));
        if checkpoint_stage.is_none() || position_blocked {
            if let Some((x, y)) = nearest_clear_town_anchor(agent.x, agent.y, obstacles) {
                agent.x = x;
                agent.y = y;
            } else {
                agent.x = config.entry_waypoints[0].0;
                agent.y = config.entry_waypoints[0].1;
            }
            agent.entry_stage = 0;
        } else {
            agent.entry_stage = checkpoint_stage.unwrap_or(0);
        }
        set_hunter_presentation(agent, HunterActionState::EnteringRegion, "hunter_walk");
    }
}
