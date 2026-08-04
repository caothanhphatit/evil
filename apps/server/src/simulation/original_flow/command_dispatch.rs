use uuid::Uuid;

use super::super::{ClientCommand, ServerMessage};
use super::{OriginalFlowSession, OriginalScreen};
use crate::simulation::evidence_policy::{
    MAIL_BLOCKERS, QUEST_BLOCKERS, REWARDED_AD_BLOCKERS, SHOP_BLOCKERS, TOPUP_BLOCKERS,
};

pub(super) struct CommandDispatcher;

impl CommandDispatcher {
    pub(super) fn dispatch(
        session: &mut OriginalFlowSession,
        command: ClientCommand,
        command_id: Uuid,
    ) -> Option<ServerMessage> {
        let message = match command {
            ClientCommand::SubmitFarmReport { .. } => session.rejected(
                "submit_farm_report",
                "farm reports are handled by the queue ingress",
            ),
            ClientCommand::RequestResync => return None,
            ClientCommand::StartBuildingService {
                instance_id,
                hunter_id,
                product_id,
            } => session.start_product_service(&instance_id, hunter_id, &product_id),
            ClientCommand::StartInfirmaryTreatment {
                instance_id,
                hunter_id,
                product_id,
            } => session.start_product_service(&instance_id, hunter_id, &product_id),
            ClientCommand::CompleteBoot => {
                if session.state.screen != OriginalScreen::Boot {
                    session.rejected("complete_boot", "boot_already_completed")
                } else {
                    session.state.boot_completed = true;
                    session.state.screen = OriginalScreen::Village;
                    session.accepted("complete_boot")
                }
            }
            ClientCommand::SelectBottomMenu { menu } => session.select_bottom_menu(menu),
            ClientCommand::NavigateBack => session.navigate_back(),
            ClientCommand::EnterField => session.enter_field(),
            ClientCommand::EnterMonsterMap { map_id } => session.enter_monster_map(&map_id),
            ClientCommand::SetMonsterDensity { level } => session.set_monster_density(level),
            ClientCommand::SetMonsterRegionDensity { region_id, level } => {
                session.set_monster_region_density(&region_id, level)
            }
            ClientCommand::SelectMonsterTarget {
                monster_id,
                hunter_id,
            } => session.select_monster_target(&monster_id, hunter_id),
            ClientCommand::SelectEntity { entity_id } => session.select_entity(&entity_id),
            ClientCommand::ConstructBuilding { building_id } => {
                session.construct_building(&building_id)
            }
            ClientCommand::ConstructBuildingAt {
                building_id,
                grid_x,
                grid_y,
            } => session.construct_building_at(&building_id, grid_x, grid_y),
            ClientCommand::UpgradeBuilding { instance_id } => {
                session.upgrade_building(&instance_id)
            }
            ClientCommand::MoveBuilding {
                instance_id,
                grid_x,
                grid_y,
            } => session.move_building(&instance_id, grid_x, grid_y),
            ClientCommand::UseBuilding { instance_id } => session.use_building(&instance_id),
            ClientCommand::SetMaterialRequest {
                instance_id,
                material_id,
                quantity,
            } => session.set_material_request(&instance_id, &material_id, quantity),
            ClientCommand::CancelMaterialRequest {
                instance_id,
                material_id,
            } => session.cancel_material_request(&instance_id, &material_id),
            ClientCommand::CraftShopItem {
                instance_id,
                recipe_id,
                material_id,
                quantity,
            } => session.craft_shop_item(
                command_id,
                &instance_id,
                &recipe_id,
                material_id.as_deref(),
                quantity,
            ),
            ClientCommand::OpenHunterProgression { .. } => {
                session.accepted("open_hunter_progression")
            }
            ClientCommand::AssignHunterHunt { hunter_id, zone_id } => {
                session.assign_hunter_hunt(command_id, hunter_id, &zone_id)
            }
            ClientCommand::ReturnHunterHunt { hunter_id } => session.apply_hunter_command(
                command_id,
                &format!("return_hunter_hunt:{hunter_id}"),
                "return_hunter_hunt",
                |roster| roster.return_from_hunt(hunter_id),
            ),
            ClientCommand::SellHunterLoot { hunter_id } => {
                session.sell_hunter_loot(command_id, hunter_id)
            }
            ClientCommand::ReviveHunter { hunter_id } => session.apply_hunter_command(
                command_id,
                &format!("revive_hunter:{hunter_id}"),
                "revive_hunter",
                |roster| roster.revive_hunter(hunter_id),
            ),
            ClientCommand::LearnHunterSkill {
                hunter_id,
                skill_id,
            } => session.learn_hunter_skill(command_id, hunter_id, &skill_id),
            ClientCommand::UseHunterSkill {
                hunter_id,
                skill_id,
                target_entity_id,
            } => session
                .use_hunter_skill(
                    command_id,
                    hunter_id,
                    &skill_id,
                    target_entity_id.as_deref(),
                    true,
                )
                .expect("player skill commands always produce a response"),
            ClientCommand::BanishHunter { hunter_id } => {
                session.banish_hunter(command_id, hunter_id)
            }
            ClientCommand::EquipHunterItem { hunter_id, item_id } => {
                session.equip_fixture_item(command_id, hunter_id, item_id)
            }
            ClientCommand::EquipHunterWeapon {
                hunter_id,
                gear_instance_id,
            } => session.equip_rebuild_weapon(command_id, hunter_id, gear_instance_id),
            ClientCommand::StartHunterEnhancement { hunter_id } => {
                session.start_hunter_enhancement(command_id, hunter_id)
            }
            ClientCommand::EnhanceHunterGear {
                hunter_id,
                gear_instance_id,
                mode,
                optional_material_ids,
            } => session.enhance_hunter_gear(
                command_id,
                hunter_id,
                gear_instance_id,
                &mode,
                &optional_material_ids,
            ),
            ClientCommand::ClaimQuestReward { .. } => {
                session.binding_blocked("claim_quest_reward", &QUEST_BLOCKERS)
            }
            ClientCommand::OpenShop { .. } => session.binding_blocked("open_shop", &SHOP_BLOCKERS),
            ClientCommand::PurchaseShopItem {
                hunter_id,
                shop_id,
                product_id,
            } => session.purchase_shop_item(command_id, hunter_id, &shop_id, &product_id),
            ClientCommand::SellShopItem {
                shop_id,
                product_id,
            } => session.sell_shop_item(&shop_id, &product_id),
            ClientCommand::ClaimMail { .. } => {
                session.binding_blocked("claim_mail", &MAIL_BLOCKERS)
            }
            ClientCommand::ClaimRewardedAd { .. } => {
                session.binding_blocked("claim_rewarded_ad", &REWARDED_AD_BLOCKERS)
            }
            ClientCommand::StartTopupPurchase { .. } => {
                session.binding_blocked("start_topup_purchase", &TOPUP_BLOCKERS)
            }
        };
        Some(message)
    }
}
