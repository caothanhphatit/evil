use super::*;

const TEST_BANDAGE_ID: &str = "product:5";

fn gear_flow(product_id: &str, sale_price: u64) -> OriginalFlowSession {
    gear_flow_for_building(product_id, sale_price, "build_10")
}

fn gear_flow_for_building(
    product_id: &str,
    sale_price: u64,
    producer_building_id: &str,
) -> OriginalFlowSession {
    let mut aggregate = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        },
        buildings: test_town_building_state(),
        hunter_roster: operational_migration_roster(),
        ..DurablePlayerAggregate::default()
    };
    aggregate
        .buildings
        .material_stocks
        .push(DurableMaterialStock {
            id: "material:11".to_owned(),
            town_quantity: 20,
            hunter_quantity: 0,
            requested: 0,
            unit_price: 1,
        });
    let mut content = (*test_authoritative_building_content()).clone();
    let mut product_parts = product_id.split(':');
    let _ = product_parts.next();
    let gear_kind = product_parts.next().expect("test gear kind");
    let gear_index = product_parts.next().expect("test gear index");
    let gear_item_id = format!("gear:{gear_kind}:{gear_index}");
    content.gameplay.items.insert(
        gear_item_id.clone(),
        EconomyItemDefinition {
            item_id: gear_item_id.clone(),
            internal_name: None,
            item_type: Some(gear_kind.to_owned()),
            stack_limit: None,
            town_pays_hunter_gold_per_unit: None,
            difficulty_rating: None,
            localized_names: BTreeMap::new(),
            buy_price: Vec::new(),
            sell_price: Vec::new(),
            hunter_pays_town_gold_by_tier: vec![sale_price; 5],
        },
    );
    content.gameplay.products.insert(
        product_id.to_owned(),
        EconomyProductDefinition {
            product_id: product_id.to_owned(),
            building_id: Some(BaseBuildingId::parse(producer_building_id).unwrap()),
            duration_ms: None,
            exact_mutation_ready: false,
            inputs: vec![EconomyAmount {
                resource_id: "material:11".to_owned(),
                quantity: 2,
            }],
            outputs: vec![EconomyAmount {
                resource_id: gear_item_id,
                quantity: 1,
            }],
            sale_price: Vec::new(),
            service: None,
            conversion_options: Vec::new(),
            random_output: None,
        },
    );
    OriginalFlowSession::from_aggregate_with_content(aggregate, 7, Arc::new(content))
}

fn building_instance_id(flow: &OriginalFlowSession, building_id: &str) -> String {
    flow.buildings
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .unwrap()
        .instance_id
        .clone()
}

fn ensure_test_trading_post(flow: &mut OriginalFlowSession) {
    if flow
        .buildings
        .buildings
        .iter()
        .any(|building| building.id == "build_3")
    {
        return;
    }
    let trading_post = test_town_building_state()
        .buildings
        .into_iter()
        .find(|building| building.id == "build_3")
        .expect("test town includes Trading Post");
    flow.buildings.buildings.push(trading_post);
}

fn advance_until_trade_settles(flow: &mut OriginalFlowSession, hunter_id: u32) {
    for _ in 0..1_000 {
        flow.advance_simulation_tick().expect("village tick");
        let pending = flow
            .hunter_roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == hunter_id)
            .and_then(|hunter| hunter.hunt.pending_trade.as_ref());
        if pending.is_none() {
            return;
        }
    }
    panic!("Hunter did not reach the Trading Post");
}

