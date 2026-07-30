use super::*;
use crate::simulation::hunter_roster::operational_migration_roster;

#[test]
fn live_progression_keeps_the_recovered_strict_experience_threshold() {
    let mut roster = operational_migration_roster();
    let hunter = &mut roster.hunters[0];
    hunter.profile.level = 0;
    hunter.profile.xp = 0;
    hunter.profile.xp_to_next_level = Some(240);

    assert_eq!(add_experience(hunter, 240), 240);
    assert_eq!((hunter.profile.level, hunter.profile.xp), (0, 240));

    assert_eq!(add_experience(hunter, 1), 1);
    assert_eq!((hunter.profile.level, hunter.profile.xp), (1, 1));
}

#[test]
fn live_progression_discards_experience_at_display_level_100() {
    let mut roster = operational_migration_roster();
    let hunter = &mut roster.hunters[0];
    hunter.profile.level = super::super::original_progression::original_hunter_max_stored_level();
    hunter.profile.xp = 12;

    assert_eq!(add_experience(hunter, 50), 0);
    assert_eq!(hunter.profile.xp, 12);
}

#[test]
fn live_hunter_damage_applies_the_recovered_stored_level_factor() {
    assert_eq!(original_level_scaled_attack(1_000, 0), Some(1_000));
    assert_eq!(original_level_scaled_attack(1_000, 1), Some(1_003));
    // Native float32 produces a value just below 1297 before integer truncation.
    assert_eq!(original_level_scaled_attack(1_000, 99), Some(1_296));
}

#[test]
fn basic_skill_effects_apply_server_owned_buff_and_multihit_state() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.reconcile_hunters(&roster, &[]);
    let monster = world.fields[0].monsters[0].clone();
    world.hunters[0].x = monster.x;
    world.hunters[0].y = monster.y;
    world.hunters[0].target_monster_id = Some(monster.entity_id.clone());

    world
        .apply_hunter_skill_effect(&roster, 1, "skill_h1_01")
        .unwrap();
    assert_eq!(world.hunters[0].skill_buff_ticks, 100);
    assert_eq!(world.hunters[0].skill_attack_percent, 10);
    assert_eq!(world.hunters[0].skill_attack_speed_milli, 2_380);

    roster.hunters[0].profile.class_id = "h3".to_owned();
    roster.hunters[0].profile.visual_family = "H3".to_owned();
    roster.hunters[0].profile.dps_milli = Some(10_000);
    world.fields[0].monsters[0].hp = 10_000;
    world
        .apply_hunter_skill_effect(&roster, 1, "skill_h3_01")
        .unwrap();
    assert_eq!(world.fields[0].monsters[0].hp, 10_000 - 4 * 14);
}

#[test]
fn density_reconciles_only_the_selected_region() {
    let mut world = MonsterWorldState::default();
    let first_region = world.fields[0].monsters.len();
    world.set_region_density("background_08", 3).unwrap();
    assert_eq!(world.fields[0].monsters.len(), first_region);
    assert_eq!(world.fields[1].monsters.len(), 9);
}

#[test]
fn ordinary_regions_never_overlap_the_town_building_zone() {
    for config in map_configs() {
        assert!(!config.bounds.intersects(TOWN_EXCLUSION_BOUNDS));
        for index in 0..9 {
            let point = spawn_point(config.bounds, index);
            assert!(config.bounds.contains(point.0, point.1));
            assert!(!TOWN_EXCLUSION_BOUNDS.contains(point.0, point.1));
        }
    }
}

#[test]
fn hunter_enters_each_field_through_recovered_bridge_corridors() {
    assert_eq!(
        map_configs()[0].entry_waypoints,
        [(1410, 690), (1356, 800), (1273, 800)]
    );
    assert_eq!(
        map_configs()[1].entry_waypoints,
        [(1410, 690), (1356, 800), (1356, 861)]
    );
    assert_eq!(
        map_configs()[2].entry_waypoints,
        [(1957, 809), (2043, 724), (2127, 724)]
    );
    for config in map_configs() {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        roster.assign_hunt(1, config.map_id).unwrap();
        for (stage, waypoint) in config.entry_waypoints.iter().enumerate() {
            let mut reached = false;
            for _ in 0..300 {
                world.tick(&mut roster);
                let agent = world
                    .hunters
                    .iter()
                    .find(|agent| agent.hunter_id == 1)
                    .unwrap();
                if usize::from(agent.entry_stage) > stage {
                    reached = true;
                    assert!(
                        squared_distance(agent.x, agent.y, waypoint.0, waypoint.1)
                            <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
                    );
                    break;
                }
            }
            assert!(
                reached,
                "Hunter never reached entry waypoint {stage} for {}",
                config.map_id
            );
        }

        let mut entered_field = false;
        for _ in 0..300 {
            world.tick(&mut roster);
            let agent = world
                .hunters
                .iter()
                .find(|agent| agent.hunter_id == 1)
                .unwrap();
            if config.bounds.contains(agent.x, agent.y) {
                entered_field = true;
                break;
            }
        }
        assert!(
            entered_field,
            "Hunter never entered field for {}",
            config.map_id
        );
    }
}

