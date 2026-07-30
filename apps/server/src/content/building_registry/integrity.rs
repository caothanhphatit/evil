use std::path::{Component, Path};

use sha2::{Digest, Sha256};

pub(super) fn is_repository_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn hex_sha256(payload: &[u8]) -> String {
    format!("{:x}", Sha256::digest(payload))
}