#[test]
fn blacksmith_stock_purchase_conserves_hunter_and_town_gold() {
    let product_id = "recipe:weapon:0:rating:0";
    let mut flow = gear_flow(product_id, 75);
    let blacksmith_id = building_instance_id(&flow, "build_10");
    let weapon_shop_id = building_instance_id(&flow, "build_7");

    let crafted = flow
        .handle_command(ClientCommand::CraftShopItem {
            instance_id: blacksmith_id,
            recipe_id: product_id.to_owned(),
            material_id: None,
            quantity: 2,
        })
        .unwrap();
    assert!(matches!(
        crafted.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    assert_eq!(flow.buildings.material_stocks[0].town_quantity, 20);
    assert!(flow.buildings.crafted_gear_stocks.is_empty());
    // Purchase semantics use an explicit reviewed fixture row while the
    // production writer remains fail-closed pending original evidence.
    flow.buildings
        .crafted_gear_stocks
        .push(DurableCraftedGearStock {
            building_instance_id: weapon_shop_id.clone(),
            gear_instance_id: Uuid::from_u128(7001),
            product_id: product_id.to_owned(),
            gear_kind: "weapon".to_owned(),
            rating: 0,
            quality: 2,
            primary_stat: 100,
            option_type: 0,
            option_value: 0,
            icon_path: "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png".to_owned(),
            ruleset: "test-reviewed-gear-row".to_owned(),
        });
    flow.buildings.product_stocks.push(DurableProductStock {
        building_instance_id: weapon_shop_id,
        product_id: product_id.to_owned(),
        quantity: 1,
    });
    flow.buildings
        .crafted_gear_stocks
        .push(DurableCraftedGearStock {
            building_instance_id: building_instance_id(&flow, "build_7"),
            gear_instance_id: Uuid::from_u128(7002),
            product_id: product_id.to_owned(),
            gear_kind: "weapon".to_owned(),
            rating: 0,
            quality: 3,
            primary_stat: 120,
            option_type: 0,
            option_value: 0,
            icon_path: "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png".to_owned(),
            ruleset: "test-reviewed-gear-row".to_owned(),
        });
    flow.buildings.product_stocks[0].quantity = 2;

    let gold_before = flow.buildings.town_gold;
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
    let purchase = flow
        .handle_command(ClientCommand::PurchaseShopItem {
            hunter_id: 1,
            shop_id: "build_7".to_owned(),
            product_id: product_id.to_owned(),
        })
        .unwrap();
    assert!(matches!(
        purchase.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.buildings.product_stocks[0].quantity, 1);
    assert_eq!(flow.buildings.town_gold, gold_before + 75);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before - 75);
    assert_eq!(
        flow.hunter_roster.hunters[0].owned_items[0].product_id,
        product_id
    );
    assert_eq!(flow.hunter_roster.hunters[0].owned_items[0].quantity, 1);
    assert_eq!(flow.buildings.crafted_gear_stocks.len(), 1);
    assert!(flow.hunter_roster.hunters[0].owned_items[0]
        .quality
        .is_some());
    assert_eq!(
        flow.hunter_roster.hunters[0].owned_items[0]
            .ruleset
            .as_deref(),
        Some("test-reviewed-gear-row")
    );
    assert_eq!(flow.buildings.hunter_equipment_purchases, 1);

    let second_purchase = flow
        .handle_command_with_id(
            ClientCommand::PurchaseShopItem {
                hunter_id: 1,
                shop_id: "build_7".to_owned(),
                product_id: product_id.to_owned(),
            },
            Uuid::from_u128(8_000),
        )
        .unwrap();
    assert!(matches!(
        second_purchase.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    let owned = &flow.hunter_roster.hunters[0].owned_items;
    assert_eq!(owned.len(), 2);
    assert!(owned.iter().all(|item| item.quantity == 1));
    assert!(owned.iter().all(|item| item.gear_instance_id.is_some()));
    assert_ne!(owned[0].gear_instance_id, owned[1].gear_instance_id);

    let recipes = flow.snapshot().village.building_system.recipes;
    assert!(recipes
        .iter()
        .any(|recipe| recipe.id == product_id && recipe.shop_id == "build_10"));
    assert_eq!(flow.buildings.product_stocks[0].quantity, 0);
}

#[test]
fn gear_enhancement_fails_closed_without_resolved_cost_and_rng_evidence() {
    let product_id = "recipe:weapon:0:rating:0";
    let mut flow = gear_flow(product_id, 75);
    let blacksmith_id = building_instance_id(&flow, "build_10");
    let crafted = flow
        .handle_command(ClientCommand::CraftShopItem {
            instance_id: blacksmith_id,
            recipe_id: product_id.to_owned(),
            material_id: None,
            quantity: 1,
        })
        .unwrap();
    assert!(matches!(
        crafted.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    flow.buildings
        .crafted_gear_stocks
        .push(DurableCraftedGearStock {
            building_instance_id: building_instance_id(&flow, "build_7"),
            gear_instance_id: Uuid::from_u128(8001),
            product_id: product_id.to_owned(),
            gear_kind: "weapon".to_owned(),
            rating: 0,
            quality: 2,
            primary_stat: 100,
            option_type: 0,
            option_value: 0,
            icon_path: "/content/releases/evil-hunter-1.411/gear-icons/weapon-0.png".to_owned(),
            ruleset: "test-reviewed-gear-row".to_owned(),
        });
    flow.buildings.product_stocks.push(DurableProductStock {
        building_instance_id: building_instance_id(&flow, "build_7"),
        product_id: product_id.to_owned(),
        quantity: 1,
    });
    let purchase = flow
        .handle_command_with_id(
            ClientCommand::PurchaseShopItem {
                hunter_id: 1,
                shop_id: "build_7".to_owned(),
                product_id: product_id.to_owned(),
            },
            Uuid::from_u128(8_001),
        )
        .unwrap();
    assert!(matches!(
        purchase.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    let gear_instance_id = flow.hunter_roster.hunters[0].owned_items[0]
        .gear_instance_id
        .expect("purchased gear keeps its crafted instance id");
    let premature = flow
        .handle_command(ClientCommand::EnhanceHunterGear {
            hunter_id: 1,
            gear_instance_id,
            mode: "single".to_owned(),
            optional_material_ids: Vec::new(),
        })
        .unwrap();
    assert!(matches!(
        premature.message,
        ServerMessage::IntentResult {
            accepted: false,
            reason: Some(ref reason),
            ..
        } if reason == "enhancement_visit_not_started"
    ));
    let started = flow
        .handle_command_with_id(
            ClientCommand::StartHunterEnhancement { hunter_id: 1 },
            Uuid::from_u128(8_002),
        )
        .unwrap();
    assert!(matches!(
        started.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(
        flow.hunter_roster.hunters[0]
            .hunt
            .gear_enhancement
            .as_ref()
            .map(|task| task.status),
        Some(GearEnhancementTaskStatus::Traveling)
    );
    for _ in 0..500 {
        flow.advance_simulation_tick();
        if flow.hunter_roster.hunters[0]
            .hunt
            .gear_enhancement
            .as_ref()
            .is_some_and(|task| task.status == GearEnhancementTaskStatus::WaitingForInteraction)
        {
            break;
        }
    }
    let ready_world = flow.snapshot();
    let ready_snapshot = ready_world.hunter_roster.active_hunters[0]
        .gear_enhancement_task
        .as_ref()
        .expect("enhancement task is projected");
    assert_eq!(ready_snapshot.status, "waiting_for_interaction");
    assert!(ready_snapshot.interaction_ready);
    let gold_before = flow.hunter_roster.hunters[0].gold;
    let result = flow
        .handle_command_with_id(
            ClientCommand::EnhanceHunterGear {
                hunter_id: 1,
                gear_instance_id,
                mode: "single".to_owned(),
                optional_material_ids: Vec::new(),
            },
            Uuid::from_u128(8_003),
        )
        .unwrap();
    assert!(matches!(
        result.message,
        ServerMessage::BindingBlocked { .. }
    ));
    assert_eq!(flow.hunter_roster.hunters[0].gold, gold_before);
    assert_eq!(
        flow.hunter_roster.hunters[0].owned_items[0].enhancement_level,
        Some(0)
    );
    let released = flow.snapshot().hunter_roster.active_hunters[0].clone();
    assert!(released.gear_enhancement_task.is_none());
    assert_eq!(flow.hunter_roster.hunters[0].profile.action_state, "idle");
}

#[test]
fn enhancement_visit_survives_reconnect_and_resumes_until_interaction_ready() {
    let aggregate = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        },
        buildings: test_town_building_state(),
        hunter_roster: operational_migration_roster(),
        ..DurablePlayerAggregate::default()
    };
    let mut flow = OriginalFlowSession::from_aggregate(aggregate, 7);
    let started = flow
        .handle_command_with_id(
            ClientCommand::StartHunterEnhancement { hunter_id: 1 },
            Uuid::from_u128(8_101),
        )
        .unwrap();
    assert!(matches!(
        started.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    for _ in 0..5 {
        flow.advance_simulation_tick();
    }

    let durable = flow.durable_state();
    let mut restored = OriginalFlowSession::from_aggregate(durable, 7);
    assert_eq!(
        restored.hunter_roster.hunters[0]
            .hunt
            .gear_enhancement
            .as_ref()
            .map(|task| task.status),
        Some(GearEnhancementTaskStatus::Traveling)
    );
    for _ in 0..500 {
        restored.advance_simulation_tick();
        if restored.hunter_roster.hunters[0]
            .hunt
            .gear_enhancement
            .as_ref()
            .is_some_and(|task| task.status == GearEnhancementTaskStatus::WaitingForInteraction)
        {
            break;
        }
    }
    let task = restored.hunter_roster.hunters[0]
        .hunt
        .gear_enhancement
        .as_ref()
        .expect("enhancement task survives reconnect");
    assert_eq!(
        task.status,
        GearEnhancementTaskStatus::WaitingForInteraction
    );
    assert_eq!(
        restored.hunter_roster.hunters[0].profile.action_state,
        "waiting_for_enhancement_interaction"
    );
}

#[test]
fn terminal_enhancement_task_is_released_when_restoring_a_session() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.gear_enhancement = Some(DurableGearEnhancementTask {
        status: GearEnhancementTaskStatus::Configuring,
        stop_reason: Some("evidence_disabled".to_owned()),
        ..DurableGearEnhancementTask::default()
    });
    roster.hunters[0].profile.action_state = "configuring_enhancement".to_owned();

    let flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            buildings: test_town_building_state(),
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );

    assert!(flow.hunter_roster.hunters[0]
        .hunt
        .gear_enhancement
        .is_none());
    assert_eq!(flow.hunter_roster.hunters[0].profile.action_state, "idle");
    assert_eq!(
        flow.hunter_roster.hunters[0].profile.animation_name,
        "hunter_stay"
    );
}

#[test]
fn legacy_enhancement_task_and_orphaned_action_are_released_on_restore() {
    let mut roster = operational_migration_roster();
    let task = serde_json::json!({
        "building_instance_id": "forge-legacy",
        "status": "waiting_for_interaction",
        "interaction_x": 1,
        "interaction_y": 2
    });
    roster.hunters[0].hunt.gear_enhancement =
        Some(serde_json::from_value(task).expect("legacy task shape remains readable"));
    roster.hunters[0].profile.action_state = "waiting_for_enhancement_interaction".to_owned();
    roster.hunters[1].profile.action_state = "configuring_enhancement".to_owned();

    let flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            buildings: test_town_building_state(),
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );

    assert!(flow.hunter_roster.hunters[0]
        .hunt
        .gear_enhancement
        .is_none());
    assert!(flow.hunter_roster.hunters[1]
        .hunt
        .gear_enhancement
        .is_none());
    assert_eq!(flow.hunter_roster.hunters[0].profile.action_state, "idle");
    assert_eq!(flow.hunter_roster.hunters[1].profile.action_state, "idle");
}

#[test]
fn alchemist_crafts_and_sells_catalog_potion_at_recovered_price() {
    let product_id = "recipe:consumable:0:level:0";
    let mut aggregate = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        },
        buildings: test_town_building_state(),
        hunter_roster: operational_migration_roster(),
        ..DurablePlayerAggregate::default()
    };
    aggregate
        .buildings
        .material_stocks
        .push(DurableMaterialStock {
            id: "material:139".to_owned(),
            town_quantity: 3,
            hunter_quantity: 0,
            requested: 0,
            unit_price: 1,
        });
    let mut content = (*test_authoritative_building_content()).clone();
    content.gameplay.products.insert(
        product_id.to_owned(),
        EconomyProductDefinition {
            product_id: product_id.to_owned(),
            building_id: Some(BaseBuildingId::parse("build_14").unwrap()),
            duration_ms: None,
            exact_mutation_ready: false,
            inputs: vec![EconomyAmount {
                resource_id: "material:139".to_owned(),
                quantity: 3,
            }],
            outputs: vec![EconomyAmount {
                resource_id: "consumable:0".to_owned(),
                quantity: 1,
            }],
            sale_price: Vec::new(),
            service: None,
            conversion_options: Vec::new(),
            random_output: None,
        },
    );
    let mut flow =
        OriginalFlowSession::from_aggregate_with_content(aggregate, 7, Arc::new(content));
    let alchemist_id = building_instance_id(&flow, "build_14");
    let crafted = flow
        .handle_command(ClientCommand::CraftShopItem {
            instance_id: alchemist_id,
            recipe_id: product_id.to_owned(),
            material_id: None,
            quantity: 1,
        })
        .unwrap();
    assert!(matches!(
        crafted.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    let potion_shop_id = building_instance_id(&flow, "build_11");
    assert_eq!(
        flow.buildings
            .product_stocks
            .iter()
            .find(|stock| stock.building_instance_id == potion_shop_id
                && stock.product_id == product_id)
            .map(|stock| stock.quantity),
        Some(1)
    );
    let potion_row = flow
        .snapshot()
        .village
        .building_system
        .recipes
        .into_iter()
        .find(|recipe| recipe.id == product_id && recipe.shop_id == "build_11")
        .expect("potion shop row");
    assert_eq!(potion_row.stock, 1);
    assert_eq!(potion_row.sale_price, 68);

    let town_gold_before = flow.buildings.town_gold;
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
    let equipment_purchase_count_before = flow.buildings.hunter_equipment_purchases;
    let purchased = flow
        .handle_command(ClientCommand::PurchaseShopItem {
            hunter_id: 1,
            shop_id: "build_11".to_owned(),
            product_id: product_id.to_owned(),
        })
        .unwrap();
    assert!(matches!(
        purchased.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(
        flow.buildings
            .product_stocks
            .iter()
            .find(|stock| stock.building_instance_id == potion_shop_id
                && stock.product_id == product_id)
            .map(|stock| stock.quantity),
        Some(0)
    );
    assert_eq!(flow.buildings.town_gold, town_gold_before + 68);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before - 68);
    assert_eq!(
        flow.buildings.hunter_equipment_purchases,
        equipment_purchase_count_before
    );
    assert!(flow.snapshot().hunter_roster.active_hunters[0]
        .gear_enhancements
        .is_empty());
    assert_eq!(
        flow.hunter_roster.hunters[0].owned_items[0].product_id,
        product_id
    );
}

#[test]
fn jeweler_crafts_accessories_into_accessory_shop_stock() {
    let product_id = "recipe:ring:0:rating:0";
    let mut flow = gear_flow_for_building(product_id, 80, "build_21");
    let jeweler_id = building_instance_id(&flow, "build_21");

    let crafted = flow
        .handle_command(ClientCommand::CraftShopItem {
            instance_id: jeweler_id,
            recipe_id: product_id.to_owned(),
            material_id: None,
            quantity: 2,
        })
        .unwrap();
    assert!(matches!(
        crafted.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("gear_creation_evidence_unresolved")
    ));
    assert!(flow.buildings.product_stocks.is_empty());
}

#[test]
fn blacksmith_routes_wearable_armor_to_armor_shop_and_enforces_difficulty_levels() {
    // Helmet 10 belongs to difficulty group 2. Its quality/rating is not
    // the building-level gate.
    let product_id = "recipe:helmet:10:rating:1";
    let mut flow = gear_flow(product_id, 90);
    let blacksmith_id = building_instance_id(&flow, "build_10");
    let before = flow.buildings.clone();

    let locked = flow
        .handle_command(ClientCommand::CraftShopItem {
            instance_id: blacksmith_id.clone(),
            recipe_id: product_id.to_owned(),
            material_id: None,
            quantity: 1,
        })
        .unwrap();
    assert!(matches!(
        locked.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("product_level_locked")
    ));
    assert_eq!(flow.buildings, before);

    flow.buildings
        .buildings
        .iter_mut()
        .find(|building| building.instance_id == blacksmith_id)
        .unwrap()
        .level = 2;
    let crafted = flow
        .handle_command(ClientCommand::CraftShopItem {
            instance_id: building_instance_id(&flow, "build_10"),
            recipe_id: product_id.to_owned(),
            material_id: None,
            quantity: 1,
        })
        .unwrap();
    assert!(matches!(
        crafted.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("gear_creation_evidence_unresolved")
    ));
}

fn infirmary_flow(roster_resolved: bool) -> OriginalFlowSession {
    let mut aggregate = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        },
        buildings: test_town_building_state(),
        hunter_roster: DurableHunterRosterState {
            roster_resolved,
            wallets_resolved: roster_resolved,
            hunters: (1..=5)
                .map(|hunter_id| DurableHunterState {
                    hunter_id,
                    gold: 1_000,
                    current_hp: 100,
                    max_hp: 1_000,
                    stamina: HunterServiceGauge {
                        current: 100,
                        maximum: 1_000,
                    },
                    satiety: HunterServiceGauge {
                        current: 100,
                        maximum: 1_000,
                    },
                    mood: HunterServiceGauge {
                        current: 100,
                        maximum: 1_000,
                    },
                    profile: DurableHunterProfile::migration_default(hunter_id),
                    hunt: Default::default(),
                    runtime: Default::default(),
                    owned_items: Vec::new(),
                })
                .collect(),
            ..DurableHunterRosterState::default()
        },
        product_services: DurableProductServiceState { visits: Vec::new() },
        ..DurablePlayerAggregate::default()
    };
    let infirmary_instance_id = aggregate
        .buildings
        .buildings
        .iter()
        .find(|building| building.id == "build_12")
        .expect("test infirmary")
        .instance_id
        .clone();
    aggregate
        .buildings
        .product_stocks
        .push(DurableProductStock {
            building_instance_id: infirmary_instance_id,
            product_id: TEST_BANDAGE_ID.to_owned(),
            quantity: 5,
        });

    let mut content = (*test_authoritative_building_content()).clone();
    content.gameplay.products.insert(
        TEST_BANDAGE_ID.to_owned(),
        EconomyProductDefinition {
            product_id: TEST_BANDAGE_ID.to_owned(),
            building_id: Some(BaseBuildingId::parse("build_12").expect("infirmary id")),
            duration_ms: Some(600),
            exact_mutation_ready: false,
            inputs: Vec::new(),
            outputs: Vec::new(),
            sale_price: Vec::new(),
            service: Some(EconomyProductService {
                source_type: 0,
                required_level: 0,
                service_time_ms: 600,
                effect_value: 250,
                use_money: 90,
                completion_counts: vec![1, 2, 10],
                required_cash_count: 3,
                cash_completion_count: 1,
                required_elemental_count: 150,
                elemental_completion_count: 1,
            }),
            conversion_options: Vec::new(),
            random_output: None,
        },
    );
    OriginalFlowSession::from_aggregate_with_content(aggregate, 7, Arc::new(content))
}

fn infirmary_instance_id(flow: &OriginalFlowSession) -> String {
    flow.buildings
        .buildings
        .iter()
        .find(|building| building.id == "build_12")
        .expect("test infirmary")
        .instance_id
        .clone()
}

fn add_test_service_product(
    flow: &mut OriginalFlowSession,
    building_id: &str,
    product_id: &str,
) -> String {
    let instance_id = flow
        .buildings
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .expect("test service building")
        .instance_id
        .clone();
    flow.buildings.product_stocks.push(DurableProductStock {
        building_instance_id: instance_id.clone(),
        product_id: product_id.to_owned(),
        quantity: 5,
    });
    Arc::make_mut(&mut flow.building_content)
        .gameplay
        .products
        .insert(
            product_id.to_owned(),
            EconomyProductDefinition {
                product_id: product_id.to_owned(),
                building_id: Some(BaseBuildingId::parse(building_id).expect("service building id")),
                duration_ms: Some(600),
                exact_mutation_ready: false,
                inputs: Vec::new(),
                outputs: Vec::new(),
                sale_price: Vec::new(),
                service: Some(EconomyProductService {
                    source_type: 0,
                    required_level: 0,
                    service_time_ms: 600,
                    effect_value: 250,
                    use_money: 90,
                    completion_counts: vec![1, 10],
                    required_cash_count: 3,
                    cash_completion_count: 1,
                    required_elemental_count: 150,
                    elemental_completion_count: 1,
                }),
                conversion_options: Vec::new(),
                random_output: None,
            },
        );
    instance_id
}

#[test]
fn infirmary_fails_closed_when_hunter_roster_is_unresolved() {
    let mut flow = infirmary_flow(false);
    let instance_id = infirmary_instance_id(&flow);
    let stock_before = flow.buildings.product_stocks[0].quantity;
    let result = flow
        .handle_command(ClientCommand::StartInfirmaryTreatment {
            instance_id,
            hunter_id: 1,
            product_id: TEST_BANDAGE_ID.to_owned(),
        })
        .expect("binding-blocked result");

    assert!(matches!(
        result.message,
        ServerMessage::BindingBlocked { .. }
    ));
    assert!(!result.durable_state_changed);
    assert_eq!(flow.buildings.product_stocks[0].quantity, stock_before);
    let snapshot = flow.infirmary_snapshot();
    assert!(!snapshot.roster_resolved);
    assert_eq!(
        snapshot.blockers,
        vec![
            "hunter_roster_binding",
            "hunter_health_state_binding",
            "hunter_wallet_state_binding",
        ]
    );
    assert!(snapshot.active.is_empty());
}

#[test]
fn infirmary_consumes_stock_then_applies_healing_and_payment_on_completion() {
    let mut flow = infirmary_flow(true);
    let instance_id = infirmary_instance_id(&flow);
    let gold_before = flow.buildings.town_gold;
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
    let accepted = flow
        .handle_command(ClientCommand::StartInfirmaryTreatment {
            instance_id,
            hunter_id: 1,
            product_id: TEST_BANDAGE_ID.to_owned(),
        })
        .expect("treatment result");

    assert!(matches!(
        accepted.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert!(accepted.durable_state_changed);
    assert_eq!(flow.buildings.product_stocks[0].quantity, 4);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before - 90);
    let started = flow.infirmary_snapshot();
    assert_eq!(started.slots, 3);
    assert_eq!(started.available_slots, 2);
    assert_eq!(started.active[0].remaining_ms, 600);
    assert_eq!(started.active[0].effect_value, 250);
    assert_eq!(started.active[0].payment_gold, 90);
    assert_eq!(started.hunters[0].treatment_state, "treating");

    flow.advance_visual_tick();
    flow.advance_visual_tick();
    assert_eq!(flow.infirmary_snapshot().active[0].remaining_ms, 200);
    assert_eq!(flow.hunter_roster.hunters[0].current_hp, 100);
    assert_eq!(flow.buildings.town_gold, gold_before);

    flow.advance_visual_tick();
    assert!(flow.infirmary_snapshot().active.is_empty());
    assert_eq!(flow.hunter_roster.hunters[0].current_hp, 350);
    assert_eq!(flow.buildings.town_gold, gold_before + 90);
    assert_eq!(
        flow.buildings.town_gold + flow.hunter_roster.hunters[0].gold,
        gold_before + hunter_gold_before
    );
    assert_eq!(flow.infirmary_snapshot().hunters[0].treatment_state, "idle");
}

#[test]
fn product_service_rejects_insufficient_hunter_gold_without_consuming_stock() {
    let mut flow = infirmary_flow(true);
    let instance_id = infirmary_instance_id(&flow);
    flow.hunter_roster.hunters[0].gold = 89;

    let result = flow
        .handle_command(ClientCommand::StartBuildingService {
            instance_id,
            hunter_id: 1,
            product_id: TEST_BANDAGE_ID.to_owned(),
        })
        .expect("service result");

    assert!(matches!(
        result.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("insufficient_hunter_gold")
    ));
    assert_eq!(flow.buildings.product_stocks[0].quantity, 5);
    assert_eq!(flow.hunter_roster.hunters[0].gold, 89);
    assert!(flow.product_services.visits.is_empty());
}

#[test]
fn inn_restaurant_and_tavern_restore_their_recovered_gauges() {
    for (building_id, product_id, effect_kind) in [
        ("build_9", "product:0", ServiceEffectKind::Stamina),
        ("build_13", "product:10", ServiceEffectKind::Satiety),
        ("build_19", "product:15", ServiceEffectKind::Mood),
    ] {
        let mut flow = infirmary_flow(true);
        let instance_id = add_test_service_product(&mut flow, building_id, product_id);
        let town_gold_before = flow.buildings.town_gold;

        let result = flow
            .handle_command(ClientCommand::StartBuildingService {
                instance_id,
                hunter_id: 1,
                product_id: product_id.to_owned(),
            })
            .expect("service result");
        assert!(matches!(
            result.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(flow.hunter_roster.hunters[0].gold, 910);
        assert_eq!(
            hunter_service_gauge(&flow.hunter_roster.hunters[0], effect_kind).current,
            100
        );

        for _ in 0..3 {
            flow.advance_visual_tick();
        }
        assert_eq!(
            hunter_service_gauge(&flow.hunter_roster.hunters[0], effect_kind).current,
            350
        );
        assert_eq!(flow.buildings.town_gold, town_gold_before + 90);
        assert!(flow.product_services.visits.is_empty());
    }
}

#[test]
fn infirmary_enforces_slots_per_building_instance_and_rejects_unknown_hunters() {
    let mut flow = infirmary_flow(true);
    let first_instance_id = infirmary_instance_id(&flow);
    let mut second = flow
        .buildings
        .buildings
        .iter()
        .find(|building| building.id == "build_12")
        .expect("test infirmary")
        .clone();
    second.instance_id = Uuid::from_u128(1200).to_string();
    second.grid_x = 24;
    flow.buildings.buildings.push(second.clone());
    flow.buildings.product_stocks.push(DurableProductStock {
        building_instance_id: second.instance_id.clone(),
        product_id: TEST_BANDAGE_ID.to_owned(),
        quantity: 2,
    });

    for hunter_id in 1..=3 {
        let result = flow
            .handle_command(ClientCommand::StartInfirmaryTreatment {
                instance_id: first_instance_id.clone(),
                hunter_id,
                product_id: TEST_BANDAGE_ID.to_owned(),
            })
            .expect("treatment result");
        assert!(matches!(
            result.message,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
    }
    let full = flow
        .handle_command(ClientCommand::StartInfirmaryTreatment {
            instance_id: first_instance_id,
            hunter_id: 4,
            product_id: TEST_BANDAGE_ID.to_owned(),
        })
        .expect("full result");
    assert!(matches!(
        full.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("service_slots_full")
    ));

    let second_instance_id = second.instance_id.clone();
    let other_instance = flow
        .handle_command(ClientCommand::StartInfirmaryTreatment {
            instance_id: second_instance_id.clone(),
            hunter_id: 4,
            product_id: TEST_BANDAGE_ID.to_owned(),
        })
        .expect("second infirmary result");
    assert!(matches!(
        other_instance.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));

    let unknown = flow
        .handle_command(ClientCommand::StartInfirmaryTreatment {
            instance_id: second_instance_id,
            hunter_id: 999,
            product_id: TEST_BANDAGE_ID.to_owned(),
        })
        .expect("unknown hunter result");
    assert!(matches!(
        unknown.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("hunter_unknown")
    ));
}

#[test]
fn infirmary_protocol_snapshot_exposes_hunters_queue_and_capacity() {
    let flow = infirmary_flow(true);
    let value = serde_json::to_value(flow.snapshot()).expect("serialize world snapshot");
    let infirmary = &value["hunter_roster"]["infirmary"];

    assert_eq!(infirmary["roster_resolved"], true);
    assert_eq!(infirmary["slots"], 3);
    assert_eq!(infirmary["available_slots"], 3);
    assert_eq!(infirmary["hunters"][0]["hunter_id"], 1);
    assert_eq!(infirmary["hunters"][0]["treatment_state"], "idle");
    assert_eq!(infirmary["active"], serde_json::json!([]));
    assert_eq!(infirmary["blockers"], serde_json::json!([]));
}

#[test]
fn infirmary_protocol_decodes_treatment_command() {
    let command: ClientCommand = serde_json::from_value(serde_json::json!({
        "type": "start_infirmary_treatment",
        "instance_id": "infirmary-1",
        "hunter_id": 7,
        "product_id": "product:5"
    }))
    .expect("decode treatment command");

    assert!(matches!(
        command,
        ClientCommand::StartInfirmaryTreatment {
            instance_id,
            hunter_id: 7,
            product_id,
        } if instance_id == "infirmary-1" && product_id == "product:5"
    ));
}

#[test]
fn infirmary_production_ignores_display_capacity_but_consumes_materials() {
    let mut flow = infirmary_flow(true);
    let instance_id = infirmary_instance_id(&flow);
    flow.buildings.material_stocks.push(DurableMaterialStock {
        id: "material:11".to_owned(),
        town_quantity: 20,
        hunter_quantity: 0,
        requested: 0,
        unit_price: 1,
    });
    Arc::make_mut(&mut flow.building_content)
        .gameplay
        .products
        .get_mut(TEST_BANDAGE_ID)
        .expect("test bandage product")
        .service = None;
    Arc::make_mut(&mut flow.building_content)
        .gameplay
        .products
        .get_mut(TEST_BANDAGE_ID)
        .expect("test bandage product")
        .conversion_options = vec![crate::buildings::EconomyConversionOption {
        input_kind: "material".to_owned(),
        input_resource_id: "material:11".to_owned(),
        input_quantity: 1,
        output_stock_quantity: 1,
    }];

    let result = flow
        .handle_command(ClientCommand::CraftShopItem {
            instance_id: instance_id.clone(),
            recipe_id: TEST_BANDAGE_ID.to_owned(),
            material_id: Some("material:11".to_owned()),
            quantity: 10,
        })
        .expect("bandage production result");

    assert!(matches!(
        result.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.buildings.material_stocks[0].town_quantity, 10);
    assert_eq!(
        flow.buildings
            .product_stocks
            .iter()
            .find(|stock| {
                stock.building_instance_id == instance_id && stock.product_id == TEST_BANDAGE_ID
            })
            .expect("bandage stock")
            .quantity,
        15
    );
}

#[test]
fn autonomous_healing_uses_owned_healing_potion_below_ten_percent() {
    let mut flow = infirmary_flow(true);
    let hunter = &mut flow.hunter_roster.hunters[0];
    hunter.current_hp = 90;
    hunter.max_hp = 1_000;
    hunter.hunt.status = "hunting".to_owned();
    hunter.hunt.zone_id = Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned());
    hunter
        .owned_items
        .push(super::super::hunter_roster::DurableHunterOwnedItem {
            product_id: "recipe:consumable:0:level:0".to_owned(),
            quantity: 2,
            ..super::super::hunter_roster::DurableHunterOwnedItem::default()
        });

    flow.apply_autonomous_hunter_healing_policy();

    assert_eq!(flow.hunter_roster.hunters[0].current_hp, 1_000);
    assert_eq!(flow.hunter_roster.hunters[0].owned_items[0].quantity, 1);
    assert_eq!(
        flow.hunter_roster.hunters[0].hunt.zone_id.as_deref(),
        Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID)
    );
    assert_eq!(
        flow.hunter_roster.hunters[0].profile.action_state,
        "using_healing_potion"
    );
}

#[test]
fn autonomous_healing_respects_recovered_potion_cooldown() {
    let mut flow = infirmary_flow(true);
    let hunter = &mut flow.hunter_roster.hunters[0];
    hunter.current_hp = 100;
    hunter.max_hp = 100_000;
    hunter.hunt.status = "hunting".to_owned();
    hunter.hunt.zone_id = Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned());
    hunter
        .owned_items
        .push(super::super::hunter_roster::DurableHunterOwnedItem {
            product_id: "recipe:consumable:0:level:0".to_owned(),
            quantity: 2,
            ..super::super::hunter_roster::DurableHunterOwnedItem::default()
        });

    flow.apply_autonomous_hunter_healing_policy();
    flow.apply_autonomous_hunter_healing_policy();

    assert_eq!(flow.hunter_roster.hunters[0].current_hp, 4_100);
    assert_eq!(flow.hunter_roster.hunters[0].owned_items[0].quantity, 1);
    assert_eq!(
        flow.hunter_roster.hunters[0]
            .hunt
            .healing_potion_cooldown_ms,
        20_000
    );

    let restored = OriginalFlowSession::from_aggregate(flow.durable_state(), 7);
    assert_eq!(
        restored.hunter_roster.hunters[0]
            .hunt
            .healing_potion_cooldown_ms,
        20_000
    );
}

#[test]
fn autonomous_healing_returns_to_infirmary_when_no_potion_is_owned() {
    let mut flow = infirmary_flow(true);
    let hunter = &mut flow.hunter_roster.hunters[0];
    hunter.current_hp = 99;
    hunter.max_hp = 1_000;
    hunter.hunt.status = "hunting".to_owned();
    hunter.hunt.zone_id = Some(super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned());

    flow.apply_autonomous_hunter_healing_policy();

    assert_eq!(flow.hunter_roster.hunters[0].current_hp, 99);
    assert!(flow.hunter_roster.hunters[0].hunt.zone_id.is_none());
    assert_eq!(flow.hunter_roster.hunters[0].hunt.status, "idle");
    assert_eq!(
        flow.hunter_roster.hunters[0].profile.action_state,
        "returning_for_infirmary"
    );
}

#[test]
fn autonomous_healing_routes_low_hp_town_hunter_to_infirmary() {
    let mut flow = infirmary_flow(true);
    let hunter = &mut flow.hunter_roster.hunters[0];
    hunter.current_hp = 99;
    hunter.max_hp = 1_000;
    hunter.hunt.status = "idle".to_owned();
    hunter.hunt.zone_id = None;
    hunter.profile.action_state = "idle".to_owned();

    flow.apply_autonomous_hunter_healing_policy();

    assert_eq!(flow.hunter_roster.hunters[0].hunt.status, "idle");
    assert_eq!(
        flow.hunter_roster.hunters[0].profile.action_state,
        "returning_for_infirmary"
    );
    assert_eq!(
        flow.hunter_roster.hunters[0].profile.animation_name,
        "hunter_walk"
    );
}

#[test]
fn autonomous_healing_walks_to_stocked_infirmary_and_starts_service() {
    let mut flow = infirmary_flow(true);
    let instance_id = infirmary_instance_id(&flow);
    let hunter = &mut flow.hunter_roster.hunters[0];
    hunter.current_hp = 99;
    hunter.max_hp = 1_000;
    hunter.hunt.status = "hunting".to_owned();
    hunter.hunt.zone_id = Some(map_configs()[0].map_id.to_owned());
    hunter.profile.action_state = "hunting".to_owned();
    let hunter_gold_before = hunter.gold;
    let stock_before = flow
        .buildings
        .product_stocks
        .iter()
        .find(|stock| {
            stock.building_instance_id == instance_id && stock.product_id == TEST_BANDAGE_ID
        })
        .unwrap()
        .quantity;
    let agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some(map_configs()[0].map_id.to_owned());
    agent.x = map_configs()[0].bounds.min_x + 120;
    agent.y = map_configs()[0].bounds.min_y + 120;

    for _ in 0..600 {
        flow.advance_simulation_tick();
        if !flow.product_services.visits.is_empty() {
            break;
        }
    }

    assert_eq!(flow.product_services.visits.len(), 1);
    assert_eq!(flow.product_services.visits[0].hunter_id, 1);
    assert_eq!(flow.product_services.visits[0].product_id, TEST_BANDAGE_ID);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before - 90);
    assert_eq!(
        flow.hunter_roster.hunters[0].profile.action_state,
        "serving"
    );
    assert_eq!(
        flow.buildings
            .product_stocks
            .iter()
            .find(|stock| {
                stock.building_instance_id == instance_id && stock.product_id == TEST_BANDAGE_ID
            })
            .unwrap()
            .quantity,
        stock_before - 1
    );

    flow.advance_product_services(600);
    assert_eq!(flow.hunter_roster.hunters[0].current_hp, 349);
    assert_eq!(flow.hunter_roster.hunters[0].profile.action_state, "idle");
}

#[test]
fn out_of_stock_service_keeps_hunter_routed_and_projects_complaint() {
    let mut flow = infirmary_flow(true);
    flow.buildings.product_stocks.clear();
    let hunter = &mut flow.hunter_roster.hunters[0];
    hunter.current_hp = 9;
    hunter.max_hp = 100;
    hunter.hunt.status = "hunting".to_owned();
    hunter.hunt.zone_id = Some(map_configs()[0].map_id.to_owned());
    hunter.profile.action_state = "hunting".to_owned();

    for _ in 0..600 {
        flow.advance_simulation_tick();
        if flow.hunter_roster.hunters[0].profile.action_state == "waiting_for_service" {
            break;
        }
    }

    assert!(matches!(
        flow.hunter_roster.hunters[0].profile.action_state.as_str(),
        "returning_for_infirmary" | "waiting_for_service"
    ));
    assert!(flow.hunter_roster.hunters[0].hunt.zone_id.is_none());
    let entity = flow
        .snapshot()
        .world
        .entities
        .into_iter()
        .find(|entity| entity.descriptor.entity_id == "village-hunter-1")
        .expect("waiting Hunter projection");
    assert!(entity.speech_label.is_some());
}

#[test]
fn hunter_roster_menu_keeps_authoritative_shared_world_focus() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
    flow.handle_command(ClientCommand::CompleteBoot);
    assert_eq!(flow.snapshot().screen, OriginalScreen::Village);

    flow.handle_command(ClientCommand::SelectBottomMenu {
        menu: BottomMenuIntent::Character,
    });
    assert_eq!(flow.snapshot().screen, OriginalScreen::Village);

    flow.handle_command(ClientCommand::EnterField);
    let result = flow
        .handle_command(ClientCommand::SelectBottomMenu {
            menu: BottomMenuIntent::Build,
        })
        .expect("shared-world menu result");
    assert!(matches!(
        result.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.snapshot().screen, OriginalScreen::Field);
}

#[test]
fn normalized_town_template_projects_28_core_bases_and_upgrades_by_instance() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
    flow.handle_command(ClientCommand::CompleteBoot);
    assert_eq!(
        flow.buildings
            .buildings
            .iter()
            .map(|building| building.id.as_str())
            .collect::<Vec<_>>(),
        (1..=28).map(|id| format!("build_{id}")).collect::<Vec<_>>()
    );
    assert!(flow.buildings.buildings.iter().all(|building| {
        building.seeded_by.as_deref() == Some("town-template:default-town-v2")
    }));

    flow.buildings.material_stocks.push(DurableMaterialStock {
        id: "material:11".to_owned(),
        town_quantity: 10,
        hunter_quantity: 0,
        requested: 0,
        unit_price: 0,
    });
    let town_hall_instance_id = flow.buildings.buildings[0].instance_id.clone();
    let result = flow
        .handle_command(ClientCommand::UpgradeBuilding {
            instance_id: town_hall_instance_id,
        })
        .expect("upgrade returns a result");
    assert!(matches!(
        result.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.buildings.buildings.len(), 28);
    assert_eq!(flow.buildings.buildings[0].level, 2);

    let system = &flow.snapshot().village.building_system;
    assert_eq!(system.definitions.len(), 79);
    assert_eq!(system.instances.len(), 28);
    let town_hall = system
        .definitions
        .iter()
        .find(|building| building.id == "build_1")
        .unwrap();
    assert_eq!(town_hall.name, "Town Hall");
    assert_eq!(town_hall.max_level, 17);
    assert_eq!(town_hall.construct_cost, 500);
    let state = system
        .states
        .iter()
        .find(|building| building.id == "build_1")
        .unwrap();
    assert_eq!(state.level, 2);
}

#[test]
fn trading_post_reservation_is_authoritative_and_sale_fails_closed_without_seller() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
    flow.handle_command(ClientCommand::CompleteBoot);
    let town_hall_instance_id = flow
        .buildings
        .buildings
        .iter()
        .find(|building| building.id == "build_1")
        .unwrap()
        .instance_id
        .clone();
    let trading_post_instance_id = flow
        .buildings
        .buildings
        .iter()
        .find(|building| building.id == "build_3")
        .unwrap()
        .instance_id
        .clone();
    assert!(flow
        .snapshot()
        .village
        .building_system
        .material_stocks
        .iter()
        .any(|stock| stock.id == "material:1" && stock.town_quantity == 0));
    let wrong_building = flow
        .handle_command(ClientCommand::SetMaterialRequest {
            instance_id: town_hall_instance_id,
            material_id: "material:1".to_owned(),
            quantity: 3,
        })
        .unwrap();
    assert!(matches!(
        wrong_building.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("building_capability_mismatch")
    ));
    let quantity_request = flow
        .handle_command(ClientCommand::SetMaterialRequest {
            instance_id: trading_post_instance_id.clone(),
            material_id: "material:1".to_owned(),
            quantity: 2,
        })
        .unwrap();
    assert!(matches!(
        quantity_request.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(
        flow.buildings
            .material_stocks
            .iter()
            .find(|stock| stock.id == "material:1")
            .unwrap()
            .requested,
        2
    );
    let locked_difficulty = flow
        .handle_command(ClientCommand::SetMaterialRequest {
            instance_id: trading_post_instance_id.clone(),
            material_id: "material:2".to_owned(),
            quantity: ACTIVE_MATERIAL_REQUEST,
        })
        .unwrap();
    assert!(matches!(
        locked_difficulty.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("material_difficulty_locked")
    ));
    flow.handle_command(ClientCommand::SetMaterialRequest {
        instance_id: trading_post_instance_id.clone(),
        material_id: "material:1".to_owned(),
        quantity: ACTIVE_MATERIAL_REQUEST,
    });
    flow.handle_command(ClientCommand::CancelMaterialRequest {
        instance_id: trading_post_instance_id.clone(),
        material_id: "material:1".to_owned(),
    });
    let material_index = flow
        .buildings
        .material_stocks
        .iter()
        .position(|stock| stock.id == "material:1")
        .unwrap();
    assert_eq!(flow.buildings.material_stocks[material_index].requested, 0);
    flow.handle_command(ClientCommand::SetMaterialRequest {
        instance_id: trading_post_instance_id,
        material_id: "material:1".to_owned(),
        quantity: ACTIVE_MATERIAL_REQUEST,
    });
    flow.buildings.material_stocks[material_index].hunter_quantity = 5;
    flow.handle_command(ClientCommand::EnterField);
    flow.handle_command(ClientCommand::NavigateBack);
    assert_eq!(flow.buildings.town_gold, 1_500);
    assert_eq!(
        flow.buildings.material_stocks[material_index].town_quantity,
        0
    );
    assert_eq!(
        flow.buildings.material_stocks[material_index].hunter_quantity,
        5
    );
    assert_eq!(
        flow.buildings.material_stocks[material_index].requested,
        ACTIVE_MATERIAL_REQUEST
    );
    assert!(flow.buildings.trade_settlements.is_empty());
    flow.settle_returning_hunters();
    assert!(flow.buildings.trade_settlements.is_empty());
}

#[test]
fn shared_field_focus_keeps_building_and_economy_commands_available() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
    flow.handle_command(ClientCommand::CompleteBoot);
    flow.handle_command(ClientCommand::EnterField);
    assert!(flow.shared_world_active());
    assert!(flow.advance_visual_clock_by(100));

    let rejection_reason = |message: ServerMessage| match message {
        ServerMessage::IntentResult {
            accepted: false,
            reason: Some(reason),
            ..
        } => reason,
        other => panic!("expected rejection, got {other:?}"),
    };
    assert_eq!(
        rejection_reason(flow.construct_building("build_2")),
        "placement_required"
    );
    assert_eq!(
        rejection_reason(flow.upgrade_building("missing-instance")),
        "building_instance_unknown"
    );
    assert_eq!(
        rejection_reason(flow.move_building("missing-instance", 0, 0)),
        "building_instance_unknown"
    );
    assert_eq!(
        rejection_reason(flow.craft_shop_item(
            Uuid::nil(),
            "missing-instance",
            "missing-recipe",
            None,
            1,
        )),
        "building_instance_unknown"
    );
    assert_eq!(
        rejection_reason(flow.start_product_service("missing-instance", 1, "missing-product",)),
        "service_instance_unknown"
    );
    assert_eq!(
        rejection_reason(flow.purchase_shop_item(
            Uuid::nil(),
            1,
            "missing-building",
            "missing-product",
        )),
        "building_unknown"
    );
    assert_eq!(
        rejection_reason(flow.start_hunter_enhancement(Uuid::nil(), u32::MAX)),
        "hunter_unknown"
    );
    assert_eq!(
        rejection_reason(flow.enhance_hunter_gear(
            Uuid::nil(),
            1,
            Uuid::nil(),
            "invalid-mode",
            &[],
        )),
        "enhancement_mode_invalid"
    );
    let trading_post_instance_id = flow
        .buildings
        .buildings
        .iter()
        .find(|building| building.id == "build_3")
        .expect("default town Trading Post")
        .instance_id
        .clone();

    let request = flow
        .handle_command(ClientCommand::SetMaterialRequest {
            instance_id: trading_post_instance_id.clone(),
            material_id: "material:1".to_owned(),
            quantity: 100,
        })
        .expect("request result");
    assert!(matches!(
        request.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert!(request.durable_state_changed);
    assert_eq!(
        flow.buildings
            .material_stocks
            .iter()
            .find(|stock| stock.id == "material:1")
            .expect("requested material")
            .requested,
        100
    );

    let cancel = flow
        .handle_command(ClientCommand::CancelMaterialRequest {
            instance_id: trading_post_instance_id,
            material_id: "material:1".to_owned(),
        })
        .expect("cancel result");
    assert!(matches!(
        cancel.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert!(cancel.durable_state_changed);
}

#[test]
fn session_constructor_does_not_repair_or_seed_building_state() {
    let mut aggregate = DurablePlayerAggregate::default();
    aggregate.buildings.buildings.push(DurableBuilding {
        instance_id: Uuid::from_u128(99).to_string(),
        id: "build_4".to_owned(),
        equipped_skin_id: None,
        level: 1,
        uses: 0,
        grid_x: 0,
        grid_y: 10,
        seeded_by: None,
    });
    let restored = OriginalFlowSession::from_aggregate(aggregate, 7);
    assert_eq!(restored.buildings.buildings.len(), 1);
    assert_eq!(restored.buildings.buildings[0].id, "build_4");
    assert_eq!(restored.buildings.buildings[0].grid_y, 10);
}

#[test]
fn field_entry_projects_visual_entities_without_enabling_gameplay() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    let result = flow
        .handle_command(ClientCommand::EnterField)
        .expect("field intent returns a result");
    assert!(result.durable_state_changed);
    let ServerMessage::IntentResult {
        accepted, snapshot, ..
    } = result.message
    else {
        panic!("field navigation should be accepted");
    };
    assert!(accepted);
    assert_eq!(snapshot.screen, OriginalScreen::Field);
    assert_eq!(snapshot.world.mode, WorldMode::Field);
    assert_eq!(
        snapshot.world.authority_scope,
        "server_authoritative_simulation"
    );
    assert_eq!(
        snapshot
            .world
            .entities
            .iter()
            .filter(|entity| entity.descriptor.kind == WorldEntityKind::Hunter)
            .count(),
        8
    );
    assert_eq!(
        snapshot
            .world
            .entities
            .iter()
            .filter(|entity| entity.descriptor.kind == WorldEntityKind::Monster)
            .count(),
        9
    );
    assert!(snapshot.field.visual_projection_runnable);
    assert!(snapshot.field.gameplay_runnable);
    assert!(snapshot.field.blockers.is_empty());
    assert!(snapshot
        .world
        .entities
        .iter()
        .all(|entity| !matches!(entity.animation.as_str(), "atk" | "die" | "dying")));
    for region in ["map_new01", "background_08", "background_11"] {
        assert!(snapshot.world.entities.iter().any(|entity| entity
            .descriptor
            .entity_id
            .starts_with(&format!("monster-{region}-"))));
    }
    flow.advance_simulation_tick();
    let live = flow.snapshot();
    assert!(live
        .world
        .entities
        .iter()
        .any(|entity| entity.descriptor.entity_id == "village-hunter-1"));
    assert!(live
        .world
        .entities
        .iter()
        .all(|entity| entity.descriptor.entity_id != "field-hunter-01"));
}

#[test]
fn village_projection_uses_authoritative_health_for_combat_actors_only() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
        screen: OriginalScreen::Village,
        boot_completed: true,
    });
    flow.hunter_roster.hunters.push(DurableHunterState {
        hunter_id: 7,
        gold: 0,
        current_hp: 19,
        max_hp: 250,
        stamina: HunterServiceGauge::default(),
        satiety: HunterServiceGauge::default(),
        mood: HunterServiceGauge::default(),
        profile: DurableHunterProfile::migration_default(7),
        runtime: Default::default(),
        hunt: Default::default(),
        owned_items: Vec::new(),
    });

    let world = flow.snapshot().world;
    let hunter = world
        .entities
        .iter()
        .find(|entity| entity.descriptor.entity_id == "village-hunter-7")
        .expect("durable Hunter is projected into the village");
    assert_eq!(
        (hunter.current_hp, hunter.maximum_hp),
        (Some(19), Some(250))
    );

    assert!(world
        .entities
        .iter()
        .filter(|entity| entity.descriptor.kind == WorldEntityKind::Monster)
        .all(|entity| entity.current_hp.is_some() && entity.maximum_hp.is_some()));

    let npc = world
        .entities
        .iter()
        .find(|entity| entity.descriptor.kind == WorldEntityKind::Npc)
        .expect("village NPC remains projected");
    assert_eq!((npc.current_hp, npc.maximum_hp), (None, None));
}

#[test]
fn entity_selection_is_authoritative_and_not_persisted() {
    let state = OriginalFlowPlayerState {
        screen: OriginalScreen::Village,
        boot_completed: true,
    };
    let mut flow = OriginalFlowSession::from_state(state.clone());
    let selected = flow
        .handle_command(ClientCommand::SelectEntity {
            entity_id: "village-npc-01".to_owned(),
        })
        .expect("selection result");
    assert!(!selected.durable_state_changed);
    assert!(matches!(
        selected.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(
        flow.snapshot().world.selected_entity_id,
        Some("village-npc-01".to_owned())
    );
    assert_eq!(flow.state(), &state);

    let rejected = flow
        .handle_command(ClientCommand::SelectEntity {
            entity_id: "client-invented-entity".to_owned(),
        })
        .expect("selection rejection");
    assert!(!rejected.durable_state_changed);
    assert!(matches!(
        rejected.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    assert_eq!(
        flow.snapshot().world.selected_entity_id,
        Some("village-npc-01".to_owned())
    );
}

#[test]
fn monsters_project_health_without_becoming_selectable_entities() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.advance_simulation_tick();
    let monster = flow
        .snapshot()
        .world
        .entities
        .into_iter()
        .find(|entity| entity.descriptor.kind == WorldEntityKind::Monster)
        .expect("monster remains in the visible-world projection");

    assert!(!monster.selectable);
    assert!(monster.current_hp.is_some());
    assert!(monster.maximum_hp.is_some());

    let rejected = flow
        .handle_command(ClientCommand::SelectEntity {
            entity_id: monster.descriptor.entity_id,
        })
        .expect("selection rejection");
    assert!(matches!(
        rejected.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    assert_eq!(flow.snapshot().world.selected_entity_id, None);
}

#[test]
fn back_from_field_persists_only_the_village_screen() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
        screen: OriginalScreen::Field,
        boot_completed: true,
    });
    flow.handle_command(ClientCommand::NavigateBack);
    assert_eq!(flow.state().screen, OriginalScreen::Village);
    assert_eq!(flow.snapshot().world.mode, WorldMode::Village);
}

#[test]
fn fixed_simulation_tick_moves_entities_without_changing_navigation_state() {
    let state = OriginalFlowPlayerState {
        screen: OriginalScreen::Village,
        boot_completed: true,
    };
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: state.clone(),
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    let before = flow.snapshot();
    let after = flow
        .advance_simulation_tick()
        .expect("active world tick")
        .world;
    assert_eq!(after.visual_tick, before.world.visual_tick + 1);
    assert_ne!(after.entities, before.world.entities);
    assert_eq!(flow.state(), &state);
}

#[test]
fn critical_service_need_rejects_hunt_assignment_without_mutation() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.advance_simulation_tick();
    let hunter_id = flow.hunter_roster.hunters[0].hunter_id;
    let original_gold = flow.hunter_roster.hunters[0].gold;
    let payment_gold = 17;
    flow.hunter_roster.hunters[0].gold = original_gold - payment_gold;
    flow.hunter_roster.hunters[0].current_hp = 1;
    flow.hunter_roster.hunters[0].max_hp = 100;
    flow.hunter_roster.hunters[0].hunt.status = "returning_for_infirmary".to_owned();
    flow.hunter_roster.hunters[0].hunt.gear_enhancement =
        Some(DurableGearEnhancementTask::default());
    let building_instance_id = "service-instance-priority-test".to_owned();
    let product_id = "service-product-priority-test".to_owned();
    flow.buildings.product_stocks.push(DurableProductStock {
        building_instance_id: building_instance_id.clone(),
        product_id: product_id.clone(),
        quantity: 0,
    });
    flow.product_services
        .visits
        .push(DurableProductServiceVisit {
            hunter_id,
            building_instance_id: building_instance_id.clone(),
            building_id: "build_12".to_owned(),
            product_id: product_id.clone(),
            effect_kind: ServiceEffectKind::Hp,
            remaining_ms: 1_000,
            effect_value: 100,
            payment_gold,
        });
    let agent_before = flow
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == hunter_id)
        .expect("Hunter agent initialized")
        .clone();
    let command_id = Uuid::from_u128(0xface);

    let result = flow
        .handle_command_with_id(
            ClientCommand::AssignHunterHunt {
                hunter_id,
                zone_id: super::super::hunter_roster::ORDINARY_HUNT_REGION_IDS[0].to_owned(),
            },
            command_id,
        )
        .expect("hunt assignment response");

    assert!(matches!(
        result.message,
        ServerMessage::IntentResult { accepted: false, reason: Some(ref reason), .. }
            if reason == "hunter_needs_service"
    ));
    assert_eq!(flow.product_services.visits.len(), 1);
    assert_eq!(
        flow.hunter_roster.hunters[0].gold,
        original_gold - payment_gold
    );
    assert_eq!(
        flow.buildings
            .product_stocks
            .iter()
            .find(|stock| {
                stock.building_instance_id == building_instance_id && stock.product_id == product_id
            })
            .expect("refunded service stock")
            .quantity,
        0
    );
    assert!(flow.hunter_roster.hunters[0]
        .hunt
        .gear_enhancement
        .is_some());
    assert_eq!(
        flow.hunter_roster.hunters[0].hunt.status,
        "returning_for_infirmary"
    );
    let assigned_agent = flow
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == hunter_id)
        .expect("assigned Hunter agent");
    assert_eq!(assigned_agent.action_state, agent_before.action_state);
    assert_eq!(assigned_agent.region_id, agent_before.region_id);

    assert!(flow.hunter_roster.hunters[0].hunt.zone_id.is_none());
}

#[test]
fn assigned_hunter_routes_never_cross_authoritative_building_footprints() {
    const ACTOR_CLEARANCE: i32 = 14;
    for config in map_configs() {
        let mut flow = OriginalFlowSession::from_aggregate(
            DurablePlayerAggregate {
                navigation: OriginalFlowPlayerState {
                    screen: OriginalScreen::Village,
                    boot_completed: true,
                },
                buildings: test_town_building_state(),
                hunter_roster: operational_migration_roster(),
                ..DurablePlayerAggregate::default()
            },
            7,
        );
        flow.handle_command(ClientCommand::AssignHunterHunt {
            hunter_id: 1,
            zone_id: config.map_id.to_owned(),
        });
        let obstacles =
            town_navigation_obstacles(&flow.buildings.buildings, &flow.building_content.catalog);
        let mut entered_field = false;

        for _ in 0..400 {
            flow.advance_simulation_tick().expect("active village tick");
            let hunter = flow
                .monster_world
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == 1)
                .unwrap();
            for obstacle in &obstacles {
                assert!(
                    hunter.x < obstacle.min_x - ACTOR_CLEARANCE
                        || hunter.x > obstacle.max_x + ACTOR_CLEARANCE
                        || hunter.y < obstacle.min_y - ACTOR_CLEARANCE
                        || hunter.y > obstacle.max_y + ACTOR_CLEARANCE,
                    "hunter {} at ({}, {}) overlaps obstacle {:?}",
                    hunter.hunter_id,
                    hunter.x,
                    hunter.y,
                    obstacle,
                );
            }
            if hunter.x >= config.bounds.min_x
                && hunter.x <= config.bounds.max_x
                && hunter.y >= config.bounds.min_y
                && hunter.y <= config.bounds.max_y
            {
                entered_field = true;
                break;
            }
        }
        assert!(
            entered_field,
            "Hunter did not reach {} without crossing a building; final position: {:?}",
            config.map_id,
            flow.monster_world
                .hunters
                .iter()
                .find(|hunter| hunter.hunter_id == 1)
                .map(|hunter| (hunter.x, hunter.y, hunter.entry_stage)),
        );
    }
}

#[test]
fn village_projects_only_active_hunters_in_deterministic_non_overlapping_lanes() {
    let flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    let snapshot = flow.snapshot();
    let hunters = snapshot
        .world
        .entities
        .iter()
        .filter(|entity| entity.descriptor.kind == WorldEntityKind::Hunter)
        .collect::<Vec<_>>();

    assert_eq!(hunters.len(), MAX_ACTIVE_TOWN_HUNTERS);
    assert!(hunters
        .iter()
        .all(|entity| entity.descriptor.entity_id != "village-hunter-9"));
    assert_eq!(
        hunters
            .iter()
            .map(|entity| (entity.x, entity.y))
            .collect::<HashSet<_>>()
            .len(),
        MAX_ACTIVE_TOWN_HUNTERS
    );
    assert!(hunters.iter().all(|entity| matches!(
        (entity.action_state, entity.animation.as_str()),
        (WorldEntityActionState::Idle, "hunter_stay")
            | (WorldEntityActionState::Walking, "hunter_walk")
    )));
}

#[test]
fn town_roaming_hunter_projects_walking_motion_for_client_interpolation() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    world.tick(&mut roster);
    let mut agent = world.hunters[0].clone();
    agent.action_state = crate::simulation::HunterActionState::TownIdle;
    agent.animation = "hunter_walk".to_owned();

    let walking = hunter_visual_entity(&agent, 100, 100);
    assert_eq!(walking.action_state, WorldEntityActionState::Walking);

    agent.animation = "hunter_stay".to_owned();
    let paused = hunter_visual_entity(&agent, 100, 100);
    assert_eq!(paused.action_state, WorldEntityActionState::Idle);
}

#[test]
fn town_roaming_hunters_stay_in_safe_floor_and_clear_buildings() {
    const ANCHOR_CLEARANCE: i32 = 14;
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            buildings: test_town_building_state(),
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    let obstacles =
        town_navigation_obstacles(&flow.buildings.buildings, &flow.building_content.catalog);
    for (x, y) in TOWN_ROAM_ANCHORS {
        for obstacle in &obstacles {
            assert!(
                x < obstacle.min_x - ANCHOR_CLEARANCE
                    || x > obstacle.max_x + ANCHOR_CLEARANCE
                    || y < obstacle.min_y - ANCHOR_CLEARANCE
                    || y > obstacle.max_y + ANCHOR_CLEARANCE,
                "town anchor ({x}, {y}) overlaps obstacle {obstacle:?}",
            );
        }
    }
    for _ in 0..240 {
        flow.advance_simulation_tick().expect("active village tick");
        for hunter in &flow.monster_world.hunters {
            if hunter.region_id.is_some() {
                continue;
            }
            assert!((TOWN_ROAM_BOUNDS.min_x..=TOWN_ROAM_BOUNDS.max_x).contains(&hunter.x));
            assert!((TOWN_ROAM_BOUNDS.min_y..=TOWN_ROAM_BOUNDS.max_y).contains(&hunter.y));
            for obstacle in &obstacles {
                assert!(
                    hunter.x < obstacle.min_x
                        || hunter.x > obstacle.max_x
                        || hunter.y < obstacle.min_y
                        || hunter.y > obstacle.max_y,
                    "town hunter {} at ({}, {}) overlaps obstacle {:?}",
                    hunter.hunter_id,
                    hunter.x,
                    hunter.y,
                    obstacle,
                );
            }
        }
    }
}

#[test]
fn hunter_info_projects_fixture_equipment_without_claiming_runtime_capture() {
    let flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    let snapshot = flow.snapshot();
    let hunter = &snapshot.hunter_roster.active_hunters[0];
    let equipment = hunter.hunter_info.equipment_slots.as_ref().unwrap();

    assert_eq!(equipment.len(), 4);
    let weapon = equipment
        .iter()
        .find(|slot| slot.slot_id == "weapon")
        .unwrap();
    assert_eq!(
        weapon.required_class_id.as_deref(),
        Some(hunter.class_id.as_str())
    );
    assert_eq!(weapon.evidence_state, "web_rebuild_test_fixture");
    assert_eq!(hunter.runtime_evidence.inventory.value, None);
}

#[test]
fn unresolved_bottom_menu_does_not_change_screen() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
        screen: OriginalScreen::Village,
        boot_completed: true,
    });
    let result = flow
        .handle_command(ClientCommand::SelectBottomMenu {
            menu: BottomMenuIntent::Store,
        })
        .expect("store intent returns a result");
    assert!(!result.durable_state_changed);
    assert!(matches!(
        result.message,
        ServerMessage::BindingBlocked { .. }
    ));
    assert_eq!(flow.snapshot().screen, OriginalScreen::Village);
}

#[test]
fn unresolved_progression_and_economy_intents_never_grant_state() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
        screen: OriginalScreen::Village,
        boot_completed: true,
    });
    let commands = [
        ClientCommand::OpenHunterProgression { hunter_id: 1 },
        ClientCommand::ClaimQuestReward {
            quest_id: "quest-1".to_owned(),
        },
        ClientCommand::OpenShop {
            shop_id: "main".to_owned(),
        },
        ClientCommand::PurchaseShopItem {
            hunter_id: 1,
            shop_id: "main".to_owned(),
            product_id: "product-1".to_owned(),
        },
        ClientCommand::ClaimMail {
            mail_id: "mail-1".to_owned(),
        },
        ClientCommand::ClaimRewardedAd {
            placement: "unknown".to_owned(),
        },
        ClientCommand::StartTopupPurchase {
            product_id: "unknown".to_owned(),
        },
    ];

    for command in commands {
        let result = flow.handle_command(command).expect("intent result");
        assert!(!result.durable_state_changed);
        match &result.message {
            ServerMessage::BindingBlocked { .. }
            | ServerMessage::IntentResult {
                accepted: false, ..
            } => {}
            ServerMessage::IntentResult {
                accepted: true,
                intent,
                ..
            } if intent == "open_hunter_progression" => {}
            _ => panic!("unresolved intent unexpectedly granted state"),
        }
        assert_eq!(flow.state().screen, OriginalScreen::Village);
    }
}