#[test]
fn movement_uses_a_bounded_constant_length_step() {
    let (mut x, mut y, mut facing_left) = (0, 0, false);
    move_toward(&mut x, &mut y, -100, 100, 10, &mut facing_left);
    assert!(facing_left);
    assert!(squared_distance(0, 0, x, y) <= 100);
    assert!(x < 0 && y > 0);
}

#[test]
fn hunter_attack_recovery_uses_base_attack_speed_and_never_reaches_zero() {
    assert_eq!(hunter_attack_recovery_ticks(Some(1_000), 0), 10);
    assert_eq!(hunter_attack_recovery_ticks(Some(2_000), 0), 20);
    assert_eq!(hunter_attack_recovery_ticks(Some(2_000), 2_000), 10);
    assert_eq!(hunter_attack_recovery_ticks(Some(250), 10_000), 3);
    assert_eq!(hunter_attack_recovery_ticks(None, 0), 10);
}

#[test]
fn monster_attack_uses_back_clip_when_target_is_above_actor() {
    assert_eq!(monster_directional_animation("atk", 500, 450), "atk_b");
    assert_eq!(monster_directional_animation("atk", 500, 550), "atk");
}

#[test]
fn hunter_move_tuning_averages_exactly_seven_and_a_half_pixels_per_tick() {
    assert_eq!(hunter_move_step(1), 7);
    assert_eq!(hunter_move_step(2), 8);
    assert_eq!(hunter_move_step(1) + hunter_move_step(2), 15);
}

#[test]
fn unassigned_hunters_roam_in_town_then_pause_without_leaving_town_bounds() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    world.tick(&mut roster);
    let initial = world.hunters[0].clone();
    for _ in 0..30 {
        world.tick(&mut roster);
    }
    let moved = world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == initial.hunter_id)
        .expect("hunter remains in world");
    assert_ne!((moved.x, moved.y), (initial.x, initial.y));
    assert_eq!(moved.region_id, None);
    assert!(TOWN_ROAM_BOUNDS.contains(moved.x, moved.y));
    assert_eq!(moved.action_state, HunterActionState::TownIdle);
    assert!(matches!(
        moved.animation.as_str(),
        "hunter_walk" | "hunter_stay"
    ));
}

#[test]
fn town_roam_anchors_stay_inside_the_confirmed_rebuild_floor() {
    for (x, y) in TOWN_ROAM_ANCHORS {
        assert!(TOWN_ROAM_BOUNDS.contains(x, y));
    }
}

#[test]
fn each_hunter_uses_a_distinct_deterministic_town_route() {
    let routes = (1..=5)
        .map(|hunter_id| {
            (0..TOWN_ROAM_ANCHORS.len())
                .map(|sequence| town_roam_anchor_index(hunter_id, u32::try_from(sequence).unwrap()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for route in &routes {
        assert_eq!(route.iter().copied().collect::<HashSet<_>>().len(), 8);
        assert!(route.windows(2).all(|pair| pair[0] != pair[1]));
    }
    assert!(routes.iter().collect::<HashSet<_>>().len() >= 4);
}

#[test]
fn town_roam_pauses_are_staggered_and_bounded_per_hunter() {
    let pause_ticks = (1..=8)
        .map(|hunter_id| town_roam_idle_ticks(hunter_id, 3))
        .collect::<Vec<_>>();

    assert!(pause_ticks
        .iter()
        .all(|ticks| (*ticks >= TOWN_ROAM_MIN_IDLE_TICKS)
            && (*ticks < TOWN_ROAM_MIN_IDLE_TICKS + TOWN_ROAM_IDLE_VARIANCE_TICKS)));
    assert!(pause_ticks.iter().copied().collect::<HashSet<_>>().len() >= 4);
}

#[test]
fn newly_added_town_hunter_walks_in_through_the_tunnel() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    let arriving = roster.hunters.pop().expect("fixture arrival hunter");
    let arriving_id = arriving.hunter_id;
    world.tick(&mut roster);

    roster.hunters.push(arriving);
    world.tick(&mut roster);
    let at_gate = world
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == arriving_id)
        .unwrap();
    assert_eq!((at_gate.x, at_gate.y), TOWN_ARRIVAL_OUTSIDE);
    assert_eq!(at_gate.entry_stage, 4);

    for _ in 0..40 {
        world.tick(&mut roster);
    }
    let inside = world
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == arriving_id)
        .unwrap();
    assert_eq!(inside.entry_stage, 0);
    assert!(TOWN_ROAM_BOUNDS.contains(inside.x, inside.y));
}

#[test]
fn completed_revival_uses_the_authoritative_sanctuary_point() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    world.tick(&mut roster);
    roster.hunters[0].current_hp = 0;
    world.hunters[0].respawn_ticks = Some(1);
    let sanctuary_point = (1498, 510);

    world.tick_with_obstacles(&mut roster, &[], Some(sanctuary_point), &HashMap::new());

    assert_eq!((world.hunters[0].x, world.hunters[0].y), sanctuary_point);
    assert_eq!(world.hunters[0].respawn_ticks, None);
    assert_eq!(roster.hunters[0].current_hp, roster.hunters[0].max_hp);
}

