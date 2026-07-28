#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CombatArithmeticError {
    NonFinite,
    OutOfRange,
    UnsupportedDomain,
}

const I64_EXCLUSIVE_UPPER_F32: f32 = 9_223_372_036_854_775_808.0_f32;
const I64_INCLUSIVE_LOWER_F32: f32 = -9_223_372_036_854_775_808.0_f32;
const I64_EXCLUSIVE_UPPER_F64: f64 = 9_223_372_036_854_775_808.0_f64;
const I64_INCLUSIVE_LOWER_F64: f64 = -9_223_372_036_854_775_808.0_f64;

/// Replays FCVTZS only over the finite, in-range domain established by the
/// recovered formulas. Native behavior outside this domain is not claimed.
pub(super) fn checked_trunc_f32_to_i64(value: f32) -> Result<i64, CombatArithmeticError> {
    if !value.is_finite() {
        return Err(CombatArithmeticError::NonFinite);
    }
    if !(I64_INCLUSIVE_LOWER_F32..I64_EXCLUSIVE_UPPER_F32).contains(&value) {
        return Err(CombatArithmeticError::OutOfRange);
    }
    Ok(value as i64)
}

/// Float64 counterpart used by the recovered CalcDamage and outgoing chains.
pub(super) fn checked_trunc_f64_to_i64(value: f64) -> Result<i64, CombatArithmeticError> {
    if !value.is_finite() {
        return Err(CombatArithmeticError::NonFinite);
    }
    if !(I64_INCLUSIVE_LOWER_F64..I64_EXCLUSIVE_UPPER_F64).contains(&value) {
        return Err(CombatArithmeticError::OutOfRange);
    }
    Ok(value as i64)
}

#[cfg(test)]
mod tests {
    use super::{
        checked_trunc_f32_to_i64, checked_trunc_f64_to_i64, CombatArithmeticError,
        I64_EXCLUSIVE_UPPER_F32, I64_EXCLUSIVE_UPPER_F64, I64_INCLUSIVE_LOWER_F32,
        I64_INCLUSIVE_LOWER_F64,
    };

    #[test]
    fn finite_in_range_conversion_truncates_toward_zero() {
        assert_eq!(checked_trunc_f32_to_i64(12.99), Ok(12));
        assert_eq!(checked_trunc_f32_to_i64(-12.99), Ok(-12));
        assert_eq!(
            checked_trunc_f32_to_i64(I64_INCLUSIVE_LOWER_F32),
            Ok(i64::MIN)
        );
        assert_eq!(checked_trunc_f64_to_i64(12.99), Ok(12));
        assert_eq!(checked_trunc_f64_to_i64(-12.99), Ok(-12));
        assert_eq!(
            checked_trunc_f64_to_i64(I64_INCLUSIVE_LOWER_F64),
            Ok(i64::MIN)
        );
    }

    #[test]
    fn unsupported_non_finite_and_out_of_range_values_fail_closed() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                checked_trunc_f32_to_i64(value),
                Err(CombatArithmeticError::NonFinite)
            );
        }
        assert_eq!(
            checked_trunc_f32_to_i64(I64_EXCLUSIVE_UPPER_F32),
            Err(CombatArithmeticError::OutOfRange)
        );
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                checked_trunc_f64_to_i64(value),
                Err(CombatArithmeticError::NonFinite)
            );
        }
        assert_eq!(
            checked_trunc_f64_to_i64(I64_EXCLUSIVE_UPPER_F64),
            Err(CombatArithmeticError::OutOfRange)
        );
    }
}
