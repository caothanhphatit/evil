#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OriginalTaxSegmentResult {
    pub post_tax_grant: i64,
    pub tax: i64,
    pub tax_remainder: f32,
}

/// Exact mechanically bound `CalVillTax` segment after its still-unnamed rate
/// operands have already produced `candidate`.
#[allow(dead_code)]
pub fn original_apply_tax_candidate(
    input_grant: i64,
    current_tax: i64,
    current_remainder: f32,
    candidate: f32,
    tax_cap: i64,
) -> OriginalTaxSegmentResult {
    if current_tax >= tax_cap || candidate <= 0.0 {
        return OriginalTaxSegmentResult {
            post_tax_grant: input_grant,
            tax: current_tax,
            tax_remainder: current_remainder,
        };
    }

    let whole = candidate as i64;
    let post_tax_grant = input_grant.wrapping_sub(whole);
    let mut tax = current_tax.wrapping_add(whole);
    let mut tax_remainder = current_remainder + (candidate - whole as f32);

    if tax_remainder >= 1.0 {
        let carried = tax_remainder as i64;
        tax = tax.wrapping_add(carried);
        tax_remainder -= carried as f32;
    }
    if tax > tax_cap {
        tax = tax_cap;
    }

    OriginalTaxSegmentResult {
        post_tax_grant,
        tax,
        tax_remainder,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPlusGoldResult {
    pub granted_gold: i64,
    pub hunter_money: i64,
}

/// Exact `PlusGold` branch and sink after Reward and village tax have produced
/// the post-tax grant. The product meaning of the 0.3 branch remains unresolved.
#[allow(dead_code)]
pub fn original_plus_gold(
    post_tax_grant: i64,
    current_money: i64,
    revive: i32,
    stage_level: i32,
) -> OriginalPlusGoldResult {
    let granted_gold = if revive > stage_level && stage_level <= 3 {
        (post_tax_grant as f32 * 0.3_f32) as i64
    } else {
        post_tax_grant
    };
    let hunter_money = if granted_gold >= 1 {
        current_money.wrapping_add(granted_gold)
    } else {
        current_money
    };
    OriginalPlusGoldResult {
        granted_gold,
        hunter_money,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        original_apply_tax_candidate, original_plus_gold, OriginalPlusGoldResult,
        OriginalTaxSegmentResult,
    };

    fn assert_tax_result(actual: OriginalTaxSegmentResult, expected: OriginalTaxSegmentResult) {
        assert_eq!(actual.post_tax_grant, expected.post_tax_grant);
        assert_eq!(actual.tax, expected.tax);
        assert!((actual.tax_remainder - expected.tax_remainder).abs() < 0.000_001);
    }

    #[test]
    fn tax_segment_replays_fraction_carry_and_cap_vectors() {
        assert_tax_result(
            original_apply_tax_candidate(20, 10, 0.4, 2.75, 100),
            OriginalTaxSegmentResult {
                post_tax_grant: 18,
                tax: 13,
                tax_remainder: 0.15,
            },
        );
        assert_tax_result(
            original_apply_tax_candidate(20, 100, 0.4, 2.75, 100),
            OriginalTaxSegmentResult {
                post_tax_grant: 20,
                tax: 100,
                tax_remainder: 0.4,
            },
        );
        assert_tax_result(
            original_apply_tax_candidate(20, 10, 0.4, 0.0, 100),
            OriginalTaxSegmentResult {
                post_tax_grant: 20,
                tax: 10,
                tax_remainder: 0.4,
            },
        );
    }

    #[test]
    fn plus_gold_replays_identity_scaling_and_minimum_vectors() {
        assert_eq!(
            original_plus_gold(10, 5, 2, 3),
            OriginalPlusGoldResult {
                granted_gold: 10,
                hunter_money: 15,
            }
        );
        assert_eq!(
            original_plus_gold(10, 5, 4, 3),
            OriginalPlusGoldResult {
                granted_gold: 3,
                hunter_money: 8,
            }
        );
        assert_eq!(
            original_plus_gold(2, 5, 4, 3),
            OriginalPlusGoldResult {
                granted_gold: 0,
                hunter_money: 5,
            }
        );
    }
}