#[test]
fn fixture_combat_is_deterministic_and_restores_from_durable_aggregate() {
    let aggregate = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Field,
            boot_completed: true,
        },
        ..DurablePlayerAggregate::default()
    };
    let mut first = OriginalFlowSession::from_aggregate(aggregate, 7);
    for _ in 0..80 {
        first.advance_simulation_tick().expect("field tick");
    }
    let durable = first.durable_state();
    let restored = OriginalFlowSession::from_aggregate(durable, 7);

    assert_eq!(
        restored.snapshot().migration_fixture_combat.world,
        first.snapshot().migration_fixture_combat.world
    );
    assert_eq!(
        restored.snapshot().migration_fixture_combat.evidence_label,
        "deterministic_migration_fixture_not_legacy_balance"
    );
}

#[test]
fn fixture_equip_command_is_idempotent_across_restore() {
    let mut combat = DurablePlayerState::default();
    combat.inventory.insert(2001, 1);
    let aggregate = DurablePlayerAggregate {
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Field,
            boot_completed: true,
        },
        migration_fixture_combat: combat,
        ..DurablePlayerAggregate::default()
    };
    let command_id = Uuid::from_u128(42);
    let mut first = OriginalFlowSession::from_aggregate(aggregate, 7);
    let accepted = first
        .handle_command_with_id(
            ClientCommand::EquipHunterItem {
                hunter_id: 1,
                item_id: 2001,
            },
            command_id,
        )
        .expect("equip result");
    assert!(accepted.durable_state_changed);
    assert_eq!(accepted.operations.len(), 1);

    let mut restored = OriginalFlowSession::from_aggregate(first.durable_state(), 7);
    let duplicate = restored
        .handle_command_with_id(
            ClientCommand::EquipHunterItem {
                hunter_id: 1,
                item_id: 2001,
            },
            command_id,
        )
        .expect("duplicate equip result");
    assert!(!duplicate.durable_state_changed);
    assert!(duplicate.operations.is_empty());
}

