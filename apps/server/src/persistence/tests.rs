use super::*;

const HUNTER_CODEC_SOURCES: [&str; 3] = [
    include_str!("hunter_runtime_load.rs"),
    include_str!("hunter_roster_save.rs"),
    include_str!("hunter_runtime_save.rs"),
];
use crate::buildings::BuildingRepository;
use crate::content::building_registry::EMBEDDED_REGISTRY_SHA256;
use crate::simulation::{DurablePlayerState, OriginalFlowSession, OriginalScreen};

#[tokio::test]
async fn local_identity_is_idempotent_and_separate_from_cache_state() {
    let repository = InMemoryPlayerRepository::default();
    let token_hash = SessionTokenHash::from_token(Uuid::new_v4());

    assert_eq!(
        repository.resolve_local_identity(token_hash).await.unwrap(),
        None
    );
    let player = repository
        .resolve_or_create_local_identity(token_hash)
        .await
        .unwrap();
    assert_eq!(
        repository
            .resolve_or_create_local_identity(token_hash)
            .await
            .unwrap(),
        player
    );
    assert_eq!(
        repository.resolve_local_identity(token_hash).await.unwrap(),
        Some(player)
    );
}

#[tokio::test]
async fn different_local_identity_hashes_receive_different_players() {
    let repository = InMemoryPlayerRepository::default();
    let first = repository
        .resolve_or_create_local_identity(SessionTokenHash::from_token(Uuid::new_v4()))
        .await
        .unwrap();
    let second = repository
        .resolve_or_create_local_identity(SessionTokenHash::from_token(Uuid::new_v4()))
        .await
        .unwrap();

    assert_ne!(first, second);
}

#[tokio::test]
async fn new_local_account_is_seeded_once_with_gold_and_five_hunters() {
    let repository = InMemoryPlayerRepository::default();
    let player = repository
        .resolve_or_create_local_identity(SessionTokenHash::from_token(Uuid::new_v4()))
        .await
        .unwrap();

    let first = repository.load_or_create(player).await.unwrap();
    assert_eq!(first.state.buildings.town_gold, 100_000);
    assert_eq!(first.state.hunter_roster.hunters.len(), 5);

    let second = repository.load_or_create(player).await.unwrap();
    assert_eq!(second.state, first.state);
}

#[test]
fn durable_identity_migration_stores_only_a_fixed_length_hash() {
    let migration =
        include_str!("../../../../infra/db/migrations/0004_durable_local_identities.sql");

    assert!(migration.contains("token_hash BYTEA PRIMARY KEY"));
    assert!(migration.contains("octet_length(token_hash) = 32"));
    assert!(!migration.contains("session_token"));
}

#[test]
fn normalized_building_schema_separates_content_and_player_state() {
    let migration =
        include_str!("../../../../infra/db/migrations/0007_normalized_building_domain.sql");

    assert!(migration.contains("CREATE TABLE building_definition"));
    assert!(migration.contains("CREATE TABLE building_skin_definition"));
    assert!(migration.contains("CREATE TABLE player_building"));
    assert!(migration.contains("CREATE TABLE town_economy_summary"));
    assert!(migration.contains("CREATE TABLE hunter_trade_settlement"));
    assert!(migration.contains("REFERENCES building_skin_definition"));
}

#[test]
fn normalized_hunter_roster_schema_preserves_capacity_fifo_and_idempotency() {
    let migration =
        include_str!("../../../../infra/db/migrations/0013_normalized_hunter_roster.sql");

    assert!(migration.contains("CREATE TABLE player_hunter_roster"));
    assert!(migration.contains("CREATE TABLE player_hunter"));
    assert!(migration.contains("roster_position < 8"));
    assert!(migration.contains("player_hunter_waiting_sequence_unique"));
    assert!(migration.contains("CREATE TABLE player_hunter_roster_command"));
}

