pub(crate) const QUEST_BLOCKERS: [&str; 2] = ["quest_catalog_binding", "quest_reward_binding"];
pub(crate) const SHOP_BLOCKERS: [&str; 2] = ["shop_catalog_binding", "shop_price_binding"];
pub(crate) const MAIL_BLOCKERS: [&str; 2] = ["mail_schema_binding", "mail_grant_binding"];
pub(crate) const REWARDED_AD_BLOCKERS: [&str; 2] = ["ad_placement_binding", "ad_reward_binding"];
pub(crate) const TOPUP_BLOCKERS: [&str; 3] = [
    "product_catalog_binding",
    "provider_receipt_binding",
    "entitlement_rules_binding",
];
pub(crate) const BUILDING_CAPABILITY_BLOCKERS: [&str; 2] = [
    "building_capability_dispatch_binding",
    "building_economy_settlement_binding",
];
pub(crate) const GEAR_ENHANCEMENT_BLOCKERS: [&str; 3] = [
    "enhancement_cost_binding",
    "enhancement_probability_binding",
    "enhancement_material_binding",
];
