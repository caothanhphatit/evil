use std::collections::BTreeMap;

use serde_json::Value;

use super::{
    Amount, BuildRow, BuildingLevel, BuildingRegistry, BuildingRegistryLoadError, BuildingSkin,
    BuildingSourceData, Collection, Condition, ConversionOption, EvidenceField, ProductServiceData,
    RandomOutputData, RuntimeResolved, RuntimeState,
};

#[derive(Debug)]
pub struct BuildingContentView {
    pub registry_id: String,
    pub globally_runnable: bool,
    pub buildings: Vec<BuildingContent>,
    pub capabilities: Vec<CapabilityContent>,
    pub items: BTreeMap<String, EconomyItemContent>,
    pub products: BTreeMap<String, EconomyProductContent>,
    pub skins: BTreeMap<String, BuildingSkinContent>,
}

#[derive(Debug)]
pub struct BuildingContent {
    pub id: String,
    pub display_name: String,
    pub category: Option<String>,
    pub source_data: BuildingSourceContent,
    pub base_sprite_asset_id: Option<String>,
    pub build_rows: Vec<BuildingMutationRow>,
    pub levels: Vec<BuildingMutationRow>,
}

#[derive(Debug)]
pub struct BuildingSourceContent {
    pub source_type: i64,
    pub max_build: i64,
    pub grid_size: Vec<i64>,
    pub movable: i64,
    pub visibility: i64,
    pub compatible_skin: i64,
    pub in_building_flag: i64,
    pub possible_remove: i64,
    pub create_build: Vec<i64>,
    pub entry_counts: Vec<i64>,
    pub first_values: Vec<i64>,
    pub second_values: Vec<i64>,
    pub third_values: Vec<i64>,
}

#[derive(Debug)]
pub struct BuildingMutationRow {
    pub level: u8,
    pub costs: Vec<ContentAmount>,
    pub exact_mutation_ready: bool,
    pub required_town_hall_level: Option<u8>,
}

#[derive(Debug)]
pub struct ContentAmount {
    pub item_id: String,
    pub quantity: u64,
}

#[derive(Debug)]
pub struct CapabilityContent {
    pub building_id: String,
    pub kind: String,
    pub static_data_ready: bool,
    pub runnable: bool,
}

#[derive(Debug)]
pub struct EconomyItemContent {
    pub id: String,
    pub internal_name: Option<String>,
    pub display_name: Option<String>,
    pub item_type: Option<String>,
    pub stack_limit: Option<u64>,
    pub buy_price: Option<Vec<ContentAmount>>,
    pub sell_price: Option<Vec<ContentAmount>>,
    pub town_pays_hunter_gold_per_unit: Option<u64>,
    pub hunter_pays_town_gold_by_tier: Option<Vec<u64>>,
}

#[derive(Debug)]
pub struct EconomyProductContent {
    pub id: String,
    pub building_id: Option<String>,
    pub inputs: Option<Vec<ContentAmount>>,
    pub outputs: Option<Vec<ContentAmount>>,
    pub duration_ms: Option<u64>,
    pub sale_price: Option<Vec<ContentAmount>>,
    pub service_data: Option<ProductServiceContent>,
    pub conversion_options: Option<Vec<ConversionOptionContent>>,
    pub random_output: Option<RandomOutputContent>,
    pub exact_mutation_ready: bool,
}

#[derive(Debug)]
pub struct ConversionOptionContent {
    pub input_kind: String,
    pub input_id: String,
    pub input_quantity: u64,
    pub output_stock_quantity: u64,
}

#[derive(Debug)]
pub struct RandomOutputContent {
    pub item_type: String,
    pub grade: u64,
    pub quantity: u64,
    pub rng_ready: bool,
}

