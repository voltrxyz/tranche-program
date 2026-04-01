import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { assert } from "chai";
import { VoltrTranching } from "../target/types/voltr_tranching";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  mintTo,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotent,
  createAssociatedTokenAccountIdempotentInstruction,
  getAccount,
  getMint,
} from "@solana/spl-token";
import { Transaction } from "@solana/web3.js";

// ─── Constants ────────────────────────────────────────────────────────────────

const VAULT_PROGRAM_ID = new PublicKey(
  "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8"
);

const VAULT_SEEDS = {
  PROTOCOL: Buffer.from("protocol"),
  VAULT_LP_MINT: Buffer.from("vault_lp_mint"),
  VAULT_LP_MINT_AUTH: Buffer.from("vault_lp_mint_auth"),
  VAULT_ASSET_IDLE_AUTH: Buffer.from("vault_asset_idle_auth"),
};

const TRANCHE_SEEDS = {
  TRANCHE_CONFIG: Buffer.from("tranche_config"),
  TRANCHE_AUTHORITY: Buffer.from("tranche_authority"),
  SENIOR_MINT: Buffer.from("senior_mint"),
  JUNIOR_MINT: Buffer.from("junior_mint"),
  WITHDRAW_RECEIPT: Buffer.from("withdraw_receipt"),
  APR_FEED: Buffer.from("apr_feed"),
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function findVaultPdas(vault: PublicKey) {
  const [protocol] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.PROTOCOL],
    VAULT_PROGRAM_ID
  );
  const [vaultLpMint] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.VAULT_LP_MINT, vault.toBuffer()],
    VAULT_PROGRAM_ID
  );
  const [vaultLpMintAuth] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.VAULT_LP_MINT_AUTH, vault.toBuffer()],
    VAULT_PROGRAM_ID
  );
  const [vaultAssetIdleAuth] = PublicKey.findProgramAddressSync(
    [VAULT_SEEDS.VAULT_ASSET_IDLE_AUTH, vault.toBuffer()],
    VAULT_PROGRAM_ID
  );
  return { protocol, vaultLpMint, vaultLpMintAuth, vaultAssetIdleAuth };
}

function findTranchePdas(vault: PublicKey, trancheProgramId: PublicKey) {
  const [trancheConfig] = PublicKey.findProgramAddressSync(
    [TRANCHE_SEEDS.TRANCHE_CONFIG, vault.toBuffer()],
    trancheProgramId
  );
  const [trancheAuthority] = PublicKey.findProgramAddressSync(
    [TRANCHE_SEEDS.TRANCHE_AUTHORITY, trancheConfig.toBuffer()],
    trancheProgramId
  );
  const [seniorMint] = PublicKey.findProgramAddressSync(
    [TRANCHE_SEEDS.SENIOR_MINT, trancheConfig.toBuffer()],
    trancheProgramId
  );
  const [juniorMint] = PublicKey.findProgramAddressSync(
    [TRANCHE_SEEDS.JUNIOR_MINT, trancheConfig.toBuffer()],
    trancheProgramId
  );
  const [aprFeed] = PublicKey.findProgramAddressSync(
    [TRANCHE_SEEDS.APR_FEED, trancheConfig.toBuffer()],
    trancheProgramId
  );
  return { trancheConfig, trancheAuthority, seniorMint, juniorMint, aprFeed };
}

function findWithdrawReceiptPda(
  trancheConfig: PublicKey,
  receiptId: BN,
  trancheProgramId: PublicKey
) {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(BigInt(receiptId.toString()));
  const [receipt] = PublicKey.findProgramAddressSync(
    [TRANCHE_SEEDS.WITHDRAW_RECEIPT, trancheConfig.toBuffer(), buf],
    trancheProgramId
  );
  return receipt;
}

/** Encode a u32 integer as Decimal bits (U80F48 fixed-point) in LE [u8; 16]. */
function intToDecimalBits(n: number): number[] {
  const buf = Buffer.alloc(16, 0);
  const val = BigInt(n) * BigInt("281474976710656"); // n << 48
  buf.writeBigUInt64LE(val & BigInt("0xFFFFFFFFFFFFFFFF"), 0);
  buf.writeBigUInt64LE(val >> BigInt(64), 8);
  return Array.from(buf);
}