#[test]
fn clearing_a_field_assignment_walks_back_without_teleporting() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, map_configs()[0].map_id).unwrap();
    world.tick(&mut roster);
    let field_position = (
        map_configs()[0].bounds.min_x + 120,
        map_configs()[0].bounds.min_y + 120,
    );
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some(map_configs()[0].map_id.to_owned());
    agent.x = field_position.0;
    agent.y = field_position.1;
    roster.hunters[0].hunt.zone_id = None;
    roster.hunters[0].hunt.status = "idle".to_owned();

    world.tick(&mut roster);

    let returning = world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert!(returning.region_id.is_none());
    assert_eq!(returning.entry_stage, 3);
    assert_eq!(returning.action_state, HunterActionState::TownIdle);
    assert_ne!((returning.x, returning.y), TOWN_ARRIVAL_OUTSIDE);
    assert!(
        squared_distance(field_position.0, field_position.1, returning.x, returning.y)
            <= i64::from(HUNTER_MOVE_MAX_PX_PER_TICK).pow(2)
    );
}

#[test]
fn field_target_acquisition_searches_the_entire_assigned_region() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.tick(&mut roster);
    let field_monster = world.fields[0].monsters[0].clone();
    for monster in world.fields[0].monsters.iter_mut().skip(1) {
        monster.hp = 0;
    }
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some("map_new01".to_owned());
    agent.entry_stage = 2;
    agent.x = map_configs()[0].bounds.max_x;
    agent.y = map_configs()[0].bounds.max_y;
    assert!(
        squared_distance(agent.x, agent.y, field_monster.x, field_monster.y)
            > i64::from(MONSTER_DETECTION_RANGE_PX).pow(2)
    );
    world.tick(&mut roster);
    let updated = world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert!(updated.target_monster_id.is_some());
    assert_eq!(updated.region_id.as_deref(), Some("map_new01"));
    assert_eq!(
        updated.target_monster_id.as_deref(),
        Some(field_monster.entity_id.as_str())
    );
}

#[test]
fn hunter_retargets_an_engaged_survivor_before_collecting_dead_target_loot() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.tick(&mut roster);

    let dead_target = world.fields[0].monsters[0].clone();
    let survivor_id = world.fields[0].monsters[1].entity_id.clone();
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some("map_new01".to_owned());
    agent.entry_stage = 2;
    agent.x = dead_target.x;
    agent.y = dead_target.y;
    agent.target_monster_id = Some(dead_target.entity_id.clone());
    agent.recovery_ticks = 0;

    world.fields[0].monsters[1].x = dead_target.x;
    world.fields[0].monsters[1].y = dead_target.y;
    world.fields[0].monsters[1].target_hunter_id = Some(1);
    world.apply_damage_to_monster(
        &dead_target.entity_id,
        1,
        dead_target.hp,
        CombatPresentationKind::NormalDamage,
    );
    assert!(!world.fields[0].drops.is_empty());

    world.tick(&mut roster);

    let agent = world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert_eq!(
        agent.target_monster_id.as_deref(),
        Some(survivor_id.as_str())
    );
    assert_eq!(agent.action_state, HunterActionState::Attacking);
    assert_eq!(agent.target_drop_id, None);
}

