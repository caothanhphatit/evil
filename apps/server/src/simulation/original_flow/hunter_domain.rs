use super::{
    drop_icon_path, hunter_trade_workflow, DurableGearEnhancementTask, DurableHunterState,
    GearEnhancementAttemptSnapshot, GearEnhancementResourceSnapshot, GearEnhancementSnapshot,
    GearEnhancementTaskSnapshot, GearEnhancementTaskStatus, HunterEquipmentSlotSnapshot,
    HunterEvidenceSection, HunterEvidenceState, HunterHuntSnapshot, HunterInfoSkillSnapshot,
    HunterInfoSnapshot, HunterLootSnapshot, HunterMaterialSnapshot, HunterProgressSnapshot,
    HunterRidingPetSnapshot, HunterRosterMemberSnapshot, HunterRuntimeAppearanceSnapshot,
    HunterRuntimeConsumableSnapshot, HunterRuntimeEvidenceSnapshot, HunterRuntimeGearSnapshot,
    HunterRuntimeGrowthSnapshot, HunterRuntimeInventorySnapshot, HunterRuntimeItemSnapshot,
    HunterRuntimeJobSnapshot, HunterRuntimeRidingPetSnapshot, HunterRuntimeSkillSnapshot,
    HunterRuntimeStatusSnapshot, HunterSkillSnapshot, HunterStatusSnapshot, HunterTraitSnapshot,
    HunterWeaponSnapshot, ServiceEffectKind, HUNT_TICKS_TO_RETURN, MAX_GEAR_ENHANCEMENT_LEVEL,
};