#[derive(Debug)]
pub struct BuildingSkinContent {
    pub key: String,
    pub building_id: String,
    pub skin_id: u64,
    pub family: String,
    pub display_name: String,
    pub costs: Vec<ContentAmount>,
    pub required_level: u64,
    pub visibility: i64,
    pub visual: Option<SkinVisualContent>,
}

#[derive(Debug)]
pub struct SkinVisualContent {
    pub asset_key: String,
    pub sprite_prefix: String,
    pub animation_clip_path_id: u64,
    pub animator_controller_path_id: u64,
    pub sprite_frames: Value,
}

#[derive(Debug)]
pub struct ProductServiceContent {
    pub source_type: u64,
    pub required_level: u64,
    pub service_time_ms: u64,
    pub effect_value: u64,
    pub use_money: u64,
    pub completion_counts: Vec<u64>,
    pub required_cash_count: u64,
    pub cash_completion_count: u64,
    pub required_elemental_count: u64,
    pub elemental_completion_count: u64,
}

impl BuildingContentView {
    pub(super) fn try_from_registry(
        registry: &BuildingRegistry,
    ) -> Result<Self, BuildingRegistryLoadError> {
        let mut buildings = Vec::with_capacity(registry.buildings.rows.len());
        for (index, building) in registry.buildings.rows.iter().enumerate() {
            let path = format!("buildings.rows[{index}]");
            let id = resolved_string(&building.build_id, &format!("{path}.buildId"))?;
            let display_name =
                resolved_localized_string(&building.display_name, &format!("{path}.displayName"))?;
            let category = optional_resolved_string(&building.category);
            let source_data =
                building_source_content(&building.source_data, &format!("{path}.sourceData"))?;
            let base_sprite_asset_id =
                optional_resolved_string(&building.visual_binding.sprite_asset_id);
            let build_rows = building
                .build_rows
                .rows
                .iter()
                .enumerate()
                .map(|(row_index, row)| {
                    mutation_row_from_build_row(row, &format!("{path}.buildRows.rows[{row_index}]"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let levels = building
                .levels
                .rows
                .iter()
                .enumerate()
                .map(|(level_index, level)| {
                    mutation_row_from_level(level, &format!("{path}.levels.rows[{level_index}]"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            buildings.push(BuildingContent {
                id,
                display_name,
                category,
                source_data,
                base_sprite_asset_id,
                build_rows,
                levels,
            });
        }
        let capabilities = registry
            .catalogs
            .capabilities
            .rows
            .iter()
            .enumerate()
            .map(|(index, capability)| {
                let path = format!("catalogs.capabilities.rows[{index}]");
                Ok(CapabilityContent {
                    building_id: resolved_string(
                        &capability.building_id,
                        &format!("{path}.buildingId"),
                    )?,
                    kind: resolved_string(&capability.kind, &format!("{path}.kind"))?,
                    static_data_ready: capability.readiness.static_data_ready,
                    runnable: capability.readiness.runnable
                        && capability.readiness.blocking_paths.is_empty()
                        && capability.validate_resolved(&path).is_ok(),
                })
            })
            .collect::<Result<Vec<_>, BuildingRegistryLoadError>>()?;
        let mut items = BTreeMap::new();
        for (index, item) in registry.catalogs.items.rows.iter().enumerate() {
            let path = format!("catalogs.items.rows[{index}]");
            let id = resolved_string(&item.item_id, &format!("{path}.itemId"))?;
            let content = EconomyItemContent {
                id: id.clone(),
                internal_name: optional_resolved_string(&item.internal_name),
                display_name: optional_resolved_localized_string(&item.display_name),
                item_type: optional_resolved_string(&item.item_type),
                stack_limit: optional_resolved_u64(&item.stack_limit),
                buy_price: item.buy_price.as_ref().and_then(|prices| {
                    optional_resolved_amounts(prices, &format!("{path}.buyPrice"))
                }),
                sell_price: item.sell_price.as_ref().and_then(|prices| {
                    optional_resolved_amounts(prices, &format!("{path}.sellPrice"))
                }),
                town_pays_hunter_gold_per_unit: item
                    .directional_economy
                    .as_ref()
                    .and_then(|economy| economy.town_pays_hunter_gold_per_unit.as_ref())
                    .and_then(optional_resolved_u64),
                hunter_pays_town_gold_by_tier: item
                    .directional_economy
                    .as_ref()
                    .and_then(|economy| economy.hunter_pays_town_gold_by_tier.as_ref())
                    .and_then(optional_resolved_u64_array),
            };
            if items.insert(id.clone(), content).is_some() {
                return Err(BuildingRegistryLoadError::DuplicateKey(id));
            }
        }
        let mut products = BTreeMap::new();
        for (index, product) in registry.catalogs.products.rows.iter().enumerate() {
            let path = format!("catalogs.products.rows[{index}]");
            let id = resolved_string(&product.product_id, &format!("{path}.productId"))?;
            let content = EconomyProductContent {
                id: id.clone(),
                building_id: optional_resolved_string(&product.building_id),
                inputs: product.inputs.as_ref().and_then(|inputs| {
                    optional_resolved_amounts(inputs, &format!("{path}.inputs"))
                }),
                outputs: product.outputs.as_ref().and_then(|outputs| {
                    optional_resolved_amounts(outputs, &format!("{path}.outputs"))
                }),
                duration_ms: optional_resolved_u64(&product.duration_ms),
                sale_price: product.sale_price.as_ref().and_then(|prices| {
                    optional_resolved_amounts(prices, &format!("{path}.salePrice"))
                }),
                service_data: product.service_data.as_ref().and_then(|service| {
                    optional_service_content(service, &format!("{path}.serviceData"))
                }),
                conversion_options: product.conversion_options.as_ref().and_then(|options| {
                    optional_conversion_options(options, &format!("{path}.conversionOptions"))
                }),
                random_output: product.random_output.as_ref().and_then(|output| {
                    optional_random_output(output, &format!("{path}.randomOutput"))
                }),
                exact_mutation_ready: product.validate_resolved(&path).is_ok(),
            };
            if products.insert(id.clone(), content).is_some() {
                return Err(BuildingRegistryLoadError::DuplicateKey(id));
            }
        }
        let mut skins = BTreeMap::new();
        for (index, skin) in registry.catalogs.skins.rows.iter().enumerate() {
            let path = format!("catalogs.skins.rows[{index}]");
            let content = building_skin_content(skin, &path)?;
            if skins.insert(content.key.clone(), content).is_some() {
                return Err(BuildingRegistryLoadError::DuplicateKey(skin.key.clone()));
            }
        }
        Ok(Self {
            registry_id: registry.registry_id.clone(),
            globally_runnable: registry.runtime_state == RuntimeState::RuntimeReady
                && registry.release_gate.runnable,
            buildings,
            capabilities,
            items,
            products,
            skins,
        })
    }

    pub fn building(&self, id: &str) -> Option<&BuildingContent> {
        self.buildings.iter().find(|building| building.id == id)
    }

    pub fn capabilities_for<'a>(
        &'a self,
        building_id: &'a str,
    ) -> impl Iterator<Item = &'a CapabilityContent> + 'a {
        self.capabilities
            .iter()
            .filter(move |capability| capability.building_id == building_id)
    }

    pub fn item(&self, id: &str) -> Option<&EconomyItemContent> {
        self.items.get(id)
    }

    pub fn product(&self, id: &str) -> Option<&EconomyProductContent> {
        self.products.get(id)
    }

    pub fn recipes_for_building<'a>(
        &'a self,
        building_id: &'a str,
    ) -> impl Iterator<Item = &'a EconomyProductContent> + 'a {
        self.products.values().filter(move |product| {
            product.building_id.as_deref() == Some(building_id) && product.inputs.is_some()
        })
    }

    pub fn skin(&self, building_id: &str, skin_id: u64) -> Option<&BuildingSkinContent> {
        self.skins.get(&format!("{building_id}:skin_{skin_id}"))
    }
}

fn mutation_row_from_build_row(
    row: &BuildRow,
    path: &str,
) -> Result<BuildingMutationRow, BuildingRegistryLoadError> {
    let required_town_hall_level = town_hall_condition(&row.conditions, path);
    Ok(BuildingMutationRow {
        level: resolved_u8(&row.level, &format!("{path}.level"))?,
        costs: resolved_amounts(&row.costs, &format!("{path}.costs"))?,
        exact_mutation_ready: required_town_hall_level.is_some()
            && resolved_amounts(&row.costs, &format!("{path}.costs")).is_ok(),
        required_town_hall_level,
    })
}

fn mutation_row_from_level(
    row: &BuildingLevel,
    path: &str,
) -> Result<BuildingMutationRow, BuildingRegistryLoadError> {
    let required_town_hall_level = town_hall_condition(&row.conditions, path);
    Ok(BuildingMutationRow {
        level: resolved_u8(&row.level, &format!("{path}.level"))?,
        costs: resolved_amounts(&row.upgrade_costs, &format!("{path}.upgradeCosts"))?,
        exact_mutation_ready: required_town_hall_level.is_some()
            && resolved_amounts(&row.upgrade_costs, &format!("{path}.upgradeCosts")).is_ok(),
        required_town_hall_level,
    })
}

fn town_hall_condition(conditions: &Collection<Condition>, path: &str) -> Option<u8> {
    conditions
        .binding
        .validate_resolved(&format!("{path}.conditions.binding"))
        .ok()?;
    let condition = conditions.rows.first()?;
    if resolved_string(
        &condition.subject_id,
        &format!("{path}.conditions.subjectId"),
    )
    .ok()?
        != "build_1.level"
        || resolved_string(&condition.operator, &format!("{path}.conditions.operator")).ok()?
            != "greater-than-or-equal"
    {
        return None;
    }
    resolved_u8(&condition.operand, &format!("{path}.conditions.operand")).ok()
}

fn resolved_amounts(
    amounts: &Collection<Amount>,
    path: &str,
) -> Result<Vec<ContentAmount>, BuildingRegistryLoadError> {
    amounts
        .binding
        .validate_resolved(&format!("{path}.binding"))?;
    amounts
        .rows
        .iter()
        .enumerate()
        .map(|(index, amount)| {
            let row_path = format!("{path}.rows[{index}]");
            Ok(ContentAmount {
                item_id: resolved_string(&amount.item_id, &format!("{row_path}.itemId"))?,
                quantity: resolved_u64(&amount.quantity, &format!("{row_path}.quantity"))?,
            })
        })
        .collect()
}

fn optional_resolved_amounts(
    amounts: &Collection<Amount>,
    path: &str,
) -> Option<Vec<ContentAmount>> {
    resolved_amounts(amounts, path).ok()
}

fn optional_resolved_string(field: &EvidenceField) -> Option<String> {
    field.validate_resolved("optional").ok()?;
    field.value.as_str().map(str::to_owned)
}

fn optional_resolved_localized_string(field: &EvidenceField) -> Option<String> {
    field.validate_resolved("optional").ok()?;
    field.value.get("en")?.as_str().map(str::to_owned)
}

fn optional_resolved_u64(field: &EvidenceField) -> Option<u64> {
    field.validate_resolved("optional").ok()?;
    field.value.as_u64()
}

fn optional_resolved_u64_array(field: &EvidenceField) -> Option<Vec<u64>> {
    field.validate_resolved("optional").ok()?;
    field.value.as_array()?.iter().map(Value::as_u64).collect()
}

fn optional_service_content(
    service: &ProductServiceData,
    path: &str,
) -> Option<ProductServiceContent> {
    service.validate_resolved(path).ok()?;
    Some(ProductServiceContent {
        source_type: optional_resolved_u64(&service.source_type)?,
        required_level: optional_resolved_u64(&service.required_level)?,
        service_time_ms: optional_resolved_u64(&service.service_time_ms)?,
        effect_value: optional_resolved_u64(&service.effect_value)?,
        use_money: optional_resolved_u64(&service.use_money)?,
        completion_counts: optional_resolved_u64_array(&service.completion_counts)?,
        required_cash_count: optional_resolved_u64(&service.required_cash_count)?,
        cash_completion_count: optional_resolved_u64(&service.cash_completion_count)?,
        required_elemental_count: optional_resolved_u64(&service.required_elemental_count)?,
        elemental_completion_count: optional_resolved_u64(&service.elemental_completion_count)?,
    })
}

fn optional_conversion_options(
    options: &Collection<ConversionOption>,
    path: &str,
) -> Option<Vec<ConversionOptionContent>> {
    options
        .binding
        .validate_resolved(&format!("{path}.binding"))
        .ok()?;
    options
        .rows
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let row_path = format!("{path}.rows[{index}]");
            option.validate_resolved(&row_path).ok()?;
            Some(ConversionOptionContent {
                input_kind: resolved_string(&option.input_kind, &format!("{row_path}.inputKind"))
                    .ok()?,
                input_id: resolved_string(&option.input_id, &format!("{row_path}.inputId")).ok()?,
                input_quantity: resolved_u64(
                    &option.input_quantity,
                    &format!("{row_path}.inputQuantity"),
                )
                .ok()?,
                output_stock_quantity: resolved_u64(
                    &option.output_stock_quantity,
                    &format!("{row_path}.outputStockQuantity"),
                )
                .ok()?,
            })
        })
        .collect()
}

fn optional_random_output(output: &RandomOutputData, path: &str) -> Option<RandomOutputContent> {
    output
        .binding
        .validate_resolved(&format!("{path}.binding"))
        .ok()?;
    Some(RandomOutputContent {
        item_type: resolved_string(&output.item_type, &format!("{path}.itemType")).ok()?,
        grade: resolved_u64(&output.grade, &format!("{path}.grade")).ok()?,
        quantity: resolved_u64(&output.quantity, &format!("{path}.quantity")).ok()?,
        rng_ready: output
            .rng_binding
            .validate_resolved(&format!("{path}.rngBinding"))
            .is_ok(),
    })
}

fn resolved_i64(field: &EvidenceField, path: &str) -> Result<i64, BuildingRegistryLoadError> {
    field.validate_resolved(path)?;
    field
        .value
        .as_i64()
        .ok_or_else(|| BuildingRegistryLoadError::UnresolvedData(path.into()))
}

fn resolved_i64_array(
    field: &EvidenceField,
    path: &str,
) -> Result<Vec<i64>, BuildingRegistryLoadError> {
    field.validate_resolved(path)?;
    field
        .value
        .as_array()
        .ok_or_else(|| BuildingRegistryLoadError::UnresolvedData(path.into()))?
        .iter()
        .map(|value| {
            value
                .as_i64()
                .ok_or_else(|| BuildingRegistryLoadError::UnresolvedData(path.into()))
        })
        .collect()
}

fn building_source_content(
    source: &BuildingSourceData,
    path: &str,
) -> Result<BuildingSourceContent, BuildingRegistryLoadError> {
    source.validate_resolved(path)?;
    Ok(BuildingSourceContent {
        source_type: resolved_i64(&source.source_type, &format!("{path}.sourceType"))?,
        max_build: resolved_i64(&source.max_build, &format!("{path}.maxBuild"))?,
        grid_size: resolved_i64_array(&source.grid_size, &format!("{path}.gridSize"))?,
        movable: resolved_i64(&source.movable, &format!("{path}.movable"))?,
        visibility: resolved_i64(&source.visibility, &format!("{path}.visibility"))?,
        compatible_skin: resolved_i64(&source.compatible_skin, &format!("{path}.compatibleSkin"))?,
        in_building_flag: resolved_i64(
            &source.in_building_flag,
            &format!("{path}.inBuildingFlag"),
        )?,
        possible_remove: resolved_i64(&source.possible_remove, &format!("{path}.possibleRemove"))?,
        create_build: resolved_i64_array(&source.create_build, &format!("{path}.createBuild"))?,
        entry_counts: resolved_i64_array(&source.entry_counts, &format!("{path}.entryCounts"))?,
        first_values: resolved_i64_array(&source.first_values, &format!("{path}.firstValues"))?,
        second_values: resolved_i64_array(&source.second_values, &format!("{path}.secondValues"))?,
        third_values: resolved_i64_array(&source.third_values, &format!("{path}.thirdValues"))?,
    })
}

fn building_skin_content(
    skin: &BuildingSkin,
    path: &str,
) -> Result<BuildingSkinContent, BuildingRegistryLoadError> {
    let visual = if skin
        .visual_binding
        .binding
        .validate_resolved(&format!("{path}.visualBinding.binding"))
        .is_ok()
    {
        Some(SkinVisualContent {
            asset_key: resolved_string(
                &skin.visual_binding.asset_key,
                &format!("{path}.visualBinding.assetKey"),
            )?,
            sprite_prefix: resolved_string(
                &skin.visual_binding.sprite_prefix,
                &format!("{path}.visualBinding.spritePrefix"),
            )?,
            animation_clip_path_id: resolved_u64(
                &skin.visual_binding.animation_clip_path_id,
                &format!("{path}.visualBinding.animationClipPathId"),
            )?,
            animator_controller_path_id: resolved_u64(
                &skin.visual_binding.animator_controller_path_id,
                &format!("{path}.visualBinding.animatorControllerPathId"),
            )?,
            sprite_frames: skin.visual_binding.sprite_frames.value.clone(),
        })
    } else {
        None
    };
    Ok(BuildingSkinContent {
        key: skin.key.clone(),
        building_id: resolved_string(&skin.building_id, &format!("{path}.buildingId"))?,
        skin_id: resolved_u64(&skin.skin_id, &format!("{path}.skinId"))?,
        family: resolved_string(&skin.family, &format!("{path}.family"))?,
        display_name: resolved_localized_string(
            &skin.display_name,
            &format!("{path}.displayName"),
        )?,
        costs: resolved_amounts(&skin.costs, &format!("{path}.costs"))?,
        required_level: resolved_u64(&skin.required_level, &format!("{path}.requiredLevel"))?,
        visibility: resolved_i64(&skin.visibility, &format!("{path}.visibility"))?,
        visual,
    })
}

fn resolved_string(field: &EvidenceField, path: &str) -> Result<String, BuildingRegistryLoadError> {
    field.validate_resolved(path)?;
    field
        .value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| BuildingRegistryLoadError::UnresolvedData(path.into()))
}

fn resolved_localized_string(
    field: &EvidenceField,
    path: &str,
) -> Result<String, BuildingRegistryLoadError> {
    field.validate_resolved(path)?;
    field
        .value
        .get("en")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BuildingRegistryLoadError::UnresolvedData(path.into()))
}

fn resolved_u64(field: &EvidenceField, path: &str) -> Result<u64, BuildingRegistryLoadError> {
    field.validate_resolved(path)?;
    field
        .value
        .as_u64()
        .ok_or_else(|| BuildingRegistryLoadError::UnresolvedData(path.into()))
}

fn resolved_u8(field: &EvidenceField, path: &str) -> Result<u8, BuildingRegistryLoadError> {
    u8::try_from(resolved_u64(field, path)?)
        .map_err(|_| BuildingRegistryLoadError::UnresolvedData(path.into()))
}