#[test]
fn hunter_forces_a_loot_pass_after_two_defeated_monsters() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.reconcile_hunters(&roster, &[]);
    let (x, y) = {
        let monster = &world.fields[0].monsters[0];
        (monster.x, monster.y)
    };
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some("map_new01".to_owned());
    agent.x = x;
    agent.y = y;
    agent.target_monster_id = None;
    agent.recovery_ticks = 0;
    world.fields[0].monsters[0].target_hunter_id = None;
    for source in ["defeated-a", "defeated-b"] {
        world.fields[0].drops.push(MonsterDrop {
            drop_id: format!("drop-{source}"),
            monster_entity_id: source.to_owned(),
            item_id: "material:1".to_owned(),
            quantity: 1,
            x,
            y,
            owner_hunter_id: 1,
            gold: 0,
            experience: 0,
        });
    }

    world.tick_hunters(&mut roster, &[], None, &HashMap::new());

    let agent = world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert_eq!(agent.action_state, HunterActionState::CollectingLoot);
    assert!(agent.target_drop_id.is_some());
    assert_eq!(agent.target_monster_id, None);
    assert_eq!(world.fields[0].drops.len(), 2);
}

#[test]
fn hunter_collects_a_single_kill_before_acquiring_a_new_monster() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.reconcile_hunters(&roster, &[]);

    let (x, y) = {
        let monster = &world.fields[0].monsters[0];
        (monster.x, monster.y)
    };
    let initial_gold = roster
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == 1)
        .unwrap()
        .gold;
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some("map_new01".to_owned());
    agent.entry_stage = 2;
    agent.x = x;
    agent.y = y;
    agent.target_monster_id = None;
    agent.recovery_ticks = 0;

    world.fields[0].drops.extend([
        MonsterDrop {
            drop_id: "drop-single-gold".to_owned(),
            monster_entity_id: "defeated-single".to_owned(),
            item_id: "gold".to_owned(),
            quantity: 37,
            x,
            y,
            owner_hunter_id: 1,
            gold: 37,
            experience: 12,
        },
        MonsterDrop {
            drop_id: "drop-single-material".to_owned(),
            monster_entity_id: "defeated-single".to_owned(),
            item_id: "material:7".to_owned(),
            quantity: 2,
            x: x + 8,
            y,
            owner_hunter_id: 1,
            gold: 0,
            experience: 0,
        },
    ]);

    world.tick_hunters(&mut roster, &[], None, &HashMap::new());
    let agent = world
        .hunters
        .iter()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    assert_eq!(agent.action_state, HunterActionState::CollectingLoot);
    assert!(agent.target_drop_id.is_some());
    assert_eq!(agent.target_monster_id, None);

    // Keep the assertion focused on the single source's pickup sequence;
    // no later combat kill should add another reward while it runs.
    for monster in &mut world.fields[0].monsters {
        monster.hp = 0;
        monster.target_hunter_id = None;
    }

    for _ in 0..8 {
        world.tick_hunters(&mut roster, &[], None, &HashMap::new());
    }

    let hunter = roster
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == 1)
        .unwrap();
    assert_eq!(hunter.gold, initial_gold + 37);
    assert_eq!(
        hunter
            .hunt
            .loot
            .iter()
            .find(|loot| loot.item_id == "material:7")
            .map(|loot| loot.quantity),
        Some(2)
    );
    assert!(world.fields[0].drops.is_empty());
}

#[test]
fn ranger_and_sorcerer_attack_from_range_while_melee_families_close_in() {
    assert_eq!(hunter_attack_range("H1"), HUNTER_MELEE_ATTACK_RANGE_PX);
    assert_eq!(hunter_attack_range("H2"), HUNTER_MELEE_ATTACK_RANGE_PX);
    assert_eq!(hunter_attack_range("H3"), HUNTER_RANGED_ATTACK_RANGE_PX);
    assert_eq!(hunter_attack_range("H4"), HUNTER_RANGED_ATTACK_RANGE_PX);
    assert_eq!(hunter_attack_range("H5"), HUNTER_MELEE_ATTACK_RANGE_PX);

    for (hunter_id, expected_state) in [
        (1, HunterActionState::Chasing),
        (3, HunterActionState::Attacking),
        (4, HunterActionState::Attacking),
    ] {
        let mut world = MonsterWorldState::default();
        let mut roster = operational_migration_roster();
        roster.assign_hunt(hunter_id, "map_new01").unwrap();
        world.tick(&mut roster);
        let monster = world.fields[0].monsters[0].clone();
        let agent = world
            .hunters
            .iter_mut()
            .find(|agent| agent.hunter_id == hunter_id)
            .unwrap();
        agent.region_id = Some("map_new01".to_owned());
        agent.entry_stage = 2;
        agent.x = monster.x + 120;
        agent.y = monster.y;
        agent.target_monster_id = Some(monster.entity_id);
        world.tick(&mut roster);

        let agent = world
            .hunters
            .iter()
            .find(|agent| agent.hunter_id == hunter_id)
            .unwrap();
        assert_eq!(agent.action_state, expected_state);
        if expected_state == HunterActionState::Attacking {
            assert!(
                agent.facing_left,
                "ranged Hunter must face its target before firing"
            );
        }
    }
}