#[test]
fn hunter_profile_schema_separates_content_owned_state_and_seeds_eight_demo_hunters() {
    let migration =
        include_str!("../../../../infra/db/migrations/0014_hunter_profiles_and_demo_account.sql");

    assert!(migration.contains("CREATE TABLE hunter_class_definition"));
    assert!(migration.contains("CREATE TABLE hunter_trait_definition"));
    assert!(migration.contains("CREATE TABLE hunter_skill_definition"));
    assert!(migration.contains("CREATE TABLE player_profile"));
    assert!(migration.contains("CREATE TABLE player_hunter_trait"));
    assert!(migration.contains("CREATE TABLE player_hunter_skill"));
    assert_eq!(
        migration
            .matches("'00000000-0000-4000-8000-00000000a001', 1, 'active'")
            .count(),
        1
    );
    assert_eq!(
        migration
            .matches("'00000000-0000-4000-8000-00000000a001', 8, 'active'")
            .count(),
        1
    );
    assert!(migration.contains("'hunter-lab:20260724'"));
}

#[test]
fn real_account_migration_supports_cross_browser_sessions_and_demo_logins() {
    let migration = include_str!("../../../../infra/db/migrations/0035_real_player_accounts.sql");
    assert!(migration.contains("CREATE TABLE player_account"));
    assert!(migration.contains("DROP CONSTRAINT IF EXISTS local_identities_player_token_key"));
    assert!(migration.contains("demo1@evil.local"));
    assert!(migration.contains("demo2@evil.local"));
    assert!(migration.contains("demo3@evil.local"));
    assert!(migration.contains("$pbkdf2-sha256$20000$"));
}

#[test]
fn demo_stock_and_service_action_state_migrations_cover_live_runtime() {
    let demo_stock =
        include_str!("../../../../infra/db/migrations/0036_full_demo_account_stock.sql");
    let service_states = include_str!(
        "../../../../infra/db/migrations/0037_hunter_service_priority_action_states.sql"
    );
    assert!(demo_stock.contains("100000000"));
    assert!(demo_stock.contains("demo_full_stock_v1"));
    assert!(demo_stock.contains("player_hunter_item_stack"));
    assert!(service_states.contains("returning_for_service"));
    assert!(service_states.contains("waiting_for_service"));
}

#[test]
fn demo_accounts_have_independent_players_and_lazy_full_stock_seeding() {
    let migration =
        include_str!("../../../../infra/db/migrations/0038_independent_demo_accounts.sql");
    assert!(migration.contains("00000000-0000-4000-8000-00000000a002"));
    assert!(migration.contains("00000000-0000-4000-8000-00000000a003"));
    assert!(migration.contains("seed_full_demo_account_stock"));
    assert!(migration.contains("target_player"));
}

#[test]
fn hunter_info_schema_separates_definitions_from_nullable_player_state() {
    let migration = include_str!("../../../../infra/db/migrations/0016_hunter_info_domain.sql");

    assert!(migration.contains("CREATE TABLE hunter_characteristic_definition"));
    assert!(migration.contains("CREATE TABLE hunter_growth_property_definition"));
    assert!(migration.contains("CREATE TABLE hunter_riding_pet_definition"));
    assert!(migration.contains("CREATE TABLE player_hunter_growth"));
    assert!(migration.contains("CREATE TABLE player_hunter_material_stack"));
    assert!(migration.contains("CREATE TABLE player_hunter_riding_pet"));
    assert_eq!(migration.matches("'resolved', 'basic',").count(), 10);
    assert_eq!(migration.matches("'resolved', 'class_change',").count(), 40);
    assert_eq!(migration.matches("'growth:").count(), 15);
    assert!(migration.contains("icon_path, animation_name"));
    assert!(!migration.contains("skill_h1_01"));
}