pub(super) fn hunter_roster_member(
    hunter: &DurableHunterState,
    roster_state: &'static str,
    position: usize,
) -> HunterRosterMemberSnapshot {
    let experience = hunter
        .profile
        .xp_to_next_level
        .map(|maximum| HunterProgressSnapshot {
            current: hunter.profile.xp,
            maximum,
        });
    let progress = |value: crate::simulation::DurableHunterProgress| HunterProgressSnapshot {
        current: u64::from(value.current),
        maximum: u64::from(value.maximum),
    };
    HunterRosterMemberSnapshot {
        hunter_id: hunter.hunter_id,
        display_name: hunter.profile.display_name.clone(),
        portrait_asset_id: hunter.profile.portrait_asset_id.clone(),
        class_id: hunter.profile.class_id.clone(),
        class_name: hunter.profile.class_name.clone(),
        class_family: hunter.profile.visual_family.clone(),
        rarity_id: hunter.profile.rarity_id.clone(),
        rarity_name: hunter.profile.rarity_name.clone(),
        level: hunter.profile.level,
        xp: hunter.profile.xp,
        gold: hunter.gold,
        current_hp: hunter.current_hp,
        max_hp: hunter.max_hp,
        stamina: hunter.stamina.current,
        max_stamina: hunter.stamina.maximum,
        satiety: hunter.satiety.current,
        max_satiety: hunter.satiety.maximum,
        mood: hunter.mood.current,
        max_mood: hunter.mood.maximum,
        attack: hunter
            .profile
            .attack
            .saturating_add(u64::from(hunter.equipped_weapon_attack_damage())),
        defense: hunter.profile.defense,
        action_state: hunter.profile.action_state.clone(),
        animation: hunter.profile.animation_name.clone(),
        trait_name: hunter
            .profile
            .traits
            .iter()
            .find(|hunter_trait| hunter_trait.equipped)
            .map(|hunter_trait| hunter_trait.display_name.clone()),
        traits: hunter
            .profile
            .traits
            .iter()
            .map(|hunter_trait| HunterTraitSnapshot {
                trait_id: hunter_trait.trait_id.clone(),
                display_name: hunter_trait.display_name.clone(),
                icon_path: hunter_trait.icon_path.clone(),
                unlocked_rank: hunter_trait.unlocked_rank,
                equipped: hunter_trait.equipped,
            })
            .collect(),
        skills: hunter
            .profile
            .skills
            .iter()
            .map(|skill| HunterSkillSnapshot {
                skill_id: skill.skill_id.clone(),
                display_name: skill.display_name.clone(),
                icon_path: skill.icon_path.clone(),
                animation_name: skill.animation_name.clone(),
                level: skill.skill_level,
                equipped_slot: skill.equipped_slot,
                ready: skill.ready,
                cooldown_remaining_ms: skill.cooldown_remaining_ms,
            })
            .collect(),
        hunt: HunterHuntSnapshot {
            status: if hunter.hunt.is_idle() {
                "idle".to_owned()
            } else {
                hunter.hunt.status.clone()
            },
            zone_id: hunter.hunt.zone_id.clone(),
            progress_ticks: hunter.hunt.progress_ticks,
            required_ticks: HUNT_TICKS_TO_RETURN,
            loot: hunter
                .hunt
                .loot
                .iter()
                .map(|loot| HunterLootSnapshot {
                    item_id: loot.item_id.clone(),
                    quantity: loot.quantity,
                })
                .collect(),
            ruleset: "web-rebuild-v1-fixture",
        },
        hunter_info: HunterInfoSnapshot {
            characteristic_name: hunter.profile.characteristic_name.clone(),
            locked: hunter.profile.is_locked,
            reincarnation: hunter.profile.reincarnation.map(progress),
            experience,
            status: HunterStatusSnapshot {
                dps_milli: hunter.profile.dps_milli,
                critical_rate_bps: hunter.profile.critical_rate_bps,
                attack_speed_milli: hunter.profile.attack_speed_milli,
                evasion_rate_bps: hunter.profile.evasion_rate_bps,
                awakening: hunter.profile.awakening.map(progress),
            },
            equipment_slots: Some(
                hunter
                    .profile
                    .equipment_slots
                    .iter()
                    .map(|equipment| HunterEquipmentSlotSnapshot {
                        slot_id: equipment.slot_id.clone(),
                        catalog_kind: equipment.catalog_kind.clone(),
                        catalog_index: equipment.catalog_index,
                        display_name: equipment.display_name.clone(),
                        icon_path: Some(equipment.icon_path.clone()),
                        placeholder_icon_path: None,
                        presentation_gender: equipment.presentation_gender.clone(),
                        required_class_id: equipment.required_class_id.clone(),
                        locked: Some(equipment.locked),
                        evidence_state: equipment.evidence_state.clone(),
                    })
                    .collect(),
            ),
            skills: Some(hunter_skill_catalog_preview(hunter)),
            growth: None,
            riding_pet: hunter.profile.riding_pet_state_resolved.then_some(
                HunterRidingPetSnapshot::Empty {
                    mounted: false,
                    can_move_to_ranch: false,
                },
            ),
            materials: Some(
                hunter
                    .hunt
                    .loot
                    .iter()
                    .enumerate()
                    .filter(|(_, loot)| loot.quantity > 0 && loot.item_id.starts_with("material:"))
                    .map(|(order, loot)| HunterMaterialSnapshot {
                        material_id: loot.item_id.clone(),
                        display_name: None,
                        icon_path: drop_icon_path(&loot.item_id),
                        quantity: u64::from(loot.quantity),
                        order: u32::try_from(order).unwrap_or(u32::MAX),
                    })
                    .collect(),
            ),
            weapons: hunter
                .owned_items
                .iter()
                .filter_map(|owned| {
                    let gear_instance_id = owned.gear_instance_id?;
                    let definition = super::super::web_rebuild_gear::rebuild_weapon_definition(
                        &owned.product_id,
                    )?;
                    Some(HunterWeaponSnapshot {
                        gear_instance_id,
                        product_id: owned.product_id.clone(),
                        weapon_id: definition.weapon_id,
                        display_name_en: definition.display_name_en,
                        display_name_vi: definition.display_name_vi,
                        icon_path: definition.icon_path,
                        quality: owned.quality?,
                        attack_damage: owned.primary_stat?,
                        attack_damage_min: definition.attack_damage_min,
                        attack_damage_max: definition.attack_damage_max,
                        enhancement_level: owned.enhancement_level.unwrap_or(0),
                        compatible: definition.visual_family == hunter.profile.visual_family,
                        equipped: hunter.profile.equipment_slots.iter().any(|slot| {
                            slot.slot_id == "weapon"
                                && slot.catalog_kind
                                    == format!("rebuild_weapon_instance:{gear_instance_id}")
                        }),
                        ruleset: owned.ruleset.clone()?,
                    })
                })
                .collect(),
        },
        gear_enhancements: hunter
            .owned_items
            .iter()
            .filter(|owned| owned.quantity > 0 && owned.gear_instance_id.is_some())
            .map(|owned| GearEnhancementSnapshot {
                product_id: owned.product_id.clone(),
                level: owned.enhancement_level,
                max_level: MAX_GEAR_ENHANCEMENT_LEVEL,
                instance_id: owned.gear_instance_id,
                evidence_state: "unresolved",
            })
            .collect(),
        gear_enhancement_task: hunter
            .hunt
            .gear_enhancement
            .as_ref()
            .map(gear_enhancement_task_snapshot),
        runtime_evidence: runtime_evidence_snapshot(hunter),
        roster_state,
        position,
    }
}

