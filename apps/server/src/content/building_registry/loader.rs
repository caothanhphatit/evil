use std::{fs, path::Path, sync::OnceLock};

use super::{
    integrity::{hex_sha256, is_sha256},
    BuildingContentView, BuildingRegistry, BuildingRegistryLoadError,
};

pub(super) const CONTRACT_TYPE: &str = "building-registry";
pub(super) const LEGACY_GAME: &str = "Evil Hunter Tycoon";
pub(super) const LEGACY_VERSION: &str = "1.411";
pub(super) const LEGACY_PACKAGE: &str = "com.superplanet.evilhunter";
pub(crate) const EMBEDDED_REGISTRY_SHA256: &str =
    "a262f6f452aa5d88b74bb8b3b739e3564c57d3cd1bcf88d36b4f7712f72e210e";
pub(super) const EMBEDDED_REGISTRY: &[u8] = include_bytes!(
    "../../../../../packages/content/releases/evil-hunter-1.411/building-registry.json"
);

static BUILDING_CONTENT: OnceLock<Result<BuildingContentView, String>> = OnceLock::new();

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