#[test]
fn operational_equipment_fixture_is_separate_from_runtime_evidence() {
    let migration =
        include_str!("../../../../infra/db/migrations/0022_hunter_test_fixture_equipment.sql");

    assert!(migration.contains("CREATE TABLE player_hunter_fixture_equipment"));
    assert!(migration.contains("web_rebuild_test_fixture"));
    assert!(migration.contains("never runtime_evidence/source_* data"));
    assert!(!migration.contains("INSERT INTO player_hunter_runtime_gear"));
    assert!(migration.contains("('h5', 252, 'Rusty Spear', 'weapon-252.png')"));
    assert_eq!(fixture_equipment_slot_order("gloves").unwrap(), 0);
    assert_eq!(fixture_equipment_slot_order("boots").unwrap(), 3);
    assert_eq!(fixture_equipment_slot_order("weapon").unwrap(), 5);
    assert_eq!(fixture_equipment_slot_order("armor").unwrap(), 6);
}

#[test]
fn hunter_runtime_schema_normalizes_full_capture_objects_without_claiming_nested_values() {
    let migration =
        include_str!("../../../../infra/db/migrations/0017_hunter_runtime_evidence.sql");
    let persistence = HUNTER_CODEC_SOURCES.concat();

    assert!(migration.contains("CREATE TABLE player_hunter_runtime_section"));
    assert!(migration.contains("CREATE TABLE player_hunter_runtime_appearance"));
    assert!(migration.contains("CREATE TABLE player_hunter_runtime_skill"));
    assert!(migration.contains("CREATE TABLE player_hunter_runtime_item"));
    assert!(migration.contains("CREATE TABLE player_hunter_runtime_gear"));
    assert!(migration.contains("additional_plus_type INTEGER[] NOT NULL"));
    assert!(migration.contains("CREATE TABLE player_hunter_runtime_growth"));
    assert!(migration.contains("CREATE TABLE player_hunter_runtime_riding_pet"));
    assert!(migration.contains("CHECK (NOT nested_values_resolved)"));
    assert!(migration.contains("CHECK (NOT pet_gear_values_resolved)"));
    assert!(!migration.contains("JSONB"));
    assert!(persistence.contains("save_hunter_runtime_in(transaction, player_token, hunter)"));
    assert!(persistence.contains("UPDATE player_hunter\n           SET source_dictionary_key"));
    for table in [
        "player_hunter_runtime_section",
        "player_hunter_runtime_appearance",
        "player_hunter_runtime_skill",
        "player_hunter_runtime_item",
        "player_hunter_runtime_gear",
        "player_hunter_runtime_consumable",
        "player_hunter_runtime_growth",
        "player_hunter_runtime_riding_pet",
    ] {
        assert!(persistence.contains(&format!("INSERT INTO {table}")));
    }
}

#[test]
fn hunter_flow_schema_persists_hunt_state_and_command_keys() {
    let migration = include_str!("../../../../infra/db/migrations/0018_hunter_flow_v1.sql");
    assert!(migration.contains("ADD COLUMN hunt_state JSONB"));
    assert!(migration.contains("CREATE TABLE player_hunter_action_command"));
    assert!(migration.contains("command_key TEXT NOT NULL"));
    assert!(HUNTER_CODEC_SOURCES
        .iter()
        .any(|source| source.contains("hunt_state")));
}

#[test]
fn enhancement_action_states_are_allowed_by_the_player_constraint() {
    let migration =
        include_str!("../../../../infra/db/migrations/0028_hunter_enhancement_action_states.sql");
    for state in [
        "traveling_to_enhancement_forge",
        "waiting_for_enhancement_interaction",
        "configuring_enhancement",
    ] {
        assert!(migration.contains(state));
    }
}

#[test]
fn autonomous_hunt_action_states_are_allowed_by_the_player_constraint() {
    let migration = include_str!(
        "../../../../infra/db/migrations/0029_hunter_entering_region_action_state.sql"
    );
    for state in [
        "entering_region",
        "returning_for_infirmary",
        "using_healing_potion",
    ] {
        assert!(migration.contains(&format!("'{state}'")));
    }
    assert!(migration.contains("player_hunter_action_state_check"));
}

#[test]
fn hunter_trade_action_state_is_allowed_by_the_player_constraint() {
    let migration =
        include_str!("../../../../infra/db/migrations/0030_hunter_trade_action_state.sql");
    assert!(migration.contains("'returning_to_trade'"));
    assert!(migration.contains("player_hunter_action_state_check"));
}