#[test]
fn banish_promotes_fifo_and_is_idempotent_across_restore() {
    let mut roster = DurableHunterRosterState {
        roster_resolved: true,
        wallets_resolved: true,
        ..DurableHunterRosterState::default()
    };
    for hunter_id in 1..=10 {
        roster
            .arrive(DurableHunterState {
                hunter_id,
                gold: 100,
                current_hp: 100,
                max_hp: 100,
                stamina: HunterServiceGauge {
                    current: 100,
                    maximum: 100,
                },
                satiety: HunterServiceGauge {
                    current: 100,
                    maximum: 100,
                },
                mood: HunterServiceGauge {
                    current: 100,
                    maximum: 100,
                },
                profile: DurableHunterProfile::migration_default(hunter_id),
                runtime: Default::default(),
                hunt: Default::default(),
                owned_items: Vec::new(),
            })
            .unwrap();
    }
    let aggregate = DurablePlayerAggregate {
        schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
        navigation: OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        },
        hunter_roster: roster,
        ..DurablePlayerAggregate::default()
    };
    let command_id = Uuid::from_u128(9001);
    let mut flow = OriginalFlowSession::from_aggregate(aggregate, 7);
    let before = flow.snapshot();
    assert!(before
        .world
        .entities
        .iter()
        .any(|entity| entity.descriptor.entity_id == "village-hunter-3"));
    assert!(!before
        .world
        .entities
        .iter()
        .any(|entity| entity.descriptor.entity_id == "village-hunter-9"));
    let accepted = flow
        .handle_command_with_id(ClientCommand::BanishHunter { hunter_id: 3 }, command_id)
        .unwrap();
    assert!(accepted.durable_state_changed);
    assert!(matches!(
        accepted.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    let roster = &accepted_snapshot(&accepted.message).hunter_roster;
    assert_eq!(roster.active_capacity, 8);
    assert_eq!(roster.active_hunters.len(), 8);
    assert_eq!(roster.active_hunters.last().unwrap().hunter_id, 9);
    assert_eq!(roster.waiting_hunters[0].hunter_id, 10);
    let world = &accepted_snapshot(&accepted.message).world;
    assert!(!world
        .entities
        .iter()
        .any(|entity| entity.descriptor.entity_id == "village-hunter-3"));
    assert!(world
        .entities
        .iter()
        .any(|entity| entity.descriptor.entity_id == "village-hunter-9"));
    assert!(!world
        .entities
        .iter()
        .any(|entity| entity.descriptor.entity_id == "village-hunter-10"));

    let mut restored = OriginalFlowSession::from_aggregate(flow.durable_state(), 7);
    let duplicate = restored
        .handle_command_with_id(ClientCommand::BanishHunter { hunter_id: 3 }, command_id)
        .unwrap();
    assert!(!duplicate.durable_state_changed);
    assert!(matches!(
        duplicate.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));

    let not_active = restored
        .handle_command_with_id(
            ClientCommand::BanishHunter { hunter_id: 10 },
            Uuid::from_u128(9002),
        )
        .unwrap();
    assert!(!not_active.durable_state_changed);
    assert!(matches!(
        not_active.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("active_hunter_unknown")
    ));
}

#[test]
fn schema_ten_roster_overflow_upgrades_to_waiting_fifo() {
    let roster = DurableHunterRosterState {
        roster_resolved: true,
        wallets_resolved: true,
        hunters: (1..=10)
            .map(|hunter_id| DurableHunterState {
                hunter_id,
                gold: 0,
                current_hp: 1,
                max_hp: 1,
                stamina: HunterServiceGauge::default(),
                satiety: HunterServiceGauge::default(),
                mood: HunterServiceGauge::default(),
                profile: DurableHunterProfile::migration_default(hunter_id),
                runtime: Default::default(),
                hunt: Default::default(),
                owned_items: Vec::new(),
            })
            .collect(),
        ..DurableHunterRosterState::default()
    };
    let flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            schema_version: 10,
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    let durable = flow.durable_state();
    assert_eq!(durable.schema_version, DURABLE_PLAYER_SCHEMA_VERSION);
    assert_eq!(durable.hunter_roster.hunters.len(), 8);
    assert_eq!(durable.hunter_roster.waiting_queue.len(), 2);
    assert_eq!(durable.hunter_roster.waiting_queue[0].hunter.hunter_id, 9);
    assert_eq!(durable.hunter_roster.waiting_queue[1].hunter.hunter_id, 10);
}

#[test]
fn removed_hunter_roster_screen_restores_to_village() {
    let flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            schema_version: 15,
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::HunterRoster,
                boot_completed: true,
            },
            ..DurablePlayerAggregate::default()
        },
        7,
    );

    assert_eq!(flow.state().screen, OriginalScreen::Village);
    assert_eq!(
        flow.durable_state().schema_version,
        DURABLE_PLAYER_SCHEMA_VERSION
    );
}

