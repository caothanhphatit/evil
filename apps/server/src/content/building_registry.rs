use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

const CONTRACT_TYPE: &str = "building-registry";
const LEGACY_GAME: &str = "Evil Hunter Tycoon";
const LEGACY_VERSION: &str = "1.411";
const LEGACY_PACKAGE: &str = "com.superplanet.evilhunter";
pub(crate) const EMBEDDED_REGISTRY_SHA256: &str =
    "a262f6f452aa5d88b74bb8b3b739e3564c57d3cd1bcf88d36b4f7712f72e210e";
const EMBEDDED_REGISTRY: &[u8] = include_bytes!(
    "../../../../packages/content/releases/evil-hunter-1.411/building-registry.json"
);

static BUILDING_CONTENT: OnceLock<Result<BuildingContentView, String>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum BuildingRegistryLoadError {
    #[error("could not read building registry {path}: {source}")]
    ReadRegistry {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("building registry payload hash is malformed")]
    MalformedExpectedHash,
    #[error("building registry payload hash mismatch: expected {expected}, found {actual}")]
    RegistryHashMismatch { expected: String, actual: String },
    #[error("building registry JSON is malformed: {0}")]
    MalformedJson(#[from] serde_json::Error),
    #[error("unsupported building registry contract")]
    UnsupportedContract,
    #[error("building registry is blocked: {reason}")]
    RuntimeBlocked { reason: String },
    #[error("building registry release gate is inconsistent: {0}")]
    MalformedRelease(String),
    #[error("building registry contains unresolved runtime data at {0}")]
    UnresolvedData(String),
    #[error("building registry contains duplicate canonical key {0}")]
    DuplicateKey(String),
    #[error("invalid evidence source path: {0}")]
    InvalidEvidencePath(String),
    #[error("could not read evidence source {path}: {source}")]
    ReadEvidence {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("evidence source {path} size mismatch: expected {expected}, found {actual}")]
    EvidenceSizeMismatch {
        path: String,
        expected: u64,
        actual: u64,
    },
    #[error("evidence source {path} hash mismatch: expected {expected}, found {actual}")]
    EvidenceHashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

pub fn load_runtime_ready_registry(
    registry_path: impl AsRef<Path>,
    repository_root: impl AsRef<Path>,
    expected_sha256: &str,
) -> Result<BuildingRegistry, BuildingRegistryLoadError> {
    let registry_path = registry_path.as_ref();
    let payload =
        fs::read(registry_path).map_err(|source| BuildingRegistryLoadError::ReadRegistry {
            path: registry_path.to_path_buf(),
            source,
        })?;
    load_runtime_ready_registry_bytes(&payload, repository_root, expected_sha256)
}

pub fn load_runtime_ready_registry_bytes(
    payload: &[u8],
    repository_root: impl AsRef<Path>,
    expected_sha256: &str,
) -> Result<BuildingRegistry, BuildingRegistryLoadError> {
    if !is_sha256(expected_sha256) {
        return Err(BuildingRegistryLoadError::MalformedExpectedHash);
    }
    let actual_sha256 = hex_sha256(payload);
    if actual_sha256 != expected_sha256 {
        return Err(BuildingRegistryLoadError::RegistryHashMismatch {
            expected: expected_sha256.to_owned(),
            actual: actual_sha256,
        });
    }

    let registry: BuildingRegistry = serde_json::from_slice(payload)?;
    registry.validate_runtime_ready(repository_root.as_ref())?;
    Ok(registry)
}

/// Returns the immutable, evidence-backed portion of the canonical registry.
///
/// The release may remain globally blocked: individual resolved fields are safe
/// to project, while every mutation separately checks its complete row binding.
pub fn canonical_building_content() -> Result<&'static BuildingContentView, &'static str> {
    BUILDING_CONTENT
        .get_or_init(|| {
            load_read_only_registry_bytes(EMBEDDED_REGISTRY, EMBEDDED_REGISTRY_SHA256)
                .and_then(|registry| BuildingContentView::try_from_registry(&registry))
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(String::as_str)
}

pub fn load_read_only_registry_bytes(
    payload: &[u8],
    expected_sha256: &str,
) -> Result<BuildingRegistry, BuildingRegistryLoadError> {
    if !is_sha256(expected_sha256) {
        return Err(BuildingRegistryLoadError::MalformedExpectedHash);
    }
    let actual_sha256 = hex_sha256(payload);
    if actual_sha256 != expected_sha256 {
        return Err(BuildingRegistryLoadError::RegistryHashMismatch {
            expected: expected_sha256.to_owned(),
            actual: actual_sha256,
        });
    }

    let registry: BuildingRegistry = serde_json::from_slice(payload)?;
    registry.validate_identity()?;
    Ok(registry)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildingRegistry {
    pub schema_version: u32,
    pub contract_type: String,
    pub registry_id: String,
    pub legacy: LegacyIdentity,
    pub runtime_state: RuntimeState,
    pub evidence_policy: EvidencePolicy,
    pub evidence_sources: Vec<EvidenceSource>,
    pub catalogs: Catalogs,
    pub buildings: Collection<Building>,
    pub release_gate: ReleaseGate,
}

impl BuildingRegistry {
    fn validate_identity(&self) -> Result<(), BuildingRegistryLoadError> {
        if self.schema_version != 1
            || self.contract_type != CONTRACT_TYPE
            || self.legacy.game != LEGACY_GAME
            || self.legacy.version != LEGACY_VERSION
            || self.legacy.package != LEGACY_PACKAGE
            || self.evidence_policy.semantic_fields != "evidence-required-per-field"
            || self.evidence_policy.unresolved_values != "fail-closed-null-or-empty"
            || self.evidence_policy.visual_binding != "separate-from-gameplay-semantics"
        {
            return Err(BuildingRegistryLoadError::UnsupportedContract);
        }
        Ok(())
    }

    fn validate_runtime_ready(
        &self,
        repository_root: &Path,
    ) -> Result<(), BuildingRegistryLoadError> {
        self.validate_identity()?;

        if self.runtime_state == RuntimeState::Blocked {
            return Err(BuildingRegistryLoadError::RuntimeBlocked {
                reason: self.release_gate.reason.clone(),
            });
        }
        if !self.release_gate.runnable {
            return Err(BuildingRegistryLoadError::MalformedRelease(
                "runtime-ready registry is not runnable".into(),
            ));
        }
        if !self.release_gate.blocking_paths.is_empty() {
            return Err(BuildingRegistryLoadError::MalformedRelease(
                "runtime-ready registry still declares blocking paths".into(),
            ));
        }
        if self.buildings.rows.is_empty() {
            return Err(BuildingRegistryLoadError::MalformedRelease(
                "runtime-ready registry has no buildings".into(),
            ));
        }

        self.catalogs.validate_resolved()?;
        self.buildings.validate_resolved("buildings")?;
        self.verify_evidence_sources(repository_root)
    }

    fn verify_evidence_sources(
        &self,
        repository_root: &Path,
    ) -> Result<(), BuildingRegistryLoadError> {
        for source in &self.evidence_sources {
            if !is_repository_relative(Path::new(&source.path)) || !is_sha256(&source.sha256) {
                return Err(BuildingRegistryLoadError::InvalidEvidencePath(
                    source.path.clone(),
                ));
            }
            let absolute_path = repository_root.join(&source.path);
            let payload = fs::read(&absolute_path).map_err(|error| {
                BuildingRegistryLoadError::ReadEvidence {
                    path: absolute_path,
                    source: error,
                }
            })?;
            if payload.len() as u64 != source.bytes {
                return Err(BuildingRegistryLoadError::EvidenceSizeMismatch {
                    path: source.path.clone(),
                    expected: source.bytes,
                    actual: payload.len() as u64,
                });
            }
            let actual = hex_sha256(&payload);
            if actual != source.sha256 {
                return Err(BuildingRegistryLoadError::EvidenceHashMismatch {
                    path: source.path.clone(),
                    expected: source.sha256.clone(),
                    actual,
                });
            }
        }
        Ok(())
    }
}

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
    fn try_from_registry(registry: &BuildingRegistry) -> Result<Self, BuildingRegistryLoadError> {
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyIdentity {
    pub game: String,
    pub version: String,
    pub package: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeState {
    Blocked,
    RuntimeReady,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidencePolicy {
    pub semantic_fields: String,
    pub unresolved_values: String,
    pub visual_binding: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSource {
    pub id: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseGate {
    pub runnable: bool,
    pub blocking_paths: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalogs {
    pub items: Collection<Item>,
    pub products: Collection<Product>,
    pub capabilities: Collection<Capability>,
    pub skins: Collection<BuildingSkin>,
}

impl Catalogs {
    fn validate_resolved(&self) -> Result<(), BuildingRegistryLoadError> {
        self.items.validate_resolved("catalogs.items")?;
        self.products.validate_resolved("catalogs.products")?;
        self.capabilities
            .validate_resolved("catalogs.capabilities")?;
        self.skins.validate_resolved("catalogs.skins")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Collection<T> {
    pub binding: Binding,
    pub rows: Vec<T>,
}

pub trait RuntimeResolved {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError>;
}

impl<T: RuntimeResolved> Collection<T> {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        self.binding.validate_resolved(&format!("{path}.binding"))?;
        for (index, row) in self.rows.iter().enumerate() {
            row.validate_resolved(&format!("{path}.rows[{index}]"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub state: ResolutionState,
    pub confidence: Confidence,
    pub evidence: Vec<EvidenceRef>,
    pub required_evidence: Option<String>,
}

impl Binding {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        if self.state != ResolutionState::Resolved
            || self.confidence == Confidence::Unknown
            || self.evidence.is_empty()
            || self.required_evidence.is_some()
        {
            return Err(BuildingRegistryLoadError::UnresolvedData(path.into()));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceField {
    pub state: ResolutionState,
    pub confidence: Confidence,
    pub value: Value,
    pub evidence: Vec<EvidenceRef>,
    pub required_evidence: Option<String>,
}

impl EvidenceField {
    fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
        if self.state != ResolutionState::Resolved
            || self.confidence == Confidence::Unknown
            || self.value.is_null()
            || self.evidence.is_empty()
            || self.required_evidence.is_some()
        {
            return Err(BuildingRegistryLoadError::UnresolvedData(path.into()));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionState {
    Resolved,
    Unresolved,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    Confirmed,
    StronglyInferred,
    Tentative,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceRef {
    pub source_id: String,
    pub locator: String,
    pub method: EvidenceMethod,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceMethod {
    SerializedRow,
    MetadataField,
    NativeCode,
    LocalizationEntry,
    SceneObject,
    UiHierarchy,
    AssetObject,
    RuntimeTrace,
}

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

fn validate_fields<const N: usize>(
    path: &str,
    fields: [(&str, &EvidenceField); N],
) -> Result<(), BuildingRegistryLoadError> {
    for (name, field) in fields {
        field.validate_resolved(&format!("{path}.{name}"))?;
    }
    Ok(())
}

fn is_repository_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hex_sha256(payload: &[u8]) -> String {
    format!("{:x}", Sha256::digest(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCKED_FIXTURE: &[u8] =
        include_bytes!("../../../../tools/tests/fixtures/building-registry.blocked.json");

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
}