#[test]
fn world_map_catalog_is_normalized_and_versioned() {
    let migration = include_str!("../../../../infra/db/migrations/0031_world_content_catalog.sql");
    assert!(migration.contains("CREATE TABLE world_map_definition"));
    assert!(migration.contains("CREATE TABLE world_map_density_definition"));
    assert!(migration.contains("CREATE TABLE world_map_entry_waypoint"));
    assert!(migration.contains("REFERENCES content_release(release_id)"));
    assert!(!migration.contains("JSONB"));
}

#[test]
fn progression_catalog_is_normalized_and_versioned() {
    let migration =
        include_str!("../../../../infra/db/migrations/0032_progression_content_catalog.sql");
    assert!(migration.contains("CREATE TABLE hunter_progression_definition"));
    assert!(migration.contains("CREATE TABLE hunter_progression_experience"));
    assert!(migration.contains("REFERENCES content_release(release_id)"));
    assert!(!migration.contains("JSONB"));
}

#[test]
fn runtime_object_catalog_has_admin_ready_relational_boundaries() {
    let migration = include_str!("../../../../infra/db/migrations/0033_runtime_object_catalog.sql");
    for table in [
        "content_source_manifest",
        "material_definition",
        "monster_definition",
        "monster_material_drop_definition",
        "ordinary_monster_pool_definition",
        "gear_definition",
        "gear_rating_definition",
        "gear_material_requirement",
        "economy_product_gear_binding",
        "consumable_definition",
        "consumable_level_definition",
        "economy_product_consumable_binding",
    ] {
        assert!(migration.contains(&format!("CREATE TABLE {table}")));
    }
    assert!(migration.contains("REFERENCES content_release(release_id)"));
    assert!(migration.contains("source_sha256 BYTEA"));
    assert!(migration.contains("unresolved_evidence JSONB"));
    assert!(migration.contains("UPDATE hunter_skill_definition"));
}

#[test]
fn hunter_purchase_and_crafted_gear_rows_have_durable_storage() {
    let ownership = include_str!("../../../../infra/db/migrations/0024_hunter_owned_items.sql");
    let gear_stock = include_str!("../../../../infra/db/migrations/0025_crafted_gear_stock.sql");
    let inventory =
        include_str!("../../../../infra/db/migrations/0034_player_inventory_authority.sql");

    assert!(ownership.contains("ADD COLUMN owned_items JSONB NOT NULL"));
    assert!(gear_stock.contains("CREATE TABLE crafted_gear_stock"));
    assert!(gear_stock.contains("gear_instance_id UUID NOT NULL"));
    assert!(gear_stock.contains("FOREIGN KEY (town_id, building_instance_id)"));
    assert!(gear_stock.contains("icon_path TEXT NOT NULL"));
    assert!(gear_stock.contains("ruleset TEXT NOT NULL"));
    assert!(gear_stock.contains("crafted_gear_stock_shop_idx"));
    assert!(inventory.contains("CREATE TABLE player_hunter_item_stack"));
    assert!(inventory.contains("CREATE TABLE player_hunter_gear_instance"));
    assert!(inventory.contains("gear_instance_id UUID PRIMARY KEY"));
    assert!(inventory.contains("REFERENCES economy_product_definition"));
    assert!(inventory.contains("legacy JSONB column"));
    let town_codec = include_str!("../buildings/postgres/town.rs");
    let ownership_codec = include_str!("hunter_owned_items_save.rs");
    assert!(town_codec.contains("INSERT INTO crafted_gear_stock"));
    assert!(town_codec.contains("DELETE FROM crafted_gear_stock WHERE town_id"));
    for field in [
        "quality",
        "primary_stat",
        "option_type",
        "option_value",
        "ruleset",
    ] {
        assert!(ownership_codec.contains(field));
    }
}