#[test]
fn runtime_evidence_keeps_uncaptured_sections_null_and_projects_captured_status() {
    let mut hunter = operational_migration_roster().hunters.remove(0);
    let unresolved = runtime_evidence_snapshot(&hunter);
    assert_eq!(
        unresolved.status.evidence_state,
        HunterEvidenceState::SchemaConfirmed
    );
    assert!(unresolved.status.value.is_none());
    assert!(unresolved.skills.value.is_none());
    assert!(unresolved.appearance.value.is_none());
    assert!(unresolved.inventory.value.is_none());
    assert!(unresolved.growth.value.is_none());
    assert!(unresolved.riding_pet.value.is_none());

    hunter.runtime.status = Some(super::super::DurableHunterRuntimeStatus {
        hp: 120,
        now_hp: 75,
        feel: 90.0,
        now_feel: 45.0,
        hungry: 80.0,
        now_hungry: 40.0,
        tire: 70.0,
        now_tire: 35.0,
        damage: 22,
        armor: 11,
        critical: 7,
        attack_speed: 1.25,
        dodge: 3,
    });
    let captured = runtime_evidence_snapshot(&hunter);
    assert_eq!(
        captured.status.evidence_state,
        HunterEvidenceState::ValueCaptured
    );
    let status = captured.status.value.expect("captured status is projected");
    assert_eq!(status.maximum_hp, 120);
    assert_eq!(status.current_hp, 75);
    assert_eq!(status.attack_speed, 1.25);
}

