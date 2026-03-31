# Voltr Tranching Program

## Keeping This File Up to Date

This `CLAUDE.md` serves as the single source of truth for all workspace agents. **Whenever you make changes that affect the architecture, account structures, math patterns, instruction flows, or security constraints described in this document, update the relevant sections of this file as part of the same PR.** This includes but is not limited to:
- Updating math patterns, types, or function signatures (§5)
- Adding/removing/renaming instructions or accounts (§3, §4, §6)
- Changing error codes or events (§8, §9)
- Updating security design or constraints (§10, §11)

Do not let this document drift from the actual codebase. The phase/ticket checklist is roadmap context, not a per-PR implementation status tracker, so it does not need to be updated whenever a ticket is implemented.

## Skills & Tools

Always load the following skills when working in this repository:

### Required Skills (pre-installed)
- **`/solana-dev`** — Load at the start of ANY Solana/Anchor program task (building, testing, deploying, debugging). Covers Anchor patterns, CPI, PDA derivation, LiteSVM testing, security checklists, and Codama client generation.
- **`/simplify`** — Run after completing any code change to review for reuse, quality, and efficiency before committing.

### Required MCP Servers (pre-installed)
- **`solana-mcp-server`** — Use `Solana_Documentation_Search` for SDK/runtime questions, `Ask_Solana_Anchor_Framework_Expert` for Anchor-specific patterns, and `Solana_Expert__Ask_For_Help` for general Solana help.
- **`linear`** — All project tickets are tracked in Linear under the "Voltr Tranching Program" project. Reference tickets by VOL-{number}. When you begin work on a ticket, look up the matching Linear issue and mark its status as `In Progress`. When you are explicitly tasked with creating a PR for a completed ticket, mark that Linear ticket as `Done` after the PR is created.

### Recommended Plugins (install if missing)

Run these to install critical plugins for this project:

```bash
# Trail of Bits security skills (smart contract auditing, static analysis, property-based testing)
claude plugin marketplace add trailofbits/skills
claude plugin install building-secure-contracts
claude plugin install static-analysis
claude plugin install property-based-testing

# Safe Solana Builder (security-first Solana program development)
# Install from: https://github.com/Frankcastleauditor/safe-solana-builder
# Download safe-solana-builder.skill from Releases, or clone into ~/.claude/skills/
```

### When to Use Which Tool

| Task | Skill / Tool |
|------|-------------|
| Writing Anchor program code | `/solana-dev` + `Ask_Solana_Anchor_Framework_Expert` |
| Debugging Solana errors | `/solana-dev` + `Solana_Expert__Ask_For_Help` |
| Looking up Solana SDK APIs | `Solana_Documentation_Search` |
| Reviewing code quality | `/simplify` |
| Security review before audit | `building-secure-contracts` + `/solana-dev` security checklist |
| Checking ticket requirements | `linear` MCP → `get_issue` with VOL-{number} |
| Starting work on a ticket | `linear` MCP → `get_issue` with VOL-{number}, then mark as `In Progress` |
| Creating a PR for a ticket | `gh pr create`, then update the matching Linear ticket to `Done` |
| Building/testing | `NO_DNA=1 anchor build`, `NO_DNA=1 anchor test` |

---

## Technical PRD

**Version:** 1.0
**Date:** 2026-03-30
**Author:** Shayn Shin
**Status:** In Development
**Program Framework:** Anchor 0.31.1 / Solana 2.1.21

---

## 1. Overview