#[tokio::test]
async fn postgres_loads_only_the_pinned_active_building_release_when_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresBuildingRepository::connect_lazy(&database_url).unwrap();
    let catalog = repository
        .load_catalog(ACTIVE_BUILDING_RELEASE_ID, EMBEDDED_REGISTRY_SHA256)
        .await
        .unwrap();
    assert_eq!(catalog.registry_id, ACTIVE_BUILDING_RELEASE_ID);
    assert_eq!(catalog.bases.len(), 79);
    assert_eq!(catalog.skins.len(), 61);
}

#[tokio::test]
async fn postgres_local_identity_contract_when_test_database_is_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
    let token_hash = SessionTokenHash::from_token(Uuid::new_v4());

    let player = repository
        .resolve_or_create_local_identity(token_hash)
        .await
        .unwrap();
    assert_eq!(
        repository.resolve_local_identity(token_hash).await.unwrap(),
        Some(player)
    );
    let stored_length = sqlx::query_scalar::<_, i32>(
        "SELECT octet_length(token_hash) FROM local_identities WHERE player_token = $1",
    )
    .bind(player)
    .fetch_one(&repository.pool)
    .await
    .unwrap();
    assert_eq!(stored_length, 32);

    sqlx::query("DELETE FROM local_identities WHERE player_token = $1")
        .bind(player)
        .execute(&repository.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn postgres_new_account_seed_is_atomic_and_idempotent_when_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
    let token_hash = SessionTokenHash::from_token(Uuid::new_v4());
    let player = repository
        .resolve_or_create_local_identity(token_hash)
        .await
        .unwrap();

    let first = repository.load_or_create(player).await.unwrap();
    assert_eq!(first.state.buildings.town_gold, 100_000);
    assert_eq!(first.state.hunter_roster.hunters.len(), 5);
    let second = repository.load_or_create(player).await.unwrap();
    assert_eq!(second.state.buildings.town_gold, 100_000);
    let first_rolls = first
        .state
        .hunter_roster
        .hunters
        .iter()
        .map(|hunter| {
            (
                hunter.hunter_id,
                hunter.profile.class_id.clone(),
                hunter.profile.rarity_id.clone(),
                hunter.max_hp,
            )
        })
        .collect::<Vec<_>>();
    let second_rolls = second
        .state
        .hunter_roster
        .hunters
        .iter()
        .map(|hunter| {
            (
                hunter.hunter_id,
                hunter.profile.class_id.clone(),
                hunter.profile.rarity_id.clone(),
                hunter.max_hp,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(second_rolls, first_rolls);

    sqlx::query("DELETE FROM local_identities WHERE player_token = $1")
        .bind(player)
        .execute(&repository.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM player_world_state WHERE player_token = $1")
        .bind(player)
        .execute(&repository.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn postgres_demo_account_loads_eight_diverse_hunter_profiles_when_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
    let player = Uuid::parse_str("00000000-0000-4000-8000-00000000a001").unwrap();

    let loaded = repository.load_or_create(player).await.unwrap();
    let hunters = &loaded.state.hunter_roster.hunters;
    assert_eq!(hunters.len(), 8);
    assert_eq!(hunters[0].profile.display_name, "Astra");
    assert_eq!(hunters[0].profile.visual_family, "H4");
    assert_eq!(hunters[7].profile.display_name, "Hale");
    assert!(hunters
        .iter()
        .all(|hunter| !hunter.profile.traits.is_empty()));
    assert!(hunters
        .iter()
        .all(|hunter| !hunter.profile.skills.is_empty()));
    assert_eq!(
        hunters
            .iter()
            .map(|hunter| hunter.profile.class_id.as_str())
            .collect::<HashSet<_>>()
            .len(),
        5
    );
    assert_eq!(
        hunters
            .iter()
            .map(|hunter| hunter.profile.rarity_id.as_str())
            .collect::<HashSet<_>>()
            .len(),
        5
    );
}

#[tokio::test]
async fn postgres_hunter_runtime_evidence_round_trips_when_test_database_is_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
    let player = Uuid::new_v4();
    let loaded = repository.load_or_create(player).await.unwrap();
    let mut state = loaded.state;
    let hunter = state.hunter_roster.hunters.first_mut().unwrap();
    hunter.runtime = DurableHunterRuntimeState {
        source_dictionary_key: Some("opaque-hunter-key".into()),
        source_index: Some(41),
        source_job: Some(2),
        source_sub_job: Some(1),
        source_third_job: Some(0),
        source_fourth_job: Some(0),
        source_personality: Some(8),
        source_grade_rank_up: Some(3),
        source_dark_soul: Some(500),
        source_used_dark_soul: Some(120),
        source_used_job_trait: Some(7),
        appearance: Some(DurableHunterRuntimeAppearance {
            body_index: 4,
            costume_index: 5,
            costume_hidden: false,
            fairy_index: 6,
            fairy_hidden: true,
            weapon_costume_index: 7,
            weapon_costume_hidden: false,
            wing_costume_index: 8,
            wing_costume_hidden: false,
            seal_costume_index: 9,
            seal_costume_hidden: true,
            ramble_pet_index: 10,
            ramble_pet_hidden: false,
            hat_hidden: true,
            costume_hat_hidden: false,
        }),
        status: Some(DurableHunterRuntimeStatus {
            hp: 1000,
            now_hp: 750,
            feel: 90.5,
            now_feel: 45.25,
            hungry: 80.5,
            now_hungry: 40.25,
            tire: 70.5,
            now_tire: 35.25,
            damage: 101,
            armor: 55,
            critical: 12,
            attack_speed: 1.25,
            dodge: 9,
        }),
        skills: Some(Vec::new()),
        inventory: Some(DurableHunterRuntimeInventory {
            items: vec![DurableHunterRuntimeItem {
                dictionary_key: "item-key".into(),
                new_check: true,
                source_index: 12,
                count: 99,
                reservation: 4,
                infinity_check: false,
            }],
            gear: vec![DurableHunterRuntimeGear {
                dictionary_key: "gear-key".into(),
                source_index: 1,
                gear_index: 2,
                inventory_index: 3,
                quality: 4,
                new_check: true,
                level: 5,
                rating: 6,
                group: 7,
                plus_type: vec![8],
                plus_value: vec![9],
                minus_type: vec![10],
                minus_value: vec![11],
                additional_plus_type: vec![12],
                additional_plus_value: vec![13],
                additional_minus_type: vec![14],
                additional_minus_value: vec![15],
                buy_gold: 16,
                buy_date: "capture-date".into(),
                buy_date_value: 17,
                quality_count: 18,
                option_count: 19,
                lock_count: 20,
                potential: 21,
                runes_index: 22,
                runes_value: 23,
                skill_runes_index: 24,
                skill_runes_value: 25,
                delete_count: 26,
                unidentified_option_count: 27,
            }],
            consumables: vec![DurableHunterRuntimeConsumable {
                dictionary_key: "consumable-key".into(),
                total_count: 28,
            }],
        }),
        growth: Some(vec![DurableHunterRuntimeGrowth {
            source_order: 0,
            property_level: 3,
        }]),
        riding_pet: Some(DurableHunterRuntimeRidingPet {
            pasture_index: 1,
            source_index: 2,
            master_index: "opaque-hunter-key".into(),
            rating: 3,
            skill_index: 4,
            trait_index: 5,
            trait_level: 6,
            use_soul: 7,
            use_growth_stone: 8,
            locked: true,
        }),
    };
    let expected = hunter.runtime.clone();

    repository
        .persist(player, &state, loaded.revision, 1, &[])
        .await
        .unwrap();
    let reloaded = repository.load_or_create(player).await.unwrap();
    assert_eq!(reloaded.state.hunter_roster.hunters[0].runtime, expected);

    sqlx::query("DELETE FROM player_world_state WHERE player_token = $1")
        .bind(player)
        .execute(&repository.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn postgres_aggregate_and_ledgers_are_atomic_when_test_database_is_configured() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresPlayerRepository::connect_lazy(&database_url).unwrap();
    let player = Uuid::new_v4();
    let reward_id = Uuid::new_v4();
    let command_id = Uuid::new_v4();
    let operations = vec![
        PendingOperation::Reward {
            operation_id: reward_id,
            gold: 10,
            item_id: 2001,
            quantity: 1,
        },
        PendingOperation::Equip {
            command_id,
            item_id: 2001,
        },
    ];

    let loaded = repository.load_or_create(player).await.unwrap();
    assert_eq!(loaded.revision, 0);
    let mut state = loaded.state;
    state.navigation = OriginalFlowPlayerState {
        screen: OriginalScreen::Field,
        boot_completed: true,
    };
    state.buildings.town_gold = 1_234;
    state.buildings.materials = 7;
    state.buildings.field_trip_id = 1;
    state.buildings.settled_field_trip_id = 1;
    state.buildings.material_stocks = vec![DurableMaterialStock {
        id: "material_1".into(),
        town_quantity: 3,
        hunter_quantity: 2,
        requested: 4,
        unit_price: 5,
    }];
    state.buildings.trade_settlements = vec![DurableTradeSettlement {
        settlement_id: "settlement-1".into(),
        field_trip_id: 1,
        material_id: "material_1".into(),
        quantity: 3,
        unit_price: 5,
        total_gold: 15,
    }];
    assert_eq!(
        repository
            .persist(player, &state, 0, 1, &operations)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        repository
            .persist(player, &state, 1, 1, &operations)
            .await
            .unwrap(),
        2
    );

    let reward_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM reward_ledger WHERE player_token = $1 AND operation_id = $2",
    )
    .bind(player)
    .bind(reward_id)
    .fetch_one(&repository.pool)
    .await
    .unwrap();
    let command_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM command_ledger WHERE player_token = $1 AND command_id = $2",
    )
    .bind(player)
    .bind(command_id)
    .fetch_one(&repository.pool)
    .await
    .unwrap();
    assert_eq!(reward_count, 1);
    assert_eq!(command_count, 1);
    let stored_has_buildings = sqlx::query_scalar::<_, bool>(
        "SELECT state ? 'buildings' FROM player_world_state WHERE player_token = $1",
    )
    .bind(player)
    .fetch_one(&repository.pool)
    .await
    .unwrap();
    assert!(!stored_has_buildings);
    let reloaded = repository.load_or_create(player).await.unwrap();
    assert_eq!(reloaded.state.buildings, state.buildings);
    let settlement_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM hunter_trade_settlement \
             WHERE town_id = (SELECT town_id FROM town WHERE player_token = $1)",
    )
    .bind(player)
    .fetch_one(&repository.pool)
    .await
    .unwrap();
    assert_eq!(settlement_count, 1);

    let rejected_reward = Uuid::new_v4();
    let conflict = repository
        .persist(
            player,
            &state,
            0,
            1,
            &[PendingOperation::Reward {
                operation_id: rejected_reward,
                gold: 10,
                item_id: 2001,
                quantity: 1,
            }],
        )
        .await;
    assert!(matches!(conflict, Err(RepositoryError::RevisionConflict)));
    let rejected_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM reward_ledger WHERE player_token = $1 AND operation_id = $2",
    )
    .bind(player)
    .bind(rejected_reward)
    .fetch_one(&repository.pool)
    .await
    .unwrap();
    let revision = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM player_world_state WHERE player_token = $1",
    )
    .bind(player)
    .fetch_one(&repository.pool)
    .await
    .unwrap();
    assert_eq!(rejected_count, 0);
    assert_eq!(revision, 2);

    sqlx::query("DELETE FROM player_world_state WHERE player_token = $1")
        .bind(player)
        .execute(&repository.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn flow_state_persists_across_reconnect() {
    let repository = InMemoryPlayerRepository::default();
    let player_token = Uuid::from_u128(1);
    let state = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::HunterRoster,
            boot_completed: true,
        },
        ..DurablePlayerAggregate::default()
    };
    repository
        .persist(player_token, &state, 0, 1, &[])
        .await
        .expect("persist");
    assert_eq!(
        repository
            .load_or_create(player_token)
            .await
            .expect("load")
            .state,
        state
    );
}

