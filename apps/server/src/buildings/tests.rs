use std::collections::BTreeMap;

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use super::*;

fn base(id: &str) -> BaseBuildingDefinition {
    BaseBuildingDefinition {
        id: BaseBuildingId::parse(id).unwrap(),
        registry_id: "release-1".into(),
        display_name: "Town Hall".into(),
        category: Some("town".into()),
        source_type: 1,
        max_instances: 1,
        grid_width: 3,
        grid_height: 3,
        movable: Some(true),
        constructible: Some(true),
        base_sprite_asset_id: Some("town-hall".into()),
    }
}

fn level(id: &str) -> BuildingLevelDefinition {
    BuildingLevelDefinition {
        building_id: BaseBuildingId::parse(id).unwrap(),
        level: 1,
        upgrade_duration_ms: Some(0),
        inventory_capacity: None,
        production_slots: None,
        costs: Vec::new(),
        prerequisites: Vec::new(),
    }
}

#[test]
fn base_and_skin_identifiers_cannot_be_confused() {
    assert!(BaseBuildingId::parse("build_0").is_ok());
    assert!(BaseBuildingId::parse("build_1").is_ok());
    assert!(BaseBuildingId::parse("buildSkin_1_1").is_err());
    assert!(BaseBuildingId::parse("build_01").is_err());
    assert!(BuildingSkinId::new(0).is_err());
}

#[test]
fn catalog_rejects_skin_without_a_base_row() {
    let catalog = BuildingCatalog {
        registry_id: "release-1".into(),
        bases: vec![base("build_1")],
        levels: vec![level("build_1")],
        skins: vec![BuildingSkinDefinition {
            key: BuildingSkinKey {
                building_id: BaseBuildingId::parse("build_2").unwrap(),
                skin_id: BuildingSkinId::new(1).unwrap(),
            },
            family: "default".into(),
            display_name: "Skin".into(),
            required_level: 1,
            visibility: 1,
            asset_key: None,
            sprite_prefix: None,
            visual_resolved: false,
        }],
    };

    assert!(matches!(
        catalog.validate(),
        Err(BuildingRepositoryError::UnknownSkinBase(_))
    ));
}

#[test]
fn catalog_rejects_mixed_content_releases() {
    let mut second = base("build_2");
    second.registry_id = "release-2".into();
    let catalog = BuildingCatalog {
        registry_id: "release-1".into(),
        bases: vec![base("build_1"), second],
        levels: vec![level("build_1"), level("build_2")],
        skins: Vec::new(),
    };

    assert!(matches!(
        catalog.validate(),
        Err(BuildingRepositoryError::MixedRegistryRelease)
    ));
}

#[test]
fn town_rejects_duplicate_instance_ids_before_persistence() {
    let instance_id = TownBuildingInstanceId::new(Uuid::new_v4());
    let building = TownBuildingInstance {
        instance_id,
        building_id: BaseBuildingId::parse("build_1").unwrap(),
        equipped_skin_id: None,
        level: 1,
        uses: 0,
        grid_x: 0,
        grid_y: 0,
        seeded_by: None,
    };
    let state = TownBuildingState {
        release_id: "release-1".into(),
        town_gold: 0,
        seed_version: 0,
        next_building_sequence: 2,
        buildings: vec![building.clone(), building],
        hunter_materials: 0,
        materials: 0,
        runes: 0,
        weapons: 0,
        armor: 0,
        hunter_equipment_purchases: 0,
        field_trip_id: 0,
        settled_field_trip_id: 0,
        material_stocks: Vec::new(),
        product_stocks: Vec::new(),
        trade_settlements: Vec::new(),
    };

    assert!(matches!(
        state.validate(),
        Err(BuildingRepositoryError::DuplicateInstance(id)) if id == instance_id
    ));
}

#[test]
fn gameplay_catalog_rejects_unknown_building_references() {
    let catalog = BuildingCatalog {
        registry_id: "release-1".into(),
        bases: vec![base("build_1")],
        levels: vec![level("build_1")],
        skins: Vec::new(),
    };
    let gameplay = BuildingGameplayCatalog {
        registry_id: "release-1".into(),
        capabilities: vec![BuildingCapabilityDefinition {
            capability_id: "capability:craft".into(),
            building_id: BaseBuildingId::parse("build_2").unwrap(),
            kind: "craft".into(),
            static_data_ready: true,
            runnable: false,
        }],
        items: BTreeMap::new(),
        products: BTreeMap::new(),
        gear_products: BTreeMap::new(),
        consumable_products: BTreeMap::new(),
    };

    assert!(matches!(
        gameplay.validate(&catalog),
        Err(BuildingRepositoryError::UnknownGameplayBase(id))
            if id == BaseBuildingId::parse("build_2").unwrap()
    ));
}