#[test]
fn authoritative_damage_records_a_monotonic_target_bound_presentation() {
    let mut world = MonsterWorldState::default();
    let monster = world.fields[0].monsters[0].clone();

    world.apply_damage_to_monster(
        &monster.entity_id,
        1,
        17,
        CombatPresentationKind::NormalDamage,
    );

    assert_eq!(world.combat_presentations.len(), 1);
    assert_eq!(
        world.combat_presentations[0],
        CombatPresentation {
            sequence: 1,
            source_entity_id: "village-hunter-1".to_owned(),
            target_entity_id: monster.entity_id.clone(),
            kind: CombatPresentationKind::NormalDamage,
            amount: Some(17),
        }
    );

    world.apply_damage_to_monster(
        &monster.entity_id,
        1,
        3,
        CombatPresentationKind::NormalDamage,
    );
    assert_eq!(world.combat_presentations[1].sequence, 2);
}

#[test]
fn monster_death_projects_gold_as_a_separate_ground_drop() {
    let mut world = MonsterWorldState::default();
    let monster = world.fields[0].monsters[0].clone();

    world.apply_damage_to_monster(
        &monster.entity_id,
        1,
        monster.hp,
        CombatPresentationKind::NormalDamage,
    );

    let gold = world.fields[0]
        .drops
        .iter()
        .find(|drop| drop.item_id == "gold")
        .expect("gold drop");
    assert_eq!(gold.gold, monster.gold);
    assert_eq!(u64::from(gold.quantity), monster.gold);
    assert!(world.fields[0]
        .drops
        .iter()
        .filter(|drop| drop.item_id.starts_with("material:"))
        .all(|drop| drop.gold == 0 && drop.experience == 0));
}

#[test]
fn connected_original_resolver_emits_critical_damage_from_the_server() {
    let mut world = MonsterWorldState::default();
    let monster = world.fields[0].monsters[0].clone();

    world.resolve_hunter_attack(
        &monster.entity_id,
        HunterAttackSource {
            hunter_id: 1,
            calculated_damage: 100,
            calculated_critical_percent: 100,
            hunter_feel: 100.0,
            hunter_now_feel: 100.0,
            attack_sequence: 1,
        },
    );

    assert_eq!(world.combat_presentations.len(), 1);
    assert_eq!(
        world.combat_presentations[0].kind,
        CombatPresentationKind::CriticalDamage
    );
    assert_eq!(world.combat_presentations[0].amount, Some(191));
    assert_eq!(world.fields[0].monsters[0].hp, monster.hp - 191);
}

#[test]
fn exact_catalog_monster_damage_stays_separate_from_fixture_attack_input() {
    let monster = spawn_monster(&map_configs()[0], 0, 0);

    assert_eq!(monster.source_index, 0);
    assert_eq!(monster.damage, 542);
    assert_eq!(fixture_monster_attack_input(monster.damage), Some(2));
}

#[test]
fn all_ordinary_monster_stats_survive_catalog_selection_into_runtime_state() {
    for config in map_configs() {
        for pool in monster_pools()
            .iter()
            .filter(|pool| pool.map_id == config.map_id)
        {
            for (index, expected) in pool.monsters.iter().enumerate() {
                let actual = spawn_monster(config, pool.global_difficulty, index);
                assert_eq!(actual.source_index, expected.source_index);
                assert_eq!(actual.max_hp, expected.hp);
                assert_eq!(actual.hp, expected.hp);
                assert_eq!(actual.damage, expected.damage);
                assert_eq!(actual.armor, expected.armor);
                assert_eq!(actual.experience, expected.experience);
                assert_eq!(actual.gold, expected.gold);
            }
        }
    }
}