pub(super) fn gear_enhancement_task_snapshot(
    task: &DurableGearEnhancementTask,
) -> GearEnhancementTaskSnapshot {
    let status = match task.status {
        GearEnhancementTaskStatus::Traveling => "traveling",
        GearEnhancementTaskStatus::WaitingForInteraction => "waiting_for_interaction",
        GearEnhancementTaskStatus::Configuring => "configuring",
        GearEnhancementTaskStatus::Processing => "processing",
        GearEnhancementTaskStatus::Result => "result",
    };
    let resources = |rows: &[super::super::hunter_roster::DurableHunterLoot]| {
        rows.iter()
            .map(|row| GearEnhancementResourceSnapshot {
                material_id: row.item_id.clone(),
                quantity: row.quantity,
            })
            .collect::<Vec<_>>()
    };
    GearEnhancementTaskSnapshot {
        building_instance_id: task.building_instance_id.clone(),
        status,
        interaction_ready: matches!(
            task.status,
            GearEnhancementTaskStatus::WaitingForInteraction
                | GearEnhancementTaskStatus::Configuring
                | GearEnhancementTaskStatus::Result
        ),
        selected_gear_instance_id: task.selected_gear_instance_id,
        selected_product_id: task.selected_product_id.clone(),
        mode: task.mode.clone(),
        target_level: task.target_level,
        optional_material_ids: task.optional_material_ids.clone(),
        next_attempt_gold_cost: None,
        next_attempt_success_bps: None,
        required_materials: Vec::new(),
        attempts: task
            .attempts
            .iter()
            .map(|attempt| GearEnhancementAttemptSnapshot {
                attempt: attempt.attempt,
                starting_level: attempt.starting_level,
                resulting_level: attempt.resulting_level,
                succeeded: attempt.succeeded,
                gold_spent: attempt.gold_spent,
                materials_spent: resources(&attempt.materials_spent),
            })
            .collect(),
        spent_gold: task.spent_gold,
        spent_materials: resources(&task.spent_materials),
        final_level: task.final_level,
        stop_reason: task.stop_reason.clone(),
        blockers: task.blockers.clone(),
    }
}