#[tokio::test]
async fn migrated_catalog_loads_complete_normalized_content() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    let release_id = "evil-hunter-1.411.buildings-v1";
    let registry_hash = sqlx::query_scalar::<_, String>(
        "SELECT encode(registry_sha256, 'hex') FROM content_release WHERE release_id = $1",
    )
    .bind(release_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let repository = PostgresBuildingRepository::from_pool(pool);

    let catalog = repository
        .load_catalog(release_id, &registry_hash)
        .await
        .unwrap();
    assert_eq!(catalog.bases.len(), 79);
    assert_eq!(catalog.levels.len(), 227);
    assert_eq!(catalog.skins.len(), 61);
    assert_eq!(
        catalog
            .levels
            .iter()
            .map(|definition| definition.costs.len())
            .sum::<usize>(),
        402
    );
    assert_eq!(
        catalog
            .levels
            .iter()
            .map(|definition| definition.prerequisites.len())
            .sum::<usize>(),
        227
    );

    let gameplay = repository
        .load_gameplay_catalog(release_id, &registry_hash)
        .await
        .unwrap();
    assert_eq!(gameplay.capabilities.len(), 10);
    assert_eq!(gameplay.items.len(), 1_107);
    assert_eq!(gameplay.products.len(), 3_457);
    assert_eq!(gameplay.gear_products.len(), 3_355);
    assert_eq!(gameplay.consumable_products.len(), 40);
    assert_eq!(
        gameplay
            .items
            .values()
            .filter(|item| item.difficulty_rating.is_some())
            .count(),
        369
    );
    let healing = gameplay
        .consumable_product("recipe:consumable:0:level:7")
        .unwrap();
    assert_eq!(
        (healing.keep_value, healing.cooldown_ms),
        (9_375_000, 20_000)
    );
    gameplay.validate(&catalog).unwrap();
}

#[tokio::test]
async fn postgres_world_map_catalog_is_complete_when_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresBuildingRepository::connect_lazy(&database_url).unwrap();
    let maps = repository
        .load_world_maps("evil-hunter-1.411.buildings-v1")
        .await
        .unwrap();
    assert_eq!(maps.len(), 3);
    assert_eq!(maps[0].map_id, "map_new01");
    assert_eq!(maps[0].density_counts, [3, 6, 9]);
    assert_eq!(maps[2].entry_waypoints[2], (2127, 724));
}

#[tokio::test]
async fn postgres_progression_and_monster_catalogs_are_complete_when_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresBuildingRepository::connect_lazy(&database_url).unwrap();
    let progression = repository
        .load_hunter_progression(
            "evil-hunter-1.411.buildings-v1",
            "evil-hunter-1.411.experience-runtime-v1",
        )
        .await
        .unwrap();
    assert_eq!(progression.max_stored_level, 99);
    assert_eq!(progression.experience_by_level.len(), 100);
    assert_eq!(
        progression.experience_by_level[1],
        [240, 960, 5_760, 46_080, 460_800, 5_529_600]
    );

    let pools = repository
        .load_ordinary_monster_pools("evil-hunter-1.411.buildings-v1")
        .await
        .unwrap();
    assert_eq!(pools.len(), 15);
    assert!(pools.iter().all(|pool| pool.monsters.len() == 3));
    let first = &pools
        .iter()
        .find(|pool| pool.map_id == "map_new01" && pool.global_difficulty == 0)
        .unwrap()
        .monsters[0];
    assert_eq!(
        (first.source_index, first.hp, first.damage),
        (0, 1_298, 542)
    );
    assert_eq!(first.materials.len(), 8);

    let hunter_content = repository
        .load_hunter_static_content(
            "migration.hunter-demo-v1",
            "evil-hunter-1.411.hunter-info-v1",
        )
        .await
        .unwrap();
    assert_eq!(hunter_content.classes.len(), 5);
    assert_eq!(hunter_content.rarities.len(), 5);
    assert_eq!(hunter_content.personalities.len(), 33);
    assert_eq!(hunter_content.basic_skills.len(), 10);
    assert_eq!(hunter_content.basic_skills[0].skill_id, "skill_h1_01");
    assert_eq!(hunter_content.basic_skills[0].cooldown_ms, 15_000);
    assert!(hunter_content.basic_skills[0].confirmed_icon_path.is_some());
}
