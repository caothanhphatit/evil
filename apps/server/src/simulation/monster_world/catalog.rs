use std::sync::OnceLock;

use crate::buildings::{OrdinaryMonsterPoolDefinition, WorldMapDefinition};

use super::{MonsterMapConfig, RegionBounds};

static MAP_CONFIGS: OnceLock<Vec<MonsterMapConfig>> = OnceLock::new();
static MONSTER_POOLS: OnceLock<Vec<OrdinaryMonsterPoolDefinition>> = OnceLock::new();

fn fixture_map_configs() -> Vec<MonsterMapConfig> {
    vec![
        MonsterMapConfig {
            map_id: "map_new01",
            area: 0,
            monster_tier: 1,
            map_asset_id: "/content/releases/visible-world-v1/maps/map_new01.png",
            density_counts: [3, 6, 9],
            bounds: RegionBounds {
                min_x: 320,
                max_x: 1030,
                min_y: 500,
                max_y: 1000,
            },
            entry_waypoints: [(1410, 690), (1356, 800), (1273, 800)],
        },
        MonsterMapConfig {
            map_id: "background_08",
            area: 1,
            monster_tier: 2,
            map_asset_id:
                "/content/releases/visible-world-v1/village/background/background_08__1530.png",
            density_counts: [3, 6, 9],
            bounds: RegionBounds {
                min_x: 1080,
                max_x: 1760,
                min_y: 1080,
                max_y: 1430,
            },
            entry_waypoints: [(1410, 690), (1356, 800), (1356, 861)],
        },
        MonsterMapConfig {
            map_id: "background_11",
            area: 2,
            monster_tier: 3,
            map_asset_id:
                "/content/releases/visible-world-v1/village/background/background_11__1508.png",
            density_counts: [3, 6, 9],
            bounds: RegionBounds {
                min_x: 2220,
                max_x: 2860,
                min_y: 500,
                max_y: 1030,
            },
            entry_waypoints: [(1957, 809), (2043, 724), (2127, 724)],
        },
    ]
}

pub(crate) fn install_map_configs(
    definitions: Vec<WorldMapDefinition>,
) -> Result<(), &'static str> {
    let configs = definitions
        .into_iter()
        .map(|definition| MonsterMapConfig {
            map_id: Box::leak(definition.map_id.into_boxed_str()),
            area: definition.area,
            monster_tier: definition.monster_tier,
            map_asset_id: Box::leak(definition.map_asset_id.into_boxed_str()),
            density_counts: definition.density_counts,
            bounds: RegionBounds {
                min_x: definition.bounds.0,
                max_x: definition.bounds.1,
                min_y: definition.bounds.2,
                max_y: definition.bounds.3,
            },
            entry_waypoints: definition.entry_waypoints,
        })
        .collect::<Vec<_>>();
    if configs.is_empty() {
        return Err("world map catalog is empty");
    }
    if let Some(installed) = MAP_CONFIGS.get() {
        return (installed == &configs)
            .then_some(())
            .ok_or("world map catalog was already installed with different data");
    }
    MAP_CONFIGS
        .set(configs)
        .map_err(|_| "world map catalog installation raced")
}

pub fn map_configs() -> &'static [MonsterMapConfig] {
    MAP_CONFIGS.get_or_init(fixture_map_configs).as_slice()
}

pub fn map_config(map_id: &str) -> Option<&'static MonsterMapConfig> {
    map_configs().iter().find(|config| config.map_id == map_id)
}

pub(crate) fn install_monster_pools(
    definitions: Vec<OrdinaryMonsterPoolDefinition>,
) -> Result<(), &'static str> {
    if definitions.is_empty() || definitions.iter().any(|pool| pool.monsters.is_empty()) {
        return Err("ordinary monster catalog is incomplete");
    }
    if let Some(installed) = MONSTER_POOLS.get() {
        return (installed == &definitions)
            .then_some(())
            .ok_or("ordinary monster catalog was already installed with different data");
    }
    MONSTER_POOLS
        .set(definitions)
        .map_err(|_| "ordinary monster catalog installation raced")
}

pub(super) fn monster_pools() -> &'static [OrdinaryMonsterPoolDefinition] {
    MONSTER_POOLS.get_or_init(test_monster_pools).as_slice()
}

#[cfg(not(test))]
fn test_monster_pools() -> Vec<OrdinaryMonsterPoolDefinition> {
    panic!("ordinary monster catalog must be installed from PostgreSQL before simulation starts")
}

#[cfg(test)]
fn test_monster_pools() -> Vec<OrdinaryMonsterPoolDefinition> {
    let source: OrdinaryMonsterMap = serde_json::from_str(include_str!(
        "../../../../../packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json"
    ))
    .expect("test monster fixture must decode");
    source
        .regions
        .into_iter()
        .flat_map(|region| {
            let map_id = map_configs()
                .iter()
                .find(|map| map.area == region.area)
                .expect("test map fixture covers monster area")
                .map_id;
            region
                .difficulties
                .into_iter()
                .map(move |difficulty| OrdinaryMonsterPoolDefinition {
                    map_id: map_id.to_owned(),
                    global_difficulty: difficulty.global_difficulty,
                    monsters: difficulty
                        .monster_pool
                        .into_iter()
                        .map(|monster| crate::buildings::MonsterDefinition {
                            source_index: monster.source_index,
                            hp: monster.hp,
                            damage: monster.damage,
                            armor: monster.armor,
                            experience: monster.experience,
                            gold: monster.gold,
                            asset_bundle_id: "mon_a_01_1".to_owned(),
                            materials: monster
                                .materials
                                .indices
                                .into_iter()
                                .zip(monster.materials.counts)
                                .zip(monster.materials.percent_values)
                                .map(|((source_index, count), raw_percent)| {
                                    crate::buildings::MonsterMaterialDefinition {
                                        source_index,
                                        count,
                                        raw_percent,
                                    }
                                })
                                .collect(),
                        })
                        .collect(),
                })
        })
        .collect()
}

#[cfg(test)]
#[derive(serde::Deserialize)]
struct OrdinaryMonsterMap {
    regions: Vec<OrdinaryRegion>,
}

#[cfg(test)]
#[derive(serde::Deserialize)]
struct OrdinaryRegion {
    area: u8,
    difficulties: Vec<OrdinaryDifficulty>,
}

#[cfg(test)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryDifficulty {
    global_difficulty: u8,
    monster_pool: Vec<OrdinaryMonster>,
}

#[cfg(test)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryMonster {
    source_index: u32,
    hp: u64,
    damage: u64,
    armor: u64,
    experience: u64,
    gold: u64,
    materials: OrdinaryMaterials,
}

#[cfg(test)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryMaterials {
    indices: Vec<u32>,
    counts: Vec<u32>,
    percent_values: Vec<u32>,
}
