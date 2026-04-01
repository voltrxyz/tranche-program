# @voltr/tranching-sdk

TypeScript client SDK for the **Voltr Tranching Program** — a two-tranche (Senior / Junior) yield structure built on Solana via [Voltr Vaults](https://voltr.xyz).

## Installation

```bash
npm install @voltr/tranching-sdk
```

## Quick Start

```typescript
import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import { BN } from "bn.js";
import {
  findAllTranchePdas,
  findWithdrawReceiptPda,
  buildDepositSeniorIx,
  buildCrankAccountingIx,
  withComputeBudget,
  seniorPrice,
  juniorPrice,
  coverageRatio,
  readDecimalBits,
  VOLTR_TRANCHING_PROGRAM_ID,
} from "@voltr/tranching-sdk";

// ── Setup ────────────────────────────────────────────────────────────────────
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace.VoltrTranching;
const vault = new PublicKey("YOUR_VAULT_ADDRESS");

// ── Derive all PDAs ──────────────────────────────────────────────────────────
const pdas = findAllTranchePdas(vault);
console.log("Tranche Config:", pdas.trancheConfig.toBase58());
console.log("Senior Mint:", pdas.seniorMint.toBase58());
console.log("Junior Mint:", pdas.juniorMint.toBase58());

// ── Deposit into Senior Tranche ──────────────────────────────────────────────
const depositIx = await buildDepositSeniorIx(
  program,
  vault,
  provider.wallet.publicKey,
  new BN(1_000_000) // 1 USDC (6 decimals)
);

const tx = new Transaction().add(
  ...withComputeBudget([depositIx], 400_000)
);
const sig = await provider.sendAndConfirm(tx);
console.log("Deposit tx:", sig);

// ── Read Tranche State ───────────────────────────────────────────────────────
const config = await program.account.trancheConfig.fetch(pdas.trancheConfig);

console.log("Total NAV:", readDecimalBits(config.totalNav));
console.log("Senior NAV:", readDecimalBits(config.seniorNav));
console.log("Junior NAV:", readDecimalBits(config.juniorNav));
console.log("Coverage Ratio:", coverageRatio(config.totalNav, config.seniorNav));

// ── Compute Prices ───────────────────────────────────────────────────────────
const seniorMintInfo = await getMint(connection, pdas.seniorMint);
console.log(
  "Senior Price:",
  seniorPrice(config.seniorNav, seniorMintInfo.supply)
);
```

## API Reference

### PDA Derivation

All PDA helpers accept an optional `programId` parameter (defaults to the deployed program ID).

```typescript
import {
  findTrancheConfigPda,
  findTrancheAuthorityPda,
  findSeniorMintPda,
  findJuniorMintPda,
  findWithdrawReceiptPda,
  findAprFeedPda,
  findAllTranchePdas,
  findVaultPdas,
} from "@voltr/tranching-sdk";

// Individual PDAs (returns [PublicKey, bump])
const [trancheConfig, bump] = findTrancheConfigPda(vault);
const [authority] = findTrancheAuthorityPda(trancheConfig);
const [seniorMint] = findSeniorMintPda(trancheConfig);
const [juniorMint] = findJuniorMintPda(trancheConfig);
const [receipt] = findWithdrawReceiptPda(trancheConfig, receiptId);
const [aprFeed] = findAprFeedPda(trancheConfig);

// All tranche PDAs at once
const pdas = findAllTranchePdas(vault);
// => { trancheConfig, trancheAuthority, seniorMint, juniorMint, aprFeed }

// Vault CPI PDAs
const vaultPdas = findVaultPdas(vault);
// => { protocol, vaultLpMint, vaultLpMintAuth, vaultAssetIdleAuth }
```

### NAV / Price Calculations

```typescript
import {
  readDecimalBits,
  seniorPrice,
  juniorPrice,
  coverageRatio,
  juniorSeniorRatioBps,
  estimateExitFee,
  estimateTokensMinted,
} from "@voltr/tranching-sdk";

// Decode on-chain DecimalBits (U80F48) to a JS number
const nav = readDecimalBits(config.totalNav);

// Price per token
const srPrice = seniorPrice(config.seniorNav, seniorSupply);
const jrPrice = juniorPrice(config.juniorNav, juniorSupply);

// Coverage = total_nav / senior_nav (>1 means junior is covering senior)
const coverage = coverageRatio(config.totalNav, config.seniorNav);

// Junior/Senior ratio in BPS
const jrSrRatio = juniorSeniorRatioBps(config.juniorNav, config.seniorNav);

// Fee estimation (ceiling division, protocol-favoring)
const fee = estimateExitFee(1_000_000n, 50); // 0.5% fee on 1 USDC

// Token minting estimation (floor division, protocol-favoring)
const tokens = estimateTokensMinted(1_000_000n, config.seniorNav, seniorSupply);
```

### Transaction Builders

All builders resolve accounts automatically (PDA derivation + ATA computation). They return a `TransactionInstruction` that you add to a `Transaction`.

```typescript
import {
  buildDepositSeniorIx,
  buildDepositJuniorIx,
  buildCrankAccountingIx,
  buildPushAprObservationIx,
  buildRequestWithdrawSeniorIx,
  buildRequestWithdrawJuniorIx,
  buildCompleteWithdrawIx,
  buildCancelWithdrawIx,
  buildInstantWithdrawSeniorIx,
  buildInstantWithdrawJuniorIx,
  withComputeBudget,
} from "@voltr/tranching-sdk";

// Deposits (inline crank_accounting, need higher CU)
const depositSrIx = await buildDepositSeniorIx(program, vault, user, amount);
const depositJrIx = await buildDepositJuniorIx(program, vault, user, amount);

// Permissionless cranks
const crankIx = await buildCrankAccountingIx(program, vault);
const aprIx = await buildPushAprObservationIx(program, vault);

// Two-step withdrawals
const requestSrIx = await buildRequestWithdrawSeniorIx(
  program, vault, user, amount, isTokenAmount, receiptId
);
const completeSrIx = await buildCompleteWithdrawIx(
  program, vault, user, receiptId, /* isSenior */ true
);
const cancelIx = await buildCancelWithdrawIx(
  program, vault, user, receiptId, /* isSenior */ true
);

// Instant withdrawals (when cooldown == 0)
const instantSrIx = await buildInstantWithdrawSeniorIx(
  program, vault, user, amount, isTokenAmount
);

// Wrap with compute budget (400K CU default)
const tx = new Transaction().add(...withComputeBudget([depositSrIx]));
```

### Constants

```typescript
import {
  VOLTR_TRANCHING_PROGRAM_ID,
  VOLTR_VAULT_PROGRAM_ID,
  SEEDS,
  BPS_PRECISION,
  SECONDS_PER_YEAR,
  MAX_RESERVE_FEE_BPS,
  MAX_EXIT_FEE_BPS,
  PAUSE_FLAGS,
} from "@voltr/tranching-sdk";

// Pause flag bits
PAUSE_FLAGS.SENIOR_DEPOSITS    // 0x01
PAUSE_FLAGS.JUNIOR_DEPOSITS    // 0x02
PAUSE_FLAGS.SENIOR_WITHDRAWALS // 0x04
PAUSE_FLAGS.JUNIOR_WITHDRAWALS // 0x08
PAUSE_FLAGS.ALL                // 0x0F
```

## IDL

The program IDL is generated at `target/idl/voltr_tranching.json` by `anchor build`. It contains all 16 instructions, 3 account types, 9 events, and 17 error codes.

## Development

```bash
# Build the SDK
cd sdk && npm run build

# Regenerate IDL after program changes
NO_DNA=1 anchor build

# Copy updated types
cp target/types/voltr_tranching.ts sdk/src/types.ts
```