#[test]
fn live_world_defaults_to_only_the_difficulty_zero_catalog_rows() {
    let world = MonsterWorldState::default();

    assert_eq!(world.world_difficulty, 0);
    assert_eq!(
        world
            .fields
            .iter()
            .map(|field| {
                field
                    .monsters
                    .iter()
                    .map(|monster| monster.source_index)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
        vec![vec![0, 5, 10], vec![15, 20, 25], vec![30, 35, 40]]
    );
}

#[test]
fn authoritative_monster_hit_records_incoming_damage_for_the_hunter() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.tick(&mut roster);

    let monster = &mut world.fields[0].monsters[0];
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some("map_new01".to_owned());
    agent.x = monster.x;
    agent.y = monster.y;
    monster.target_hunter_id = Some(1);
    monster.recovery_ticks = 0;
    let hunter = roster
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == 1)
        .unwrap();
    let expected_tick = world.tick.saturating_add(1);
    let expected_dodge_roll = deterministic_combat_percent_roll(
        expected_tick,
        hunter.hunter_id,
        expected_tick,
        monster.source_index,
    );
    let expected_pet_roll = i32::try_from(
        (deterministic_roll(
            expected_tick,
            expected_tick,
            monster.source_index,
            u64::from(hunter.hunter_id).wrapping_add(1),
        ) - 1)
            % 1000,
    )
    .unwrap_or(0);
    let expected = resolve_original_neutral_monster_attack(OriginalMonsterAttackInputs {
        incoming_damage: fixture_monster_attack_input(monster.damage).unwrap(),
        rand_damage_multiplier: 0.91,
        hunter_armor: i64::try_from(hunter.profile.defense).unwrap(),
        hunter_feel: hunter.mood.maximum as f32,
        hunter_now_feel: hunter.mood.current as f32,
        hunter_hp: i64::try_from(hunter.current_hp).unwrap(),
        hunter_calc_dodge: hunter.profile.calc_dodge(),
        hunter_dodge_primary_roll_zero_to_ninety_nine: expected_dodge_roll,
        hunter_riding_pet_dodge: 0,
        hunter_riding_pet_roll_zero_to_nine_ninety_nine: expected_pet_roll,
        ..OriginalMonsterAttackInputs::default()
    })
    .unwrap();
    let monster_entity_id = monster.entity_id.clone();

    world.tick(&mut roster);

    let expected_kind = match expected.presentation {
        OriginalHitPresentation::Normal => CombatPresentationKind::IncomingDamage,
        OriginalHitPresentation::Miss => CombatPresentationKind::Miss,
        OriginalHitPresentation::Evade => CombatPresentationKind::Evade,
        OriginalHitPresentation::Critical => unreachable!(),
    };
    assert!(world.combat_presentations.iter().any(|presentation| {
        presentation.source_entity_id == monster_entity_id
            && presentation.target_entity_id == "village-hunter-1"
            && presentation.kind == expected_kind
            && presentation.amount
                == u64::try_from(expected.final_damage)
                    .ok()
                    .filter(|_| expected.presentation == OriginalHitPresentation::Normal)
    }));
}

#[test]
fn a_new_server_tick_expires_prior_combat_presentations() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    let monster_id = world.fields[0].monsters[0].entity_id.clone();
    world.apply_damage_to_monster(&monster_id, 1, 1, CombatPresentationKind::NormalDamage);
    assert_eq!(world.combat_presentations.len(), 1);

    world.tick(&mut roster);

    assert!(world.combat_presentations.is_empty());
}

#[test]
fn gold_only_drop_never_emits_an_invalid_item_reward_operation() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    let config = &map_configs()[0];
    let (x, y) = spawn_point(config.bounds, 0);
    world.hunters.push(HunterAgentState {
        hunter_id: 1,
        region_id: Some(config.map_id.to_owned()),
        x,
        y,
        facing_left: false,
        action_state: HunterActionState::CollectingLoot,
        animation: "hunter_walk".to_owned(),
        target_monster_id: None,
        target_drop_id: None,
        recovery_ticks: 0,
        respawn_ticks: None,
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
        entry_stage: 1,
        town_roam_sequence: 0,
        town_roam_idle_ticks: 0,
        trade_sequence: 0,
        trade_gold: 0,
        trade_materials: Vec::new(),
    });
    world.fields[0].drops.push(MonsterDrop {
        drop_id: "gold-only".to_owned(),
        monster_entity_id: "monster-1".to_owned(),
        item_id: "gold".to_owned(),
        quantity: 0,
        x,
        y,
        owner_hunter_id: 1,
        gold: 11,
        experience: 7,
    });
    let gold_before = roster
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == 1)
        .unwrap()
        .gold;
    let mut operations = Vec::new();

    assert!(world.try_collect_drop(0, &mut roster, &mut operations));
    assert_eq!(world.fields[0].drops.len(), 1);
    assert_eq!(
        world.hunters[0].target_drop_id.as_deref(),
        Some("gold-only")
    );
    world.hunters[0].recovery_ticks = 0;
    assert!(world.try_collect_drop(0, &mut roster, &mut operations));
    assert_eq!(
        roster
            .hunters
            .iter()
            .find(|hunter| hunter.hunter_id == 1)
            .unwrap()
            .gold,
        gold_before + 11
    );
    assert!(roster
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == 1)
        .unwrap()
        .hunt
        .loot
        .is_empty());
    assert_eq!(world.combat_presentations.len(), 1);
    assert_eq!(world.combat_presentations[0].source_entity_id, "monster-1");
    assert_eq!(
        world.combat_presentations[0].target_entity_id,
        "village-hunter-1"
    );
    assert_eq!(
        world.combat_presentations[0].kind,
        CombatPresentationKind::Experience
    );
    assert_eq!(world.combat_presentations[0].amount, Some(7));
    assert!(operations.is_empty());
}

