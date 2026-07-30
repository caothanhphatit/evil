use std::{fs, path::Path};

use serde_json::Value;

use super::{
    integrity::hex_sha256,
    loader::{EMBEDDED_REGISTRY, EMBEDDED_REGISTRY_SHA256},
    *,
};

const BLOCKED_FIXTURE: &[u8] =
    include_bytes!("../../../../../tools/tests/fixtures/building-registry.blocked.json");

#[test]
fn rejects_blocked_registry() {
    let hash = hex_sha256(BLOCKED_FIXTURE);
    let error = load_runtime_ready_registry_bytes(BLOCKED_FIXTURE, ".", &hash).unwrap_err();

    assert!(matches!(
        error,
        BuildingRegistryLoadError::RuntimeBlocked { .. }
    ));
}

#[test]
fn blocked_registry_exposes_individually_resolved_fields_and_mutation_rows() {
    let registry =
        load_read_only_registry_bytes(EMBEDDED_REGISTRY, EMBEDDED_REGISTRY_SHA256).unwrap();
    assert_eq!(registry.runtime_state, RuntimeState::Blocked);

    let content = BuildingContentView::try_from_registry(&registry).unwrap();
    assert_eq!(content.buildings.len(), 79);
    assert!(!content.globally_runnable);
    let town_hall = content.building("build_1").unwrap();
    assert_eq!(town_hall.display_name, "Town Hall");
    assert_eq!(town_hall.levels.len(), 17);
    assert_eq!(town_hall.levels[1].level, 2);
    assert_eq!(
        town_hall.levels[1]
            .costs
            .iter()
            .find(|cost| cost.item_id == "currency:gold")
            .unwrap()
            .quantity,
        1_000
    );
    assert!(town_hall.levels[1].exact_mutation_ready);
    assert_eq!(town_hall.levels[1].required_town_hall_level, Some(1));
    assert_eq!(content.capabilities.len(), 10);
    let trading = content.capabilities_for("build_3").next().unwrap();
    assert_eq!(trading.kind, "loot-purchase-reservations");
    assert!(!trading.static_data_ready);
    assert!(!trading.runnable);
    let weapon_shop = content.building("build_7").unwrap();
    assert_eq!(weapon_shop.source_data.source_type, 0);
    assert_eq!(weapon_shop.source_data.max_build, 1);
    assert_eq!(weapon_shop.source_data.grid_size, [2, 2]);
    assert_eq!(weapon_shop.source_data.movable, 0);
    assert_eq!(weapon_shop.source_data.visibility, 0);
    assert_eq!(weapon_shop.source_data.compatible_skin, 0);
    assert_eq!(weapon_shop.source_data.in_building_flag, 0);
    assert_eq!(weapon_shop.source_data.possible_remove, -1);
    assert_eq!(weapon_shop.source_data.create_build, [0]);
    assert_eq!(weapon_shop.source_data.entry_counts, [0, 0, 0, 0, 0]);
    assert_eq!(weapon_shop.source_data.first_values, [0, 1, 2, 3, 4]);
    assert_eq!(weapon_shop.source_data.second_values, [0, 0, 0, 0, 0]);
    assert_eq!(weapon_shop.source_data.third_values, [0, 0, 0, 0, 0]);
    assert_eq!(content.items.len(), 1_107);
    assert_eq!(content.products.len(), 3_457);
    assert_eq!(content.skins.len(), 61);
    assert_eq!(
        content
            .skins
            .values()
            .filter(|skin| skin.visual.is_some())
            .count(),
        47
    );
    assert_eq!(
        content
            .skins
            .values()
            .filter(|skin| skin.visual.is_none())
            .count(),
        14
    );
    assert!(!content.skins.contains_key("build_3:skin_29"));

    let medieval_town_hall = content.skin("build_1", 1).unwrap();
    assert_eq!(medieval_town_hall.family, "middle-ages");
    assert_eq!(medieval_town_hall.display_name, "Middle Ages Town Hall");
    assert_eq!(medieval_town_hall.required_level, 4);
    assert_eq!(medieval_town_hall.visibility, 0);
    assert_eq!(medieval_town_hall.costs.len(), 5);
    assert_eq!(medieval_town_hall.costs[0].item_id, "currency:gold");
    assert_eq!(medieval_town_hall.costs[0].quantity, 1_000_000);
    let visual = medieval_town_hall.visual.as_ref().unwrap();
    assert_eq!(visual.asset_key, "buildSkin_1_0");
    assert_eq!(visual.sprite_prefix, "bd_a_cos_001_");
    assert_eq!(visual.animation_clip_path_id, 396);
    assert_eq!(visual.animator_controller_path_id, 1_067);
    assert_eq!(visual.sprite_frames.as_array().unwrap().len(), 5);

    let unresolved_skin = content.skin("build_16", 1).unwrap();
    assert_eq!(unresolved_skin.display_name, "Middle Ages Dungeon Entrance");
    assert!(unresolved_skin.visual.is_none());

    let fur = content.item("material:32").unwrap();
    assert_eq!(fur.id, "material:32");
    assert_eq!(fur.display_name.as_deref(), Some("Young Lycan Fur"));
    assert_eq!(fur.item_type.as_deref(), Some("material"));
    assert!(fur.internal_name.is_none());
    assert!(fur.stack_limit.is_none());
    assert!(fur.buy_price.is_none());
    assert!(fur.sell_price.is_none());
    assert_eq!(fur.town_pays_hunter_gold_per_unit, Some(10));
    assert!(fur.hunter_pays_town_gold_by_tier.is_none());

    let junk_sword = content.item("gear:weapon:0").unwrap();
    assert!(junk_sword.town_pays_hunter_gold_per_unit.is_none());
    assert_eq!(
        junk_sword.hunter_pays_town_gold_by_tier.as_deref(),
        Some([200, 300, 400, 500, 600].as_slice())
    );
    let healing_potion = content.item("consumable:0").unwrap();
    assert_eq!(
        healing_potion.hunter_pays_town_gold_by_tier.as_deref(),
        Some([68, 203, 608, 1_823, 5_468, 24_605, 118_098, 247_500].as_slice())
    );

    let legacy_product = content.product("product:0").unwrap();
    assert_eq!(legacy_product.building_id.as_deref(), Some("build_9"));
    assert!(legacy_product.inputs.is_none());
    let options = legacy_product.conversion_options.as_ref().unwrap();
    assert_eq!(options.len(), 5);
    assert_eq!(options[0].input_kind, "material");
    assert_eq!(options[0].input_id, "material:32");
    assert_eq!(options[0].input_quantity, 1);
    assert_eq!(options[0].output_stock_quantity, 1);
    assert_eq!(options[1].input_id, "material:92");
    assert_eq!(options[1].output_stock_quantity, 2);
    assert_eq!(options[2].input_id, "material:16");
    assert_eq!(options[2].output_stock_quantity, 10);
    assert_eq!(options[3].input_kind, "gem");
    assert_eq!(options[3].input_id, "currency:gem");
    assert_eq!(options[3].input_quantity, 3);
    assert_eq!(options[3].output_stock_quantity, 1);
    assert_eq!(options[4].input_kind, "elemental");
    assert_eq!(options[4].input_id, "currency:elemental");
    assert_eq!(options[4].input_quantity, 150);
    assert_eq!(options[4].output_stock_quantity, 1);
    assert!(legacy_product.outputs.is_none());
    assert_eq!(legacy_product.duration_ms, Some(10_000));
    assert!(legacy_product.sale_price.is_none());
    assert!(!legacy_product.exact_mutation_ready);
    let service = legacy_product.service_data.as_ref().unwrap();
    assert_eq!(service.source_type, 0);
    assert_eq!(service.required_level, 0);
    assert_eq!(service.service_time_ms, 10_000);
    assert_eq!(service.effect_value, 140);
    assert_eq!(service.use_money, 90);
    assert_eq!(service.completion_counts, [1, 2, 10]);
    assert_eq!(service.required_cash_count, 3);
    assert_eq!(service.cash_completion_count, 1);
    assert_eq!(service.required_elemental_count, 150);
    assert_eq!(service.elemental_completion_count, 1);

    let weapon_recipe = content.product("recipe:weapon:0:rating:0").unwrap();
    assert_eq!(weapon_recipe.building_id.as_deref(), Some("build_10"));
    let outputs = weapon_recipe.outputs.as_ref().unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].item_id, "gear:weapon:0");
    assert_eq!(outputs[0].quantity, 1);
    assert!(weapon_recipe.duration_ms.is_none());
    assert!(weapon_recipe.sale_price.is_none());
    assert!(weapon_recipe.service_data.is_none());
    assert!(weapon_recipe.conversion_options.is_none());
    assert!(weapon_recipe.random_output.is_none());
    assert!(!weapon_recipe.exact_mutation_ready);

    let random_rune = content.product("recipe:rune-random:0").unwrap();
    let rune_inputs = random_rune.inputs.as_ref().unwrap();
    assert_eq!(rune_inputs.len(), 1);
    assert_eq!(rune_inputs[0].item_id, "material:189");
    assert_eq!(rune_inputs[0].quantity, 5);
    assert!(random_rune.outputs.is_none());
    assert!(random_rune.sale_price.is_none());
    let random_output = random_rune.random_output.as_ref().unwrap();
    assert_eq!(random_output.item_type, "rune");
    assert_eq!(random_output.grade, 0);
    assert_eq!(random_output.quantity, 1);
    assert!(!random_output.rng_ready);
    assert!(content
        .recipes_for_building("build_10")
        .any(|recipe| recipe.id == "recipe:weapon:0:rating:0"));
    assert!(content
        .products
        .values()
        .all(|product| !product.exact_mutation_ready));

    let trading_post = registry
        .buildings
        .rows
        .iter()
        .find(|building| building.key == "build_3")
        .unwrap();
    for condition in trading_post
        .levels
        .rows
        .iter()
        .flat_map(|level| &level.conditions.rows)
    {
        assert_eq!(condition.subject_id.value, "build_1.level");
        assert_eq!(condition.operator.value, "greater-than-or-equal");
        condition.subject_id.validate_resolved("subjectId").unwrap();
        condition.operator.validate_resolved("operator").unwrap();
    }
}

