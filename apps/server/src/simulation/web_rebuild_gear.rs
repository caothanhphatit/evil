use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The original writer has not yet yielded a reviewed pool/order/price
/// contract. Keep gear creation unavailable instead of emitting fabricated
/// quality, option, or price values into the authoritative economy.
#[allow(dead_code)]
pub const GEAR_CREATION_EVIDENCE_RULESET: &str = "original-gear-creation-unresolved";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GearCreationEvidenceError;

#[allow(dead_code)]
pub fn roll_crafted_gear(
    _command_id: Uuid,
    _row_index: u32,
    _product_id: &str,
    _kind: &str,
    _rating: u16,
) -> Result<(), GearCreationEvidenceError> {
    Err(GearCreationEvidenceError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gear_roll_fails_closed_without_writer_evidence() {
        assert!(roll_crafted_gear(
            Uuid::from_u128(42),
            0,
            "recipe:weapon:0:rating:1",
            "weapon",
            1,
        )
        .is_err());
        assert_eq!(
            GEAR_CREATION_EVIDENCE_RULESET,
            "original-gear-creation-unresolved"
        );
    }
}