#[tokio::test]
async fn stale_revision_or_fence_cannot_overwrite_state() {
    let repository = InMemoryPlayerRepository::default();
    let player = Uuid::new_v4();
    let state = DurablePlayerAggregate::default();
    assert_eq!(
        repository.persist(player, &state, 0, 5, &[]).await.unwrap(),
        1
    );
    assert!(matches!(
        repository.persist(player, &state, 0, 5, &[]).await,
        Err(RepositoryError::RevisionConflict)
    ));
    assert!(matches!(
        repository.persist(player, &state, 1, 4, &[]).await,
        Err(RepositoryError::RevisionConflict)
    ));
}

#[test]
fn legacy_navigation_json_upgrades_to_versioned_aggregate() {
    let aggregate = decode_player_state(serde_json::json!({
        "screen": "field",
        "boot_completed": true
    }))
    .unwrap();

    assert_eq!(aggregate.schema_version, DURABLE_PLAYER_SCHEMA_VERSION);
    assert_eq!(aggregate.navigation.screen, OriginalScreen::Field);
    assert_eq!(
        aggregate.migration_fixture_combat,
        DurablePlayerState::default()
    );
}

#[test]
fn authoritative_json_excludes_the_normalized_building_domain() {
    let mut aggregate = DurablePlayerAggregate::default();
    aggregate.navigation.boot_completed = true;
    aggregate.buildings.town_gold = 99;

    let encoded = encode_non_building_state(&aggregate).unwrap();

    assert_eq!(encoded["navigation"]["boot_completed"], true);
    assert!(encoded.get("buildings").is_none());
}

