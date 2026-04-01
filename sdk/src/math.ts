import { BPS_PRECISION, DECIMAL_FRACTION_BITS } from "./constants";

/**
 * The U80F48 shift factor as a BigInt: `1n << 48n`.
 */
const SHIFT_48 = 1n << BigInt(DECIMAL_FRACTION_BITS);

/**
 * Encode a number (integer) as DecimalBits (U80F48 fixed-point, LE [u8; 16]).
 *
 * This matches the on-chain representation:
 * `Decimal::from_num(n).to_decimal_bits().to_le_bytes()`
 */
export function intToDecimalBits(n: number | bigint): number[] {
  const buf = Buffer.alloc(16, 0);
  const val = BigInt(n) * SHIFT_48;
  buf.writeBigUInt64LE(val & 0xffff_ffff_ffff_ffffn, 0);
  buf.writeBigUInt64LE(val >> 64n, 8);
  return Array.from(buf);
}

/**
 * Decode DecimalBits (LE [u8; 16] of U80F48) to the raw u128 bit representation.
 */
export function decimalBitsToRaw(bits: number[] | Uint8Array | Buffer): bigint {
  const buf = Buffer.from(bits);
  const lo = buf.readBigUInt64LE(0);
  const hi = buf.readBigUInt64LE(8);
  return (hi << 64n) | lo;
}

/**
 * Decode DecimalBits to an approximate JavaScript number.
 *
 * Precision: safe for display / UI; not for on-chain math.
 */
export function readDecimalBits(
  bits: number[] | Uint8Array | Buffer
): number {
  const raw = decimalBitsToRaw(bits);
  return Number(raw) / Number(SHIFT_48);
}

/**
 * Calculate the senior price per token (NAV / supply).
 *
 * @param seniorNavBits - DecimalBits from TrancheConfig.seniorNav
 * @param seniorSupply  - Total supply of the senior mint (u64)
 * @returns Price as a number (approximate, for display)
 */
export function seniorPrice(
  seniorNavBits: number[] | Uint8Array | Buffer,
  seniorSupply: bigint | number
): number {
  const supply = BigInt(seniorSupply);
  if (supply === 0n) return 1.0;
  const nav = readDecimalBits(seniorNavBits);
  return nav / Number(supply);
}

/**
 * Calculate the junior price per token (NAV / supply).
 *
 * @param juniorNavBits - DecimalBits from TrancheConfig.juniorNav
 * @param juniorSupply  - Total supply of the junior mint (u64)
 * @returns Price as a number (approximate, for display)
 */
export function juniorPrice(
  juniorNavBits: number[] | Uint8Array | Buffer,
  juniorSupply: bigint | number
): number {
  const supply = BigInt(juniorSupply);
  if (supply === 0n) return 1.0;
  const nav = readDecimalBits(juniorNavBits);
  return nav / Number(supply);
}

/**
 * Calculate the coverage ratio: total_nav / senior_nav.
 *
 * A value > 1.0 means junior capital is covering the senior tranche.
 *
 * @param totalNavBits  - DecimalBits from TrancheConfig.totalNav
 * @param seniorNavBits - DecimalBits from TrancheConfig.seniorNav
 * @returns Coverage ratio as a number, or Infinity if senior NAV is 0
 */
export function coverageRatio(
  totalNavBits: number[] | Uint8Array | Buffer,
  seniorNavBits: number[] | Uint8Array | Buffer
): number {
  const seniorNav = readDecimalBits(seniorNavBits);
  if (seniorNav === 0) return Infinity;
  const totalNav = readDecimalBits(totalNavBits);
  return totalNav / seniorNav;
}

/**
 * Calculate the junior/senior ratio in basis points.
 *
 * @returns Ratio in BPS (e.g. 5000 = 50%), or Infinity if senior NAV is 0
 */
export function juniorSeniorRatioBps(
  juniorNavBits: number[] | Uint8Array | Buffer,
  seniorNavBits: number[] | Uint8Array | Buffer
): number {
  const seniorNav = readDecimalBits(seniorNavBits);
  if (seniorNav === 0) return Infinity;
  const juniorNav = readDecimalBits(juniorNavBits);
  return (juniorNav / seniorNav) * BPS_PRECISION;
}

/**
 * Estimate the exit fee for a withdrawal amount.
 *
 * Uses ceiling division (protocol-favoring), matching the on-chain
 * `safe_div_ceil` pattern.
 *
 * @param assetAmount - Withdrawal amount (before fee)
 * @param feeBps     - Exit fee in basis points
 * @returns Fee amount (rounded up)
 */
export function estimateExitFee(
  assetAmount: bigint | number,
  feeBps: number
): bigint {
  const amount = BigInt(assetAmount);
  const fee = BigInt(feeBps);
  const precision = BigInt(BPS_PRECISION);
  // Ceiling division: (amount * feeBps + BPS_PRECISION - 1) / BPS_PRECISION
  return (amount * fee + precision - 1n) / precision;
}

/**
 * Estimate tokens minted for a deposit amount given current NAV and supply.
 *
 * Uses floor rounding (protocol-favoring), matching on-chain `to_floor()`.
 *
 * @param depositAmount - Amount of assets to deposit
 * @param currentNav    - Current tranche NAV (as DecimalBits)
 * @param currentSupply - Current tranche token supply
 * @returns Estimated tokens to be minted
 */
export function estimateTokensMinted(
  depositAmount: bigint | number,
  currentNav: number[] | Uint8Array | Buffer,
  currentSupply: bigint | number
): bigint {
  const supply = BigInt(currentSupply);
  const amount = BigInt(depositAmount);
  if (supply === 0n) return amount; // First deposit: 1:1
  const nav = decimalBitsToRaw(currentNav);
  // tokens = amount * supply / nav (floor division, matching on-chain)
  // Use raw bits for precision: nav is shifted by 48 bits
  return (amount * supply * SHIFT_48) / nav;
}