The Voltr Tranching Program is a Solana program implementing a **two-tranche yield structure (Senior / Junior)** on top of [Voltr Vaults](https://voltr.xyz) via CPI. Users deposit into either a **Senior tranche** (lower risk, protected yield) or **Junior tranche** (first-loss, leveraged yield). The program manages yield splitting via an on-chain waterfall, loss absorption, and coverage-aware deposit/withdrawal mechanics.

### 1.1 Design Goals

- Wrap any Voltr Vault (with `withdrawal_waiting_period == 0`) into a senior/junior tranche pair
- Deterministic yield waterfall: senior receives a risk-adjusted target APR; junior receives residual
- Junior-first loss absorption protects senior depositors
- Permissionless crank-based accounting — no trust in external keepers for correctness
- Full CPI integration with Voltr Vault for deposits, withdrawals, and LP token custody

---

## 2. Architecture

### 2.1 System Context

```
┌──────────┐     deposit/withdraw      ┌─────────────────────┐
│   User   │ ◄──────────────────────►  │  Voltr Tranching    │
│  Wallet  │    tranche tokens          │  Program            │
└──────────┘                            │                     │
                                        │  ┌───────────────┐  │
┌──────────┐     crank (permissionless) │  │ TrancheConfig  │  │
│  Keeper  │ ────────────────────────►  │  │ AprFeed        │  │
│  (bot)   │                            │  │ WithdrawReceipt│  │
└──────────┘                            │  └───────────────┘  │
                                        │         │ CPI       │
┌──────────┐     admin instructions     │         ▼           │
│ Authority│ ────────────────────────►  │  ┌───────────────┐  │
│  (owner) │                            │  │  Voltr Vault   │  │
└──────────┘                            │  │  (external)    │  │
                                        │  └───────────────┘  │
                                        └─────────────────────┘
```

### 2.2 Trust Boundaries

| Actor | Trust Level | Capabilities |
|-------|------------|--------------|
| **User** | Untrusted | Deposit, withdraw (own funds only), cancel withdrawals |
| **Authority** | Trusted admin | Configure parameters, pause/unpause, withdraw reserve, transfer authority |
| **Keeper/Cranker** | Untrusted (permissionless) | Trigger `crank_accounting`, `push_apr_observation` |
| **Voltr Vault** | Trusted external program | Holds underlying assets, provides LP token mechanics |

### 2.3 CPI Integration with Voltr Vault

The tranching program interacts with Voltr Vault via three CPI calls:

1. **`deposit_vault(amount)`** — deposits assets into the vault, receives LP tokens
2. **`instant_withdraw_vault(amount, is_token_amount, only_idle)`** — redeems LP tokens for assets
3. All CPI calls are signed by the **`tranche_authority`** PDA

The `tranche_authority` PDA acts as the depositor identity in Voltr Vault, custodying LP tokens in `vault_lp_custody` and holding an `asset_escrow` for intermediate transfers.

---

## 3. On-Chain Account Structures

### 3.1 TrancheConfig (zero-copy)

**PDA:** `[b"tranche_config", voltr_vault.key()]`
**Attributes:** `#[account(zero_copy(unsafe))]`, `#[repr(C)]`
**DecimalBits:** little-endian `[u8; 16]` bytes of `Decimal::to_decimal_bits()`, used in zero-copy accounts to keep Anchor loading alignment-safe.

| Field Group | Fields | Type | Description |
|------------|--------|------|-------------|
| **Identity** | `vault` | `Pubkey` | Underlying Voltr Vault address |
| | `authority` | `Pubkey` | Admin authority |
| | `pending_authority` | `Pubkey` | For 2-step transfer |
| **Tranche Mints** | `senior_mint` | `Pubkey` | Senior tranche token mint |
| | `junior_mint` | `Pubkey` | Junior tranche token mint |
| **Voltr Integration** | `lp_custody` | `Pubkey` | ATA holding vault LP tokens |
| | `asset_mint` | `Pubkey` | Underlying asset (e.g., USDC) |
| **NAV Tracking** | `total_nav` | `DecimalBits` | Total net asset value (little-endian bytes of `to_decimal_bits()`) |
| | `senior_nav` | `DecimalBits` | Senior tranche NAV (little-endian Decimal bits) |
| | `junior_nav` | `DecimalBits` | Junior tranche NAV (little-endian Decimal bits) |
| | `reserve_nav` | `DecimalBits` | Protocol reserve (little-endian Decimal bits) |
| **Senior Target** | `senior_target_index` | `DecimalBits` | Compound growth index (little-endian Decimal bits, init: `Decimal::ONE`) |
| | `index_timestamp` | `i64` | Last index update time |
| **APR Parameters** | `apr_floor_bps` | `u16` | Minimum senior APR (basis points) |
| | `risk_x_bps` | `u16` | Risk premium constant X |
| | `risk_y_bps` | `u16` | Risk premium constant Y |
| | `risk_k` | `DecimalBits` | Risk premium exponent K (stored as Decimal bits but must encode a whole-number `u32` exponent in v1) |
| | `base_apr_bps` | `u16` | Current vault base APR |
| **Fee Parameters** | `reserve_fee_bps` | `u16` | Performance fee (max 2000 = 20%) |
| | `senior_exit_fee_bps` | `u16` | Senior withdrawal fee (max 100 = 1%) |
| | `junior_exit_fee_bps` | `u16` | Junior withdrawal fee (max 100 = 1%) |
| **Coverage Thresholds** | `min_jr_sr_ratio_bps` | `u16` | Min junior/senior ratio |
| | `min_coverage_for_mint_bps` | `u16` | Min coverage to allow senior deposits |
| | `min_coverage_buffer_bps` | `u16` | Coverage buffer for senior minting |
| **Withdrawal Config** | `senior_cooldown_seconds` | `u64` | Senior withdrawal cooldown |
| | `junior_cooldown_seconds` | `u64` | Junior withdrawal cooldown |
| **Operational** | `paused` | `u8` | Bitflag pause state (see §6.2) |
| | `next_receipt_id` | `u64` | Auto-incrementing receipt counter |
| | `bumps` | various | PDA bumps for all derived accounts |
| **Reserved** | `_reserved` | `[u8; 248]` | Future expansion |

### 3.2 WithdrawReceipt (standard account)

**PDA:** `[b"withdraw_receipt", tranche_config.key(), receipt_id.to_le_bytes()]`

| Field | Type | Description |
|-------|------|-------------|
| `tranche_config` | `Pubkey` | Parent config |
| `user` | `Pubkey` | Withdrawing user |
| `receipt_id` | `u64` | Unique, auto-incremented |
| `is_senior` | `bool` | Senior or junior tranche |
| `tranche_tokens_escrowed` | `u64` | Tokens held in escrow |
| `asset_amount_at_request` | `u128` | NAV snapshot at request time (value cap) |
| `withdrawable_from_ts` | `i64` | Cooldown expiry timestamp |
| `bump` | `u8` | PDA bump |

### 3.3 AprFeed (zero-copy)

**PDA:** `[b"apr_feed", tranche_config.key()]`

| Field | Type | Description |
|-------|------|-------------|
| `head` | `u16` | Ring buffer head pointer |
| `count` | `u16` | Number of observations (max 72) |
| `observations` | `[AprObservation; 72]` | Circular buffer |

**AprObservation:**

| Field | Type | Description |
|-------|------|-------------|
| `vault_nav` | `DecimalBits` | Vault NAV at observation |
| `vault_lp_supply` | `DecimalBits` | Vault LP token supply |
| `timestamp` | `i64` | Unix timestamp |

---

## 4. PDA Map & Seed Constants

| Account | Seeds | Signer? |
|---------|-------|---------|
| `tranche_config` | `[b"tranche_config", vault.key()]` | No |
| `tranche_authority` | `[b"tranche_authority", tranche_config.key()]` | Yes (CPI signer) |
| `senior_mint` | `[b"senior_mint", tranche_config.key()]` | No |
| `junior_mint` | `[b"junior_mint", tranche_config.key()]` | No |
| `withdraw_receipt` | `[b"withdraw_receipt", config.key(), receipt_id.to_le_bytes()]` | No |
| `apr_feed` | `[b"apr_feed", tranche_config.key()]` | No |

The `tranche_authority` PDA is the sole signer for all CPI calls to Voltr Vault and all token operations (mint_to, burn, transfer). Its bump is stored in `TrancheConfig` at initialization.

---

## 5. Math Library

All NAV, APR, and waterfall calculations re-use the **Voltr Vault math utilities** (`voltr_vault::utils`), which provide `Decimal` (`U80F48` — 80-bit integer, 48-bit fractional binary fixed-point), `SafeMath`, `DecimalExtra`, and `U256` for wide intermediates. The tranching program's `math` module re-exports these types and adds tranching-specific constants. No custom fixed-point functions are needed.

### 5.1 Re-exported Types (from `voltr_vault::utils`)

| Type / Trait | Description |
|-------------|-------------|
| `Decimal` (`U80F48`) | Binary fixed-point type for ratios, indices, and per-share prices. Logically represented as `u128` bits; zero-copy accounts persist those bits as little-endian `[u8; 16]` via `to_decimal_bits().to_le_bytes()` |
| `SafeMath` | Checked `safe_add`, `safe_sub`, `safe_mul`, `safe_div`, `safe_div_ceil` for `u128`, `u64`, `Decimal`, and other primitives |
| `DecimalExtra` | `from_bps()`, `to_bps()`, `from_percent()`, `checked_pow()`, `mul_int_ratio()`, `full_mul_int_ratio()`, `to_floor()`, `to_ceil()` |
| `CheckedCeilDiv` | `checked_ceil_div()` for integer types |
| `U256` | 256-bit unsigned integer for overflow-safe wide intermediates in `full_mul_int_ratio()` |

### 5.2 Tranching-Specific Constants

| Constant | Value | Type | Purpose |
|----------|-------|------|---------|
| `BPS_PRECISION` | `10_000` | `u16` | Basis points denominator |
| `SECONDS_PER_YEAR` | `31_557_600` | `u64` | APR annualization (365.25 days) |

### 5.3 Math Patterns

**Token amounts** stay as `u64`/`u128` and use `SafeMath` for checked integer arithmetic:
```rust
let fee = amount.safe_mul(fee_bps as u128)?.safe_div_ceil(BPS_PRECISION as u128)?;
```

**Ratios and indices** use `Decimal`. In zero-copy accounts, persist `to_decimal_bits().to_le_bytes()` and reconstruct with `u128::from_le_bytes(...)` before `Decimal::from_decimal_bits(...)`:
```rust
let ratio = Decimal::from_num(deposit_amount).full_mul_int_ratio(total_supply, total_nav);
let tokens_to_mint: u64 = ratio.to_floor(); // round down for minting
```

**BPS conversions** use `DecimalExtra`:
```rust
let rate = Decimal::from_bps(500u64); // 5%
```

### 5.4 Rounding Convention

- **`.to_floor()`** for minting / amounts out (fewer tokens issued = protocol-favoring)
- **`.to_ceil()` / `safe_div_ceil()`** for fees / LP burns (more fee collected = protocol-favoring)
- All operations use `checked_*` or `safe_*` arithmetic; no `unwrap()` in any math path

---

## 6. Instructions

### 6.1 Initialization

#### `initialize_tranche_config`
**Signer:** Authority (payer)

Creates a new TrancheConfig wrapping a Voltr Vault. Initializes all derived accounts:

- TrancheConfig PDA
- tranche_authority PDA
- senior_mint PDA (decimals = asset mint decimals)
- junior_mint PDA (decimals = asset mint decimals)
- vault_lp_custody ATA
- asset_escrow ATA
- AprFeed PDA

**Validations:**
- Voltr Vault account `owner == voltr_vault::ID`
- `vault.withdrawal_waiting_period == 0` (v1 constraint)
- Mint authority for both tranche mints = `tranche_authority`
- `risk_k` must be a whole-number exponent representable as `u32`

**Initial state:** `senior_target_index = Decimal::ONE` (stored as `Decimal::ONE.to_decimal_bits().to_le_bytes()`), `index_timestamp = Clock::now`

---

### 6.2 Crank Instructions (Permissionless)

#### `crank_accounting`
**Signer:** Fee payer only (permissionless)

The core accounting engine. Reads Voltr Vault state (no CPI) and recalculates tranche NAVs via the yield waterfall.

**NAV Calculation:**
```
current_nav = our_lp_balance * vault_total_value / vault_lp_supply
delta = current_nav - total_nav
```

**Yield Waterfall (delta > 0 — gain):**

1. **Reserve fee:** `reserve_gain = delta * reserve_fee_bps / 10_000`
2. **Net gain:** `net_gain = delta - reserve_gain`
3. **Senior APR computation:**
   ```
   risk_premium = risk_x + risk_y * (senior_tvl / total_tvl) ^ risk_k
   senior_apr = max(apr_floor, base_apr * (1 - risk_premium))
   ```
4. **Senior target index:** compound growth based on elapsed time and `senior_apr`
5. **Senior gain:** `min(senior_target_gain, net_gain)`
6. **Junior gain:** `net_gain - senior_gain` (residual)

**Loss Waterfall (delta < 0 — loss):**

1. **Junior absorbs first:** `junior_loss = min(junior_nav, abs(delta))`
2. **Reserve absorbs next:** `reserve_loss = min(reserve_nav, remaining)`
3. **Senior absorbs last:** `senior_loss = min(senior_nav, remaining)`

**Auto-pause:** If junior price-per-share falls below shortfall threshold, deposit pause flags are set automatically.

**Emits:** `CrankAccountingEvent { delta, senior_nav, junior_nav, reserve_nav, total_nav, base_apr, coverage_ratio, slot }`

#### `push_apr_observation`
**Signer:** Fee payer only (permissionless)

Pushes a new observation to the AprFeed ring buffer (~hourly via keeper).

**APR Derivation:**
```
base_apr = (nav_per_lp_now / nav_per_lp_oldest - 1) * SECONDS_PER_YEAR / time_window
```

Ring buffer capacity: 72 observations (≈3 days at hourly intervals).

---

### 6.3 Deposit Instructions

#### `deposit_senior`
**Signer:** User

1. Validate: not paused (`0x01`), amount > 0
2. Inline `crank_accounting()` — sync NAV before pricing
3. **Coverage check:** `(total_nav + amount) / (senior_nav + amount) >= min_coverage_buffer`
4. Transfer: user → asset_escrow
5. CPI: `voltr_vault::deposit_vault(amount)` (tranche_authority signs)
6. Mint senior tokens: `amount * senior_supply / senior_nav` (or `amount` if first deposit)
7. Update: `senior_nav += amount`, `total_nav += amount`
8. Emit `DepositSeniorEvent`

#### `deposit_junior`
**Signer:** User

Same flow as `deposit_senior` except:
- **No coverage check** — junior deposits always improve coverage
- Mints junior tokens, updates `junior_nav`
- Pause flag: `0x02`

---

### 6.4 Withdrawal Instructions (Two-Step)

The two-step withdrawal flow protects against cooldown gaming and ensures fair pricing.

#### `request_withdraw_senior`
**Signer:** User

1. Validate: not paused (`0x04`), amount > 0, senior supply > 0, senior NAV > 0
2. Inline `crank_accounting()`
3. Calculate token/asset amounts based on `is_token_amount` flag
4. Apply exit fee: `fee = asset_value * senior_exit_fee_bps / 10_000` (rounded up), `net_asset = asset_value - fee`
5. `reserve_nav += fee`
6. Transfer senior tokens → `senior_token_escrow` (ATA owned by `tranche_authority`)
7. Increment `next_receipt_id`, create WithdrawReceipt PDA with `withdrawable_from_ts = now + senior_cooldown_seconds`
8. Emit `RequestWithdrawEvent`

#### `request_withdraw_junior`
**Signer:** User

Same as senior request, plus:
- **Junior ratio check:** `junior_nav - asset_value >= senior_nav * min_jr_sr_ratio_bps / 10_000`
- Uses `junior_exit_fee_bps` and `junior_cooldown_seconds`
- Pause flag: `0x08`

#### `complete_withdraw`
**Signer:** User (receipt owner)

1. Validate: `Clock::now >= withdrawable_from_ts`
2. Inline `crank_accounting()`
3. Recalculate current value: `current_value = tokens * tranche_nav / tranche_supply`
4. **Value cap:** `final_value = min(current_value, asset_amount_at_request)` — prevents cooldown gaming
5. CPI: `voltr_vault::instant_withdraw_vault(final_value)` (tranche_authority signs)
6. Transfer: asset_escrow → user
7. Burn escrowed tranche tokens
8. Update NAVs
9. Junior ratio re-checked for junior withdrawals
10. Close WithdrawReceipt (rent → user)
11. Emit `CompleteWithdrawEvent`

#### `cancel_withdraw`
**Signer:** User (receipt owner)

1. Return escrowed tranche tokens → user
2. Close WithdrawReceipt (rent → user)
3. **Exit fee is NOT refunded** — prevents fee gaming
4. Emit `CancelWithdrawEvent`

---

### 6.5 Instant Withdrawal Instructions

#### `instant_withdraw_senior` / `instant_withdraw_junior`
**Signer:** User

Single-transaction withdrawals when `cooldown_seconds == 0`.

1. Require: `cooldown_seconds == 0` for respective tranche
2. Inline `crank_accounting()`
3. Calculate amounts, apply exit fee
4. CPI: `voltr_vault::instant_withdraw_vault`
5. Transfer: escrow → user
6. Burn tranche tokens directly (no receipt)
7. Junior ratio enforced for junior withdrawals
8. Emit events

---

### 6.6 Admin Instructions

#### `update_tranche_config`
**Signer:** Authority

Uses an enum `TrancheConfigField` pattern (one field per call). Updatable fields:

| Field | Max Value | Notes |
|-------|-----------|-------|
| `apr_floor_bps` | — | Minimum senior APR |
| `risk_x_bps`, `risk_y_bps`, `risk_k` | — | Risk premium curve (`risk_k` must be a whole-number `u32` in v1) |
| `reserve_fee_bps` | 2000 (20%) | Performance fee |
| `senior_exit_fee_bps` | 100 (1%) | Senior exit fee |
| `junior_exit_fee_bps` | 100 (1%) | Junior exit fee |
| `min_jr_sr_ratio_bps` | — | Junior/senior floor |
| `min_coverage_for_mint_bps` | — | Coverage for minting |
| `min_coverage_buffer_bps` | — | Coverage buffer |
| `senior_cooldown_seconds` | — | Senior cooldown |
| `junior_cooldown_seconds` | — | Junior cooldown |

Fee bounds validated; returns `InvalidFee` if exceeded. Invalid whole-number `risk_k` values return `InvalidRiskConfig`.

#### `set_paused`
**Signer:** Authority

Sets the `paused` bitflag as a full replacement (not toggle):

| Bit | Flag | Effect |
|-----|------|--------|
| `0x01` | Senior deposits paused | `deposit_senior` blocked |
| `0x02` | Junior deposits paused | `deposit_junior` blocked |
| `0x04` | Senior withdrawals paused | `request_withdraw_senior` blocked |
| `0x08` | Junior withdrawals paused | `request_withdraw_junior` blocked |

Authority can clear auto-pause flags set by `crank_accounting`.

#### `transfer_authority` / `accept_authority`
**Signer:** Current authority / Pending authority

Two-step authority transfer (matching Voltr Vault's pattern):
1. `transfer_authority(new_authority)` — sets `pending_authority`
2. `accept_authority()` — called by `pending_authority`, replaces `authority`, clears `pending_authority`

Old authority retains full control until accept is called.

#### `withdraw_reserve`
**Signer:** Authority

Withdraws accumulated protocol reserve (performance fees):

1. Inline `crank_accounting()`
2. Calculate LP to redeem: `lp_amount = reserve_nav * total_lp / total_nav`
3. CPI: `voltr_vault::instant_withdraw_vault`
4. Transfer: escrow → authority destination ATA
5. Supports partial withdrawal (capped at `reserve_nav`)

---

## 7. Yield Waterfall — Worked Examples

**Setup:** 1M USDC total, 700K Senior NAV, 300K Junior NAV, 5% reserve fee, 7% senior target APR.

| Scenario | Vault Yield | Reserve (5%) | Senior | Junior |
|----------|------------|--------------|--------|--------|
| Normal +10% | +100K | 5K | 49K | 46K |
| High +15% | +150K | 7.5K | 49K | 93.5K |
| Low +3% | +30K | 1.5K | 19.95K | 8.55K |
| Loss -5% | -50K | 0 | 0 | -50K |
| Large loss -40% | -400K | 0 | -100K | -300K |

**Key properties:**
- Senior is protected up to the point where junior NAV is fully exhausted
- Junior absorbs all losses before senior is affected
- Reserve takes its cut only on gains (never on losses)
- Junior receives all excess yield above senior target — leveraged upside

---

## 8. Error Codes

| Error | Trigger |
|-------|---------|
| `SeniorDepositsPaused` | `deposit_senior` when flag `0x01` set |
| `JuniorDepositsPaused` | `deposit_junior` when flag `0x02` set |
| `SeniorWithdrawalsPaused` | `request_withdraw_senior` when flag `0x04` set |
| `JuniorWithdrawalsPaused` | `request_withdraw_junior` when flag `0x08` set |
| `InsufficientCoverage` | Senior deposit would breach coverage buffer |
| `JuniorRatioBreach` | Junior withdrawal would breach min junior/senior ratio |
| `CooldownNotElapsed` | `complete_withdraw` before cooldown expiry |
| `ZeroAmount` | Deposit/withdraw with amount = 0 |
| `MathOverflow` | Checked arithmetic failure |
| `Unauthorized` | Caller is not the authority |
| `InvalidFee` | Fee parameter exceeds bounds |
| `InvalidRiskConfig` | `risk_k` is fractional or does not fit in `u32` |
| `JuniorShortfall` | Auto-pause triggered by junior price collapse |
| `VaultWaitingPeriodNotZero` | Vault has non-zero withdrawal waiting period (v1 unsupported) |
| `InvalidVault` | Vault account mismatch or wrong owner |
| `InvalidPauseFlags` | `set_paused` received bits outside `0x0F` |
| `CooldownMustBeZero` | `instant_withdraw_*` when tranche cooldown_seconds != 0 |

---

## 9. Events

| Event | Key Fields | Emitted By |
|-------|-----------|------------|
| `DepositSeniorEvent` | user, asset_amount, tokens_minted, senior_nav, coverage_ratio | `deposit_senior` |
| `DepositJuniorEvent` | user, asset_amount, tokens_minted, junior_nav, coverage_ratio | `deposit_junior` |
| `RequestWithdrawEvent` | user, receipt_id, is_senior, tokens_escrowed, asset_value, timestamp | `request_withdraw_*` |
| `CompleteWithdrawEvent` | user, receipt_id, is_senior, final_value, tokens_burned | `complete_withdraw` |
| `CancelWithdrawEvent` | user, receipt_id, tokens_returned | `cancel_withdraw` |
| `InstantWithdrawEvent` | user, is_senior, asset_amount, tokens_burned | `instant_withdraw_senior`, `instant_withdraw_junior` |
| `UpdateTrancheConfigEvent` | authority, field_index, slot | `update_tranche_config` |
| `CrankAccountingEvent` | delta, senior_nav, junior_nav, reserve_nav, total_nav, base_apr, coverage_ratio, slot | `crank_accounting` |

---

## 10. Security Design

### 10.1 Anti-Exploitation Mechanisms

| Attack Vector | Mitigation |
|--------------|------------|
| **Stale price exploitation** | Inline `crank_accounting()` at the top of every deposit/withdraw — NAV always fresh |
| **Cooldown gaming** | Value capped at request-time snapshot (`min(current, request_time)`) |
| **Fee gaming via cancel** | Exit fee is non-refundable — taken at request, not reversed on cancel |
| **Sandwich deposits** | Inline crank ensures price reflects latest vault state before minting |
| **First-depositor attack** | Dead-weight or minimum deposit logic prevents LP manipulation |
| **Receipt replay** | PDA with unique `receipt_id` — can't reuse |
| **Fake tranche_authority** | PDA seeds + bump validation |
| **Flash loan attacks** | Inline crank + single-tx atomic operations |

### 10.2 Account Validation Checklist

- All account owners validated via typed `Account<'info, T>` or explicit owner check
- All signer requirements explicit via `Signer<'info>` or `has_one`
- PDA seeds and canonical bumps validated by Anchor
- Token mint ↔ token account relationships validated
- No `init_if_needed` used anywhere
- No duplicate mutable accounts possible
- Voltr Vault program ID validated via `Program<'info, T>`
- All CPI `invoke_signed` seeds are correct and canonical

### 10.3 Arithmetic Safety

- All math uses `checked_*` / `safe_*` operations via Voltr Vault's `Decimal` (U80F48) and `SafeMath` traits
- No unchecked casts (u128 → u64 must be checked)
- Rounding always favors the protocol (`.to_floor()` for minting, `.to_ceil()` / `safe_div_ceil()` for fees)
- U256 intermediates prevent overflow in `full_mul_int_ratio()` for large proportional calculations

---

## 11. V1 Constraints & Assumptions

1. **Voltr Vault `withdrawal_waiting_period` must be 0** — the tranching program uses `instant_withdraw_vault` for all redemptions. Non-zero waiting period vaults are rejected at initialization.
2. **Single vault per TrancheConfig** — each config wraps exactly one Voltr Vault.
3. **No rebalancing** — the tranche ratio is market-driven by deposits/withdrawals, not actively managed.
4. **Keeper dependency** — `push_apr_observation` requires an external keeper calling ~hourly for accurate APR. `crank_accounting` is inlined into deposits/withdrawals but can also be called standalone.
5. **No token extensions** — tranche mints use standard SPL Token (not Token-2022).

---

## 12. Implementation Phases

### Phase 1: Foundation
- VOL-5: Scaffold Anchor project (0.31.1 / Solana 2.1.21)
- VOL-6: Math module re-exporting Voltr Vault's Decimal (U80F48) / SafeMath utilities
- VOL-7: Define account structures (TrancheConfig, WithdrawReceipt, AprFeed)
- VOL-8: Define error codes and event structs
- VOL-9: Define PDA authority and seed constants

### Phase 2: Core Instructions
- VOL-10: Implement initialize_tranche_config instruction
- VOL-11: Implement crank_accounting instruction (yield waterfall)
- VOL-12: Implement push_apr_observation crank instruction
- VOL-13: Implement deposit_senior instruction (CPI to Voltr)
- VOL-14: Implement deposit_junior instruction (CPI to Voltr)
- VOL-15: Implement request_withdraw_senior instruction
- VOL-16: Implement request_withdraw_junior instruction
- VOL-17: Implement complete_withdraw instruction (CPI to Voltr)
- VOL-18: Implement cancel_withdraw instruction
- VOL-19: Implement instant_withdraw_senior and instant_withdraw_junior

### Phase 3: Admin & Governance
- VOL-20: Implement update_tranche_config instruction
- VOL-21: Implement set_paused instruction
- VOL-22: Implement transfer_authority and accept_authority
- VOL-23: Implement withdraw_reserve instruction

### Phase 4: Testing & Security
- VOL-24: Unit tests: math module (Decimal, SafeMath, rounding conventions)
- VOL-25: Unit tests: waterfall yield distribution logic
- VOL-26: Integration tests: full deposit/withdraw flows with Voltr CPI
- VOL-27: Integration tests: coverage ratio and pause enforcement
- VOL-28: Adversarial tests: sandwich attacks and edge cases
- VOL-29: Fuzz testing with Trident
- VOL-30: Security review and checklist audit

### Phase 5: Deployment
- VOL-31: Generate IDL and TypeScript client SDK
- VOL-32: Deploy to devnet and run smoke tests
- VOL-33: Prepare external audit package

---

## 13. Testing Strategy

### Unit Tests
- Math module: Decimal arithmetic, SafeMath overflow, BPS conversions, rounding direction (floor vs ceil), `full_mul_int_ratio` with large values
- Waterfall logic: 5 PRD test vectors, loss waterfall priority, risk premium formula, NAV invariant

### Integration Tests (LiteSVM)
- Full lifecycle: init → deposit → crank → withdraw → verify balances
- Coverage ratio enforcement (9 test cases)
- Adversarial scenarios (9 attack vectors)
- Reserve withdrawal flows

### Fuzz Testing (Trident)
- Random instruction sequences, >1M iterations
- Invariants: `senior_nav + junior_nav + reserve_nav == total_nav`, no phantom tokens, coverage/ratio enforcement

### Security Review
- Internal checklist audit against Solana security best practices
- External audit package preparation with threat model documentation
