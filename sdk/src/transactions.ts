import {
  ComputeBudgetProgram,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Program, BN } from "@coral-xyz/anchor";
import type { VoltrTranching } from "./types";
import {
  findAllTranchePdas,
  findVaultPdas,
  findWithdrawReceiptPda,
  TranchePdas,
  VaultPdas,
} from "./pda";
import { VOLTR_VAULT_PROGRAM_ID } from "./constants";

/**
 * Default compute budget for tranche instructions that include inline crank_accounting.
 */
const DEFAULT_COMPUTE_UNITS = 400_000;

/**
 * Prepend a compute budget instruction (set compute unit limit).
 */
export function withComputeBudget(
  instructions: TransactionInstruction[],
  units: number = DEFAULT_COMPUTE_UNITS
): TransactionInstruction[] {
  return [
    ComputeBudgetProgram.setComputeUnitLimit({ units }),
    ...instructions,
  ];
}

// ─── Shared Account Resolution ────────────────────────────────────────────────

interface ResolvedAccounts {
  pdas: TranchePdas;
  vaultPdas: VaultPdas;
  assetMint: PublicKey;
  assetEscrow: PublicKey;
  vaultLpCustody: PublicKey;
  vaultAssetIdleAta: PublicKey;
}

/**
 * Resolve the common accounts needed across most instructions.
 * Accepts optional pre-computed values to avoid redundant PDA derivation and RPC calls.
 */
async function resolveCommonAccounts(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  opts?: {
    pdas?: TranchePdas;
    vaultPdas?: VaultPdas;
    assetMint?: PublicKey;
  }
): Promise<ResolvedAccounts> {
  const pdas = opts?.pdas ?? findAllTranchePdas(vault, program.programId);
  const vaultPdas =
    opts?.vaultPdas ?? findVaultPdas(vault, VOLTR_VAULT_PROGRAM_ID);
  const assetMint =
    opts?.assetMint ?? (await fetchAssetMint(program, pdas.trancheConfig));
  const assetEscrow = getAssociatedTokenAddressSync(
    assetMint,
    pdas.trancheAuthority,
    true
  );
  const vaultLpCustody = getAssociatedTokenAddressSync(
    vaultPdas.vaultLpMint,
    pdas.trancheAuthority,
    true
  );
  const vaultAssetIdleAta = getAssociatedTokenAddressSync(
    assetMint,
    vaultPdas.vaultAssetIdleAuth,
    true
  );
  return { pdas, vaultPdas, assetMint, assetEscrow, vaultLpCustody, vaultAssetIdleAta };
}

/**
 * Optional overrides to skip redundant PDA derivation and RPC calls.
 */
export interface BuildIxOptions {
  pdas?: TranchePdas;
  vaultPdas?: VaultPdas;
  assetMint?: PublicKey;
}

// ─── Deposit Instructions ─────────────────────────────────────────────────────

/**
 * Build a `deposit_senior` instruction with all required accounts resolved.
 */
export async function buildDepositSeniorIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  return buildDepositIx(program, vault, user, amount, "senior", opts);
}

/**
 * Build a `deposit_junior` instruction with all required accounts resolved.
 */
export async function buildDepositJuniorIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  return buildDepositIx(program, vault, user, amount, "junior", opts);
}

async function buildDepositIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  side: "senior" | "junior",
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  const { pdas, vaultPdas, assetMint, assetEscrow, vaultLpCustody, vaultAssetIdleAta } =
    await resolveCommonAccounts(program, vault, opts);

  const trancheMint = side === "senior" ? pdas.seniorMint : pdas.juniorMint;
  const userTrancheAta = getAssociatedTokenAddressSync(trancheMint, user);
  const userAssetAta = getAssociatedTokenAddressSync(assetMint, user);

  const accounts: Record<string, PublicKey> = {
    user,
    trancheConfig: pdas.trancheConfig,
    trancheAuthority: pdas.trancheAuthority,
    seniorMint: pdas.seniorMint,
    juniorMint: pdas.juniorMint,
    userAssetAta,
    assetEscrow,
    vaultLpCustody,
    assetMint,
    vault,
    vaultLpMint: vaultPdas.vaultLpMint,
    protocol: vaultPdas.protocol,
    vaultAssetIdleAta,
    vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
    vaultLpMintAuth: vaultPdas.vaultLpMintAuth,
    voltrVaultProgram: VOLTR_VAULT_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
  };

  if (side === "senior") {
    accounts.userSeniorAta = userTrancheAta;
    return program.methods
      .depositSenior(new BN(amount.toString()))
      .accountsPartial(accounts)
      .instruction();
  } else {
    accounts.userJuniorAta = userTrancheAta;
    return program.methods
      .depositJunior(new BN(amount.toString()))
      .accountsPartial(accounts)
      .instruction();
  }
}

// ─── Crank Instructions ───────────────────────────────────────────────────────

/**
 * Build a `crank_accounting` instruction (permissionless, sync — no RPC call).
 */
