use super::{
    drop_icon_path, hunter_visual_entity, monster_visual_entity, village_hunter_entity_id,
    village_hunter_motion, visual_entity, BaseBuildingId, BindingConfidence,
    CombatPresentationSnapshot, Facing, GearEnhancementTaskStatus, HunterAgentState,
    OriginalFlowSession, OriginalScreen, ServerMessage, TradeMaterialPresentationSnapshot,
    WorldDropProjection, WorldEntityActionState, WorldEntityKind, WorldEntityProjection, WorldMode,
    WorldProjection, BUILDING_CAPABILITY_BLOCKERS,
};

fn service_speech(
    action_state: &str,
    hp: u64,
    max_hp: u64,
    stamina: u64,
    max_stamina: u64,
    satiety: u64,
    max_satiety: u64,
    mood: u64,
    max_mood: u64,
) -> Option<String> {
    let critical = |current: u64, maximum: u64| {
        maximum > 0 && u128::from(current) * 100 < u128::from(maximum) * 10
    };
    match action_state {
        "waiting_for_service" => {
            if critical(hp, max_hp) {
                Some("Bệnh xá hết thuốc rồi… mình chịu thêm được bao lâu đây?".to_owned())
            } else if critical(stamina, max_stamina) {
                Some("Chỉ cần một chiếc giường thôi mà…".to_owned())
            } else if critical(satiety, max_satiety) {
                Some("Bụng đói thế này thì săn kiểu gì đây?".to_owned())
            } else if critical(mood, max_mood) {
                Some("Quán rượu còn gì uống không vậy?".to_owned())
            } else {
                Some("Mình cần ghé dịch vụ một lát…".to_owned())
            }
        }
        _ => None,
    }
}

impl OriginalFlowSession {
    pub(super) fn world_projection(&self) -> WorldProjection {
        WorldProjection {
            mode: match self.state.screen {
                OriginalScreen::Village => WorldMode::Village,
                OriginalScreen::Field => WorldMode::Field,
                OriginalScreen::Boot | OriginalScreen::HunterRoster => WorldMode::Inactive,
            },
            visual_tick: self.visual_tick,
            coordinate_space: "scene_pixels_v1",
            authority_scope: "server_authoritative_simulation",
            entities: self.world_entities(),
            selected_entity_id: self.selected_entity_id.clone(),
            drops: self
                .monster_world
                .fields
                .iter()
                .flat_map(|field| &field.drops)
                .map(|drop| WorldDropProjection {
                    drop_id: drop.drop_id.clone(),
                    item_id: drop.item_id.clone(),
                    quantity: drop.quantity,
                    x: drop.x,
                    y: drop.y,
                    icon_path: drop_icon_path(&drop.item_id),
                })
                .collect(),
            combat_presentations: self
                .monster_world
                .combat_presentations
                .iter()
                .map(|event| CombatPresentationSnapshot {
                    sequence: event.sequence,
                    source_entity_id: event.source_entity_id.clone(),
                    target_entity_id: event.target_entity_id.clone(),
                    kind: event.kind,
                    amount: event.amount,
                })
                .collect(),
        }
    }