#[test]
fn authoritative_hunt_tick_returns_loot_and_sale_conserves_economy() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    flow.handle_command_with_id(
        ClientCommand::AssignHunterHunt {
            hunter_id: 1,
            zone_id: super::super::hunter_roster::FIXTURE_HUNT_ZONE_ID.to_owned(),
        },
        Uuid::from_u128(100),
    )
    .unwrap();
    for _ in 0..HUNT_TICKS_TO_RETURN {
        flow.advance_simulation_tick().unwrap();
    }
    assert_eq!(flow.hunter_roster.hunters[0].hunt.status, "returning");
    flow.handle_command_with_id(
        ClientCommand::ReturnHunterHunt { hunter_id: 1 },
        Uuid::from_u128(101),
    )
    .unwrap();
    let town_gold_before = flow.buildings.town_gold;
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;
    let expected_price = flow
        .building_content
        .gameplay
        .item("material:1")
        .and_then(|item| item.town_pays_hunter_gold_per_unit)
        .unwrap();
    let sell_id = Uuid::from_u128(102);
    let sold = flow
        .handle_command_with_id(ClientCommand::SellHunterLoot { hunter_id: 1 }, sell_id)
        .unwrap();
    assert!(matches!(
        sold.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.buildings.town_gold, town_gold_before);
    advance_until_trade_settles(&mut flow, 1);
    assert_eq!(flow.buildings.town_gold, town_gold_before - expected_price);
    assert_eq!(
        flow.hunter_roster.hunters[0].gold,
        hunter_gold_before + expected_price
    );
    assert_eq!(
        flow.buildings
            .material_stocks
            .iter()
            .find(|stock| stock.id == "material:1")
            .unwrap()
            .town_quantity,
        1
    );
    let after_sale = flow.durable_state();
    let duplicate = flow
        .handle_command_with_id(ClientCommand::SellHunterLoot { hunter_id: 1 }, sell_id)
        .unwrap();
    assert!(matches!(
        duplicate.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.durable_state(), after_sale);
    let conflict = flow
        .handle_command_with_id(ClientCommand::ReturnHunterHunt { hunter_id: 1 }, sell_id)
        .unwrap();
    assert!(matches!(
        conflict.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
}

#[test]
fn hunter_sells_multiple_catalog_materials_in_one_authoritative_settlement() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.loot = vec![
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:32".to_owned(),
            quantity: 2,
        },
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:92".to_owned(),
            quantity: 3,
        },
    ];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    let town_gold_before = flow.buildings.town_gold;
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

    let sold = flow
        .handle_command_with_id(
            ClientCommand::SellHunterLoot { hunter_id: 1 },
            Uuid::from_u128(103),
        )
        .unwrap();

    assert!(matches!(
        sold.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.buildings.town_gold, town_gold_before);
    assert!(flow.hunter_roster.hunters[0].hunt.pending_trade.is_some());
    let destination = flow.hunter_roster.hunters[0]
        .hunt
        .pending_trade
        .as_ref()
        .map(|task| (task.interaction_x, task.interaction_y))
        .unwrap();
    let banish = flow
        .handle_command_with_id(
            ClientCommand::BanishHunter { hunter_id: 1 },
            Uuid::from_u128(104),
        )
        .unwrap();
    assert!(matches!(
        banish.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    advance_until_trade_settles(&mut flow, 1);
    assert_eq!(flow.buildings.town_gold, town_gold_before - 80);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 80);
    assert!(flow.hunter_roster.hunters[0].hunt.loot.is_empty());
    assert_eq!(flow.buildings.trade_settlements.len(), 2);
    let agent = flow
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert_eq!((agent.x, agent.y), destination);
    assert_eq!(agent.trade_gold, 80);
    assert_eq!(agent.trade_materials.len(), 2);
    assert_eq!(
        flow.buildings.trade_settlements[0].material_id,
        "material:32"
    );
    assert_eq!(flow.buildings.trade_settlements[0].total_gold, 20);
    assert_eq!(
        flow.buildings.trade_settlements[1].material_id,
        "material:92"
    );
    assert_eq!(flow.buildings.trade_settlements[1].total_gold, 60);
    assert_eq!(
        flow.buildings
            .material_stocks
            .iter()
            .find(|stock| stock.id == "material:32")
            .unwrap()
            .town_quantity,
        2
    );
    assert_eq!(
        flow.buildings
            .material_stocks
            .iter()
            .find(|stock| stock.id == "material:92")
            .unwrap()
            .town_quantity,
        3
    );
}

#[test]
fn pending_trade_restore_normalizes_or_releases_inconsistent_state() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
        item_id: "material:32".to_owned(),
        quantity: 1,
    }];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    flow.handle_command_with_id(
        ClientCommand::SellHunterLoot { hunter_id: 1 },
        Uuid::from_u128(105),
    )
    .expect("schedule trade");

    let mut compatible = flow.durable_state();
    compatible.hunter_roster.hunters[0].hunt.status = "idle".to_owned();
    compatible.hunter_roster.hunters[0].profile.action_state = "idle".to_owned();
    compatible.hunter_roster.hunters[0].profile.animation_name = "hunter_stay".to_owned();
    let restored = OriginalFlowSession::from_aggregate(compatible.clone(), 7);
    assert!(restored.hunter_roster.hunters[0]
        .hunt
        .pending_trade
        .is_some());
    assert_eq!(
        restored.hunter_roster.hunters[0].profile.action_state,
        "returning_to_trade"
    );

    compatible
        .buildings
        .buildings
        .retain(|building| building.id != "build_3");
    let missing_building = OriginalFlowSession::from_aggregate(compatible, 7);
    assert!(missing_building.hunter_roster.hunters[0]
        .hunt
        .pending_trade
        .is_none());
    assert_eq!(
        missing_building.hunter_roster.hunters[0]
            .profile
            .action_state,
        "idle"
    );

    let mut orphaned = flow.durable_state();
    orphaned.hunter_roster.hunters[0].hunt.pending_trade = None;
    let orphaned = OriginalFlowSession::from_aggregate(orphaned, 7);
    assert_eq!(
        orphaned.hunter_roster.hunters[0].profile.action_state,
        "idle"
    );
}

#[test]
fn hunter_trade_does_not_preempt_service_or_recovery() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
        item_id: "material:32".to_owned(),
        quantity: 1,
    }];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            product_services: DurableProductServiceState {
                visits: vec![DurableProductServiceVisit {
                    hunter_id: 1,
                    building_instance_id: "service-instance".to_owned(),
                    building_id: "build_12".to_owned(),
                    product_id: "service:test".to_owned(),
                    effect_kind: ServiceEffectKind::Hp,
                    remaining_ms: 1_000,
                    effect_value: 1,
                    payment_gold: 1,
                }],
            },
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);

    let service_rejection = flow
        .handle_command_with_id(
            ClientCommand::SellHunterLoot { hunter_id: 1 },
            Uuid::from_u128(106),
        )
        .expect("service trade result");
    assert!(matches!(
        service_rejection.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("hunter is unavailable for trade")
    ));
    assert!(flow.hunter_roster.hunters[0].hunt.pending_trade.is_none());

    flow.product_services.visits.clear();
    flow.hunter_roster.hunters[0].hunt.status = "returning_for_infirmary".to_owned();
    flow.hunter_roster.hunters[0].profile.action_state = "returning_for_infirmary".to_owned();
    let recovery_rejection = flow
        .handle_command_with_id(
            ClientCommand::SellHunterLoot { hunter_id: 1 },
            Uuid::from_u128(107),
        )
        .expect("recovery trade result");
    assert!(matches!(
        recovery_rejection.message,
        ServerMessage::IntentResult {
            accepted: false,
            ref reason,
            ..
        } if reason.as_deref() == Some("hunter is unavailable for trade")
    ));
    assert!(flow.hunter_roster.hunters[0].hunt.pending_trade.is_none());
}