export function buildCrankAccountingIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  payer?: PublicKey,
  opts?: { pdas?: TranchePdas; vaultPdas?: VaultPdas }
): Promise<TransactionInstruction> {
  const pdas = opts?.pdas ?? findAllTranchePdas(vault, program.programId);
  const vaultPdas =
    opts?.vaultPdas ?? findVaultPdas(vault, VOLTR_VAULT_PROGRAM_ID);
  const vaultLpCustody = getAssociatedTokenAddressSync(
    vaultPdas.vaultLpMint,
    pdas.trancheAuthority,
    true
  );

  const accounts: Record<string, PublicKey> = {
    trancheConfig: pdas.trancheConfig,
    vault,
    vaultLpMint: vaultPdas.vaultLpMint,
    vaultLpCustody,
    seniorMint: pdas.seniorMint,
    juniorMint: pdas.juniorMint,
  };
  if (payer) accounts.payer = payer;

  return program.methods
    .crankAccounting()
    .accountsPartial(accounts)
    .instruction();
}

/**
 * Build a `push_apr_observation` instruction (permissionless, sync — no RPC call).
 */
export function buildPushAprObservationIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  opts?: { pdas?: TranchePdas; vaultPdas?: VaultPdas }
): Promise<TransactionInstruction> {
  const pdas = opts?.pdas ?? findAllTranchePdas(vault, program.programId);
  const vaultPdas =
    opts?.vaultPdas ?? findVaultPdas(vault, VOLTR_VAULT_PROGRAM_ID);

  return program.methods
    .pushAprObservation()
    .accountsPartial({
      trancheConfig: pdas.trancheConfig,
      aprFeed: pdas.aprFeed,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
    })
    .instruction();
}

// ─── Request Withdraw Instructions ────────────────────────────────────────────

/**
 * Build a `request_withdraw_senior` instruction.
 */
export async function buildRequestWithdrawSeniorIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  isTokenAmount: boolean,
  receiptId: BN | number | bigint,
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  return buildRequestWithdrawIx(
    program, vault, user, amount, isTokenAmount, receiptId, "senior", opts
  );
}

/**
 * Build a `request_withdraw_junior` instruction.
 */
export async function buildRequestWithdrawJuniorIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  isTokenAmount: boolean,
  receiptId: BN | number | bigint,
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  return buildRequestWithdrawIx(
    program, vault, user, amount, isTokenAmount, receiptId, "junior", opts
  );
}

async function buildRequestWithdrawIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  isTokenAmount: boolean,
  receiptId: BN | number | bigint,
  side: "senior" | "junior",
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  const pdas = opts?.pdas ?? findAllTranchePdas(vault, program.programId);
  const vaultPdas =
    opts?.vaultPdas ?? findVaultPdas(vault, VOLTR_VAULT_PROGRAM_ID);
  const trancheMint = side === "senior" ? pdas.seniorMint : pdas.juniorMint;
  const userTrancheAta = getAssociatedTokenAddressSync(trancheMint, user);
  const trancheTokenEscrow = getAssociatedTokenAddressSync(
    trancheMint,
    pdas.trancheAuthority,
    true
  );
  const vaultLpCustody = getAssociatedTokenAddressSync(
    vaultPdas.vaultLpMint,
    pdas.trancheAuthority,
    true
  );
  const [withdrawReceipt] = findWithdrawReceiptPda(
    pdas.trancheConfig,
    receiptId,
    program.programId
  );

  const accounts: Record<string, PublicKey> = {
    user,
    trancheConfig: pdas.trancheConfig,
    trancheAuthority: pdas.trancheAuthority,
    seniorMint: pdas.seniorMint,
    juniorMint: pdas.juniorMint,
    vaultLpCustody,
    vault,
    vaultLpMint: vaultPdas.vaultLpMint,
    withdrawReceipt,
    tokenProgram: TOKEN_PROGRAM_ID,
  };

  if (side === "senior") {
    accounts.userSeniorAta = userTrancheAta;
    accounts.seniorTokenEscrow = trancheTokenEscrow;
    return program.methods
      .requestWithdrawSenior(new BN(amount.toString()), isTokenAmount)
      .accountsPartial(accounts)
      .instruction();
  } else {
    accounts.userJuniorAta = userTrancheAta;
    accounts.juniorTokenEscrow = trancheTokenEscrow;
    return program.methods
      .requestWithdrawJunior(new BN(amount.toString()), isTokenAmount)
      .accountsPartial(accounts)
      .instruction();
  }
}

// ─── Complete / Cancel Withdraw ───────────────────────────────────────────────

/**
 * Build a `complete_withdraw` instruction.
 */