#[test]
fn an_in_progress_pickup_finishes_even_when_a_monster_is_engaged() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.reconcile_hunters(&roster, &[]);
    let monster = world.fields[0].monsters[0].clone();
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some("map_new01".to_owned());
    agent.x = monster.x;
    agent.y = monster.y;
    agent.target_monster_id = Some(monster.entity_id.clone());
    agent.target_drop_id = Some("pending-material".to_owned());
    agent.recovery_ticks = 1;
    world.fields[0].monsters[0].target_hunter_id = Some(1);
    world.fields[0].drops.push(MonsterDrop {
        drop_id: "pending-material".to_owned(),
        monster_entity_id: "defeated-monster".to_owned(),
        item_id: "material:1".to_owned(),
        quantity: 2,
        x: monster.x,
        y: monster.y,
        owner_hunter_id: 1,
        gold: 0,
        experience: 0,
    });

    let operations = world.tick_hunters(&mut roster, &[], None, &HashMap::new());

    assert!(world.fields[0].drops.is_empty());
    assert_eq!(world.hunters[0].target_drop_id, None);
    assert_eq!(world.hunters[0].loot_item_id.as_deref(), Some("material:1"));
    assert_eq!(world.hunters[0].loot_quantity, 2);
    assert_eq!(roster.hunters[0].hunt.loot[0].quantity, 2);
    assert_eq!(operations.len(), 1);
}

#[test]
fn monster_animation_sequence_advances_only_with_authoritative_hits() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    world.reconcile_hunters(&roster, &[]);
    let monster = &mut world.fields[0].monsters[0];
    let agent = world
        .hunters
        .iter_mut()
        .find(|agent| agent.hunter_id == 1)
        .unwrap();
    agent.region_id = Some("map_new01".to_owned());
    agent.x = monster.x;
    agent.y = monster.y;
    monster.target_hunter_id = Some(1);
    monster.recovery_ticks = 0;

    world.tick_monsters(&mut roster);
    let hp_after_first_hit = roster.hunters[0].current_hp;
    assert_eq!(world.fields[0].monsters[0].attack_sequence, 1);
    assert_eq!(world.fields[0].monsters[0].recovery_ticks, 8);
    for _ in 0..7 {
        world.tick_monsters(&mut roster);
        assert_eq!(roster.hunters[0].current_hp, hp_after_first_hit);
        assert_eq!(world.fields[0].monsters[0].attack_sequence, 1);
    }
    world.tick_monsters(&mut roster);
    assert!(roster.hunters[0].current_hp < hp_after_first_hit);
    assert_eq!(world.fields[0].monsters[0].attack_sequence, 2);
}

#[test]
fn hunter_navigation_detours_around_building_footprints() {
    let obstacle = NavigationObstacle {
        min_x: 40,
        max_x: 60,
        min_y: -20,
        max_y: 20,
    };
    let (mut x, mut y, mut facing_left) = (0, 0, false);
    for _ in 0..30 {
        move_toward_avoiding(&mut x, &mut y, 100, 0, 10, &mut facing_left, &[obstacle]);
        assert!(!obstacle.expanded(12).contains(x, y));
    }
    assert!(x > obstacle.max_x);
}

#[test]
fn hunter_navigation_advances_from_an_obstacle_corner() {
    let obstacle = NavigationObstacle {
        min_x: 40,
        max_x: 60,
        min_y: 40,
        max_y: 60,
    };
    let expanded = obstacle.expanded(14);
    let (mut x, mut y, mut facing_left) = (expanded.min_x - 1, expanded.min_y - 1, false);

    move_toward_avoiding(&mut x, &mut y, 50, 100, 8, &mut facing_left, &[obstacle]);

    assert!(y > expanded.min_y - 1);
    assert!(!expanded.contains(x, y));
}