#[test]
fn idle_hunter_auto_sells_only_requested_materials() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.loot = vec![
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:32".to_owned(),
            quantity: 2,
        },
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:92".to_owned(),
            quantity: 3,
        },
    ];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    flow.buildings.material_stocks = vec![DurableMaterialStock {
        id: "material:32".to_owned(),
        town_quantity: 0,
        hunter_quantity: 0,
        requested: 1,
        unit_price: 10,
    }];
    let town_gold_before = flow.buildings.town_gold;
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

    flow.advance_simulation_tick().expect("village tick");
    assert_eq!(flow.buildings.town_gold, town_gold_before);
    advance_until_trade_settles(&mut flow, 1);

    assert_eq!(flow.buildings.town_gold, town_gold_before - 10);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 10);
    assert_eq!(flow.buildings.material_stocks[0].town_quantity, 1);
    assert_eq!(flow.buildings.material_stocks[0].requested, 0);
    assert_eq!(
        flow.hunter_roster.hunters[0].hunt.loot,
        vec![
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:32".to_owned(),
                quantity: 1,
            },
            super::super::hunter_roster::DurableHunterLoot {
                item_id: "material:92".to_owned(),
                quantity: 3,
            },
        ]
    );
    assert_eq!(flow.buildings.trade_settlements.len(), 1);
}

#[test]
fn idle_hunter_can_auto_sell_again_without_starting_a_new_field_trip() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
        item_id: "material:32".to_owned(),
        quantity: 2,
    }];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    flow.buildings.material_stocks = vec![DurableMaterialStock {
        id: "material:32".to_owned(),
        town_quantity: 0,
        hunter_quantity: 0,
        requested: 1,
        unit_price: 10,
    }];

    flow.advance_simulation_tick().expect("first village tick");
    advance_until_trade_settles(&mut flow, 1);
    flow.buildings.material_stocks[0].requested = 1;
    flow.advance_simulation_tick().expect("second village tick");
    advance_until_trade_settles(&mut flow, 1);

    assert_eq!(flow.buildings.town_gold, 1_480);
    assert_eq!(flow.hunter_roster.hunters[0].gold, 1_020);
    assert!(flow.hunter_roster.hunters[0].hunt.loot.is_empty());
    assert_eq!(flow.buildings.material_stocks[0].town_quantity, 2);
    assert_eq!(flow.buildings.material_stocks[0].requested, 0);
    assert_eq!(flow.buildings.trade_settlements.len(), 2);
    assert_eq!(flow.buildings.field_trip_id, 1);
    assert!(flow
        .buildings
        .trade_settlements
        .iter()
        .all(|settlement| settlement.field_trip_id == 1));
    assert_ne!(
        flow.buildings.trade_settlements[0].settlement_id,
        flow.buildings.trade_settlements[1].settlement_id
    );
}

#[test]
fn ordinary_field_hunter_auto_sells_requested_material_and_ignores_legacy_gold_row() {
    let mut roster = operational_migration_roster();
    roster
        .assign_hunt(1, super::super::hunter_roster::ORDINARY_HUNT_REGION_IDS[0])
        .unwrap();
    roster.hunters[0].hunt.loot = vec![
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "gold".to_owned(),
            quantity: 500,
        },
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:32".to_owned(),
            quantity: 2,
        },
    ];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    flow.buildings.material_stocks = vec![DurableMaterialStock {
        id: "material:32".to_owned(),
        town_quantity: 0,
        hunter_quantity: 0,
        requested: 2,
        unit_price: 0,
    }];
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

    flow.advance_simulation_tick().expect("village tick");
    advance_until_trade_settles(&mut flow, 1);

    assert_eq!(flow.buildings.town_gold, 1_480);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 20);
    assert_eq!(flow.buildings.material_stocks[0].unit_price, 10);
    assert!(flow.hunter_roster.hunters[0].hunt.loot.is_empty());
    assert_eq!(flow.buildings.material_stocks[0].town_quantity, 2);
    assert_eq!(flow.buildings.material_stocks[0].requested, 0);
}

#[test]
fn ordinary_field_hunter_sell_command_uses_the_requested_material_lane() {
    let mut roster = operational_migration_roster();
    roster
        .assign_hunt(1, super::super::hunter_roster::ORDINARY_HUNT_REGION_IDS[0])
        .unwrap();
    roster.hunters[0].hunt.loot = vec![
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:32".to_owned(),
            quantity: 2,
        },
        super::super::hunter_roster::DurableHunterLoot {
            item_id: "material:92".to_owned(),
            quantity: 3,
        },
    ];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    flow.buildings.material_stocks = vec![DurableMaterialStock {
        id: "material:32".to_owned(),
        town_quantity: 0,
        hunter_quantity: 0,
        requested: 1,
        unit_price: 10,
    }];

    let result = flow
        .handle_command_with_id(
            ClientCommand::SellHunterLoot { hunter_id: 1 },
            Uuid::from_u128(8_001),
        )
        .unwrap();

    assert!(matches!(
        result.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    advance_until_trade_settles(&mut flow, 1);
    assert_eq!(flow.buildings.material_stocks[0].town_quantity, 1);
    assert_eq!(flow.buildings.material_stocks[0].requested, 0);
    assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[0].quantity, 1);
    assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[1].quantity, 3);
}

#[test]
fn auto_sale_buys_only_the_quantity_the_town_wallet_can_afford() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
        item_id: "material:32".to_owned(),
        quantity: 2,
    }];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    ensure_test_trading_post(&mut flow);
    flow.buildings.town_gold = 15;
    flow.buildings.material_stocks = vec![DurableMaterialStock {
        id: "material:32".to_owned(),
        town_quantity: 0,
        hunter_quantity: 0,
        requested: 2,
        unit_price: 10,
    }];
    let hunter_gold_before = flow.hunter_roster.hunters[0].gold;

    flow.advance_simulation_tick().expect("village tick");
    advance_until_trade_settles(&mut flow, 1);

    assert_eq!(flow.buildings.town_gold, 5);
    assert_eq!(flow.hunter_roster.hunters[0].gold, hunter_gold_before + 10);
    assert_eq!(flow.buildings.material_stocks[0].town_quantity, 1);
    assert_eq!(flow.buildings.material_stocks[0].requested, 1);
    assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[0].quantity, 1);
}

#[test]
fn auto_sale_does_not_enter_rejection_path_when_wallet_cannot_buy_one_unit() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.loot = vec![super::super::hunter_roster::DurableHunterLoot {
        item_id: "material:32".to_owned(),
        quantity: 2,
    }];
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.buildings.town_gold = 9;
    flow.buildings.material_stocks = vec![DurableMaterialStock {
        id: "material:32".to_owned(),
        town_quantity: 0,
        hunter_quantity: 0,
        requested: 2,
        unit_price: 10,
    }];

    let hunter = &flow.hunter_roster.hunters[0];
    assert!(!flow.has_affordable_auto_sale(hunter));
    flow.advance_simulation_tick().expect("village tick");
    assert_eq!(flow.buildings.town_gold, 9);
    assert_eq!(flow.buildings.trade_settlements.len(), 0);
    assert_eq!(flow.hunter_roster.hunters[0].hunt.loot[0].quantity, 2);
}

#[test]
fn skill_and_revive_commands_are_whitelisted_and_persisted() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    let rejected = flow
        .handle_command_with_id(
            ClientCommand::LearnHunterSkill {
                hunter_id: 1,
                skill_id: "arbitrary".to_owned(),
            },
            Uuid::from_u128(201),
        )
        .unwrap();
    assert!(matches!(
        rejected.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    flow.handle_command_with_id(
        ClientCommand::LearnHunterSkill {
            hunter_id: 1,
            skill_id: "skill_h1_01".to_owned(),
        },
        Uuid::from_u128(202),
    )
    .unwrap();
    flow.hunter_roster.hunters[1].profile.class_id = "h2".to_owned();
    flow.hunter_roster.hunters[1].profile.visual_family = "H2".to_owned();
    let cross_job = flow
        .handle_command_with_id(
            ClientCommand::LearnHunterSkill {
                hunter_id: 2,
                skill_id: "skill_h1_01".to_owned(),
            },
            Uuid::from_u128(204),
        )
        .unwrap();
    assert!(matches!(
        cross_job.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    flow.hunter_roster.defeat_hunter(1).unwrap();
    flow.handle_command_with_id(
        ClientCommand::ReviveHunter { hunter_id: 1 },
        Uuid::from_u128(203),
    )
    .unwrap();
    let restored = OriginalFlowSession::from_aggregate(flow.durable_state(), 7);
    assert_eq!(
        restored.hunter_roster.hunters[0].profile.skills[0].skill_id,
        "skill_h1_01"
    );
    assert_eq!(
        restored.hunter_roster.hunters[0].current_hp,
        restored.hunter_roster.hunters[0].max_hp
    );
}

#[test]
fn all_basic_jobs_start_with_and_can_activate_their_two_catalog_skills() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.state.screen = OriginalScreen::Village;
    let jobs = [
        (1, ["skill_h1_01", "skill_h1_02"], 15_000),
        (2, ["skill_h2_01", "skill_h2_02"], 8_000),
        (3, ["skill_h3_01", "skill_h3_02"], 6_000),
        (4, ["skill_h4_01", "skill_h4_02"], 6_000),
        (5, ["skill_h5_01", "skill_h5_02"], 6_000),
    ];
    for (hunter_id, skill_ids, _) in jobs {
        let hunter = &flow.hunter_roster.hunters[usize::try_from(hunter_id - 1).unwrap()];
        assert_eq!(
            hunter
                .profile
                .skills
                .iter()
                .map(|skill| skill.skill_id.as_str())
                .collect::<Vec<_>>(),
            skill_ids
        );
    }

    flow.monster_world.tick(&mut flow.hunter_roster);
    let (target_id, target_x, target_y) = {
        let target = &flow.monster_world.fields[0].monsters[0];
        (target.entity_id.clone(), target.x, target.y)
    };
    let ranger_agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 3)
        .unwrap();
    ranger_agent.region_id = Some(flow.monster_world.fields[0].map_id.clone());
    ranger_agent.x = target_x;
    ranger_agent.y = target_y;
    ranger_agent.target_monster_id = Some(target_id);
    for (command, (hunter_id, skill_ids, cooldown_ms)) in (211_u128..).zip(jobs) {
        let used = flow
            .handle_command_with_id(
                ClientCommand::UseHunterSkill {
                    hunter_id,
                    skill_id: skill_ids[0].to_owned(),
                    target_entity_id: None,
                },
                Uuid::from_u128(command),
            )
            .unwrap();
        assert!(
            matches!(
                used.message,
                ServerMessage::IntentResult { accepted: true, .. }
            ),
            "hunter {hunter_id} failed to activate {}: {:?}",
            skill_ids[0],
            used.message
        );
        let skill = &flow.hunter_roster.hunters[usize::try_from(hunter_id - 1).unwrap()]
            .profile
            .skills[0];
        assert!(!skill.ready);
        assert_eq!(skill.cooldown_remaining_ms, cooldown_ms);
    }
    let ranger = flow
        .world_projection()
        .entities
        .into_iter()
        .find(|entity| entity.descriptor.entity_id == "village-hunter-3")
        .unwrap();
    // Exact skill-to-animation bindings are unresolved; activation keeps a
    // neutral recovered Hunter clip rather than inventing an H3 mapping.
    assert_eq!(ranger.animation, "hunter_stay");
    assert_eq!(ranger.attack_effect_key, None);

    flow.refresh_skill_cooldowns(16_000);
    assert!(flow
        .hunter_roster
        .hunters
        .iter()
        .take(5)
        .all(|hunter| hunter.profile.skills[0].ready));
}

#[test]
fn hunter_automatically_casts_the_first_ready_skill_on_an_acquired_target() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.state.screen = OriginalScreen::Village;
    flow.monster_world.tick(&mut flow.hunter_roster);
    let target = &flow.monster_world.fields[0].monsters[0];
    let target_id = target.entity_id.clone();
    let target_position = (target.x, target.y);
    let agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some(flow.monster_world.fields[0].map_id.clone());
    agent.x = target_position.0;
    agent.y = target_position.1;
    agent.target_monster_id = Some(target_id);
    agent.active_skill_id = None;
    agent.action_state = HunterActionState::Attacking;

    flow.auto_cast_ready_hunter_skills();

    let skill = &flow.hunter_roster.hunters[0].profile.skills[0];
    assert_eq!(skill.skill_id, "skill_h1_01");
    assert!(!skill.ready);
    assert_eq!(skill.cooldown_remaining_ms, 15_000);
    let agent = flow
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert_eq!(agent.active_skill_id.as_deref(), Some("skill_h1_01"));
    assert_eq!(agent.recovery_ticks, 3);
    assert_eq!(agent.skill_attack_percent, 10);
    assert_eq!(agent.skill_attack_speed_milli, 2_380);

    flow.advance_simulation_tick().expect("active village tick");
    assert_eq!(
        flow.hunter_roster.hunters[0].profile.skills[0].cooldown_remaining_ms,
        14_900
    );
}

