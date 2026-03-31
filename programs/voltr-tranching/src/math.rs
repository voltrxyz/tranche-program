pub use voltr_vault::utils::{
    CheckedCeilDiv, Decimal, DecimalExtra, SafeMath, U128, U256,
};

pub const BPS_PRECISION: u16 = 10_000;
pub const SECONDS_PER_YEAR: u64 = 31_557_600;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Constants ───────────────────────────────────────────────

    #[test]
    fn test_constants() {
        assert_eq!(BPS_PRECISION, 10_000);
        assert_eq!(SECONDS_PER_YEAR, 31_557_600);
    }

    // ─── SafeMath (checked integer arithmetic) ──────────────────

    #[test]
    fn test_safe_math_u128() {
        assert_eq!(100u128.safe_add(200u128).unwrap(), 300u128);
        assert_eq!(200u128.safe_sub(100u128).unwrap(), 100u128);
        assert_eq!(10u128.safe_mul(20u128).unwrap(), 200u128);
        assert_eq!(100u128.safe_div(5u128).unwrap(), 20u128);
    }

    #[test]
    fn test_safe_math_u64() {
        assert_eq!(100u64.safe_add(200u64).unwrap(), 300u64);
        assert_eq!(200u64.safe_sub(100u64).unwrap(), 100u64);
    }

    #[test]
    fn test_safe_math_overflow() {
        assert!(u128::MAX.safe_add(1u128).is_err());
        assert!(0u128.safe_sub(1u128).is_err());
        assert!(u64::MAX.safe_add(1u64).is_err());
    }

    #[test]
    fn test_safe_math_div_by_zero() {
        assert!(100u128.safe_div(0u128).is_err());
        assert!(100u64.safe_div(0u64).is_err());
    }

    #[test]
    fn test_safe_div_ceil() {
        // 10 / 3 = 3.33... → ceil = 4
        assert_eq!(10u128.safe_div_ceil(3u128).unwrap(), 4u128);
        // Exact: 10 / 5 = 2
        assert_eq!(10u128.safe_div_ceil(5u128).unwrap(), 2u128);
        // 1 / 1 = 1
        assert_eq!(1u128.safe_div_ceil(1u128).unwrap(), 1u128);
    }

    // ─── CheckedCeilDiv ─────────────────────────────────────────

    #[test]
    fn test_checked_ceil_div() {
        assert_eq!(10u128.checked_ceil_div(3), Some(4));
        assert_eq!(10u128.checked_ceil_div(5), Some(2));
        assert_eq!(0u128.checked_ceil_div(5), Some(0));
        assert_eq!(10u128.checked_ceil_div(0), None);
    }

    // ─── Decimal basic arithmetic ───────────────────────────────

    #[test]
    fn test_decimal_checked_mul() {
        let two = Decimal::from_num(2);
        let three = Decimal::from_num(3);
        assert_eq!(two.checked_mul(three), Some(Decimal::from_num(6)));
    }

    #[test]
    fn test_decimal_checked_div() {
        let six = Decimal::from_num(6);
        let two = Decimal::from_num(2);
        assert_eq!(six.checked_div(two), Some(Decimal::from_num(3)));
    }

    #[test]
    fn test_decimal_div_by_zero() {
        assert_eq!(Decimal::ONE.checked_div(Decimal::ZERO), None);
    }

    #[test]
    fn test_decimal_safe_math() {
        let a = Decimal::from_num(100);
        let b = Decimal::from_num(200);
        assert_eq!(a.safe_add(b).unwrap(), Decimal::from_num(300));
        assert_eq!(b.safe_sub(a).unwrap(), Decimal::from_num(100));
        assert_eq!(a.safe_mul(Decimal::from_num(3)).unwrap(), Decimal::from_num(300));
        assert_eq!(b.safe_div(a).unwrap(), Decimal::from_num(2));
    }

    // ─── DecimalExtra (BPS / percent / pow) ─────────────────────

    #[test]
    fn test_bps_roundtrip() {
        let val = Decimal::from_bps(500u64); // 5%
        let bps_back: u64 = val.to_bps().unwrap();
        assert_eq!(bps_back, 500);
    }

    #[test]
    fn test_percent_roundtrip() {
        let val = Decimal::from_percent(5u64);
        let pct_back: u64 = val.to_percent().unwrap();
        assert_eq!(pct_back, 5);
    }

    #[test]
    fn test_checked_pow() {
        let two = Decimal::from_num(2);
        assert_eq!(two.checked_pow(10), Some(Decimal::from_num(1024)));
        assert_eq!(Decimal::ONE.checked_pow(1000), Some(Decimal::ONE));
        assert_eq!(Decimal::from_num(5).checked_pow(0), Some(Decimal::ONE));
    }

    // ─── Decimal bits representation roundtrip ──────────────────

    #[test]
    fn test_decimal_bits_roundtrip() {
        let val = Decimal::from_num(123456);
        let bits: u128 = val.to_decimal_bits();
        let recovered = Decimal::from_decimal_bits(bits);
        assert_eq!(val, recovered);
    }

    #[test]
    fn test_decimal_bits_zero() {
        let bits = Decimal::ZERO.to_decimal_bits();
        assert_eq!(bits, 0);
        assert_eq!(Decimal::from_decimal_bits(0), Decimal::ZERO);
    }

    // ─── Decimal floor / ceil (token amount conversion) ─────────

    #[test]
    fn test_to_floor() {
        // 3.7 → floor → 3
        let val = Decimal::from_num(3) + Decimal::from_num(7) / Decimal::from_num(10);
        let floored: u64 = val.to_floor();
        assert_eq!(floored, 3);
    }

    #[test]
    fn test_to_ceil() {
        // 3.1 → ceil → 4
        let val = Decimal::from_num(3) + Decimal::from_bits(1);
        let ceiled: u64 = val.to_ceil();
        assert_eq!(ceiled, 4);
    }

    #[test]
    fn test_floor_ceil_exact() {
        let val = Decimal::from_num(42);
        let floored: u64 = val.to_floor();
        let ceiled: u64 = val.to_ceil();
        assert_eq!(floored, 42);
        assert_eq!(ceiled, 42);
    }

    // ─── mul_int_ratio (vault-style proportional calculation) ───

    #[test]
    fn test_mul_int_ratio_basic() {
        // 1000 * 500 / 10_000 = 50
        let nav = Decimal::from_num(1000);
        let result = nav.mul_int_ratio(500u128, 10_000u128);
        let amount: u64 = result.to_floor();
        assert_eq!(amount, 50);
    }

    #[test]
    fn test_mul_int_ratio_fee_calculation() {
        // 1_000_000 tokens * 200 bps / 10_000 = 20_000 tokens (2% fee)
        let amount = Decimal::from_num(1_000_000u64);
        let fee = amount.mul_int_ratio(200u128, u128::from(BPS_PRECISION));
        let fee_tokens: u64 = fee.to_ceil(); // round up for fees
        assert_eq!(fee_tokens, 20_000);
    }

    #[test]
    fn test_full_mul_int_ratio_large() {
        // U256 intermediate prevents overflow for large values
        let val = Decimal::from_num(1_000_000);
        let result = val.full_mul_int_ratio(
            U256::from(u128::MAX / 2),
            U256::from(u128::MAX / 2),
        );
        assert_eq!(result, Decimal::from_num(1_000_000));
    }

    // ─── Token amount patterns (matching vault accounting) ──────

    #[test]
    fn test_lp_mint_ratio() {
        // deposit 100 tokens, total_supply=1000, total_assets=2000
        // tokens_to_mint = 100 * 1000 / 2000 = 50
        let deposit = 100u128;
        let total_supply = 1000u128;
        let total_assets = 2000u128;
        let to_mint = deposit.safe_mul(total_supply).unwrap().safe_div(total_assets).unwrap();
        assert_eq!(to_mint, 50u128);
    }

    #[test]
    fn test_lp_mint_ratio_decimal() {
        // Same calculation via Decimal for fractional precision
        let deposit = Decimal::from_num(100u64);
        let result = deposit.full_mul_int_ratio(1000u64, 2000u64);
        let to_mint: u64 = result.to_floor(); // round down for minting
        assert_eq!(to_mint, 50);
    }

    #[test]
    fn test_fee_bps_integer_math() {
        // 500_000 USDC * 100 bps / 10_000 = 5_000 USDC (1% fee)
        let amount = 500_000u128;
        let fee_bps = 100u128;
        let fee = amount
            .safe_mul(fee_bps)
            .unwrap()
            .safe_div_ceil(u128::from(BPS_PRECISION))
            .unwrap();
        assert_eq!(fee, 5_000);
    }

    #[test]
    fn test_fee_bps_rounds_up() {
        // 333 * 100 / 10_000 = 3.33 → ceil = 4 (protocol-favoring)
        let amount = 333u128;
        let fee_bps = 100u128;
        let fee_floor = amount.safe_mul(fee_bps).unwrap().safe_div(u128::from(BPS_PRECISION)).unwrap();
        let fee_ceil = amount.safe_mul(fee_bps).unwrap().safe_div_ceil(u128::from(BPS_PRECISION)).unwrap();
        assert_eq!(fee_floor, 3);
        assert_eq!(fee_ceil, 4);
    }

    #[test]
    fn test_protocol_favoring_rounding() {
        // Minting: round down tokens issued (fewer tokens = protocol-favoring)
        let deposit = Decimal::from_num(1001u64);
        let ratio = deposit.full_mul_int_ratio(999u64, 1000u64);
        let tokens_floor: u64 = ratio.to_floor();
        let tokens_ceil: u64 = ratio.to_ceil();
        assert!(tokens_floor <= tokens_ceil);

        // Fees: round up amount collected (more fee = protocol-favoring)
        let amount = 1001u128;
        let fee_floor = amount.safe_mul(100).unwrap().safe_div(u128::from(BPS_PRECISION)).unwrap();
        let fee_ceil = amount.safe_mul(100).unwrap().safe_div_ceil(u128::from(BPS_PRECISION)).unwrap();
        assert!(fee_ceil >= fee_floor);
    }

    // ─── Edge cases ─────────────────────────────────────────────

    #[test]
    fn test_decimal_from_num_zero() {
        assert_eq!(Decimal::from_num(0u64), Decimal::ZERO);
    }

    #[test]
    fn test_decimal_one() {
        assert_eq!(Decimal::ONE.to_floor::<u64>(), 1);
    }

    #[test]
    fn test_u128_large_multiply() {
        // Large u128 multiply that fits: 1e18 * 1e18 / 1e18 = 1e18
        let a = 1_000_000_000_000_000_000u128;
        let b = 1_000_000_000_000_000_000u128;
        let result = a.safe_mul(b);
        assert!(result.is_ok()); // 1e36 fits in u128 (max ~3.4e38)
    }

    #[test]
    fn test_u128_overflow() {
        // u128::MAX * 2 overflows
        assert!(u128::MAX.safe_mul(2u128).is_err());
    }
}
