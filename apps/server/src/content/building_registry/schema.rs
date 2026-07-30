use serde::Deserialize;

use super::{
    schema_common::{validate_fields, Binding, Collection, EvidenceField, RuntimeResolved},
    BuildingRegistryLoadError,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Condition {
    pub key: String,
    pub kind: EvidenceField,
    pub subject_id: EvidenceField,
    pub operator: EvidenceField,
    pub operand: EvidenceField,
}

impl RuntimeResolved for Condition {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("kind", &self.kind),
                ("subjectId", &self.subject_id),
                ("operator", &self.operator),
                ("operand", &self.operand),
            ],
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Amount {
    pub key: String,
    pub item_id: EvidenceField,
    pub quantity: EvidenceField,
}

impl RuntimeResolved for Amount {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [("itemId", &self.item_id), ("quantity", &self.quantity)],
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub key: String,
    pub id: EvidenceField,
}

impl RuntimeResolved for Reference {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        self.id.validate_resolved(&format!("{path}.id"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub key: String,
    pub item_id: EvidenceField,
    pub internal_name: EvidenceField,
    pub display_name: EvidenceField,
    pub item_type: EvidenceField,
    pub stack_limit: EvidenceField,
    pub buy_price: Option<Collection<Amount>>,
    pub sell_price: Option<Collection<Amount>>,
    pub directional_economy: Option<ItemDirectionalEconomy>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDirectionalEconomy {
    pub binding: Binding,
    pub town_pays_hunter_gold_per_unit: Option<EvidenceField>,
    pub hunter_pays_town_gold_by_tier: Option<EvidenceField>,
}

impl RuntimeResolved for Item {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("itemId", &self.item_id),
                ("internalName", &self.internal_name),
                ("displayName", &self.display_name),
                ("itemType", &self.item_type),
                ("stackLimit", &self.stack_limit),
            ],
        )?;
        if let Some(buy_price) = &self.buy_price {
            buy_price.validate_resolved(&format!("{path}.buyPrice"))?;
        }
        if let Some(sell_price) = &self.sell_price {
            sell_price.validate_resolved(&format!("{path}.sellPrice"))?;
        }
        if let Some(economy) = &self.directional_economy {
            economy
                .binding
                .validate_resolved(&format!("{path}.directionalEconomy.binding"))?;
            if let Some(field) = &economy.town_pays_hunter_gold_per_unit {
                field.validate_resolved(&format!(
                    "{path}.directionalEconomy.townPaysHunterGoldPerUnit"
                ))?;
            }
            if let Some(field) = &economy.hunter_pays_town_gold_by_tier {
                field.validate_resolved(&format!(
                    "{path}.directionalEconomy.hunterPaysTownGoldByTier"
                ))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Product {
    pub key: String,
    pub product_id: EvidenceField,
    pub building_id: EvidenceField,
    pub inputs: Option<Collection<Amount>>,
    pub outputs: Option<Collection<Amount>>,
    pub duration_ms: EvidenceField,
    pub sale_price: Option<Collection<Amount>>,
    pub conditions: Collection<Condition>,
    pub service_data: Option<ProductServiceData>,
    pub conversion_options: Option<Collection<ConversionOption>>,
    pub random_output: Option<RandomOutputData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConversionOption {
    pub key: String,
    pub input_kind: EvidenceField,
    pub input_id: EvidenceField,
    pub input_quantity: EvidenceField,
    pub output_stock_quantity: EvidenceField,
}

impl RuntimeResolved for ConversionOption {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("inputKind", &self.input_kind),
                ("inputId", &self.input_id),
                ("inputQuantity", &self.input_quantity),
                ("outputStockQuantity", &self.output_stock_quantity),
            ],
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RandomOutputData {
    pub binding: Binding,
    pub item_type: EvidenceField,
    pub grade: EvidenceField,
    pub quantity: EvidenceField,
    pub rng_binding: Binding,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductServiceData {
    pub binding: Binding,
    pub source_type: EvidenceField,
    pub required_level: EvidenceField,
    pub service_time_ms: EvidenceField,
    pub effect_value: EvidenceField,
    pub use_money: EvidenceField,
    pub completion_counts: EvidenceField,
    pub required_cash_count: EvidenceField,
    pub cash_completion_count: EvidenceField,
    pub required_elemental_count: EvidenceField,
    pub elemental_completion_count: EvidenceField,
}

impl RuntimeResolved for ProductServiceData {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        self.binding.validate_resolved(&format!("{path}.binding"))?;
        validate_fields(
            path,
            [
                ("sourceType", &self.source_type),
                ("requiredLevel", &self.required_level),
                ("serviceTimeMs", &self.service_time_ms),
                ("effectValue", &self.effect_value),
                ("useMoney", &self.use_money),
                ("completionCounts", &self.completion_counts),
                ("requiredCashCount", &self.required_cash_count),
                ("cashCompletionCount", &self.cash_completion_count),
                ("requiredElementalCount", &self.required_elemental_count),
                ("elementalCompletionCount", &self.elemental_completion_count),
            ],
        )
    }
}

impl RuntimeResolved for Product {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("productId", &self.product_id),
                ("buildingId", &self.building_id),
                ("durationMs", &self.duration_ms),
            ],
        )?;
        if let Some(inputs) = &self.inputs {
            inputs.validate_resolved(&format!("{path}.inputs"))?;
        }
        if let Some(outputs) = &self.outputs {
            outputs.validate_resolved(&format!("{path}.outputs"))?;
        }
        if let Some(sale_price) = &self.sale_price {
            sale_price.validate_resolved(&format!("{path}.salePrice"))?;
        }
        self.conditions
            .validate_resolved(&format!("{path}.conditions"))?;
        if let Some(service_data) = &self.service_data {
            service_data.validate_resolved(&format!("{path}.serviceData"))?;
        }
        if let Some(conversion_options) = &self.conversion_options {
            conversion_options.validate_resolved(&format!("{path}.conversionOptions"))?;
        }
        if let Some(random_output) = &self.random_output {
            random_output
                .binding
                .validate_resolved(&format!("{path}.randomOutput.binding"))?;
            validate_fields(
                &format!("{path}.randomOutput"),
                [
                    ("itemType", &random_output.item_type),
                    ("grade", &random_output.grade),
                    ("quantity", &random_output.quantity),
                ],
            )?;
            random_output
                .rng_binding
                .validate_resolved(&format!("{path}.randomOutput.rngBinding"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capability {
    pub key: String,
    pub capability_id: EvidenceField,
    pub building_id: EvidenceField,
    pub kind: EvidenceField,
    pub parameters: EvidenceField,
    pub popup_template_id: EvidenceField,
    pub popup_binding: Binding,
    pub runtime_binding: Binding,
    pub conditions: Collection<Condition>,
    pub readiness: CapabilityReadiness,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityReadiness {
    pub static_data_ready: bool,
    pub runnable: bool,
    pub blocking_paths: Vec<String>,
    pub reason: String,
}

impl RuntimeResolved for Capability {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("capabilityId", &self.capability_id),
                ("buildingId", &self.building_id),
                ("kind", &self.kind),
                ("parameters", &self.parameters),
                ("popupTemplateId", &self.popup_template_id),
            ],
        )?;
        self.popup_binding
            .validate_resolved(&format!("{path}.popupBinding"))?;
        self.runtime_binding
            .validate_resolved(&format!("{path}.runtimeBinding"))?;
        self.conditions
            .validate_resolved(&format!("{path}.conditions"))?;
        if !self.readiness.runnable || !self.readiness.blocking_paths.is_empty() {
            return Err(BuildingRegistryLoadError::UnresolvedData(format!(
                "{path}.readiness"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildRow {
    pub key: String,
    pub source_row_id: EvidenceField,
    pub build_id: EvidenceField,
    pub level: EvidenceField,
    pub conditions: Collection<Condition>,
    pub costs: Collection<Amount>,
    pub duration_ms: EvidenceField,
}

impl RuntimeResolved for BuildRow {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("sourceRowId", &self.source_row_id),
                ("buildId", &self.build_id),
                ("level", &self.level),
                ("durationMs", &self.duration_ms),
            ],
        )?;
        self.conditions
            .validate_resolved(&format!("{path}.conditions"))?;
        self.costs.validate_resolved(&format!("{path}.costs"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildingLevel {
    pub key: String,
    pub level: EvidenceField,
    pub conditions: Collection<Condition>,
    pub upgrade_costs: Collection<Amount>,
    pub upgrade_duration_ms: EvidenceField,
    pub inventory_capacity: EvidenceField,
    pub production_slots: EvidenceField,
    pub capability_ids: Collection<Reference>,
    pub product_ids: Collection<Reference>,
}

impl RuntimeResolved for BuildingLevel {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("level", &self.level),
                ("upgradeDurationMs", &self.upgrade_duration_ms),
                ("inventoryCapacity", &self.inventory_capacity),
                ("productionSlots", &self.production_slots),
            ],
        )?;
        self.conditions
            .validate_resolved(&format!("{path}.conditions"))?;
        self.upgrade_costs
            .validate_resolved(&format!("{path}.upgradeCosts"))?;
        self.capability_ids
            .validate_resolved(&format!("{path}.capabilityIds"))?;
        self.product_ids
            .validate_resolved(&format!("{path}.productIds"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeRule {
    pub key: String,
    pub item_id: EvidenceField,
    pub direction: EvidenceField,
    pub unit_price: Collection<Amount>,
    pub quantity_limit: EvidenceField,
    pub conditions: Collection<Condition>,
}

impl RuntimeResolved for TradeRule {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("itemId", &self.item_id),
                ("direction", &self.direction),
                ("quantityLimit", &self.quantity_limit),
            ],
        )?;
        self.unit_price
            .validate_resolved(&format!("{path}.unitPrice"))?;
        self.conditions
            .validate_resolved(&format!("{path}.conditions"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualBinding {
    pub binding: Binding,
    pub sprite_asset_id: EvidenceField,
    pub controller_class: EvidenceField,
    pub popup_class: EvidenceField,
    pub town_position: EvidenceField,
    pub sorting: EvidenceField,
    pub collider: EvidenceField,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildingSkin {
    pub key: String,
    pub building_id: EvidenceField,
    pub skin_id: EvidenceField,
    pub family: EvidenceField,
    pub display_name: EvidenceField,
    pub costs: Collection<Amount>,
    pub required_level: EvidenceField,
    pub visibility: EvidenceField,
    pub visual_binding: SkinVisualBinding,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkinVisualBinding {
    pub binding: Binding,
    pub asset_key: EvidenceField,
    pub sprite_prefix: EvidenceField,
    pub animation_clip_path_id: EvidenceField,
    pub animator_controller_path_id: EvidenceField,
    pub sprite_frames: EvidenceField,
}

impl RuntimeResolved for BuildingSkin {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("buildingId", &self.building_id),
                ("skinId", &self.skin_id),
                ("family", &self.family),
                ("displayName", &self.display_name),
                ("requiredLevel", &self.required_level),
                ("visibility", &self.visibility),
            ],
        )?;
        self.costs.validate_resolved(&format!("{path}.costs"))?;
        self.visual_binding
            .binding
            .validate_resolved(&format!("{path}.visualBinding.binding"))?;
        validate_fields(
            &format!("{path}.visualBinding"),
            [
                ("assetKey", &self.visual_binding.asset_key),
                ("spritePrefix", &self.visual_binding.sprite_prefix),
                (
                    "animationClipPathId",
                    &self.visual_binding.animation_clip_path_id,
                ),
                (
                    "animatorControllerPathId",
                    &self.visual_binding.animator_controller_path_id,
                ),
                ("spriteFrames", &self.visual_binding.sprite_frames),
            ],
        )
    }
}

impl RuntimeResolved for VisualBinding {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        self.binding.validate_resolved(&format!("{path}.binding"))?;
        validate_fields(
            path,
            [
                ("spriteAssetId", &self.sprite_asset_id),
                ("controllerClass", &self.controller_class),
                ("popupClass", &self.popup_class),
                ("townPosition", &self.town_position),
                ("sorting", &self.sorting),
                ("collider", &self.collider),
            ],
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Building {
    pub key: String,
    pub build_id: EvidenceField,
    pub internal_name: EvidenceField,
    pub display_name: EvidenceField,
    pub category: EvidenceField,
    pub source_data: BuildingSourceData,
    pub build_rows: Collection<BuildRow>,
    pub levels: Collection<BuildingLevel>,
    pub trade_rules: Collection<TradeRule>,
    pub product_ids: Collection<Reference>,
    pub capability_ids: Collection<Reference>,
    pub visual_binding: VisualBinding,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildingSourceData {
    pub binding: Binding,
    pub source_type: EvidenceField,
    pub max_build: EvidenceField,
    pub grid_size: EvidenceField,
    pub movable: EvidenceField,
    pub visibility: EvidenceField,
    pub compatible_skin: EvidenceField,
    pub in_building_flag: EvidenceField,
    pub possible_remove: EvidenceField,
    pub create_build: EvidenceField,
    pub entry_counts: EvidenceField,
    pub first_values: EvidenceField,
    pub second_values: EvidenceField,
    pub third_values: EvidenceField,
}

impl RuntimeResolved for BuildingSourceData {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        self.binding.validate_resolved(&format!("{path}.binding"))?;
        validate_fields(
            path,
            [
                ("sourceType", &self.source_type),
                ("maxBuild", &self.max_build),
                ("gridSize", &self.grid_size),
                ("movable", &self.movable),
                ("visibility", &self.visibility),
                ("compatibleSkin", &self.compatible_skin),
                ("inBuildingFlag", &self.in_building_flag),
                ("possibleRemove", &self.possible_remove),
                ("createBuild", &self.create_build),
                ("entryCounts", &self.entry_counts),
                ("firstValues", &self.first_values),
                ("secondValues", &self.second_values),
                ("thirdValues", &self.third_values),
            ],
        )
    }
}

impl RuntimeResolved for Building {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        validate_fields(
            path,
            [
                ("buildId", &self.build_id),
                ("internalName", &self.internal_name),
                ("displayName", &self.display_name),
                ("category", &self.category),
            ],
        )?;
        self.source_data
            .validate_resolved(&format!("{path}.sourceData"))?;
        self.build_rows
            .validate_resolved(&format!("{path}.buildRows"))?;
        self.levels.validate_resolved(&format!("{path}.levels"))?;
        self.trade_rules
            .validate_resolved(&format!("{path}.tradeRules"))?;
        self.product_ids
            .validate_resolved(&format!("{path}.productIds"))?;
        self.capability_ids
            .validate_resolved(&format!("{path}.capabilityIds"))?;
        self.visual_binding
            .validate_resolved(&format!("{path}.visualBinding"))
    }
}