#[test]
fn hunter_does_not_attempt_auto_cast_while_chasing_an_out_of_range_target() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.state.screen = OriginalScreen::Village;
    flow.monster_world.tick(&mut flow.hunter_roster);
    let target_id = flow.monster_world.fields[0].monsters[0].entity_id.clone();
    let agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.target_monster_id = Some(target_id);
    agent.action_state = HunterActionState::Chasing;

    flow.auto_cast_ready_hunter_skills();

    let skill = &flow.hunter_roster.hunters[0].profile.skills[0];
    assert!(skill.ready);
    assert_eq!(skill.cooldown_remaining_ms, 0);
    assert!(flow.monster_world.hunters[0].active_skill_id.is_none());
}

#[test]
fn rejected_targeted_skill_does_not_mutate_combat_presentation() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.monster_world.tick(&mut flow.hunter_roster);
    let agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 3)
        .unwrap();
    agent.target_monster_id = None;
    let before = agent.clone();

    let result = flow
        .handle_command_with_id(
            ClientCommand::UseHunterSkill {
                hunter_id: 3,
                skill_id: "skill_h3_01".to_owned(),
                target_entity_id: None,
            },
            Uuid::from_u128(301),
        )
        .unwrap();

    assert!(matches!(
        result.message,
        ServerMessage::IntentResult {
            accepted: false,
            ..
        }
    ));
    assert_eq!(
        flow.monster_world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == 3)
            .unwrap(),
        &before
    );
}

#[test]
fn durable_aggregate_restores_hunter_runtime_but_excludes_monsters_and_drops() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.zone_id = Some("background_11".to_owned());
    roster.hunters[0].hunt.status = "hunting".to_owned();
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Field,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.monster_world.enter_map("background_11").unwrap();
    flow.monster_world.set_density(3).unwrap();
    flow.monster_world.tick = 99;
    flow.monster_world.tick(&mut flow.hunter_roster);
    let target_id = flow.monster_world.fields[2].monsters[0].entity_id.clone();
    let agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.x = 411;
    agent.y = 733;
    agent.facing_left = true;
    agent.action_state = HunterActionState::Attacking;
    agent.animation = "hunter_atk".to_owned();
    agent.target_monster_id = Some(target_id.clone());
    agent.recovery_ticks = 4;
    flow.monster_world
        .current_field_mut()
        .drops
        .push(crate::simulation::MonsterDrop {
            drop_id: "drop-monster-background_11-0-test".to_owned(),
            monster_entity_id: "monster-background_11-0".to_owned(),
            item_id: "material:32".to_owned(),
            quantity: 1,
            x: 0,
            y: 0,
            owner_hunter_id: 1,
            gold: 0,
            experience: 0,
        });

    let durable = flow.durable_state();
    let json = serde_json::to_value(&durable).unwrap();
    assert!(json.get("monster_world").is_none());
    assert_eq!(json["hunter_world_runtime"][0]["x"], 411);
    assert!(json["monster_field_config"].get("tier_id").is_none());
    assert!(json["monster_field_config"].get("density_level").is_none());
    assert_eq!(
        json["monster_field_config"]["densities"][2]["map_id"],
        "background_11"
    );
    assert_eq!(
        json["monster_field_config"]["densities"][2]["density_level"],
        3
    );

    let restored = OriginalFlowSession::from_aggregate(durable, 7);
    assert_eq!(restored.monster_world.current_map_id, "map_new01");
    restored.monster_world.fields.iter().for_each(|field| {
        let expected = if field.map_id == "background_11" {
            3
        } else {
            1
        };
        assert_eq!(field.density_level, expected);
    });
    assert_eq!(restored.monster_world.tick, 0);
    assert!(restored
        .monster_world
        .fields
        .iter()
        .all(|field| field.drops.is_empty()));
    let restored_agent = restored
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert_eq!((restored_agent.x, restored_agent.y), (411, 733));
    assert!(restored_agent.facing_left);
    assert_eq!(restored_agent.action_state, HunterActionState::Attacking);
    assert_eq!(restored_agent.animation, "hunter_atk");
    assert_eq!(
        restored_agent.target_monster_id.as_deref(),
        Some(target_id.as_str())
    );
    assert_eq!(restored_agent.recovery_ticks, 4);
}

#[test]
fn reconnect_drops_an_unrestorable_loot_action_without_resetting_position() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.zone_id = Some("background_08".to_owned());
    roster.hunters[0].hunt.status = "hunting".to_owned();
    let runtime = HunterAgentState {
        hunter_id: 1,
        region_id: Some("background_08".to_owned()),
        x: 123,
        y: 456,
        facing_left: false,
        action_state: HunterActionState::CollectingLoot,
        animation: "hunter_stay".to_owned(),
        target_monster_id: None,
        target_drop_id: Some("expired-drop".to_owned()),
        recovery_ticks: 12,
        respawn_ticks: None,
        attack_sequence: 3,
        loot_sequence: 8,
        loot_item_id: Some("material:32".to_owned()),
        loot_quantity: 2,
        active_skill_id: None,
        skill_buff_ticks: 0,
        skill_attack_percent: 0,
        skill_defense_percent: 0,
        skill_evasion_percent: 0,
        skill_critical_percent: 0,
        skill_attack_speed_milli: 0,
        ice_armor_active: false,
        entry_stage: 2,
        town_roam_sequence: 0,
        town_roam_idle_ticks: 0,
        trade_sequence: 0,
        trade_gold: 0,
        trade_materials: Vec::new(),
    };
    let restored = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: roster,
            hunter_world_runtime: vec![runtime],
            ..DurablePlayerAggregate::default()
        },
        7,
    );

    let agent = restored
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert_eq!((agent.x, agent.y), (123, 456));
    assert_eq!(agent.action_state, HunterActionState::AcquiringTarget);
    assert!(agent.target_drop_id.is_none());
    assert!(agent.loot_item_id.is_none());
    assert_eq!(agent.loot_quantity, 0);
    assert_eq!(agent.recovery_ticks, 0);
}

#[test]
fn reassignment_resumes_a_persisted_bridge_checkpoint_without_backtracking() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.zone_id = Some("background_08".to_owned());
    roster.hunters[0].hunt.status = "hunting".to_owned();
    let mut seed = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    seed.advance_simulation_tick();
    let mut durable = seed.durable_state();
    let runtime = durable
        .hunter_world_runtime
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .expect("persisted Hunter runtime");
    // Exact stale live checkpoint observed after the corrected Bridge C
    // route shipped: beside the bridge exit, but persisted as stage 0.
    runtime.x = 1_324;
    runtime.y = 807;
    runtime.entry_stage = 0;
    runtime.action_state = HunterActionState::Attacking;
    runtime.animation = "hunter_atk".to_owned();
    runtime.target_monster_id = Some("expired-target".to_owned());

    let mut restored = OriginalFlowSession::from_aggregate(durable, 7);
    let accepted = restored
        .handle_command_with_id(
            ClientCommand::AssignHunterHunt {
                hunter_id: 1,
                zone_id: "background_08".to_owned(),
            },
            Uuid::from_u128(0x00b1_1d63),
        )
        .expect("reassignment response");
    assert!(matches!(
        accepted.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    let assigned = restored
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .expect("assigned Hunter runtime");
    assert_eq!((assigned.x, assigned.y), (1_324, 807));
    assert_eq!(assigned.entry_stage, 3);
    let y_before = assigned.y;

    restored.advance_simulation_tick();
    let moved = restored
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .expect("moving Hunter runtime");
    assert!(moved.y > y_before);
}

#[test]
fn reassignment_relocates_a_persisted_position_outside_town_route_and_field() {
    let mut roster = operational_migration_roster();
    roster.hunters[0].hunt.zone_id = Some("map_new01".to_owned());
    roster.hunters[0].hunt.status = "hunting".to_owned();
    let mut seed = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: roster,
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    seed.advance_simulation_tick();
    let mut durable = seed.durable_state();
    let runtime = durable
        .hunter_world_runtime
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .expect("persisted Hunter runtime");
    runtime.x = 40;
    runtime.y = 1_400;
    runtime.entry_stage = u8::MAX;

    let mut restored = OriginalFlowSession::from_aggregate(durable, 7);
    restored
        .handle_command_with_id(
            ClientCommand::AssignHunterHunt {
                hunter_id: 1,
                zone_id: "map_new01".to_owned(),
            },
            Uuid::from_u128(0x57a1e),
        )
        .expect("reassignment response");
    let assigned = restored
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .expect("assigned Hunter runtime");
    assert!((TOWN_ROAM_BOUNDS.min_x..=TOWN_ROAM_BOUNDS.max_x).contains(&assigned.x));
    assert!((TOWN_ROAM_BOUNDS.min_y..=TOWN_ROAM_BOUNDS.max_y).contains(&assigned.y));
    assert_eq!(assigned.entry_stage, 0);
    assert_eq!(assigned.action_state, HunterActionState::EnteringRegion);
}

#[test]
fn colony_route_advances_from_the_live_blocked_town_checkpoint() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            navigation: OriginalFlowPlayerState {
                screen: OriginalScreen::Village,
                boot_completed: true,
            },
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.advance_simulation_tick();
    let hunter_id = flow.hunter_roster.hunters[0].hunter_id;
    flow.hunter_roster
        .assign_hunt(hunter_id, map_configs()[0].map_id)
        .unwrap();
    let agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == hunter_id)
        .unwrap();
    agent.region_id = Some(map_configs()[0].map_id.to_owned());
    agent.x = 1_522;
    agent.y = 690;
    agent.entry_stage = 0;

    for _ in 0..400 {
        flow.advance_simulation_tick();
        let agent = flow
            .monster_world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == hunter_id)
            .unwrap();
        if agent.entry_stage > 0 {
            break;
        }
    }

    let agent = flow
        .monster_world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == hunter_id)
        .unwrap();
    assert!(
        agent.entry_stage > 0,
        "Hunter remained blocked at ({}, {})",
        agent.x,
        agent.y
    );
}

#[test]
fn world_projection_includes_the_collected_gold_quantity() {
    let mut flow = OriginalFlowSession::from_aggregate(
        DurablePlayerAggregate {
            hunter_roster: operational_migration_roster(),
            ..DurablePlayerAggregate::default()
        },
        7,
    );
    flow.state.screen = OriginalScreen::Village;
    flow.monster_world.tick(&mut flow.hunter_roster);
    let agent = flow
        .monster_world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.loot_sequence = 1;
    agent.loot_item_id = Some("gold".to_owned());
    agent.loot_quantity = 39;

    let hunter = flow
        .world_entities()
        .into_iter()
        .find(|entity| entity.descriptor.entity_id == "village-hunter-1")
        .unwrap();

    assert_eq!(hunter.loot_label.as_deref(), Some("Gold +39"));
}

#[test]
fn village_density_board_updates_only_the_selected_hunting_region() {
    let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
        screen: OriginalScreen::Village,
        boot_completed: true,
    });

    let result = flow
        .handle_command(ClientCommand::SetMonsterRegionDensity {
            region_id: "background_08".to_owned(),
            level: 3,
        })
        .expect("density board result");

    assert!(matches!(
        result.message,
        ServerMessage::IntentResult { accepted: true, .. }
    ));
    assert_eq!(flow.monster_world.current_map_id, "map_new01");
    assert_eq!(flow.monster_world.fields[0].density_level, 1);
    assert_eq!(flow.monster_world.fields[1].density_level, 3);
    assert_eq!(flow.monster_world.fields[2].density_level, 1);
}

#[test]
fn legacy_single_map_density_migrates_without_persisting_the_visited_map() {
    let aggregate = DurablePlayerAggregate {
        monster_field_config: serde_json::from_value(serde_json::json!({
            "tier_id": "background_08",
            "density_level": 3
        }))
        .unwrap(),
        ..DurablePlayerAggregate::default()
    };

    let restored = OriginalFlowSession::from_aggregate(aggregate, 7);
    assert_eq!(restored.monster_world.current_map_id, "map_new01");
    assert_eq!(
        restored
            .monster_world
            .fields
            .iter()
            .find(|field| field.map_id == "background_08")
            .unwrap()
            .density_level,
        3
    );
}

#[test]
fn simulation_outcome_is_invariant_to_scheduler_tick_rate() {
    let state = OriginalFlowPlayerState {
        screen: OriginalScreen::Village,
        boot_completed: true,
    };
    let mut ten_hz = OriginalFlowSession::from_state(state.clone());
    let mut twenty_hz = OriginalFlowSession::from_state(state);

    let mut ten_hz_result = None;
    for _ in 0..10 {
        ten_hz_result = ten_hz.advance_simulation_step(100_000_000);
    }
    let mut twenty_hz_result = None;
    for _ in 0..20 {
        if let Some(result) = twenty_hz.advance_simulation_step(50_000_000) {
            twenty_hz_result = Some(result);
        }
    }

    let ten_hz_result = ten_hz_result.expect("10 Hz produces a domain frame");
    let twenty_hz_result = twenty_hz_result.expect("20 Hz produces a domain frame");
    assert_eq!(
        ten_hz_result.simulation_tick,
        twenty_hz_result.simulation_tick
    );
    assert_eq!(ten_hz_result.world, twenty_hz_result.world);
    assert_eq!(ten_hz.durable_state(), twenty_hz.durable_state());
}

fn accepted_snapshot(message: &ServerMessage) -> &OriginalFlowSnapshot {
    match message {
        ServerMessage::IntentResult { snapshot, .. }
        | ServerMessage::BindingBlocked { snapshot, .. }
        | ServerMessage::Resync { snapshot }
        | ServerMessage::WorldUpdate { snapshot }
        | ServerMessage::Welcome { snapshot, .. } => snapshot,
        ServerMessage::WorldFrame { .. } | ServerMessage::FarmReportQueued { .. } => {
            panic!("lightweight transport messages do not carry domain snapshots")
        }
    }
}
