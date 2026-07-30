use std::{fs, path::Path};

use serde::Deserialize;

use super::{
    integrity::{hex_sha256, is_repository_relative, is_sha256},
    loader::{CONTRACT_TYPE, LEGACY_GAME, LEGACY_PACKAGE, LEGACY_VERSION},
    Building, BuildingRegistryLoadError, Catalogs, Collection, EvidencePolicy, EvidenceSource,
    LegacyIdentity, ReleaseGate, RuntimeState,
};

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
    pub(super) fn validate_identity(&self) -> Result<(), BuildingRegistryLoadError> {
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

    pub(super) fn validate_runtime_ready(
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

    pub(super) fn verify_evidence_sources(
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
