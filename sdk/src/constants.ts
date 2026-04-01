import { PublicKey } from "@solana/web3.js";

/** Voltr Tranching Program ID */
export const VOLTR_TRANCHING_PROGRAM_ID = new PublicKey(
  "HcqA5TydrpKnVotpjp5fNFdKp8sXktcp92Pz6Z6hCjpF"
);

/** Voltr Vault Program ID */
export const VOLTR_VAULT_PROGRAM_ID = new PublicKey(
  "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
);

/** PDA seeds matching on-chain constants (programs/voltr-tranching/src/constants.rs) */
export const SEEDS = {
  TRANCHE_CONFIG: Buffer.from("tranche_config"),
  TRANCHE_AUTHORITY: Buffer.from("tranche_authority"),
  SENIOR_MINT: Buffer.from("senior_mint"),
  JUNIOR_MINT: Buffer.from("junior_mint"),
  WITHDRAW_RECEIPT: Buffer.from("withdraw_receipt"),
  APR_FEED: Buffer.from("apr_feed"),
} as const;

/** Vault PDA seeds (for CPI integration) */
export const VAULT_SEEDS = {
  PROTOCOL: Buffer.from("protocol"),
  VAULT_LP_MINT: Buffer.from("vault_lp_mint"),
  VAULT_LP_MINT_AUTH: Buffer.from("vault_lp_mint_auth"),
  VAULT_ASSET_IDLE_AUTH: Buffer.from("vault_asset_idle_auth"),
} as const;

/** Basis points precision denominator */
export const BPS_PRECISION = 10_000;

/** Seconds per year (365.25 days), matching on-chain SECONDS_PER_YEAR */
export const SECONDS_PER_YEAR = 31_557_600;

/** U80F48 fractional bits for DecimalBits encoding */
export const DECIMAL_FRACTION_BITS = 48;

/** Maximum reserve fee in basis points (20%) */
export const MAX_RESERVE_FEE_BPS = 2000;

/** Maximum exit fee in basis points (1%) */
export const MAX_EXIT_FEE_BPS = 100;

/** Pause flag bits */
export const PAUSE_FLAGS = {
  SENIOR_DEPOSITS: 0x01,
  JUNIOR_DEPOSITS: 0x02,
  SENIOR_WITHDRAWALS: 0x04,
  JUNIOR_WITHDRAWALS: 0x08,
  ALL: 0x0f,
} as const;
