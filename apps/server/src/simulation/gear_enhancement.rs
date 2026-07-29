use std::collections::BTreeMap;

pub const MAX_ENHANCEMENT_LEVEL: u8 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnhancementMode {
    Single,
    To10,
    To15,
    To20,
}

impl EnhancementMode {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "single" => Self::Single,
            "to_10" => Self::To10,
            "to_15" => Self::To15,
            "to_20" => Self::To20,
            _ => return None,
        })
    }

    fn target(self, current_level: u8) -> u8 {
        match self {
            Self::Single => current_level.saturating_add(1).min(MAX_ENHANCEMENT_LEVEL),
            Self::To10 => 10,
            Self::To15 => 15,
            Self::To20 => MAX_ENHANCEMENT_LEVEL,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnhancementAttemptRule {
    pub gold_cost: u64,
    pub material_costs: BTreeMap<String, u32>,
    pub success_threshold_bps: u32,
    pub failure_result_level: u8,
}

pub trait EnhancementRuleProvider {
    fn next_attempt_rule(
        &self,
        product_id: &str,
        current_level: u8,
        optional_material_ids: &[String],
    ) -> Option<EnhancementAttemptRule>;
}

pub trait EnhancementRollSource {
    fn roll_bps(&mut self) -> u32;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnhancementAttemptResult {
    pub attempt: u32,
    pub starting_level: u8,
    pub resulting_level: u8,
    pub succeeded: bool,
    pub gold_spent: u64,
    pub materials_spent: BTreeMap<String, u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnhancementStopReason {
    SingleCompleted,
    TargetReached,
    CapReached,
    InsufficientGold,
    InsufficientMaterial,
    RuleUnavailable,
    InvalidRule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnhancementExecutionResult {
    pub final_level: u8,
    pub remaining_gold: u64,
    pub remaining_materials: BTreeMap<String, u32>,
    pub attempts: Vec<EnhancementAttemptResult>,
    pub spent_gold: u64,
    pub spent_materials: BTreeMap<String, u32>,
    pub stop_reason: EnhancementStopReason,
}

/// Executes only with a supplied authoritative rule provider and RNG source.
/// No live provider exists until the original cost/material/probability bindings are recovered.
#[allow(clippy::too_many_arguments)]
pub fn execute_enhancement<P: EnhancementRuleProvider, R: EnhancementRollSource>(
    provider: &P,
    rng: &mut R,
    product_id: &str,
    starting_level: u8,
    mode: EnhancementMode,
    optional_material_ids: &[String],
    mut gold: u64,
    mut materials: BTreeMap<String, u32>,
) -> EnhancementExecutionResult {
    let mut level = starting_level.min(MAX_ENHANCEMENT_LEVEL);
    let target = mode.target(level);
    let mut attempts = Vec::new();
    let mut spent_gold = 0_u64;
    let mut spent_materials = BTreeMap::<String, u32>::new();

    let stop_reason = loop {
        if level >= MAX_ENHANCEMENT_LEVEL {
            break EnhancementStopReason::CapReached;
        }
        if level >= target {
            break EnhancementStopReason::TargetReached;
        }
        let Some(rule) = provider.next_attempt_rule(product_id, level, optional_material_ids)
        else {
            break EnhancementStopReason::RuleUnavailable;
        };
        if rule.success_threshold_bps > 10_000
            || rule.failure_result_level > MAX_ENHANCEMENT_LEVEL
            || (rule.gold_cost == 0 && rule.material_costs.values().all(|quantity| *quantity == 0))
        {
            break EnhancementStopReason::InvalidRule;
        }
        if gold < rule.gold_cost {
            break EnhancementStopReason::InsufficientGold;
        }
        if rule.material_costs.iter().any(|(material_id, required)| {
            materials.get(material_id).copied().unwrap_or(0) < *required
        }) {
            break EnhancementStopReason::InsufficientMaterial;
        }

        gold -= rule.gold_cost;
        spent_gold = spent_gold.saturating_add(rule.gold_cost);
        for (material_id, required) in &rule.material_costs {
            let remaining = materials.entry(material_id.clone()).or_default();
            *remaining -= *required;
            let spent = spent_materials.entry(material_id.clone()).or_default();
            *spent = spent.saturating_add(*required);
        }
        let starting_level = level;
        let succeeded = rng.roll_bps() < rule.success_threshold_bps;
        level = if succeeded {
            level.saturating_add(1).min(MAX_ENHANCEMENT_LEVEL)
        } else {
            rule.failure_result_level
        };
        attempts.push(EnhancementAttemptResult {
            attempt: u32::try_from(attempts.len() + 1).unwrap_or(u32::MAX),
            starting_level,
            resulting_level: level,
            succeeded,
            gold_spent: rule.gold_cost,
            materials_spent: rule.material_costs,
        });
        if mode == EnhancementMode::Single {
            break EnhancementStopReason::SingleCompleted;
        }
    };

    EnhancementExecutionResult {
        final_level: level,
        remaining_gold: gold,
        remaining_materials: materials,
        attempts,
        spent_gold,
        spent_materials,
        stop_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRules;

    impl EnhancementRuleProvider for TestRules {
        fn next_attempt_rule(
            &self,
            _product_id: &str,
            current_level: u8,
            _optional_material_ids: &[String],
        ) -> Option<EnhancementAttemptRule> {
            Some(EnhancementAttemptRule {
                gold_cost: 10,
                material_costs: BTreeMap::from([("test-material".to_owned(), 1)]),
                success_threshold_bps: 5_000,
                failure_result_level: current_level.saturating_sub(1),
            })
        }
    }

    struct TestRolls(Vec<u32>);

    impl EnhancementRollSource for TestRolls {
        fn roll_bps(&mut self) -> u32 {
            self.0.remove(0)
        }
    }

    #[test]
    fn target_mode_commits_completed_attempts_then_stops_before_unfunded_attempt() {
        let result = execute_enhancement(
            &TestRules,
            &mut TestRolls(vec![0, 9_999]),
            "test-gear",
            8,
            EnhancementMode::To10,
            &[],
            25,
            BTreeMap::from([("test-material".to_owned(), 2)]),
        );
        assert_eq!(result.attempts.len(), 2);
        assert_eq!(result.final_level, 8);
        assert_eq!(result.spent_gold, 20);
        assert_eq!(result.remaining_gold, 5);
        assert_eq!(result.stop_reason, EnhancementStopReason::InsufficientGold);
    }

    #[test]
    fn single_mode_runs_exactly_one_authoritative_attempt() {
        let result = execute_enhancement(
            &TestRules,
            &mut TestRolls(vec![0]),
            "test-gear",
            19,
            EnhancementMode::Single,
            &[],
            10,
            BTreeMap::from([("test-material".to_owned(), 1)]),
        );
        assert_eq!(result.final_level, MAX_ENHANCEMENT_LEVEL);
        assert_eq!(result.attempts.len(), 1);
        assert_eq!(result.stop_reason, EnhancementStopReason::SingleCompleted);
    }
}
