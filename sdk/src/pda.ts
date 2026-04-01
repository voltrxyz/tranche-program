import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import {
  VOLTR_TRANCHING_PROGRAM_ID,
  VOLTR_VAULT_PROGRAM_ID,
  SEEDS,
  VAULT_SEEDS,
} from "./constants";

/**
 * All tranche PDAs derived from a vault address.
 */
export interface TranchePdas {
  trancheConfig: PublicKey;
  trancheAuthority: PublicKey;
  seniorMint: PublicKey;
  juniorMint: PublicKey;
  aprFeed: PublicKey;
}

/**
 * Vault-side PDAs needed for CPI calls.
 */
export interface VaultPdas {
  protocol: PublicKey;
  vaultLpMint: PublicKey;
  vaultLpMintAuth: PublicKey;
  vaultAssetIdleAuth: PublicKey;
}

/**
 * Derive the TrancheConfig PDA for a given vault.
 *
 * Seeds: `["tranche_config", vault]`
 */
export function findTrancheConfigPda(
  vault: PublicKey,
  programId: PublicKey = VOLTR_TRANCHING_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.TRANCHE_CONFIG, vault.toBuffer()],
    programId
  );
}

/**
 * Derive the tranche authority PDA (CPI signer).
 *
 * Seeds: `["tranche_authority", trancheConfig]`
 */
export function findTrancheAuthorityPda(
  trancheConfig: PublicKey,
  programId: PublicKey = VOLTR_TRANCHING_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.TRANCHE_AUTHORITY, trancheConfig.toBuffer()],
    programId
  );
}

/**
 * Derive the senior tranche token mint PDA.
 *
 * Seeds: `["senior_mint", trancheConfig]`
 */
export function findSeniorMintPda(
  trancheConfig: PublicKey,
  programId: PublicKey = VOLTR_TRANCHING_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.SENIOR_MINT, trancheConfig.toBuffer()],
    programId
  );
}

/**
 * Derive the junior tranche token mint PDA.
 *
 * Seeds: `["junior_mint", trancheConfig]`
 */
export function findJuniorMintPda(
  trancheConfig: PublicKey,
  programId: PublicKey = VOLTR_TRANCHING_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.JUNIOR_MINT, trancheConfig.toBuffer()],
    programId
  );
}

/**
 * Derive a WithdrawReceipt PDA for a given receipt ID.
 *
 * Seeds: `["withdraw_receipt", trancheConfig, receiptId.to_le_bytes()]`
 */
export function findWithdrawReceiptPda(
  trancheConfig: PublicKey,
  receiptId: BN | number | bigint,
  programId: PublicKey = VOLTR_TRANCHING_PROGRAM_ID
): [PublicKey, number] {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(BigInt(receiptId.toString()));
  return PublicKey.findProgramAddressSync(
    [SEEDS.WITHDRAW_RECEIPT, trancheConfig.toBuffer(), buf],
    programId
  );
}

/**
 * Derive the AprFeed PDA for a given tranche config.
 *
 * Seeds: `["apr_feed", trancheConfig]`
 */
export function findAprFeedPda(
  trancheConfig: PublicKey,
  programId: PublicKey = VOLTR_TRANCHING_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.APR_FEED, trancheConfig.toBuffer()],
    programId
  );
}

/**
 * Derive all tranche PDAs for a given vault in one call.
 */
export function findAllTranchePdas(
  vault: PublicKey,
  programId: PublicKey = VOLTR_TRANCHING_PROGRAM_ID
): TranchePdas {
  const [trancheConfig] = findTrancheConfigPda(vault, programId);
  const [trancheAuthority] = findTrancheAuthorityPda(trancheConfig, programId);
  const [seniorMint] = findSeniorMintPda(trancheConfig, programId);
  const [juniorMint] = findJuniorMintPda(trancheConfig, programId);
  const [aprFeed] = findAprFeedPda(trancheConfig, programId);
  return { trancheConfig, trancheAuthority, seniorMint, juniorMint, aprFeed };
}

/**
 * Derive Voltr Vault PDAs needed for CPI calls.
 */
export function findVaultPdas(
  vault: PublicKey,
  vaultProgramId: PublicKey = VOLTR_VAULT_PROGRAM_ID
): VaultPdas {
  const [protocol] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.PROTOCOL],
    vaultProgramId
  );
  const [vaultLpMint] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.VAULT_LP_MINT, vault.toBuffer()],
    vaultProgramId
  );
  const [vaultLpMintAuth] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.VAULT_LP_MINT_AUTH, vault.toBuffer()],
    vaultProgramId
  );
  const [vaultAssetIdleAuth] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.VAULT_ASSET_IDLE_AUTH, vault.toBuffer()],
    vaultProgramId
  );
  return { protocol, vaultLpMint, vaultLpMintAuth, vaultAssetIdleAuth };
}
