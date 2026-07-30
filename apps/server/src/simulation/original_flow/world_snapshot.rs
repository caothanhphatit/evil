use super::{
    map_config, monster_action_name, MonsterDropSnapshot, MonsterMapSnapshot, MonsterSnapshot,
    MonsterWorldSnapshot, MonsterWorldState, MONSTER_RULESET,
};

pub(super) fn monster_world_snapshot(world: &MonsterWorldState) -> MonsterWorldSnapshot {
    let field = world.current_field();
    let config = map_config(&field.map_id).expect("fixture monster map must have a config");
    let cluster_active = field.density_level == 3;
    MonsterWorldSnapshot {
        ruleset: MONSTER_RULESET,
        tick: world.tick,
        map_id: field.map_id.clone(),
        monster_tier: config.monster_tier,
        map_asset_id: config.map_asset_id.to_owned(),
        world_difficulty: world.world_difficulty,
        maps: world
            .fields
            .iter()
            .map(|field| {
                let config =
                    map_config(&field.map_id).expect("fixture monster map must have a config");
                MonsterMapSnapshot {
                    map_id: field.map_id.clone(),
                    monster_tier: config.monster_tier,
                    map_asset_id: config.map_asset_id.to_owned(),
                    density_level: field.density_level,
                }
            })
            .collect(),
        density_level: field.density_level,
        spawn_count: field.spawn_count,
        spawn_min: config.density_counts[0],
        spawn_max: config.density_counts[2],
        cluster_active,
        banner_message: None,
        monsters: field
            .monsters
            .iter()
            .map(|monster| MonsterSnapshot {
                entity_id: monster.entity_id.clone(),
                monster_id: monster.monster_id.clone(),
                source_index: monster.source_index,
                asset_bundle_id: monster.asset_bundle_id.clone(),
                hp: monster.hp,
                max_hp: monster.max_hp,
                damage: monster.damage,
                armor: monster.armor,
                experience: monster.experience,
                gold: monster.gold,
                x: monster.x,
                y: monster.y,
                action_state: monster_action_name(monster.action_state).to_owned(),
                animation: monster.animation.clone(),
                target_hunter_id: monster.target_hunter_id,
                respawn_ticks: monster.respawn_ticks,
            })
            .collect(),
        drops: field
            .drops
            .iter()
            .map(|drop| MonsterDropSnapshot {
                monster_entity_id: drop.monster_entity_id.clone(),
                item_id: drop.item_id.clone(),
                quantity: drop.quantity,
            })
            .collect(),
    }
}