pub(super) fn hunter_skill_catalog_preview(
    hunter: &DurableHunterState,
) -> Vec<HunterInfoSkillSnapshot> {
    let rows: [(&str, &str, Option<&str>, &str); 2] = match hunter.profile.class_id.as_str() {
        "h1" => [
            (
                "skill_h1_01",
                "Fury",
                Some("skills/skill_h1_01__1395.png"),
                "Attacks quickly for a certain time and increases Attack Speed.",
            ),
            (
                "skill_h1_02",
                "War Cry",
                Some("skills/skill_h1_02__5620.png"),
                "Charge to enemy and Stun it.",
            ),
        ],
        "h2" => [
            (
                "skill_h2_01",
                "Holy Light",
                None,
                "Hits and provokes nearby enemies with holy power.",
            ),
            (
                "skill_h2_02",
                "Barrier",
                None,
                "Defends against enemy attacks by summoning a barrier.",
            ),
        ],
        "h3" => [
            (
                "skill_h3_01",
                "Multishot",
                None,
                "Rapidly shoots multiple arrows.",
            ),
            (
                "skill_h3_02",
                "Dodge",
                None,
                "Increases Evasion for a certain time.",
            ),
        ],
        "h4" => [
            (
                "skill_h4_01",
                "Thunderbolt",
                None,
                "Lightning inflicts damage to nearby enemies.",
            ),
            (
                "skill_h4_02",
                "Ice Armor",
                None,
                "Enemy who attacked hunter will lose ATK SPD.",
            ),
        ],
        _ => [
            (
                "skill_h5_01",
                "Round Slash",
                None,
                "Swing the spear horizontally to release energy, dealing damage to enemies nearby.",
            ),
            (
                "skill_h5_02",
                "Concentrate",
                None,
                "Concentrate to increase the chance of dealing a critical strike.",
            ),
        ],
    };
    rows.into_iter()
        .map(|(skill_id, display_name, icon, description)| {
            let learned = hunter
                .profile
                .skills
                .iter()
                .find(|skill| skill.skill_id == skill_id);
            HunterInfoSkillSnapshot {
                skill_id: skill_id.to_owned(),
                display_name: display_name.to_owned(),
                icon_path: icon.map(|icon| {
                    format!("/content/releases/evil-hunter-1.411/hunter-assets/ui/{icon}")
                }),
                level: learned.map(|skill| skill.skill_level),
                description: Some(format!(
                    "{description} Catalog definition; learned state remains server-owned."
                )),
                group: Some("Basic Skills".to_owned()),
                unlocked: learned.map(|_| true),
                unlock_requirement: learned
                    .is_none()
                    .then(|| "Learned-state fixture unresolved".to_owned()),
                ready: learned.map(|skill| skill.ready),
                cooldown_remaining_ms: learned.map(|skill| skill.cooldown_remaining_ms),
            }
        })
        .collect()
}

pub(super) fn release_hunter_from_enhancement(hunter: &mut DurableHunterState) {
    hunter.hunt.gear_enhancement = None;
    hunter.profile.action_state = "idle".to_owned();
    hunter.profile.animation_name = "hunter_stay".to_owned();
}

pub(super) fn release_hunter_from_trade(hunter: &mut DurableHunterState) {
    hunter_trade_workflow::HunterTradeWorkflow::release(hunter);
}

pub(super) fn enhancement_task_terminal(task: &DurableGearEnhancementTask) -> bool {
    task.status == GearEnhancementTaskStatus::Result
        || (task.status == GearEnhancementTaskStatus::Configuring && task.stop_reason.is_some())
}

pub(super) fn is_enhancement_action_state(action_state: &str) -> bool {
    matches!(
        action_state,
        "traveling_to_enhancement_forge"
            | "waiting_for_enhancement_interaction"
            | "configuring_enhancement"
    )
}

