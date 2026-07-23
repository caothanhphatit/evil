use serde::{Deserialize, Serialize};

use super::{ClientCommand, ServerMessage};

const FIELD_GAMEPLAY_BLOCKERS: [&str; 3] = [
    "field_map_exact_binding",
    "field_monster_gameplay_binding",
    "combat_rules_binding",
];
const HUNTER_PROGRESSION_BLOCKERS: [&str; 3] = [
    "hunter_catalog_binding",
    "starter_stats_binding",
    "progression_rules_binding",
];
const EQUIPMENT_BLOCKERS: [&str; 2] = ["equipment_catalog_binding", "equipment_rules_binding"];
const QUEST_BLOCKERS: [&str; 2] = ["quest_catalog_binding", "quest_reward_binding"];
const SHOP_BLOCKERS: [&str; 2] = ["shop_catalog_binding", "shop_price_binding"];
const MAIL_BLOCKERS: [&str; 2] = ["mail_schema_binding", "mail_grant_binding"];
const REWARDED_AD_BLOCKERS: [&str; 2] = ["ad_placement_binding", "ad_reward_binding"];
const TOPUP_BLOCKERS: [&str; 3] = [
    "product_catalog_binding",
    "provider_receipt_binding",
    "entitlement_rules_binding",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalScreen {
    #[default]
    Boot,
    Village,
    HunterRoster,
    Field,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BottomMenuIntent {
    Build,
    Character,
    Archive,
    Store,
    Raid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldMode {
    Inactive,
    Village,
    Field,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldEntityKind {
    Hunter,
    Npc,
    Monster,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    Right,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OriginalFlowPlayerState {
    pub screen: OriginalScreen,
    pub boot_completed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingConfidence {
    Confirmed,
    StronglyInferred,
    Tentative,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvidenceBinding {
    pub id: &'static str,
    pub confidence: BindingConfidence,
    pub resolved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VillageSnapshot {
    pub source_scene: &'static str,
    pub canvas_nodes: Vec<&'static str>,
    pub world_nodes: Vec<&'static str>,
    pub bottom_menu: Vec<BottomMenuIntent>,
    pub bindings: Vec<EvidenceBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HunterRosterSnapshot {
    pub scene_nodes: Vec<&'static str>,
    pub hunter_spine_source_confirmed: bool,
    pub starter_composition_resolved: bool,
    pub starter_stats_resolved: bool,
    pub bindings: Vec<EvidenceBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldSnapshot {
    pub scene_nodes: Vec<&'static str>,
    pub visual_projection_runnable: bool,
    pub gameplay_runnable: bool,
    pub blockers: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldEntityDescriptor {
    pub entity_id: &'static str,
    pub kind: WorldEntityKind,
    pub asset_bundle_id: &'static str,
    pub source_skeleton_name: &'static str,
    pub role: &'static str,
    pub source_binding: EvidenceBinding,
    pub placement_binding: EvidenceBinding,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldEntityProjection {
    pub descriptor: WorldEntityDescriptor,
    pub x: i32,
    pub y: i32,
    pub facing: Facing,
    pub animation: &'static str,
    pub selectable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldProjection {
    pub mode: WorldMode,
    pub visual_tick: u64,
    pub coordinate_space: &'static str,
    pub authority_scope: &'static str,
    pub entities: Vec<WorldEntityProjection>,
    pub selected_entity_id: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OriginalFlowSnapshot {
    pub screen: OriginalScreen,
    pub content_release_id: &'static str,
    pub content_release_runnable: bool,
    pub flow_order: Vec<OriginalScreen>,
    pub village: VillageSnapshot,
    pub hunter_roster: HunterRosterSnapshot,
    pub field: FieldSnapshot,
    pub world: WorldProjection,
}

#[derive(Debug)]
pub struct OriginalFlowSession {
    state: OriginalFlowPlayerState,
    selected_entity_id: Option<&'static str>,
    visual_tick: u64,
}

impl OriginalFlowSession {
    pub fn from_state(state: OriginalFlowPlayerState) -> Self {
        Self {
            state,
            selected_entity_id: None,
            visual_tick: 0,
        }
    }

    pub fn state(&self) -> &OriginalFlowPlayerState {
        &self.state
    }

    pub fn advance_visual_tick(&mut self) -> Option<OriginalFlowSnapshot> {
        if !matches!(
            self.state.screen,
            OriginalScreen::Village | OriginalScreen::Field
        ) {
            return None;
        }
        self.visual_tick = self.visual_tick.wrapping_add(1);
        Some(self.snapshot())
    }

    pub fn snapshot(&self) -> OriginalFlowSnapshot {
        OriginalFlowSnapshot {
            screen: self.state.screen,
            content_release_id: "original-flow-v1",
            content_release_runnable: false,
            flow_order: vec![
                OriginalScreen::Boot,
                OriginalScreen::Village,
                OriginalScreen::HunterRoster,
                OriginalScreen::Field,
            ],
            village: VillageSnapshot {
                source_scene: "level1",
                canvas_nodes: vec!["UICanvas", "MainCanvas", "WorldCanvas"],
                world_nodes: vec!["MapManager", "BuildGroup", "BottomView"],
                bottom_menu: vec![
                    BottomMenuIntent::Build,
                    BottomMenuIntent::Character,
                    BottomMenuIntent::Archive,
                    BottomMenuIntent::Store,
                    BottomMenuIntent::Raid,
                ],
                bindings: vec![
                    binding("scene.level1", BindingConfidence::Confirmed, true),
                    binding("village.background", BindingConfidence::Tentative, false),
                    binding("village.camera_bounds", BindingConfidence::Unknown, false),
                    binding(
                        "village.building_anchors",
                        BindingConfidence::Unknown,
                        false,
                    ),
                ],
            },
            hunter_roster: HunterRosterSnapshot {
                scene_nodes: vec!["HunterManager", "HunterGroup", "HunterBorder"],
                hunter_spine_source_confirmed: true,
                starter_composition_resolved: false,
                starter_stats_resolved: false,
                bindings: vec![
                    binding("hunter.spine_bundle", BindingConfidence::Confirmed, true),
                    binding(
                        "hunter.roster_ui",
                        BindingConfidence::StronglyInferred,
                        false,
                    ),
                    binding(
                        "hunter.starter_composition",
                        BindingConfidence::Unknown,
                        false,
                    ),
                    binding("hunter.starter_stats", BindingConfidence::Unknown, false),
                ],
            },
            field: FieldSnapshot {
                scene_nodes: vec!["World", "Hunter", "Evil", "HpBar", "StatusGroup"],
                visual_projection_runnable: true,
                gameplay_runnable: false,
                blockers: FIELD_GAMEPLAY_BLOCKERS.to_vec(),
            },
            world: self.world_projection(),
        }
    }

    pub fn handle_command(&mut self, command: ClientCommand) -> Option<ServerMessage> {
        match command {
            ClientCommand::RequestResync => None,
            ClientCommand::CompleteBoot => {
                if self.state.screen != OriginalScreen::Boot {
                    return Some(self.rejected("complete_boot", "boot_already_completed"));
                }
                self.state.boot_completed = true;
                self.state.screen = OriginalScreen::Village;
                Some(self.accepted("complete_boot"))
            }
            ClientCommand::SelectBottomMenu { menu } => Some(self.select_bottom_menu(menu)),
            ClientCommand::NavigateBack => Some(self.navigate_back()),
            ClientCommand::EnterField => Some(self.enter_field()),
            ClientCommand::SelectEntity { entity_id } => Some(self.select_entity(&entity_id)),
            ClientCommand::OpenHunterProgression { .. } => {
                Some(self.binding_blocked("open_hunter_progression", &HUNTER_PROGRESSION_BLOCKERS))
            }
            ClientCommand::EquipHunterItem { .. } => {
                Some(self.binding_blocked("equip_hunter_item", &EQUIPMENT_BLOCKERS))
            }
            ClientCommand::ClaimQuestReward { .. } => {
                Some(self.binding_blocked("claim_quest_reward", &QUEST_BLOCKERS))
            }
            ClientCommand::OpenShop { .. } => {
                Some(self.binding_blocked("open_shop", &SHOP_BLOCKERS))
            }
            ClientCommand::PurchaseShopItem { .. } => {
                Some(self.binding_blocked("purchase_shop_item", &SHOP_BLOCKERS))
            }
            ClientCommand::ClaimMail { .. } => {
                Some(self.binding_blocked("claim_mail", &MAIL_BLOCKERS))
            }
            ClientCommand::ClaimRewardedAd { .. } => {
                Some(self.binding_blocked("claim_rewarded_ad", &REWARDED_AD_BLOCKERS))
            }
            ClientCommand::StartTopupPurchase { .. } => {
                Some(self.binding_blocked("start_topup_purchase", &TOPUP_BLOCKERS))
            }
        }
    }

    fn select_bottom_menu(&mut self, menu: BottomMenuIntent) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("select_bottom_menu", "bottom_menu_unavailable");
        }
        match menu {
            BottomMenuIntent::Character => {
                self.state.screen = OriginalScreen::HunterRoster;
                self.selected_entity_id = None;
                self.accepted("select_bottom_menu.character")
            }
            BottomMenuIntent::Build => {
                self.binding_blocked("select_bottom_menu.build", &["building_bindings"])
            }
            BottomMenuIntent::Archive => {
                self.binding_blocked("select_bottom_menu.archive", &["archive_rules_binding"])
            }
            BottomMenuIntent::Store => {
                self.binding_blocked("select_bottom_menu.store", &["store_catalog_binding"])
            }
            BottomMenuIntent::Raid => {
                self.binding_blocked("select_bottom_menu.raid", &["raid_rules_binding"])
            }
        }
    }

    fn navigate_back(&mut self) -> ServerMessage {
        match self.state.screen {
            OriginalScreen::HunterRoster | OriginalScreen::Field => {
                self.state.screen = OriginalScreen::Village;
                self.selected_entity_id = None;
                self.accepted("navigate_back")
            }
            _ => self.rejected("navigate_back", "navigation_unavailable"),
        }
    }

    fn enter_field(&mut self) -> ServerMessage {
        if self.state.screen != OriginalScreen::Village {
            return self.rejected("enter_field", "field_entry_unavailable");
        }
        self.state.screen = OriginalScreen::Field;
        self.selected_entity_id = None;
        self.accepted("enter_field")
    }

    fn select_entity(&mut self, entity_id: &str) -> ServerMessage {
        let selected = self
            .world_entities()
            .into_iter()
            .find(|entity| entity.descriptor.entity_id == entity_id && entity.selectable)
            .map(|entity| entity.descriptor.entity_id);
        let Some(selected) = selected else {
            return self.rejected("select_entity", "entity_unavailable");
        };
        self.selected_entity_id = Some(selected);
        self.accepted("select_entity")
    }

    fn world_projection(&self) -> WorldProjection {
        WorldProjection {
            mode: match self.state.screen {
                OriginalScreen::Village => WorldMode::Village,
                OriginalScreen::Field => WorldMode::Field,
                OriginalScreen::Boot | OriginalScreen::HunterRoster => WorldMode::Inactive,
            },
            visual_tick: self.visual_tick,
            coordinate_space: "normalized_viewport_1000",
            authority_scope: "visual_roaming_only",
            entities: self.world_entities(),
            selected_entity_id: self.selected_entity_id,
        }
    }

    fn world_entities(&self) -> Vec<WorldEntityProjection> {
        match self.state.screen {
            OriginalScreen::Village => vec![
                visual_entity(
                    "village-hunter-01",
                    WorldEntityKind::Hunter,
                    "hunter",
                    "hunter",
                    BindingConfidence::Confirmed,
                    roam(self.visual_tick, 300, 440, 80).0,
                    665,
                    roam(self.visual_tick, 300, 440, 80).1,
                    "hunter_walk",
                ),
                visual_entity(
                    "village-npc-01",
                    WorldEntityKind::Npc,
                    "npc",
                    "Npc",
                    BindingConfidence::Confirmed,
                    625,
                    625,
                    Facing::Left,
                    "npc_stay",
                ),
            ],
            OriginalScreen::Field => vec![
                visual_entity(
                    "field-hunter-01",
                    WorldEntityKind::Hunter,
                    "hunter",
                    "hunter",
                    BindingConfidence::Confirmed,
                    roam(self.visual_tick, 235, 390, 90).0,
                    650,
                    roam(self.visual_tick, 235, 390, 90).1,
                    "hunter_walk",
                ),
                visual_entity(
                    "field-monster-candidate-01",
                    WorldEntityKind::Monster,
                    "mon_goldblin",
                    "mon_goldblin",
                    BindingConfidence::Confirmed,
                    roam(self.visual_tick + 37, 610, 780, 110).0,
                    650,
                    roam(self.visual_tick + 37, 610, 780, 110).1,
                    "walk",
                ),
            ],
            OriginalScreen::Boot | OriginalScreen::HunterRoster => Vec::new(),
        }
    }

    fn accepted(&self, intent: &str) -> ServerMessage {
        ServerMessage::IntentResult {
            intent: intent.to_owned(),
            accepted: true,
            reason: None,
            snapshot: self.snapshot(),
        }
    }

    fn rejected(&self, intent: &str, reason: &str) -> ServerMessage {
        ServerMessage::IntentResult {
            intent: intent.to_owned(),
            accepted: false,
            reason: Some(reason.to_owned()),
            snapshot: self.snapshot(),
        }
    }

    fn binding_blocked(&self, intent: &str, blockers: &[&str]) -> ServerMessage {
        ServerMessage::BindingBlocked {
            intent: intent.to_owned(),
            blockers: blockers
                .iter()
                .map(|blocker| (*blocker).to_owned())
                .collect(),
            snapshot: self.snapshot(),
        }
    }
}

fn binding(id: &'static str, confidence: BindingConfidence, resolved: bool) -> EvidenceBinding {
    EvidenceBinding {
        id,
        confidence,
        resolved,
    }
}

#[allow(clippy::too_many_arguments)]
fn visual_entity(
    entity_id: &'static str,
    kind: WorldEntityKind,
    asset_bundle_id: &'static str,
    source_skeleton_name: &'static str,
    source_confidence: BindingConfidence,
    x: i32,
    y: i32,
    facing: Facing,
    animation: &'static str,
) -> WorldEntityProjection {
    WorldEntityProjection {
        descriptor: WorldEntityDescriptor {
            entity_id,
            kind,
            asset_bundle_id,
            source_skeleton_name,
            role: "migration_visual_candidate",
            source_binding: binding("actor.spine_bundle", source_confidence, true),
            // Exact legacy spawn coordinates are still unavailable; these anchors are presentation-only.
            placement_binding: binding("actor.world_placement", BindingConfidence::Unknown, false),
        },
        x,
        y,
        facing,
        animation,
        selectable: true,
    }
}

fn roam(tick: u64, min: i32, max: i32, period: u64) -> (i32, Facing) {
    let span = (max - min).max(1) as u64;
    let phase = tick % (period * 2);
    let offset = if phase <= period {
        phase * span / period
    } else {
        (period * 2 - phase) * span / period
    };
    let facing = if phase < period {
        Facing::Right
    } else {
        Facing::Left
    };
    (min + offset as i32, facing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_flow_reaches_village_and_roster_without_fixture_combat() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState::default());
        flow.handle_command(ClientCommand::CompleteBoot);
        assert_eq!(flow.snapshot().screen, OriginalScreen::Village);

        flow.handle_command(ClientCommand::SelectBottomMenu {
            menu: BottomMenuIntent::Character,
        });
        assert_eq!(flow.snapshot().screen, OriginalScreen::HunterRoster);
    }

    #[test]
    fn field_entry_projects_visual_entities_without_enabling_gameplay() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        });
        let message = flow
            .handle_command(ClientCommand::EnterField)
            .expect("field intent returns a result");
        let ServerMessage::IntentResult {
            accepted, snapshot, ..
        } = message
        else {
            panic!("field navigation should be accepted");
        };
        assert!(accepted);
        assert_eq!(snapshot.screen, OriginalScreen::Field);
        assert_eq!(snapshot.world.mode, WorldMode::Field);
        assert_eq!(snapshot.world.authority_scope, "visual_roaming_only");
        assert_eq!(snapshot.world.entities.len(), 2);
        assert!(snapshot.field.visual_projection_runnable);
        assert!(!snapshot.field.gameplay_runnable);
        assert_eq!(snapshot.field.blockers, FIELD_GAMEPLAY_BLOCKERS);
        assert!(snapshot.world.entities.iter().all(|entity| {
            !matches!(entity.animation, "atk" | "die" | "dying")
                && !entity.descriptor.placement_binding.resolved
        }));
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
        assert!(matches!(
            selected,
            ServerMessage::IntentResult { accepted: true, .. }
        ));
        assert_eq!(
            flow.snapshot().world.selected_entity_id,
            Some("village-npc-01")
        );
        assert_eq!(flow.state(), &state);

        let rejected = flow
            .handle_command(ClientCommand::SelectEntity {
                entity_id: "client-invented-entity".to_owned(),
            })
            .expect("selection rejection");
        assert!(matches!(
            rejected,
            ServerMessage::IntentResult {
                accepted: false,
                ..
            }
        ));
        assert_eq!(
            flow.snapshot().world.selected_entity_id,
            Some("village-npc-01")
        );
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
    fn visual_tick_moves_entities_without_changing_durable_state() {
        let state = OriginalFlowPlayerState {
            screen: OriginalScreen::Field,
            boot_completed: true,
        };
        let mut flow = OriginalFlowSession::from_state(state.clone());
        let before = flow.snapshot();
        let after = flow.advance_visual_tick().expect("active world tick");
        assert_eq!(after.world.visual_tick, before.world.visual_tick + 1);
        assert_ne!(after.world.entities, before.world.entities);
        assert_eq!(flow.state(), &state);
    }

    #[test]
    fn unresolved_bottom_menu_does_not_change_screen() {
        let mut flow = OriginalFlowSession::from_state(OriginalFlowPlayerState {
            screen: OriginalScreen::Village,
            boot_completed: true,
        });
        let message = flow
            .handle_command(ClientCommand::SelectBottomMenu {
                menu: BottomMenuIntent::Store,
            })
            .expect("store intent returns a result");
        assert!(matches!(message, ServerMessage::BindingBlocked { .. }));
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
            ClientCommand::EquipHunterItem {
                hunter_id: 1,
                item_id: 2001,
            },
            ClientCommand::ClaimQuestReward {
                quest_id: "quest-1".to_owned(),
            },
            ClientCommand::OpenShop {
                shop_id: "main".to_owned(),
            },
            ClientCommand::PurchaseShopItem {
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
            let message = flow.handle_command(command).expect("intent result");
            assert!(matches!(message, ServerMessage::BindingBlocked { .. }));
            assert_eq!(flow.state().screen, OriginalScreen::Village);
        }
    }
}
