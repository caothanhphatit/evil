use super::{
    monster_pools, spawn_point, MonsterActionState, MonsterFieldState, MonsterMapConfig,
    MonsterMaterialDefinition, MonsterState, RegionBounds,
};

impl MonsterFieldState {
    pub(super) fn spawned(
        config: &MonsterMapConfig,
        density_level: u8,
        world_difficulty: u8,
    ) -> Self {
        let spawn_count = config.density_counts[usize::from(density_level - 1)];
        let mut field = Self {
            map_id: config.map_id.to_owned(),
            density_level,
            spawn_count,
            monsters: Vec::new(),
            drops: Vec::new(),
        };
        field.reconcile_spawn_count(config, world_difficulty);
        field
    }

    pub(super) fn reconcile_spawn_count(
        &mut self,
        config: &MonsterMapConfig,
        world_difficulty: u8,
    ) {
        let target = usize::try_from(self.spawn_count).unwrap_or(0);
        if self.monsters.len() > target {
            self.monsters.truncate(target);
        }
        for (index, monster) in self.monsters.iter_mut().enumerate() {
            if !config.bounds.contains(monster.spawn_x, monster.spawn_y) {
                let (x, y) = spawn_point(config.bounds, index);
                monster.x = x;
                monster.y = y;
                monster.spawn_x = x;
                monster.spawn_y = y;
                monster.target_hunter_id = None;
                monster.action_state = MonsterActionState::Idle;
                monster.animation = "stay".to_owned();
                monster.patrol_idle_ticks = 0;
            }
        }
        while self.monsters.len() < target {
            let index = self.monsters.len();
            self.monsters
                .push(spawn_monster(config, world_difficulty, index));
        }
    }
}

impl RegionBounds {
    pub(in crate::simulation) fn contains(self, x: i32, y: i32) -> bool {
        (self.min_x..=self.max_x).contains(&x) && (self.min_y..=self.max_y).contains(&y)
    }

    pub(super) fn closest_point(self, x: i32, y: i32, inset: i32) -> (i32, i32) {
        (
            x.clamp(self.min_x + inset, self.max_x - inset),
            y.clamp(self.min_y + inset, self.max_y - inset),
        )
    }

    #[cfg(test)]
    pub(super) fn intersects(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

pub(super) fn spawn_monster(
    config: &MonsterMapConfig,
    world_difficulty: u8,
    index: usize,
) -> MonsterState {
    let definition = monster_pools()
        .iter()
        .find(|pool| pool.map_id == config.map_id && pool.global_difficulty == world_difficulty)
        .and_then(|pool| pool.monsters.get(index % pool.monsters.len().max(1)));
    let Some(monster) = definition else {
        panic!(
            "validated ordinary monster pool is missing for area {} difficulty {}",
            config.area, world_difficulty
        );
    };
    let materials = monster
        .materials
        .iter()
        .map(|material| MonsterMaterialDefinition {
            source_index: material.source_index,
            count: material.count,
            raw_percent: material.raw_percent,
        })
        .collect();
    let (source_index, hp, damage, armor, experience, gold) = (
        monster.source_index,
        monster.hp,
        monster.damage,
        monster.armor,
        monster.experience,
        monster.gold,
    );
    let (x, y) = spawn_point(config.bounds, index);
    MonsterState {
        entity_id: format!("monster-{}-{index}", config.map_id),
        monster_id: format!("monster:{source_index}"),
        source_index,
        asset_bundle_id: monster.asset_bundle_id.clone(),
        hp,
        max_hp: hp,
        damage,
        armor,
        experience,
        gold,
        x,
        y,
        spawn_x: x,
        spawn_y: y,
        patrol_phase: u16::try_from(index * 13).unwrap_or(0),
        patrol_idle_ticks: 0,
        action_state: MonsterActionState::Idle,
        animation: "stay".to_owned(),
        facing_left: index % 2 == 0,
        target_hunter_id: None,
        recovery_ticks: 0,
        respawn_ticks: None,
        attack_sequence: 0,
        stun_ticks: 0,
        slow_ticks: 0,
        materials,
    }
}
