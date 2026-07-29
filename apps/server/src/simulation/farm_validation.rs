use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct FarmReport {
    pub window_id: u64,
    pub from_revision: i64,
    pub elapsed_ms: u64,
    pub distance_px: u64,
    pub damage: u64,
    pub ordinary_kills: u32,
    pub common_materials: u32,
    #[serde(default)]
    pub protected_claims: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FarmValidationPolicy {
    pub maximum_elapsed_ms: u64,
    pub maximum_distance_px_per_second: u64,
    pub maximum_damage_per_second: u64,
    pub maximum_kills_per_second: u32,
    pub maximum_common_materials_per_kill: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FarmReportViolation {
    InvalidRevision,
    InvalidElapsedTime,
    ProtectedValueClaim,
    DistanceBudgetExceeded,
    DamageBudgetExceeded,
    KillBudgetExceeded,
    MaterialBudgetExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedFarmReport {
    pub window_id: u64,
    pub accepted_common_materials: u32,
    pub risk_points: u16,
}

pub fn validate_farm_report(
    report: &FarmReport,
    expected_revision: i64,
    policy: FarmValidationPolicy,
) -> Result<AcceptedFarmReport, FarmReportViolation> {
    if report.from_revision != expected_revision {
        return Err(FarmReportViolation::InvalidRevision);
    }
    if report.elapsed_ms == 0 || report.elapsed_ms > policy.maximum_elapsed_ms {
        return Err(FarmReportViolation::InvalidElapsedTime);
    }
    if !report.protected_claims.is_empty() {
        return Err(FarmReportViolation::ProtectedValueClaim);
    }

    let seconds = report.elapsed_ms.div_ceil(1_000);
    if report.distance_px
        > policy
            .maximum_distance_px_per_second
            .saturating_mul(seconds)
    {
        return Err(FarmReportViolation::DistanceBudgetExceeded);
    }
    if report.damage > policy.maximum_damage_per_second.saturating_mul(seconds) {
        return Err(FarmReportViolation::DamageBudgetExceeded);
    }
    let kill_budget = policy
        .maximum_kills_per_second
        .saturating_mul(u32::try_from(seconds).unwrap_or(u32::MAX));
    if report.ordinary_kills > kill_budget {
        return Err(FarmReportViolation::KillBudgetExceeded);
    }
    let material_budget = report
        .ordinary_kills
        .saturating_mul(policy.maximum_common_materials_per_kill);
    if report.common_materials > material_budget {
        return Err(FarmReportViolation::MaterialBudgetExceeded);
    }

    Ok(AcceptedFarmReport {
        window_id: report.window_id,
        accepted_common_materials: report.common_materials,
        risk_points: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> FarmValidationPolicy {
        FarmValidationPolicy {
            maximum_elapsed_ms: 3_000,
            maximum_distance_px_per_second: 100,
            maximum_damage_per_second: 1_000,
            maximum_kills_per_second: 2,
            maximum_common_materials_per_kill: 3,
        }
    }

    fn report() -> FarmReport {
        FarmReport {
            window_id: 7,
            from_revision: 11,
            elapsed_ms: 2_000,
            distance_px: 180,
            damage: 1_900,
            ordinary_kills: 3,
            common_materials: 7,
            protected_claims: Vec::new(),
        }
    }

    #[test]
    fn accepts_only_common_farm_value_inside_server_issued_budgets() {
        let accepted = validate_farm_report(&report(), 11, policy()).unwrap();
        assert_eq!(accepted.accepted_common_materials, 7);
    }

    #[test]
    fn protected_value_never_enters_the_async_farm_lane() {
        let mut report = report();
        report.protected_claims.push("premium_currency".to_owned());
        assert_eq!(
            validate_farm_report(&report, 11, policy()),
            Err(FarmReportViolation::ProtectedValueClaim)
        );
    }

    #[test]
    fn rejects_implausible_client_damage_without_granting_partial_value() {
        let mut report = report();
        report.damage = 2_001;
        assert_eq!(
            validate_farm_report(&report, 11, policy()),
            Err(FarmReportViolation::DamageBudgetExceeded)
        );
    }
}
