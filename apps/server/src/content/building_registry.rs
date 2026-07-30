mod error;
mod integrity;
mod loader;
mod runtime_content;
mod schema;
mod schema_common;
mod validation;

pub use error::BuildingRegistryLoadError;
pub(crate) use loader::EMBEDDED_REGISTRY_SHA256;
pub use loader::{
    canonical_building_content, load_read_only_registry_bytes, load_runtime_ready_registry,
    load_runtime_ready_registry_bytes,
};
pub use runtime_content::{
    BuildingContent, BuildingContentView, BuildingMutationRow, BuildingSkinContent,
    BuildingSourceContent, CapabilityContent, ContentAmount, ConversionOptionContent,
    EconomyItemContent, EconomyProductContent, ProductServiceContent, RandomOutputContent,
    SkinVisualContent,
};
pub use schema::{
    Amount, BuildRow, Building, BuildingLevel, BuildingSkin, BuildingSourceData, Capability,
    CapabilityReadiness, Condition, ConversionOption, Item, ItemDirectionalEconomy, Product,
    ProductServiceData, RandomOutputData, Reference, SkinVisualBinding, TradeRule, VisualBinding,
};
pub use schema_common::{
    Binding, Catalogs, Collection, Confidence, EvidenceField, EvidenceMethod, EvidencePolicy,
    EvidenceRef, EvidenceSource, LegacyIdentity, ReleaseGate, ResolutionState, RuntimeResolved,
    RuntimeState,
};
pub use validation::BuildingRegistry;

#[cfg(test)]
mod tests;