/** Read DecimalBits (LE [u8; 16] of U80F48) as approximate number. */
function readDecimalBits(bits: number[]): number {
  const buf = Buffer.from(bits);
  const lo = buf.readBigUInt64LE(0);
  const hi = buf.readBigUInt64LE(8);
  const raw = (hi << BigInt(64)) | lo;
  return Number(raw) / 2 ** 48;
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("voltr-tranching integration", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const trancheProgram = anchor.workspace
    .VoltrTranching as Program<VoltrTranching>;
  const vaultProgram = new anchor.Program(
    require("./fixtures/voltr_vault.json"),
    provider
  );

  // Test keypairs
  const payerKp = Keypair.generate();
  const payer = payerKp.publicKey;
  const protocolAdminKp = Keypair.generate();
  const vaultKp = Keypair.generate();
  const vault = vaultKp.publicKey;
  const authority = provider.wallet.publicKey;
  const userKp = Keypair.generate();
  const user = userKp.publicKey;

  // Derived addresses (populated in setup)
  let assetMint: PublicKey;
  let vaultPdas: ReturnType<typeof findVaultPdas>;
  let vaultAssetIdleAta: PublicKey;
  let tranchePdas: ReturnType<typeof findTranchePdas>;
  let vaultLpCustody: PublicKey;
  let assetEscrow: PublicKey;
  let seniorTokenEscrow: PublicKey;
  let juniorTokenEscrow: PublicKey;
  let userAssetAta: PublicKey;
  let userSeniorAta: PublicKey;
  let userJuniorAta: PublicKey;

  // ─── Setup ──────────────────────────────────────────────────────────────

  before(async () => {
    // Airdrop SOL
    for (const pk of [payer, user]) {
      const sig = await provider.connection.requestAirdrop(
        pk,
        100 * LAMPORTS_PER_SOL
      );
      const bh = await provider.connection.getLatestBlockhash();
      await provider.connection.confirmTransaction({
        blockhash: bh.blockhash,
        lastValidBlockHeight: bh.lastValidBlockHeight,
        signature: sig,
      });
    }

    // Create asset mint (6 decimals like USDC)
    assetMint = await createMint(
      provider.connection,
      payerKp,
      payer,
      null,
      6
    );

    // Derive vault PDAs
    vaultPdas = findVaultPdas(vault);
    vaultAssetIdleAta = getAssociatedTokenAddressSync(
      assetMint,
      vaultPdas.vaultAssetIdleAuth,
      true // allowOwnerOffCurve for PDA
    );

    // Fund user with assets
    userAssetAta = await createAssociatedTokenAccountIdempotent(
      provider.connection,
      payerKp,
      assetMint,
      user
    );
    await mintTo(
      provider.connection,
      payerKp,
      assetMint,
      userAssetAta,
      payerKp,
      50_000_000_000 // 50,000 USDC
    );

    // Fund authority with assets
    await createAssociatedTokenAccountIdempotent(
      provider.connection,
      payerKp,
      assetMint,
      authority
    );
  });

  // ─── Account builders ─────────────────────────────────────────────────────

  function depositSeniorAccounts() {
    return {
      user,
      trancheConfig: tranchePdas.trancheConfig,
      trancheAuthority: tranchePdas.trancheAuthority,
      seniorMint: tranchePdas.seniorMint,
      juniorMint: tranchePdas.juniorMint,
      userAssetAta,
      userSeniorAta,
      assetEscrow,
      vaultLpCustody,
      assetMint,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
      protocol: vaultPdas.protocol,
      vaultAssetIdleAta,
      vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
      vaultLpMintAuth: vaultPdas.vaultLpMintAuth,
      voltrVaultProgram: VAULT_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };
  }

  function depositJuniorAccounts() {
    return {
      user,
      trancheConfig: tranchePdas.trancheConfig,
      trancheAuthority: tranchePdas.trancheAuthority,
      seniorMint: tranchePdas.seniorMint,
      juniorMint: tranchePdas.juniorMint,
      userAssetAta,
      userJuniorAta,
      assetEscrow,
      vaultLpCustody,
      assetMint,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
      protocol: vaultPdas.protocol,
      vaultAssetIdleAta,
      vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
      vaultLpMintAuth: vaultPdas.vaultLpMintAuth,
      voltrVaultProgram: VAULT_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };
  }

  function crankAccounts() {
    return {
      payer: user,
      trancheConfig: tranchePdas.trancheConfig,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
      vaultLpCustody,
      seniorMint: tranchePdas.seniorMint,
      juniorMint: tranchePdas.juniorMint,
    };
  }

  function requestWithdrawSenior(receiptId: BN) {
    const receipt = findWithdrawReceiptPda(
      tranchePdas.trancheConfig,
      receiptId,
      trancheProgram.programId
    );
    return {
      receipt,
      accounts: {
        user,
        trancheConfig: tranchePdas.trancheConfig,
        trancheAuthority: tranchePdas.trancheAuthority,
        seniorMint: tranchePdas.seniorMint,
        juniorMint: tranchePdas.juniorMint,
        userSeniorAta,
        seniorTokenEscrow,
        vaultLpCustody,
        vault,
        vaultLpMint: vaultPdas.vaultLpMint,
        withdrawReceipt: receipt,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      },
    };
  }

  function requestWithdrawJunior(receiptId: BN) {
    const receipt = findWithdrawReceiptPda(
      tranchePdas.trancheConfig,
      receiptId,
      trancheProgram.programId
    );
    return {
      receipt,
      accounts: {
        user,
        trancheConfig: tranchePdas.trancheConfig,
        trancheAuthority: tranchePdas.trancheAuthority,
        seniorMint: tranchePdas.seniorMint,
        juniorMint: tranchePdas.juniorMint,
        userJuniorAta,
        juniorTokenEscrow,
        vaultLpCustody,
        vault,
        vaultLpMint: vaultPdas.vaultLpMint,
        withdrawReceipt: receipt,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      },
    };
  }

  function completeWithdrawAccounts(receipt: PublicKey, isSenior: boolean) {
    return {
      user,
      trancheConfig: tranchePdas.trancheConfig,
      trancheAuthority: tranchePdas.trancheAuthority,
      seniorMint: tranchePdas.seniorMint,
      juniorMint: tranchePdas.juniorMint,
      trancheTokenEscrow: isSenior ? seniorTokenEscrow : juniorTokenEscrow,
      assetEscrow,
      userAssetAta,
      vaultLpCustody,
      assetMint,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
      protocol: vaultPdas.protocol,
      vaultAssetIdleAta,
      vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
      withdrawReceipt: receipt,
      voltrVaultProgram: VAULT_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };
  }

  function cancelWithdrawAccounts(receipt: PublicKey, isSenior: boolean) {
    return {
      user,
      trancheConfig: tranchePdas.trancheConfig,
      trancheAuthority: tranchePdas.trancheAuthority,
      withdrawReceipt: receipt,
      trancheMint: isSenior ? tranchePdas.seniorMint : tranchePdas.juniorMint,
      trancheTokenEscrow: isSenior ? seniorTokenEscrow : juniorTokenEscrow,
      userTrancheAta: isSenior ? userSeniorAta : userJuniorAta,
      tokenProgram: TOKEN_PROGRAM_ID,
    };
  }

  function instantWithdrawSeniorAccounts() {
    return {
      user,
      trancheConfig: tranchePdas.trancheConfig,
      trancheAuthority: tranchePdas.trancheAuthority,
      seniorMint: tranchePdas.seniorMint,
      juniorMint: tranchePdas.juniorMint,
      userSeniorAta,
      userAssetAta,
      assetEscrow,
      vaultLpCustody,
      assetMint,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
      protocol: vaultPdas.protocol,
      vaultAssetIdleAta,
      vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
      voltrVaultProgram: VAULT_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };
  }

  function instantWithdrawJuniorAccounts() {
    return {
      user,
      trancheConfig: tranchePdas.trancheConfig,
      trancheAuthority: tranchePdas.trancheAuthority,
      seniorMint: tranchePdas.seniorMint,
      juniorMint: tranchePdas.juniorMint,
      userJuniorAta,
      userAssetAta,
      assetEscrow,
      vaultLpCustody,
      assetMint,
      vault,
      vaultLpMint: vaultPdas.vaultLpMint,
      protocol: vaultPdas.protocol,
      vaultAssetIdleAta,
      vaultAssetIdleAuth: vaultPdas.vaultAssetIdleAuth,
      voltrVaultProgram: VAULT_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };
  }

  async function bal(ata: PublicKey): Promise<bigint> {
    return (await getAccount(provider.connection, ata)).amount;
  }

  async function getConfig() {
    return trancheProgram.account.trancheConfig.fetch(
      tranchePdas.trancheConfig
    );
  }

  // ─── Vault Setup ─────────────────────────────────────────────────────────

  it("initializes Voltr protocol", async () => {
    try {
      await vaultProgram.methods
        .initProtocol(8191) // all operations enabled
        .accounts({ payer, admin: protocolAdminKp.publicKey })
        .signers([payerKp, protocolAdminKp])
        .rpc();
    } catch {
      // Protocol may already be initialized by another test suite — OK
    }

    const proto = await vaultProgram.account.protocol.fetch(vaultPdas.protocol);
    assert.equal(proto.operationalState, 8191);
  });

  it("initializes Voltr vault (withdrawal_waiting_period=0)", async () => {
    await vaultProgram.methods
      .initializeVault(
        {
          maxCap: new BN(1_000_000_000_000),
          startAtTs: new BN(0),
          lockedProfitDegradationDuration: new BN(0),
          managerPerformanceFee: 0,
          adminPerformanceFee: 0,
          managerManagementFee: 0,
          adminManagementFee: 0,
          redemptionFee: 0,
          issuanceFee: 0,
          withdrawalWaitingPeriod: new BN(0),
        },
        "Test Vault",
        "Integration test vault"
      )
      .accounts({
        payer,
        manager: authority,
        admin: authority,
        vault,
        vaultAssetMint: assetMint,
        vaultAssetIdleAta,
        assetTokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([payerKp, vaultKp])
      .rpc();

    const v = await vaultProgram.account.vault.fetch(vault);
    assert.ok(v.asset.mint.equals(assetMint));
    assert.equal(v.vaultConfiguration.withdrawalWaitingPeriod.toNumber(), 0);
  });

  // ─── Tranche Init ────────────────────────────────────────────────────────

  it("initializes tranche config", async () => {
    tranchePdas = findTranchePdas(vault, trancheProgram.programId);

    vaultLpCustody = getAssociatedTokenAddressSync(
      vaultPdas.vaultLpMint,
      tranchePdas.trancheAuthority,
      true
    );
    assetEscrow = getAssociatedTokenAddressSync(
      assetMint,
      tranchePdas.trancheAuthority,
      true
    );

    await trancheProgram.methods
      .initializeTrancheConfig({
        aprFloorBps: 200,
        riskXBps: 500,
        riskYBps: 1000,
        riskK: intToDecimalBits(2),
        reserveFeeBps: 500, // 5%
        seniorExitFeeBps: 50, // 0.5%
        juniorExitFeeBps: 50, // 0.5%
        minJrSrRatioBps: 1000, // 10%
        minCoverageForMintBps: 10000, // 100%
        minCoverageBufferBps: 11000, // 110%
        seniorCooldownSeconds: new BN(0), // instant by default
        juniorCooldownSeconds: new BN(0),
        maxObservationAgeSeconds: new BN(259200),
      })
      .accounts({
        authority,
        vault,
        trancheConfig: tranchePdas.trancheConfig,
        trancheAuthority: tranchePdas.trancheAuthority,
        seniorMint: tranchePdas.seniorMint,
        juniorMint: tranchePdas.juniorMint,
        assetMint,
        vaultLpMint: vaultPdas.vaultLpMint,
        vaultLpCustody,
        assetEscrow,
        aprFeed: tranchePdas.aprFeed,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const config = await getConfig();
    assert.ok(config.vault.equals(vault));
    assert.ok(config.authority.equals(authority));
    assert.equal(config.paused, 0);
    assert.equal(config.nextReceiptId.toNumber(), 0);
  });

  it("creates user ATAs and tranche token escrows", async () => {
    // User tranche token ATAs
    userSeniorAta = await createAssociatedTokenAccountIdempotent(
      provider.connection,
      payerKp,
      tranchePdas.seniorMint,
      user
    );
    userJuniorAta = await createAssociatedTokenAccountIdempotent(
      provider.connection,
      payerKp,
      tranchePdas.juniorMint,
      user
    );

    // Token escrows for withdrawal operations (owned by tranche_authority PDA)
    seniorTokenEscrow = getAssociatedTokenAddressSync(
      tranchePdas.seniorMint,
      tranchePdas.trancheAuthority,
      true // allowOwnerOffCurve for PDA
    );
    juniorTokenEscrow = getAssociatedTokenAddressSync(
      tranchePdas.juniorMint,
      tranchePdas.trancheAuthority,
      true
    );

    // Create escrow ATAs via instruction (PDA owner requires explicit instruction)
    const tx = new Transaction();
    tx.add(
      createAssociatedTokenAccountIdempotentInstruction(
        payer,
        seniorTokenEscrow,
        tranchePdas.trancheAuthority,
        tranchePdas.seniorMint
      )
    );
    tx.add(
      createAssociatedTokenAccountIdempotentInstruction(
        payer,
        juniorTokenEscrow,
        tranchePdas.trancheAuthority,
        tranchePdas.juniorMint
      )
    );
    const sig = await provider.connection.sendTransaction(tx, [payerKp]);
    const bh = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: bh.blockhash,
      lastValidBlockHeight: bh.lastValidBlockHeight,
      signature: sig,
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Flow 1: Happy path senior
  // ═══════════════════════════════════════════════════════════════════════════

  describe("Flow 1: Happy path senior", () => {
    it("deposits junior first for coverage", async () => {
      await trancheProgram.methods
        .depositJunior(new BN(2_000_000_000)) // 2000 USDC
        .accounts(depositJuniorAccounts())
        .signers([userKp])
        .rpc();

      const jBal = await bal(userJuniorAta);
      assert.isTrue(jBal > BigInt(0), "Junior tokens minted");
    });

    it("deposits senior tokens", async () => {
      const assetBefore = await bal(userAssetAta);

      await trancheProgram.methods
        .depositSenior(new BN(500_000_000)) // 500 USDC (coverage = 2500/500 = 5.0 > 1.1)
        .accounts(depositSeniorAccounts())
        .signers([userKp])
        .rpc();

      const assetAfter = await bal(userAssetAta);
      assert.isTrue(assetBefore > assetAfter, "Assets decreased");

      const sBal = await bal(userSeniorAta);
      assert.isTrue(sBal > BigInt(0), "Senior tokens minted");
    });

    it("crank_accounting runs without errors", async () => {
      await trancheProgram.methods
        .crankAccounting()
        .accounts(crankAccounts())
        .signers([userKp])
        .rpc();

      const config = await getConfig();
      assert.isTrue(
        readDecimalBits(config.totalNav as any) > 0,
        "Total NAV non-zero"
      );
    });

    it("request_withdraw_senior escrows tokens", async () => {
      const config = await getConfig();
      const { receipt, accounts } = requestWithdrawSenior(config.nextReceiptId);

      const sBal = await bal(userSeniorAta);
      const withdrawTokens = new BN((sBal / BigInt(2)).toString());

      await trancheProgram.methods
        .requestWithdrawSenior(withdrawTokens, true)
        .accounts(accounts)
        .signers([userKp])
        .rpc();

      const rcpt = await trancheProgram.account.withdrawReceipt.fetch(receipt);
      assert.ok(rcpt.user.equals(user));
      assert.isTrue(rcpt.isSenior);
      assert.equal(rcpt.trancheTokensEscrowed.toString(), withdrawTokens.toString());
    });

    it("complete_withdraw returns assets (cooldown=0)", async () => {
      const config = await getConfig();
      const receiptId = new BN(config.nextReceiptId.toNumber() - 1);
      const receipt = findWithdrawReceiptPda(
        tranchePdas.trancheConfig,
        receiptId,
        trancheProgram.programId
      );

      const assetBefore = await bal(userAssetAta);

      await trancheProgram.methods
        .completeWithdraw()
        .accounts(completeWithdrawAccounts(receipt, true))
        .signers([userKp])
        .rpc();

      const assetAfter = await bal(userAssetAta);
      assert.isTrue(assetAfter > assetBefore, "User received assets");

      // Receipt should be closed
      const info = await provider.connection.getAccountInfo(receipt);
      assert.isNull(info, "Receipt closed");
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Flow 2: Happy path junior
  // ═══════════════════════════════════════════════════════════════════════════

  describe("Flow 2: Happy path junior", () => {
    it("deposits junior tokens", async () => {
      const jBefore = await bal(userJuniorAta);

      await trancheProgram.methods
        .depositJunior(new BN(500_000_000))
        .accounts(depositJuniorAccounts())
        .signers([userKp])
        .rpc();

      const jAfter = await bal(userJuniorAta);
      assert.isTrue(jAfter > jBefore, "Junior balance increased");
    });

    it("request_withdraw_junior and complete", async () => {
      const config = await getConfig();
      const { receipt, accounts } = requestWithdrawJunior(config.nextReceiptId);

      // Withdraw small amount to avoid ratio breach
      await trancheProgram.methods
        .requestWithdrawJunior(new BN(50_000_000), true)
        .accounts(accounts)
        .signers([userKp])
        .rpc();

      const assetBefore = await bal(userAssetAta);

      await trancheProgram.methods
        .completeWithdraw()
        .accounts(completeWithdrawAccounts(receipt, false))
        .signers([userKp])
        .rpc();

      const assetAfter = await bal(userAssetAta);
      assert.isTrue(assetAfter > assetBefore, "Assets received from junior withdrawal");

      const info = await provider.connection.getAccountInfo(receipt);
      assert.isNull(info, "Receipt closed");
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Flow 3: Mixed deposits, waterfall verification
  // ═══════════════════════════════════════════════════════════════════════════

  describe("Flow 3: Mixed deposits and waterfall", () => {
    it("senior + junior deposits, NAV tracks correctly", async () => {
      const configBefore = await getConfig();
      const navBefore = readDecimalBits(configBefore.totalNav as any);

      // Junior deposit
      await trancheProgram.methods
        .depositJunior(new BN(1_000_000_000))
        .accounts(depositJuniorAccounts())
        .signers([userKp])
        .rpc();

      // Senior deposit
      await trancheProgram.methods
        .depositSenior(new BN(200_000_000))
        .accounts(depositSeniorAccounts())
        .signers([userKp])
        .rpc();

      const configAfter = await getConfig();
      const navAfter = readDecimalBits(configAfter.totalNav as any);
      assert.isTrue(navAfter > navBefore, "Total NAV increased");
    });

    it("crank accounting and NAV invariant", async () => {
      await trancheProgram.methods
        .crankAccounting()
        .accounts(crankAccounts())
        .signers([userKp])
        .rpc();

      const config = await getConfig();
      const total = readDecimalBits(config.totalNav as any);
      const senior = readDecimalBits(config.seniorNav as any);
      const junior = readDecimalBits(config.juniorNav as any);
      const reserve = readDecimalBits(config.reserveNav as any);
      const sum = senior + junior + reserve;
      const diff = Math.abs(total - sum);

      assert.isTrue(
        diff < 1000,
        `NAV invariant: total=${total.toFixed(2)}, sum=${sum.toFixed(2)}, diff=${diff.toFixed(6)}`
      );
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Flow 4: Instant withdraw (cooldown=0)
  // ═══════════════════════════════════════════════════════════════════════════

  describe("Flow 4: Instant withdraw", () => {
    it("instant_withdraw_senior", async () => {
      const sBal = await bal(userSeniorAta);
      const assetBefore = await bal(userAssetAta);
      // Use token amount (is_token_amount=true) — withdraw 10% of senior tokens
      const withdrawTokens = new BN((sBal / BigInt(10)).toString());

      await trancheProgram.methods
        .instantWithdrawSenior(withdrawTokens, true)
        .accounts(instantWithdrawSeniorAccounts())
        .signers([userKp])
        .rpc();

      const sBalAfter = await bal(userSeniorAta);
      const assetAfter = await bal(userAssetAta);
      assert.isTrue(sBalAfter < sBal, "Senior tokens burned");
      assert.isTrue(assetAfter > assetBefore, "Assets received");
    });

    it("instant_withdraw_junior", async () => {
      const jBal = await bal(userJuniorAta);
      const assetBefore = await bal(userAssetAta);
      // Withdraw 2% of junior tokens (small to avoid ratio breach)
      const withdrawTokens = new BN((jBal / BigInt(50)).toString());

      await trancheProgram.methods
        .instantWithdrawJunior(withdrawTokens, true)
        .accounts(instantWithdrawJuniorAccounts())
        .signers([userKp])
        .rpc();

      const jBalAfter = await bal(userJuniorAta);
      const assetAfter = await bal(userAssetAta);
      assert.isTrue(jBalAfter < jBal, "Junior tokens burned");
      assert.isTrue(assetAfter > assetBefore, "Assets received");
    });

    it("instant_withdraw fails when cooldown != 0", async () => {
      // Set cooldown to 60 — Anchor 0.31 enum variants with unnamed fields need array format
      await trancheProgram.methods
        .updateTrancheConfig({ seniorCooldownSeconds: [new BN(60)] })
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      const configAfterUpdate = await getConfig();
      assert.equal(
        configAfterUpdate.seniorCooldownSeconds.toString(),
        "60",
        "Cooldown should be updated to 60"
      );

      const sBal = await bal(userSeniorAta);
      assert.isTrue(sBal > BigInt(0), "User should have senior tokens");
      const withdrawTokens = new BN((sBal / BigInt(20)).toString());

      let thrown = false;
      try {
        await trancheProgram.methods
          .instantWithdrawSenior(withdrawTokens, true)
          .accounts(instantWithdrawSeniorAccounts())
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "CooldownMustBeZero");
      }
      assert.isTrue(thrown, "instant_withdraw_senior should have failed with CooldownMustBeZero");

      // Reset
      await trancheProgram.methods
        .updateTrancheConfig({ seniorCooldownSeconds: [new BN(0)] })
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Flow 5: Cancel withdraw (fee not reversed)
  // ═══════════════════════════════════════════════════════════════════════════

  describe("Flow 5: Cancel withdraw", () => {
    it("request → cancel: tokens returned, fee NOT refunded", async () => {
      // Set cooldown > 0 so we can cancel (otherwise would just complete)
      await trancheProgram.methods
        .updateTrancheConfig({ seniorCooldownSeconds: [new BN(3600)] })
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      const config = await getConfig();
      const reserveBefore = readDecimalBits(config.reserveNav as any);
      const { receipt, accounts } = requestWithdrawSenior(config.nextReceiptId);

      const sBefore = await bal(userSeniorAta);

      // Request withdrawal with a meaningful amount
      await trancheProgram.methods
        .requestWithdrawSenior(new BN(100_000_000), true)
        .accounts(accounts)
        .signers([userKp])
        .rpc();

      const configAfterReq = await getConfig();
      const reserveAfterReq = readDecimalBits(configAfterReq.reserveNav as any);

      // Exit fee collected
      assert.isTrue(
        reserveAfterReq >= reserveBefore,
        "Reserve should be >= before (fee applied)"
      );

      // Cancel
      await trancheProgram.methods
        .cancelWithdraw()
        .accounts(cancelWithdrawAccounts(receipt, true))
        .signers([userKp])
        .rpc();

      const sAfter = await bal(userSeniorAta);
      assert.equal(sAfter.toString(), sBefore.toString(), "All tokens returned");

      const configAfterCancel = await getConfig();
      const reserveAfterCancel = readDecimalBits(configAfterCancel.reserveNav as any);
      assert.isTrue(
        reserveAfterCancel >= reserveAfterReq,
        "Fee NOT refunded on cancel"
      );

      // Receipt closed
      const info = await provider.connection.getAccountInfo(receipt);
      assert.isNull(info, "Receipt closed");

      // Reset cooldown
      await trancheProgram.methods
        .updateTrancheConfig({ seniorCooldownSeconds: [new BN(0)] })
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Flow 6: Multiple receipts
  // ═══════════════════════════════════════════════════════════════════════════

  describe("Flow 6: Multiple pending withdrawals", () => {
    it("create two receipts, complete out of order", async () => {
      // Receipt 1
      let config = await getConfig();
      const { receipt: r1, accounts: a1 } = requestWithdrawSenior(config.nextReceiptId);
      await trancheProgram.methods
        .requestWithdrawSenior(new BN(20_000_000), true)
        .accounts(a1)
        .signers([userKp])
        .rpc();

      // Receipt 2
      config = await getConfig();
      const { receipt: r2, accounts: a2 } = requestWithdrawSenior(config.nextReceiptId);
      await trancheProgram.methods
        .requestWithdrawSenior(new BN(20_000_000), true)
        .accounts(a2)
        .signers([userKp])
        .rpc();

      // Verify both exist
      const rd1 = await trancheProgram.account.withdrawReceipt.fetch(r1);
      const rd2 = await trancheProgram.account.withdrawReceipt.fetch(r2);
      assert.ok(rd1.user.equals(user));
      assert.ok(rd2.user.equals(user));

      // Complete #2 first
      await trancheProgram.methods
        .completeWithdraw()
        .accounts(completeWithdrawAccounts(r2, true))
        .signers([userKp])
        .rpc();
      assert.isNull(
        await provider.connection.getAccountInfo(r2),
        "Receipt 2 closed"
      );
      assert.isNotNull(
        await provider.connection.getAccountInfo(r1),
        "Receipt 1 still open"
      );

      // Complete #1
      await trancheProgram.methods
        .completeWithdraw()
        .accounts(completeWithdrawAccounts(r1, true))
        .signers([userKp])
        .rpc();
      assert.isNull(
        await provider.connection.getAccountInfo(r1),
        "Receipt 1 closed"
      );
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Flow 7: Reserve accumulation
  // ═══════════════════════════════════════════════════════════════════════════

  describe("Flow 7: Reserve accumulation", () => {
    it("exit fees accumulate in reserve", async () => {
      // Deposit large amount for meaningful fees
      await trancheProgram.methods
        .depositJunior(new BN(2_000_000_000))
        .accounts(depositJuniorAccounts())
        .signers([userKp])
        .rpc();
      await trancheProgram.methods
        .depositSenior(new BN(500_000_000))
        .accounts(depositSeniorAccounts())
        .signers([userKp])
        .rpc();

      const configBefore = await getConfig();
      const reserveBefore = readDecimalBits(configBefore.reserveNav as any);

      // Withdraw senior tokens to generate exit fee
      const sBalance = await bal(userSeniorAta);
      assert.isTrue(sBalance > BigInt(0), "Must have senior tokens");
      const withdrawTokens = new BN((sBalance / BigInt(5)).toString());
      await trancheProgram.methods
        .instantWithdrawSenior(withdrawTokens, true)
        .accounts(instantWithdrawSeniorAccounts())
        .signers([userKp])
        .rpc();

      const configAfter = await getConfig();
      const reserveAfter = readDecimalBits(configAfter.reserveNav as any);

      assert.isTrue(
        reserveAfter >= reserveBefore,
        `Reserve increased from exit fee: ${reserveBefore} -> ${reserveAfter}`
      );
    });

    it("final NAV invariant holds", async () => {
      await trancheProgram.methods
        .crankAccounting()
        .accounts(crankAccounts())
        .signers([userKp])
        .rpc();

      const config = await getConfig();
      const total = readDecimalBits(config.totalNav as any);
      const senior = readDecimalBits(config.seniorNav as any);
      const junior = readDecimalBits(config.juniorNav as any);
      const reserve = readDecimalBits(config.reserveNav as any);
      const sum = senior + junior + reserve;
      const diff = Math.abs(total - sum);

      assert.isTrue(
        diff < 10000,
        `NAV invariant: total=${total.toFixed(2)}, sr=${senior.toFixed(2)}, jr=${junior.toFixed(2)}, rsv=${reserve.toFixed(2)}, diff=${diff.toFixed(6)}`
      );

      // LP custody still holds tokens
      const lp = await bal(vaultLpCustody);
      assert.isTrue(lp > BigInt(0), "LP custody non-empty");

      // Tranche supplies positive
      const sm = await getMint(provider.connection, tranchePdas.seniorMint);
      const jm = await getMint(provider.connection, tranchePdas.juniorMint);
      assert.isTrue(sm.supply > BigInt(0), "Senior supply > 0");
      assert.isTrue(jm.supply > BigInt(0), "Junior supply > 0");
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // APR Observation
  // ═══════════════════════════════════════════════════════════════════════════

  describe("APR observation crank", () => {
    it("pushes an observation", async () => {
      await trancheProgram.methods
        .pushAprObservation()
        .accounts({
          trancheConfig: tranchePdas.trancheConfig,
          aprFeed: tranchePdas.aprFeed,
          vault,
          vaultLpMint: vaultPdas.vaultLpMint,
        })
        .rpc();

      const feed = await trancheProgram.account.aprFeed.fetch(tranchePdas.aprFeed);
      assert.isTrue(feed.count > 0, "Observation count > 0");
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // VOL-27: Coverage ratio and pause enforcement
  // ═══════════════════════════════════════════════════════════════════════════

  describe("VOL-27: Coverage ratio enforcement", () => {
    it("TC1: senior deposit blocked when coverage < min_coverage_buffer", async () => {
      // The current state has a high junior/senior ratio (lots of coverage headroom).
      // To trigger InsufficientCoverage, set min_coverage_buffer_bps to max u16
      // (65535 = 655.35%) and deposit a large senior amount. The post-CPI check:
      //   (total_nav + deposit) * 10000 >= (senior_nav + deposit) * 65535
      // will fail because the large deposit shifts the ratio toward 1:1.
      await trancheProgram.methods
        .updateTrancheConfig({ minCoverageBufferBps: [65535] }) // 655.35% buffer
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      let thrown = false;
      try {
        await trancheProgram.methods
          .depositSenior(new BN(5_000_000_000)) // 5000 USDC — large enough to breach
          .accounts(depositSeniorAccounts())
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "InsufficientCoverage");
      }
      assert.isTrue(thrown, "Senior deposit should fail with InsufficientCoverage");

      // Restore
      await trancheProgram.methods
        .updateTrancheConfig({ minCoverageBufferBps: [11000] }) // 110%
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();
    });

    it("TC2: senior deposit succeeds when coverage above buffer", async () => {
      // Ensure plenty of junior coverage exists
      await trancheProgram.methods
        .depositJunior(new BN(1_000_000_000)) // 1000 USDC junior
        .accounts(depositJuniorAccounts())
        .signers([userKp])
        .rpc();

      // Now senior deposit should succeed with good coverage
      const sBefore = await bal(userSeniorAta);
      await trancheProgram.methods
        .depositSenior(new BN(100_000_000)) // 100 USDC
        .accounts(depositSeniorAccounts())
        .signers([userKp])
        .rpc();

      const sAfter = await bal(userSeniorAta);
      assert.isTrue(sAfter > sBefore, "Senior tokens minted when coverage sufficient");
    });

    it("TC3: junior withdrawal blocked when ratio would breach min_jr_sr_ratio", async () => {
      // The current state has a very high junior/senior ratio. Set min ratio
      // to max u16 (65535 = 655.35%) so that withdrawing ~33% of junior breaches it.
      // Check: post_withdraw_junior >= senior * 65535 / 10000 = senior * 6.5535
      // With a large withdrawal, the remaining junior won't satisfy this.
      await trancheProgram.methods
        .updateTrancheConfig({ minJrSrRatioBps: [65535] }) // max u16
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      const config = await getConfig();
      const { receipt, accounts } = requestWithdrawJunior(config.nextReceiptId);

      // Try to withdraw a meaningful amount of junior tokens
      const jBal = await bal(userJuniorAta);
      const withdrawTokens = new BN((jBal / BigInt(3)).toString()); // 33% of junior

      let thrown = false;
      try {
        await trancheProgram.methods
          .requestWithdrawJunior(withdrawTokens, true)
          .accounts(accounts)
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "JuniorRatioBreach");
      }
      assert.isTrue(thrown, "Junior withdrawal should fail with JuniorRatioBreach");

      // Restore
      await trancheProgram.methods
        .updateTrancheConfig({ minJrSrRatioBps: [1000] }) // 10%
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();
    });

    it("TC4: junior withdrawal succeeds when ratio stays above minimum", async () => {
      const config = await getConfig();
      const { receipt, accounts } = requestWithdrawJunior(config.nextReceiptId);

      // Withdraw small amount — should stay above 10% ratio
      const withdrawTokens = new BN(10_000_000); // 10 USDC worth

      await trancheProgram.methods
        .requestWithdrawJunior(withdrawTokens, true)
        .accounts(accounts)
        .signers([userKp])
        .rpc();

      const rcpt = await trancheProgram.account.withdrawReceipt.fetch(receipt);
      assert.ok(rcpt.user.equals(user));
      assert.isFalse(rcpt.isSenior, "Should be junior receipt");

      // Complete to clean up
      await trancheProgram.methods
        .completeWithdraw()
        .accounts(completeWithdrawAccounts(receipt, false))
        .signers([userKp])
        .rpc();
    });
  });

  describe("VOL-27: Manual pause enforcement", () => {
    it("TC5: authority sets pause flags, each operation respects its flag", async () => {
      // Test 0x01 — senior deposits blocked
      await trancheProgram.methods
        .setPaused(0x01)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      let thrown = false;
      try {
        await trancheProgram.methods
          .depositSenior(new BN(100_000_000))
          .accounts(depositSeniorAccounts())
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "SeniorDepositsPaused");
      }
      assert.isTrue(thrown, "Senior deposit blocked by 0x01");

      // Test 0x02 — junior deposits blocked
      await trancheProgram.methods
        .setPaused(0x02)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      thrown = false;
      try {
        await trancheProgram.methods
          .depositJunior(new BN(100_000_000))
          .accounts(depositJuniorAccounts())
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "JuniorDepositsPaused");
      }
      assert.isTrue(thrown, "Junior deposit blocked by 0x02");

      // Test 0x04 — senior withdrawals blocked
      await trancheProgram.methods
        .setPaused(0x04)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      thrown = false;
      try {
        const config = await getConfig();
        const { accounts } = requestWithdrawSenior(config.nextReceiptId);
        await trancheProgram.methods
          .requestWithdrawSenior(new BN(10_000_000), true)
          .accounts(accounts)
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "SeniorWithdrawalsPaused");
      }
      assert.isTrue(thrown, "Senior withdrawal blocked by 0x04");

      // Test 0x08 — junior withdrawals blocked
      await trancheProgram.methods
        .setPaused(0x08)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      thrown = false;
      try {
        const config = await getConfig();
        const { accounts } = requestWithdrawJunior(config.nextReceiptId);
        await trancheProgram.methods
          .requestWithdrawJunior(new BN(10_000_000), true)
          .accounts(accounts)
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "JuniorWithdrawalsPaused");
      }
      assert.isTrue(thrown, "Junior withdrawal blocked by 0x08");

      // Test combined flags: 0x0F blocks everything
      await trancheProgram.methods
        .setPaused(0x0F)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      const config = await getConfig();
      assert.equal(config.paused, 0x0F, "All pause bits set");

      // Unpause all
      await trancheProgram.methods
        .setPaused(0x00)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      const configAfter = await getConfig();
      assert.equal(configAfter.paused, 0, "All pause bits cleared");
    });
  });

  describe("VOL-27: Deposit pause enforcement and authority unpause", () => {
    // NOTE: The auto-pause *mechanism* (crank_accounting sets bits 0x01|0x02
    // when junior_nav==0 && junior_supply>0) is tested in Rust unit tests:
    //   crank_accounting::tests::run_inline_crank_auto_pauses_on_junior_wipeout
    //   crank_accounting::tests::run_inline_crank_auto_pause_exact_junior_wipeout
    //   crank_accounting::tests::run_inline_crank_auto_pause_ors_into_existing_flags
    //   crank_accounting::tests::run_inline_crank_no_auto_pause_when_junior_supply_zero
    //   crank_accounting::tests::run_inline_crank_no_auto_pause_on_partial_junior_loss
    //
    // Simulating a vault loss in integration tests requires direct account data
    // manipulation (e.g. bankrun/LiteSVM setAccount) which the standard Anchor
    // test-validator does not support. These integration tests instead verify
    // that pause bits 0x01|0x02 (the same bits auto-pause sets) correctly block
    // deposits and that the authority can clear them.

    it("TC6: deposit pause bits (0x01|0x02) block both deposit types, withdrawals still work", async () => {
      // Verify current state is unpaused
      let config = await getConfig();
      assert.equal(config.paused, 0, "Should start unpaused");

      // Set the same bits that auto-pause would set: 0x01 | 0x02
      await trancheProgram.methods
        .setPaused(0x03)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      config = await getConfig();
      assert.equal(config.paused, 0x03, "Pause bits 0x01|0x02 should be set");

      // Verify senior deposits blocked
      let seniorBlocked = false;
      try {
        await trancheProgram.methods
          .depositSenior(new BN(100_000_000))
          .accounts(depositSeniorAccounts())
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        seniorBlocked = true;
        assert.include(err.toString(), "SeniorDepositsPaused");
      }
      assert.isTrue(seniorBlocked, "Senior deposits blocked when bit 0x01 set");

      // Verify junior deposits blocked
      let juniorBlocked = false;
      try {
        await trancheProgram.methods
          .depositJunior(new BN(100_000_000))
          .accounts(depositJuniorAccounts())
          .signers([userKp])
          .rpc();
      } catch (err: any) {
        juniorBlocked = true;
        assert.include(err.toString(), "JuniorDepositsPaused");
      }
      assert.isTrue(juniorBlocked, "Junior deposits blocked when bit 0x02 set");

      // Withdrawals should still work (bits 0x04/0x08 not set)
      const sBal = await bal(userSeniorAta);
      if (sBal > BigInt(0)) {
        const withdrawTokens = new BN((sBal / BigInt(100)).toString());
        if (withdrawTokens.gt(new BN(0))) {
          await trancheProgram.methods
            .instantWithdrawSenior(withdrawTokens, true)
            .accounts(instantWithdrawSeniorAccounts())
            .signers([userKp])
            .rpc();
        }
      }
    });

    it("TC7: authority clears pause flags, deposits resume", async () => {
      // Confirm still paused from TC6
      let config = await getConfig();
      assert.equal(config.paused & 0x03, 0x03, "Still paused from TC6");

      // Authority clears pause flags
      await trancheProgram.methods
        .setPaused(0x00)
        .accounts({ authority, trancheConfig: tranchePdas.trancheConfig })
        .rpc();

      config = await getConfig();
      assert.equal(config.paused, 0, "Pause flags cleared");

      // Senior deposit should now succeed
      const sBefore = await bal(userSeniorAta);
      await trancheProgram.methods
        .depositSenior(new BN(50_000_000))
        .accounts(depositSeniorAccounts())
        .signers([userKp])
        .rpc();
      const sAfter = await bal(userSeniorAta);
      assert.isTrue(sAfter > sBefore, "Senior deposit works after unpause");

      // Junior deposit should now succeed
      const jBefore = await bal(userJuniorAta);
      await trancheProgram.methods
        .depositJunior(new BN(50_000_000))
        .accounts(depositJuniorAccounts())
        .signers([userKp])
        .rpc();
      const jAfter = await bal(userJuniorAta);
      assert.isTrue(jAfter > jBefore, "Junior deposit works after unpause");
    });
  });

  describe("VOL-27: Vault validation", () => {
    it("TC8: init with vault waiting_period > 0 fails", async () => {
      const badVaultKp = Keypair.generate();
      const badVault = badVaultKp.publicKey;

      // Create a vault with non-zero withdrawal waiting period
      const badVaultPdas = findVaultPdas(badVault);
      const badVaultAssetIdleAta = getAssociatedTokenAddressSync(
        assetMint,
        badVaultPdas.vaultAssetIdleAuth,
        true
      );

      await vaultProgram.methods
        .initializeVault(
          {
            maxCap: new BN(1_000_000_000_000),
            startAtTs: new BN(0),
            lockedProfitDegradationDuration: new BN(0),
            managerPerformanceFee: 0,
            adminPerformanceFee: 0,
            managerManagementFee: 0,
            adminManagementFee: 0,
            redemptionFee: 0,
            issuanceFee: 0,
            withdrawalWaitingPeriod: new BN(3600), // 1 hour — non-zero!
          },
          "Bad Vault",
          "Vault with waiting period"
        )
        .accounts({
          payer,
          manager: authority,
          admin: authority,
          vault: badVault,
          vaultAssetMint: assetMint,
          vaultAssetIdleAta: badVaultAssetIdleAta,
          assetTokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([payerKp, badVaultKp])
        .rpc();

      // Verify it was created with non-zero waiting period
      const v = await vaultProgram.account.vault.fetch(badVault);
      assert.notEqual(
        v.vaultConfiguration.withdrawalWaitingPeriod.toNumber(),
        0,
        "Bad vault should have non-zero waiting period"
      );

      // Try to initialize tranche config with this vault
      const badTranchePdas = findTranchePdas(badVault, trancheProgram.programId);
      const badVaultLpCustody = getAssociatedTokenAddressSync(
        badVaultPdas.vaultLpMint,
        badTranchePdas.trancheAuthority,
        true
      );
      const badAssetEscrow = getAssociatedTokenAddressSync(
        assetMint,
        badTranchePdas.trancheAuthority,
        true
      );

      let thrown = false;
      try {
        await trancheProgram.methods
          .initializeTrancheConfig({
            aprFloorBps: 200,
            riskXBps: 500,
            riskYBps: 1000,
            riskK: intToDecimalBits(2),
            reserveFeeBps: 500,
            seniorExitFeeBps: 50,
            juniorExitFeeBps: 50,
            minJrSrRatioBps: 1000,
            minCoverageForMintBps: 10000,
            minCoverageBufferBps: 11000,
            seniorCooldownSeconds: new BN(0),
            juniorCooldownSeconds: new BN(0),
            maxObservationAgeSeconds: new BN(259200),
          })
          .accounts({
            authority,
            vault: badVault,
            trancheConfig: badTranchePdas.trancheConfig,
            trancheAuthority: badTranchePdas.trancheAuthority,
            seniorMint: badTranchePdas.seniorMint,
            juniorMint: badTranchePdas.juniorMint,
            assetMint,
            vaultLpMint: badVaultPdas.vaultLpMint,
            vaultLpCustody: badVaultLpCustody,
            assetEscrow: badAssetEscrow,
            aprFeed: badTranchePdas.aprFeed,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
      } catch (err: any) {
        thrown = true;
        assert.include(err.toString(), "VaultWaitingPeriodNotZero");
      }
      assert.isTrue(thrown, "Init should fail with VaultWaitingPeriodNotZero");
    });

    it("TC9: spoofed vault (wrong program owner) rejected", async () => {
      // Create a fake vault account that is NOT owned by the Voltr Vault program.
      // The Anchor `AccountLoader<'info, voltr_vault::state::Vault>` constraint
      // requires the account to be owned by the Voltr Vault program ID.
      // Passing any other account will be rejected by Anchor deserialization.
      const fakeVaultKp = Keypair.generate();

      // Create a system-owned account (not vault-program-owned)
      const sig = await provider.connection.requestAirdrop(
        fakeVaultKp.publicKey,
        LAMPORTS_PER_SOL
      );
      const bh = await provider.connection.getLatestBlockhash();
      await provider.connection.confirmTransaction({
        blockhash: bh.blockhash,
        lastValidBlockHeight: bh.lastValidBlockHeight,
        signature: sig,
      });

      const fakeTranchePdas = findTranchePdas(
        fakeVaultKp.publicKey,
        trancheProgram.programId
      );
      const fakeVaultPdas = findVaultPdas(fakeVaultKp.publicKey);
      const fakeVaultLpCustody = getAssociatedTokenAddressSync(
        fakeVaultPdas.vaultLpMint,
        fakeTranchePdas.trancheAuthority,
        true
      );
      const fakeAssetEscrow = getAssociatedTokenAddressSync(
        assetMint,
        fakeTranchePdas.trancheAuthority,
        true
      );

      let thrown = false;
      try {
        await trancheProgram.methods
          .initializeTrancheConfig({
            aprFloorBps: 200,
            riskXBps: 500,
            riskYBps: 1000,
            riskK: intToDecimalBits(2),
            reserveFeeBps: 500,
            seniorExitFeeBps: 50,
            juniorExitFeeBps: 50,
            minJrSrRatioBps: 1000,
            minCoverageForMintBps: 10000,
            minCoverageBufferBps: 11000,
            seniorCooldownSeconds: new BN(0),
            juniorCooldownSeconds: new BN(0),
            maxObservationAgeSeconds: new BN(259200),
          })
          .accounts({
            authority,
            vault: fakeVaultKp.publicKey,
            trancheConfig: fakeTranchePdas.trancheConfig,
            trancheAuthority: fakeTranchePdas.trancheAuthority,
            seniorMint: fakeTranchePdas.seniorMint,
            juniorMint: fakeTranchePdas.juniorMint,
            assetMint,
            vaultLpMint: fakeVaultPdas.vaultLpMint,
            vaultLpCustody: fakeVaultLpCustody,
            assetEscrow: fakeAssetEscrow,
            aprFeed: fakeTranchePdas.aprFeed,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
      } catch (err: any) {
        thrown = true;
        // Anchor rejects account not owned by expected program
        // The error could be AccountOwnedByWrongProgram or AccountNotInitialized
        assert.isTrue(
          err.toString().includes("AccountOwnedByWrongProgram") ||
            err.toString().includes("AccountNotInitialized") ||
            err.toString().includes("AccountDidNotDeserialize") ||
            err.toString().includes("has_one"),
          `Expected owner/deserialization error, got: ${err.toString().slice(0, 200)}`
        );
      }
      assert.isTrue(thrown, "Spoofed vault should be rejected");
    });
  });
});
