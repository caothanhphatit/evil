use serde::Deserialize;
use serde_json::Value;

use super::{
    schema::{BuildingSkin, Capability, Item, Product},
    BuildingRegistryLoadError,
};

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
    pub(super) fn validate_resolved(&self) -> Result<(), BuildingRegistryLoadError> {
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
    pub(super) fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
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
    pub(super) fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
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
    pub(super) fn validate_resolved(&self, path: &str) -> Result<(), BuildingRegistryLoadError> {
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

pub(super) fn validate_fields<const N: usize>(
    path: &str,
    fields: [(&str, &EvidenceField); N],
) -> Result<(), BuildingRegistryLoadError> {
    for (name, field) in fields {
        field.validate_resolved(&format!("{path}.{name}"))?;
    }
    Ok(())
}
