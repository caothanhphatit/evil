use std::path::PathBuf;

use thiserror::Error;

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