#[test]
fn hunter_navigation_routes_around_clustered_buildings_toward_third_field() {
    let obstacles = [
        NavigationObstacle {
            min_x: 1740,
            max_x: 1840,
            min_y: 660,
            max_y: 790,
        },
        NavigationObstacle {
            min_x: 1840,
            max_x: 1940,
            min_y: 710,
            max_y: 840,
        },
    ];
    let target = map_configs()[2].entry_waypoints[0];
    let (mut x, mut y) = TOWN_RESPAWN_POINT;
    let mut facing_left = false;
    for _ in 0..160 {
        move_toward_avoiding(
            &mut x,
            &mut y,
            target.0,
            target.1,
            8,
            &mut facing_left,
            &obstacles,
        );
        assert!(obstacles
            .iter()
            .all(|obstacle| !obstacle.expanded(14).contains(x, y)));
        if squared_distance(x, y, target.0, target.1) <= 64 {
            break;
        }
    }
    assert!(
        squared_distance(x, y, target.0, target.1) <= 64,
        "stopped at ({x}, {y}) toward ({}, {})",
        target.0,
        target.1
    );
}

#[test]
fn monster_patrol_uses_short_segments_then_idles_for_two_and_a_half_seconds() {
    let config = &map_configs()[0];
    let mut monster = spawn_monster(config, 0, 0);
    let origin = (monster.spawn_x, monster.spawn_y);

    for _ in 0..40 {
        patrol(&mut monster, config.bounds);
        if monster.action_state == MonsterActionState::Idle
            && monster.patrol_idle_ticks == MONSTER_PATROL_IDLE_TICKS
        {
            break;
        }
    }

    assert_eq!(monster.action_state, MonsterActionState::Idle);
    assert_eq!(monster.animation, "stay");
    assert_eq!(monster.patrol_idle_ticks, MONSTER_PATROL_IDLE_TICKS);
    assert_eq!(MONSTER_PATROL_IDLE_TICKS + 1, 25);
    assert!(
        squared_distance(origin.0, origin.1, monster.x, monster.y)
            <= i64::from(MONSTER_PATROL_RADIUS_PX).pow(2)
    );

    let resting_at = (monster.x, monster.y);
    for _ in 0..MONSTER_PATROL_IDLE_TICKS {
        patrol(&mut monster, config.bounds);
        assert_eq!((monster.x, monster.y), resting_at);
        assert_eq!(monster.action_state, MonsterActionState::Idle);
        assert_eq!(monster.animation, "stay");
    }

    patrol(&mut monster, config.bounds);
    assert_eq!(monster.action_state, MonsterActionState::Patrolling);
    assert_eq!(monster.animation, "walk");
    assert_ne!((monster.x, monster.y), resting_at);
}

#[test]
fn monster_patrol_waypoints_stay_inside_their_region() {
    for config in map_configs() {
        for index in 0..9 {
            let mut monster = spawn_monster(config, 0, index);
            for phase in 0..8 {
                monster.patrol_phase = phase;
                let waypoint = patrol_waypoint(&monster, config.bounds);
                assert!(config.bounds.contains(waypoint.0, waypoint.1));
                assert!(
                    squared_distance(monster.spawn_x, monster.spawn_y, waypoint.0, waypoint.1,)
                        <= i64::from(MONSTER_PATROL_RADIUS_PX).pow(2)
                );
            }
        }
    }
}

#[test]
fn assigned_hunter_enters_region_fights_collects_and_monster_respawns() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.hunters[0].profile.attack = 200;
    roster.hunters[0].current_hp = 130;
    roster.hunters[0].max_hp = 130;
    let gold_before = roster.hunters[0].gold;
    let xp_before = roster.hunters[0].profile.xp;
    roster.assign_hunt(1, "map_new01").unwrap();
    for _ in 0..1_200 {
        world.tick(&mut roster);
    }
    let hunter = roster
        .hunters
        .iter()
        .find(|hunter| hunter.hunter_id == 1)
        .unwrap();
    assert!(hunter.gold > gold_before);
    assert!(hunter.profile.xp != xp_before || hunter.profile.level > 1);
    assert!(!world.fields[0].monsters.is_empty());
}

#[test]
fn ordinary_material_roll_is_inclusive_and_bounded() {
    for slot in 0..64 {
        assert!((1..=10_000).contains(&deterministic_roll(10, 2, 34, slot)));
    }
}

#[test]
fn durable_dead_hunter_resumes_the_authoritative_respawn_clock_after_reconnect() {
    let mut world = MonsterWorldState::default();
    let mut roster = operational_migration_roster();
    roster.assign_hunt(1, "map_new01").unwrap();
    roster.defeat_hunter(1).unwrap();
    for _ in 0..=HUNTER_RESPAWN_TICKS {
        world.tick(&mut roster);
    }
    assert_eq!(roster.hunters[0].current_hp, roster.hunters[0].max_hp);
    assert_eq!(roster.hunters[0].hunt.status, "hunting");
}