#[test]
fn canonical_content_is_loaded_once_from_hash_pinned_payload() {
    let first = canonical_building_content().unwrap();
    let second = canonical_building_content().unwrap();
    assert!(std::ptr::eq(first, second));
    assert_eq!(first.registry_id, "evil-hunter-1.411.buildings-v1");
}

#[test]
fn rejects_runtime_ready_release_with_declared_blockers() {
    let mut registry: Value = serde_json::from_slice(BLOCKED_FIXTURE).unwrap();
    registry["runtimeState"] = Value::String("runtime-ready".into());
    registry["releaseGate"]["runnable"] = Value::Bool(true);
    let payload = serde_json::to_vec(&registry).unwrap();
    let hash = hex_sha256(&payload);

    let error = load_runtime_ready_registry_bytes(&payload, ".", &hash).unwrap_err();
    assert!(matches!(
        error,
        BuildingRegistryLoadError::MalformedRelease(_)
    ));
}

#[test]
fn rejects_registry_payload_hash_mismatch_before_loading_content() {
    let error =
        load_runtime_ready_registry_bytes(BLOCKED_FIXTURE, ".", &"0".repeat(64)).unwrap_err();

    assert!(matches!(
        error,
        BuildingRegistryLoadError::RegistryHashMismatch { .. }
    ));
}

#[test]
fn rejects_evidence_source_hash_mismatch() {
    let mut registry: BuildingRegistry = serde_json::from_slice(BLOCKED_FIXTURE).unwrap();
    let bytes = fs::metadata("Cargo.toml").unwrap().len();
    registry.evidence_sources.push(EvidenceSource {
        id: "server-cargo".into(),
        path: "Cargo.toml".into(),
        bytes,
        sha256: "0".repeat(64),
    });

    let error = registry
        .verify_evidence_sources(Path::new("."))
        .unwrap_err();
    assert!(matches!(
        error,
        BuildingRegistryLoadError::EvidenceHashMismatch { .. }
    ));
}