pub(super) fn runtime_evidence_snapshot(
    hunter: &DurableHunterState,
) -> HunterRuntimeEvidenceSnapshot {
    let runtime = &hunter.runtime;
    let job = match (
        runtime.source_job,
        runtime.source_sub_job,
        runtime.source_third_job,
        runtime.source_fourth_job,
        runtime.source_personality,
    ) {
        (Some(job), Some(sub_job), Some(third_job), Some(fourth_job), Some(personality)) => {
            Some(HunterRuntimeJobSnapshot {
                job,
                sub_job,
                third_job,
                fourth_job,
                personality,
                grade_rank_up: runtime.source_grade_rank_up,
                dark_soul: runtime.source_dark_soul,
                used_dark_soul: runtime.source_used_dark_soul,
                used_job_trait: runtime.source_used_job_trait,
            })
        }
        _ => None,
    };
    let status = runtime
        .status
        .as_ref()
        .map(|status| HunterRuntimeStatusSnapshot {
            maximum_hp: status.hp,
            current_hp: status.now_hp,
            maximum_mood: status.feel,
            current_mood: status.now_feel,
            maximum_satiety: status.hungry,
            current_satiety: status.now_hungry,
            maximum_stamina: status.tire,
            current_stamina: status.now_tire,
            attack: status.damage,
            defense: status.armor,
            critical: status.critical,
            attack_speed: status.attack_speed,
            evasion: status.dodge,
        });
    let skills = runtime.skills.as_ref().map(|skills| {
        skills
            .iter()
            .map(|skill| HunterRuntimeSkillSnapshot {
                source_key: skill.dictionary_key.clone(),
                source_index: skill.source_index,
                skill_definition_index: skill.skill_index,
                cooldown_raw: skill.cool_time,
                level: skill.level,
            })
            .collect()
    });
    let appearance =
        runtime
            .appearance
            .as_ref()
            .map(|appearance| HunterRuntimeAppearanceSnapshot {
                body_index: appearance.body_index,
                costume_index: appearance.costume_index,
                costume_hidden: appearance.costume_hidden,
                fairy_index: appearance.fairy_index,
                fairy_hidden: appearance.fairy_hidden,
                weapon_costume_index: appearance.weapon_costume_index,
                weapon_costume_hidden: appearance.weapon_costume_hidden,
                wing_costume_index: appearance.wing_costume_index,
                wing_costume_hidden: appearance.wing_costume_hidden,
                seal_costume_index: appearance.seal_costume_index,
                seal_costume_hidden: appearance.seal_costume_hidden,
                companion_index: appearance.ramble_pet_index,
                companion_hidden: appearance.ramble_pet_hidden,
                hat_hidden: appearance.hat_hidden,
                costume_hat_hidden: appearance.costume_hat_hidden,
            });
    let inventory = runtime
        .inventory
        .as_ref()
        .map(|inventory| HunterRuntimeInventorySnapshot {
            items: inventory
                .items
                .iter()
                .map(|item| HunterRuntimeItemSnapshot {
                    source_key: item.dictionary_key.clone(),
                    definition_index: item.source_index,
                    count: item.count,
                    reserved_count: item.reservation,
                    is_new: item.new_check,
                    is_infinite: item.infinity_check,
                })
                .collect(),
            gear: inventory
                .gear
                .iter()
                .map(|gear| HunterRuntimeGearSnapshot {
                    source_key: gear.dictionary_key.clone(),
                    definition_index: gear.gear_index,
                    inventory_index: gear.inventory_index,
                    quality: gear.quality,
                    level: gear.level,
                    rating: gear.rating,
                    group: gear.group,
                    is_new: gear.new_check,
                })
                .collect(),
            consumables: inventory
                .consumables
                .iter()
                .map(|consumable| HunterRuntimeConsumableSnapshot {
                    source_key: consumable.dictionary_key.clone(),
                    total_count: consumable.total_count,
                    nested_values_resolved: false,
                })
                .collect(),
        });
    let growth = runtime.growth.as_ref().map(|growth| {
        growth
            .iter()
            .map(|property| HunterRuntimeGrowthSnapshot {
                property_order: property.source_order,
                level: property.property_level,
            })
            .collect()
    });
    let riding_pet = runtime
        .riding_pet
        .as_ref()
        .map(|pet| HunterRuntimeRidingPetSnapshot {
            pasture_index: pet.pasture_index,
            definition_index: pet.source_index,
            master_key: pet.master_index.clone(),
            rating: pet.rating,
            skill_index: pet.skill_index,
            trait_index: pet.trait_index,
            trait_level: pet.trait_level,
            used_soul: pet.use_soul,
            used_growth_stone: pet.use_growth_stone,
            locked: pet.locked,
            gear_values_resolved: false,
        });
    HunterRuntimeEvidenceSnapshot {
        source_key: runtime.source_dictionary_key.clone(),
        source_index: runtime.source_index,
        job: evidence_section(job),
        status: evidence_section(status),
        skills: evidence_section(skills),
        appearance: evidence_section(appearance),
        inventory: evidence_section(inventory),
        growth: evidence_section(growth),
        riding_pet: evidence_section(riding_pet),
    }
}

pub(super) fn evidence_section<T>(value: Option<T>) -> HunterEvidenceSection<T> {
    HunterEvidenceSection {
        evidence_state: if value.is_some() {
            HunterEvidenceState::ValueCaptured
        } else {
            HunterEvidenceState::SchemaConfirmed
        },
        value,
    }
}

pub(super) fn restore_hunter_service_gauge(
    hunter: &mut DurableHunterState,
    effect_kind: ServiceEffectKind,
    amount: u64,
) {
    match effect_kind {
        ServiceEffectKind::Hp => {
            hunter.current_hp = hunter.current_hp.saturating_add(amount).min(hunter.max_hp);
        }
        ServiceEffectKind::Stamina => hunter.stamina.restore(amount),
        ServiceEffectKind::Satiety => hunter.satiety.restore(amount),
        ServiceEffectKind::Mood => hunter.mood.restore(amount),
    }
}
