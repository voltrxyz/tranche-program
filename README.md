# Voltr Tranching Program

A Solana program implementing a two-tranche yield structure (Senior / Junior) on top of [Voltr Vaults](https://voltr.xyz) via CPI.

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.94+ | [rustup.rs](https://rustup.rs) |
| Solana CLI | 2.1.21 | `sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.21/install)"` |
| Anchor CLI | 0.31.1 | `cargo install --git https://github.com/coral-xyz/anchor avm --force && avm install 0.31.1 && avm use 0.31.1` |
| Node.js | 18+ | [nodejs.org](https://nodejs.org) |
| Yarn | 1.x | `npm install -g yarn` |
| Surfpool | 1.1+ | [docs.surfpool.dev](https://docs.surfpool.dev) — see install instructions below |

### Installing Surfpool

```bash
# macOS / Linux
curl -sSfL https://install.surfpool.dev | sh

# Verify
surfpool --version
```

## Setup

```bash
# Install JS dependencies
yarn install

# Generate a local keypair (if you don't have one)
solana-keygen new --no-bip39-passphrase

# Build the program
NO_DNA=1 anchor build
```

## Running Tests

Tests use **Surfpool** as the local validator replacement. Surfpool must be started manually before running `anchor test`.

### 1. Clean any stale Surfpool state

```bash
rm -rf .surfpool/
```

Surfpool persists state (including time-travel advances) in a local SQLite database. Removing `.surfpool/` ensures a fresh start.

### 2. Start Surfpool

In a separate terminal:

```bash
surfpool start \
  --legacy-anchor-compatibility \
  --anchor-test-config-path ./Anchor.toml \
  --no-tui \
  --offline
```

Flags:
- `--legacy-anchor-compatibility` — compatibility with Anchor 0.31.x test harness
- `--anchor-test-config-path ./Anchor.toml` — loads `[[test.genesis]]` fixtures (e.g., the Voltr Vault `.so`)
- `--no-tui` — runs without the interactive terminal UI
- `--offline` — disables RPC forwarding; all state is local-only

Wait for `Surfpool is ready` in the output before proceeding.

### 3. Run the tests

```bash
NO_DNA=1 anchor test --skip-local-validator
```

- `NO_DNA=1` — required build flag for this project
- `--skip-local-validator` — tells Anchor not to start `solana-test-validator` (Surfpool is already running)

### Running unit tests only

```bash
cargo test -p voltr-tranching --lib
```

This runs the Rust unit tests (math, state, waterfall logic) without needing Surfpool or any validator.

## Test Suites

| File | Tests | Description |
|------|-------|-------------|
| `tests/voltr-tranching.ts` | ~48 | Full integration: init, deposit, crank, withdraw, coverage ratio, pause enforcement |
| `tests/adversarial.ts` | ~10 | Adversarial scenarios: sandwich attacks, cooldown gaming, edge cases |

### Surfpool Cheatcodes Used

The integration tests use Surfpool-specific RPC methods:

- **`surfnet_setAccount`** — Directly patches on-chain account data (used in TC6 to simulate vault loss for auto-pause testing)
- **`surfnet_timeTravel`** — Advances slots and wall-clock time (used for cooldown expiry tests instead of `setTimeout`)

These methods are no-ops / will fail on `solana-test-validator`. The test suite requires Surfpool.

## Project Structure

```
programs/voltr-tranching/   Anchor program source (Rust)
  src/instructions/         Instruction handlers
  src/math.rs               Fixed-point math (re-exports from voltr-vault)
  src/state.rs              Account structures
  src/error.rs              Error codes
  src/events.rs             Event definitions
  src/constants.rs          PDA seeds and constants
tests/                      Integration tests (TypeScript / Mocha)
  fixtures/                 Voltr Vault program binary + IDL for local testing
```