#[tokio::test]
async fn fixture_ledgers_are_idempotent_with_state_revision() {
    let repository = InMemoryPlayerRepository::default();
    let player = Uuid::new_v4();
    let reward_id = Uuid::from_u128(100);
    let command_id = Uuid::from_u128(200);
    let operations = vec![
        PendingOperation::Reward {
            operation_id: reward_id,
            gold: 10,
            item_id: 2001,
            quantity: 1,
        },
        PendingOperation::Equip {
            command_id,
            item_id: 2001,
        },
    ];
    let state = DurablePlayerAggregate::default();

    assert_eq!(
        repository
            .persist(player, &state, 0, 1, &operations)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        repository
            .persist(player, &state, 1, 1, &operations)
            .await
            .unwrap(),
        2
    );
    let durable = repository.durable.read().await;
    assert_eq!(durable.reward_operations.len(), 1);
    assert_eq!(durable.command_operations.len(), 1);
}

#[tokio::test]
async fn field_checkpoint_restores_combat_and_pending_reward_together() {
    let repository = InMemoryPlayerRepository::default();
    let player = Uuid::new_v4();
    let aggregate = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Field,
            boot_completed: true,
        },
        ..DurablePlayerAggregate::default()
    };
    let mut session = OriginalFlowSession::from_aggregate(aggregate, 7);
    let mut operations = Vec::new();
    for _ in 0..100 {
        let tick = session.advance_simulation_tick().expect("field tick");
        operations.extend(tick.operations);
        if !operations.is_empty() {
            break;
        }
    }
    assert!(operations
        .iter()
        .any(|operation| matches!(operation, PendingOperation::Reward { .. })));

    let mut expected = session.snapshot().migration_fixture_combat.world;
    expected.events.clear();
    repository
        .persist(player, &session.durable_state(), 0, 1, &operations)
        .await
        .unwrap();
    let loaded = repository.load_or_create(player).await.unwrap();
    let restored = OriginalFlowSession::from_aggregate(loaded.state, 7);

    assert_eq!(restored.snapshot().migration_fixture_combat.world, expected);
    assert_eq!(repository.durable.read().await.reward_operations.len(), 1);
}