    pub(super) fn world_entities(&self) -> Vec<WorldEntityProjection> {
        match self.state.screen {
            OriginalScreen::Village => {
                let mut entities = self
                    .hunter_roster
                    .hunters
                    .iter()
                    .enumerate()
                    .map(|(slot, hunter)| {
                        let mut entity = if let Some(agent) = self
                            .monster_world
                            .hunters
                            .iter()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)
                        {
                            hunter_visual_entity(agent, hunter.current_hp, hunter.max_hp)
                        } else {
                            let motion = village_hunter_motion(self.visual_tick, slot);
                            visual_entity(
                                village_hunter_entity_id(hunter.hunter_id),
                                WorldEntityKind::Hunter,
                                "hunter",
                                "hunter",
                                BindingConfidence::Confirmed,
                                motion.x,
                                motion.y,
                                motion.facing,
                                motion.action_state,
                                motion.animation,
                            )
                        };
                        entity.class_family = Some(hunter.profile.visual_family.clone());
                        entity.speech_label = service_speech(
                            &hunter.profile.action_state,
                            hunter.current_hp,
                            hunter.max_hp,
                            hunter.stamina.current,
                            hunter.stamina.maximum,
                            hunter.satiety.current,
                            hunter.satiety.maximum,
                            hunter.mood.current,
                            hunter.mood.maximum,
                        );
                        entity.loot_label =
                            self.monster_world
                                .hunters
                                .iter()
                                .find(|agent| agent.hunter_id == hunter.hunter_id)
                                .and_then(|agent| {
                                    agent
                                        .loot_item_id
                                        .as_deref()
                                        .map(|item_id| (item_id, agent.loot_quantity))
                                })
                                .and_then(|(item_id, loot_quantity)| {
                                    if item_id == "gold" {
                                        Some(format!("Gold +{loot_quantity}"))
                                    } else {
                                        self.building_content.gameplay.item(item_id).and_then(
                                            |item| {
                                                item.localized_names
                                                    .get("en")
                                                    .cloned()
                                                    .or_else(|| item.internal_name.clone())
                                                    .map(|name| format!("{name} x{loot_quantity}"))
                                            },
                                        )
                                    }
                                });
                        entity.trade_sequence = self
                            .monster_world
                            .hunters
                            .iter()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)
                            .map_or(0, |agent| agent.trade_sequence);
                        entity.trade_gold = self
                            .monster_world
                            .hunters
                            .iter()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)
                            .map_or(0, |agent| agent.trade_gold);
                        entity.trade_materials = self
                            .monster_world
                            .hunters
                            .iter()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)
                            .map(|agent| self.trade_material_presentations(agent))
                            .unwrap_or_default();
                        entity.attack_effect_key =
                            match (entity.action_state, hunter.profile.visual_family.as_str()) {
                                (WorldEntityActionState::Attacking, "H3")
                                    if entity.skill_presentation_key.is_none()
                                        && !entity.animation.ends_with("_skill") =>
                                {
                                    Some("ranger_basic_arrow")
                                }
                                _ => None,
                            };
                        entity.current_hp = Some(hunter.current_hp);
                        entity.maximum_hp = Some(hunter.max_hp);
                        entity.interaction_prompt_key = hunter
                            .hunt
                            .gear_enhancement
                            .as_ref()
                            .filter(|task| {
                                matches!(
                                    task.status,
                                    GearEnhancementTaskStatus::WaitingForInteraction
                                        | GearEnhancementTaskStatus::Configuring
                                        | GearEnhancementTaskStatus::Result
                                )
                            })
                            .map(|_| "hunter_enhancement_ready");
                        entity
                    })
                    .collect::<Vec<_>>();
                entities.push(visual_entity(
                    "village-npc-01",
                    WorldEntityKind::Npc,
                    "npc",
                    "Npc",
                    BindingConfidence::Confirmed,
                    1760,
                    684,
                    Facing::Left,
                    WorldEntityActionState::Idle,
                    "npc_stay",
                ));
                entities.extend(
                    self.monster_world
                        .fields
                        .iter()
                        .flat_map(|field| field.monsters.iter().map(monster_visual_entity)),
                );
                entities
            }
            OriginalScreen::Field => {
                // Field rendering must use the authoritative hunting agents.
                // The former roaming fixture had a different entity id, which
                // hid movement and caused target-bound EXP events to expire.
                let mut entities = self
                    .hunter_roster
                    .hunters
                    .iter()
                    .filter_map(|hunter| {
                        let agent = self
                            .monster_world
                            .hunters
                            .iter()
                            .find(|agent| agent.hunter_id == hunter.hunter_id)?;
                        let mut entity =
                            hunter_visual_entity(agent, hunter.current_hp, hunter.max_hp);
                        entity.class_family = Some(hunter.profile.visual_family.clone());
                        entity.speech_label = service_speech(
                            &hunter.profile.action_state,
                            hunter.current_hp,
                            hunter.max_hp,
                            hunter.stamina.current,
                            hunter.stamina.maximum,
                            hunter.satiety.current,
                            hunter.satiety.maximum,
                            hunter.mood.current,
                            hunter.mood.maximum,
                        );
                        entity.loot_label = agent.loot_item_id.as_deref().and_then(|item_id| {
                            if item_id == "gold" {
                                Some(format!("Gold +{}", agent.loot_quantity))
                            } else {
                                self.building_content
                                    .gameplay
                                    .item(item_id)
                                    .and_then(|item| {
                                        item.localized_names
                                            .get("en")
                                            .cloned()
                                            .or_else(|| item.internal_name.clone())
                                            .map(|name| format!("{name} x{}", agent.loot_quantity))
                                    })
                            }
                        });
                        entity.trade_sequence = agent.trade_sequence;
                        entity.trade_gold = agent.trade_gold;
                        entity.trade_materials = self.trade_material_presentations(agent);
                        entity.current_hp = Some(hunter.current_hp);
                        entity.maximum_hp = Some(hunter.max_hp);
                        Some(entity)
                    })
                    .collect::<Vec<_>>();
                entities.extend(
                    self.monster_world
                        .fields
                        .iter()
                        .flat_map(|field| field.monsters.iter().map(monster_visual_entity)),
                );
                entities
            }
            OriginalScreen::Boot | OriginalScreen::HunterRoster => Vec::new(),
        }
    }

    pub(super) fn trade_material_presentations(
        &self,
        agent: &HunterAgentState,
    ) -> Vec<TradeMaterialPresentationSnapshot> {
        agent
            .trade_materials
            .iter()
            .map(|material| TradeMaterialPresentationSnapshot {
                material_id: material.material_id.clone(),
                display_name: self
                    .building_content
                    .gameplay
                    .item(&material.material_id)
                    .and_then(|item| {
                        item.localized_names
                            .get("vi")
                            .or_else(|| item.localized_names.get("en"))
                            .cloned()
                            .or_else(|| item.internal_name.clone())
                    })
                    .unwrap_or_else(|| material.material_id.clone()),
                quantity: material.quantity,
            })
            .collect()
    }

    pub(super) fn accepted(&self, intent: &str) -> ServerMessage {
        ServerMessage::IntentResult {
            intent: intent.to_owned(),
            accepted: true,
            reason: None,
            snapshot: self.snapshot(),
        }
    }

    pub(super) fn rejected(&self, intent: &str, reason: &str) -> ServerMessage {
        ServerMessage::IntentResult {
            intent: intent.to_owned(),
            accepted: false,
            reason: Some(reason.to_owned()),
            snapshot: self.snapshot(),
        }
    }

    pub(super) fn binding_blocked(&self, intent: &str, blockers: &[&str]) -> ServerMessage {
        ServerMessage::BindingBlocked {
            intent: intent.to_owned(),
            blockers: blockers
                .iter()
                .map(|blocker| (*blocker).to_owned())
                .collect(),
            snapshot: self.snapshot(),
        }
    }

    pub(super) fn capability_blocked(
        &self,
        intent: &str,
        building_id: &str,
        expected_kinds: &[&str],
    ) -> ServerMessage {
        let Ok(building_id) = BaseBuildingId::parse(building_id) else {
            return self.binding_blocked(intent, &["building_base_id_parse"]);
        };
        let matching = self
            .building_content
            .gameplay
            .capabilities_for(&building_id)
            .filter(|capability| {
                expected_kinds.is_empty() || expected_kinds.contains(&capability.kind.as_str())
            })
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return self.binding_blocked(intent, &["building_capability_identity_binding"]);
        }
        if matching.iter().any(|capability| capability.runnable) {
            return self.binding_blocked(intent, &["building_capability_executor_binding"]);
        }
        self.binding_blocked(intent, &BUILDING_CAPABILITY_BLOCKERS)
    }
}