export async function buildCompleteWithdrawIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  receiptId: BN | number | bigint,
  isSenior: boolean,
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  const { pdas, vaultPdas, assetEscrow, vaultLpCustody, vaultAssetIdleAta } =
    await resolveCommonAccounts(program, vault, opts);
  const assetMint =
    opts?.assetMint ?? (await fetchAssetMint(program, pdas.trancheConfig));
  const userAssetAta = getAssociatedTokenAddressSync(assetMint, user);
  const trancheTokenEscrow = getAssociatedTokenAddressSync(
    isSenior ? pdas.seniorMint : pdas.juniorMint,
    pdas.trancheAuthority,
    true
  );
  const [withdrawReceipt] = findWithdrawReceiptPda(
    pdas.trancheConfig,
    receiptId,
    program.programId
  );

  return program.methods
    .completeWithdraw()
    .accountsPartial({
      user,
      trancheConfig: pdas.trancheConfig,
      trancheAuthority: pdas.trancheAuthority,
      seniorMint: pdas.seniorMint,
      juniorMint: pdas.juniorMint,
      trancheTokenEscrow,
      assetEscrow,
      userAssetAta,
      vaultLpCustody,
      assetMint,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
      protocol: vaultPdas.protocol,
      vaultAssetIdleAta,
      vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
      withdrawReceipt,
      voltrVaultProgram: VOLTR_VAULT_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
}

/**
 * Build a `cancel_withdraw` instruction (sync — no RPC call).
 */
export function buildCancelWithdrawIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  receiptId: BN | number | bigint,
  isSenior: boolean,
  opts?: { pdas?: TranchePdas }
): Promise<TransactionInstruction> {
  const pdas = opts?.pdas ?? findAllTranchePdas(vault, program.programId);
  const trancheMint = isSenior ? pdas.seniorMint : pdas.juniorMint;
  const userTrancheAta = getAssociatedTokenAddressSync(trancheMint, user);
  const trancheTokenEscrow = getAssociatedTokenAddressSync(
    trancheMint,
    pdas.trancheAuthority,
    true
  );
  const [withdrawReceipt] = findWithdrawReceiptPda(
    pdas.trancheConfig,
    receiptId,
    program.programId
  );

  return program.methods
    .cancelWithdraw()
    .accountsPartial({
      user,
      trancheConfig: pdas.trancheConfig,
      trancheAuthority: pdas.trancheAuthority,
      withdrawReceipt,
      trancheMint,
      trancheTokenEscrow,
      userTrancheAta,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
}

// ─── Instant Withdraw Instructions ────────────────────────────────────────────

/**
 * Build an `instant_withdraw_senior` instruction.
 */
export async function buildInstantWithdrawSeniorIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  isTokenAmount: boolean,
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  return buildInstantWithdrawIx(
    program, vault, user, amount, isTokenAmount, "senior", opts
  );
}

/**
 * Build an `instant_withdraw_junior` instruction.
 */
export async function buildInstantWithdrawJuniorIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  isTokenAmount: boolean,
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  return buildInstantWithdrawIx(
    program, vault, user, amount, isTokenAmount, "junior", opts
  );
}

async function buildInstantWithdrawIx(
  program: Program<VoltrTranching>,
  vault: PublicKey,
  user: PublicKey,
  amount: BN | number | bigint,
  isTokenAmount: boolean,
  side: "senior" | "junior",
  opts?: BuildIxOptions
): Promise<TransactionInstruction> {
  const { pdas, vaultPdas, assetMint, assetEscrow, vaultLpCustody, vaultAssetIdleAta } =
    await resolveCommonAccounts(program, vault, opts);
  const trancheMint = side === "senior" ? pdas.seniorMint : pdas.juniorMint;
  const userTrancheAta = getAssociatedTokenAddressSync(trancheMint, user);
  const userAssetAta = getAssociatedTokenAddressSync(assetMint, user);

  const accounts: Record<string, PublicKey> = {
    user,
    trancheConfig: pdas.trancheConfig,
    trancheAuthority: pdas.trancheAuthority,
    seniorMint: pdas.seniorMint,
    juniorMint: pdas.juniorMint,
    userAssetAta,
    assetEscrow,
    vaultLpCustody,
    assetMint,
    vault,
    vaultLpMint: vaultPdas.vaultLpMint,
    protocol: vaultPdas.protocol,
    vaultAssetIdleAta,
    vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
    voltrVaultProgram: VOLTR_VAULT_PROGRAM_ID,
    tokenProgram: TOKEN_PROGRAM_ID,
  };

  if (side === "senior") {
    accounts.userSeniorAta = userTrancheAta;
    return program.methods
      .instantWithdrawSenior(new BN(amount.toString()), isTokenAmount)
      .accountsPartial(accounts)
      .instruction();
  } else {
    accounts.userJuniorAta = userTrancheAta;
    return program.methods
      .instantWithdrawJunior(new BN(amount.toString()), isTokenAmount)
      .accountsPartial(accounts)
      .instruction();
  }
}

// ─── Internal Helpers ────────────────────────────────────────────────────────

/**
 * Fetch the asset mint from a TrancheConfig account.
 */
async function fetchAssetMint(
  program: Program<VoltrTranching>,
  trancheConfig: PublicKey
): Promise<PublicKey> {
  const config = await (
    program.account as any
  ).trancheConfig.fetch(trancheConfig);
  return config.assetMint;
}
