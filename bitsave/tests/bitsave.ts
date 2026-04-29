import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bitsave } from "../target/types/bitsave";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

describe("bitsave", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.bitsave as Program<Bitsave>;

  const admin = (provider.wallet as anchor.Wallet).payer;
  const user = Keypair.generate();
  
  let stableCoinMint: PublicKey;
  let csTokenMint: PublicKey;
  
  let userStableTokenAccount: PublicKey;
  let vaultStableTokenAccount: PublicKey;

  // PDAs
  const [globalStatePDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_state")],
    program.programId
  );

  const [userVaultPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("user_vault"), user.publicKey.toBuffer()],
    program.programId
  );

  const solSavingName = "My_SOL_Saving";
  const [solSavingPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("saving"), userVaultPDA.toBuffer(), Buffer.from(solSavingName)],
    program.programId
  );

  const tokenSavingName = "My_Token_Saving";
  const [tokenSavingPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("saving"), userVaultPDA.toBuffer(), Buffer.from(tokenSavingName)],
    program.programId
  );

  before(async () => {
    // Airdrop SOL to user
    const signature = await provider.connection.requestAirdrop(user.publicKey, 10 * LAMPORTS_PER_SOL);
    const latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature
    });

    stableCoinMint = await createMint(provider.connection, admin, admin.publicKey, null, 6);
    csTokenMint = await createMint(provider.connection, admin, admin.publicKey, null, 6);

    const userAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection, user, stableCoinMint, user.publicKey
    );
    userStableTokenAccount = userAccount.address;

    await mintTo(provider.connection, admin, stableCoinMint, userStableTokenAccount, admin, 1000_000_000);
    
    const vaultAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection, user, stableCoinMint, userVaultPDA, true
    );
    vaultStableTokenAccount = vaultAccount.address;
  });

  it("Is initialized!", async () => {
    await program.methods
      .initialize(stableCoinMint, csTokenMint)
      .accountsStrict({
        globalState: globalStatePDA,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const state = await program.account.globalState.fetch(globalStatePDA);
    assert.equal(state.admin.toBase58(), admin.publicKey.toBase58());
    assert.equal(state.stableCoinMint.toBase58(), stableCoinMint.toBase58());
  });

  it("User joins bitsave", async () => {
    await program.methods
      .joinBitsave()
      .accountsStrict({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        user: user.publicKey,
        adminAccount: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    const vault = await program.account.userVault.fetch(userVaultPDA);
    assert.equal(vault.owner.toBase58(), user.publicKey.toBase58());
  });

  it("User creates a SOL saving", async () => {
    const amount = new anchor.BN(1 * LAMPORTS_PER_SOL);
    const maturityTime = new anchor.BN(Math.floor(Date.now() / 1000) + 10);
    
    await program.methods
      .createSolSaving(solSavingName, maturityTime, 10, false, amount)
      .accountsStrict({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: solSavingPDA,
        user: user.publicKey,
        adminAccount: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    const saving = await program.account.saving.fetch(solSavingPDA);
    assert.equal(saving.amount.toNumber(), amount.toNumber());
    assert.isTrue(saving.isValid);
  });

  it("User creates a Token saving", async () => {
    const amount = new anchor.BN(100_000_000);
    const maturityTime = new anchor.BN(Math.floor(Date.now() / 1000) + 10);
    
    await program.methods
      .createTokenSaving(tokenSavingName, maturityTime, 10, false, amount)
      .accountsStrict({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: tokenSavingPDA,
        user: user.publicKey,
        adminAccount: admin.publicKey,
        tokenMint: stableCoinMint,
        userTokenAccount: userStableTokenAccount,
        vaultTokenAccount: vaultStableTokenAccount,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const saving = await program.account.saving.fetch(tokenSavingPDA);
    assert.equal(saving.amount.toNumber(), amount.toNumber());
    assert.equal(saving.tokenMint.toBase58(), stableCoinMint.toBase58());
  });

  it("User increments SOL saving", async () => {
    const incrementAmount = new anchor.BN(0.5 * LAMPORTS_PER_SOL);
    const preSaving = await program.account.saving.fetch(solSavingPDA);
    
    await program.methods
      .incrementSolSaving(incrementAmount)
      .accountsStrict({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: solSavingPDA,
        user: user.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    const postSaving = await program.account.saving.fetch(solSavingPDA);
    assert.equal(postSaving.amount.toNumber(), preSaving.amount.toNumber() + incrementAmount.toNumber());
  });

  it("User withdraws SOL saving prematurely (with penalty)", async () => {
    const preUserBalance = await provider.connection.getBalance(user.publicKey);
    
    await program.methods
      .withdrawSolSaving()
      .accountsStrict({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: solSavingPDA,
        user: user.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();
      
    try {
      await program.account.saving.fetch(solSavingPDA);
      assert.fail("Saving account should have been closed");
    } catch (e) {}

    const postUserBalance = await provider.connection.getBalance(user.publicKey);
    assert.isTrue(postUserBalance > preUserBalance);
  });

  it("User increments Token saving", async () => {
    const incrementAmount = new anchor.BN(50_000_000); // 50 tokens
    const preSaving = await program.account.saving.fetch(tokenSavingPDA);
    
    await program.methods
      .incrementTokenSaving(incrementAmount)
      .accountsStrict({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: tokenSavingPDA,
        user: user.publicKey,
        userTokenAccount: userStableTokenAccount,
        vaultTokenAccount: vaultStableTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const postSaving = await program.account.saving.fetch(tokenSavingPDA);
    assert.equal(postSaving.amount.toNumber(), preSaving.amount.toNumber() + incrementAmount.toNumber());
  });

  it("User withdraws Token saving prematurely (with penalty)", async () => {
    const preUserBalance = await provider.connection.getTokenAccountBalance(userStableTokenAccount);
    
    await program.methods
      .withdrawTokenSaving()
      .accountsStrict({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: tokenSavingPDA,
        user: user.publicKey,
        userTokenAccount: userStableTokenAccount,
        vaultTokenAccount: vaultStableTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();
      
    try {
      await program.account.saving.fetch(tokenSavingPDA);
      assert.fail("Saving account should have been closed");
    } catch (e) {}

    const postUserBalance = await provider.connection.getTokenAccountBalance(userStableTokenAccount);
    assert.isTrue(Number(postUserBalance.value.amount) > Number(preUserBalance.value.amount));
  });

  // --- FAILING SCENARIOS ---

  it("Fails to create a SOL saving with past maturity time", async () => {
    const amount = new anchor.BN(1 * LAMPORTS_PER_SOL);
    const pastMaturityTime = new anchor.BN(Math.floor(Date.now() / 1000) - 100);
    const failSavingName = "Fail_Past_Time";
    const [failSavingPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("saving"), userVaultPDA.toBuffer(), Buffer.from(failSavingName)],
      program.programId
    );

    try {
      await program.methods
        .createSolSaving(failSavingName, pastMaturityTime, 10, false, amount)
        .accountsStrict({
          globalState: globalStatePDA,
          userVault: userVaultPDA,
          saving: failSavingPDA,
          user: user.publicKey,
          adminAccount: admin.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();
      assert.fail("Should have failed with InvalidTime");
    } catch (e: any) {
      assert.include(e.message, "Invalid maturity time");
    }
  });

  it("Fails to create a Token saving with safe mode enabled", async () => {
    const amount = new anchor.BN(100_000_000);
    const maturityTime = new anchor.BN(Math.floor(Date.now() / 1000) + 10);
    const failSavingName = "Fail_Safe_Mode";
    const [failSavingPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("saving"), userVaultPDA.toBuffer(), Buffer.from(failSavingName)],
      program.programId
    );

    try {
      await program.methods
        .createTokenSaving(failSavingName, maturityTime, 10, true, amount)
        .accountsStrict({
          globalState: globalStatePDA,
          userVault: userVaultPDA,
          saving: failSavingPDA,
          user: user.publicKey,
          adminAccount: admin.publicKey,
          tokenMint: stableCoinMint,
          userTokenAccount: userStableTokenAccount,
          vaultTokenAccount: vaultStableTokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
      assert.fail("Should have failed with NotSupported");
    } catch (e: any) {
      assert.include(e.message, "Safe mode not yet supported");
    }
  });

  it("Fails to create a saving using another user's vault (PDA Security)", async () => {
    const maliciousUser = Keypair.generate();
    
    // Fund malicious user
    const sig = await provider.connection.requestAirdrop(maliciousUser.publicKey, 1 * LAMPORTS_PER_SOL);
    const latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: sig
    });

    const amount = new anchor.BN(1 * LAMPORTS_PER_SOL);
    const maturityTime = new anchor.BN(Math.floor(Date.now() / 1000) + 10);
    const failSavingName = "Malicious_Saving";
    const [failSavingPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("saving"), userVaultPDA.toBuffer(), Buffer.from(failSavingName)],
      program.programId
    );

    try {
      await program.methods
        .createSolSaving(failSavingName, maturityTime, 10, false, amount)
        .accountsStrict({
          globalState: globalStatePDA,
          userVault: userVaultPDA, // Legitimate vault belonging to the first user
          saving: failSavingPDA,
          user: maliciousUser.publicKey, // Malicious signer
          adminAccount: admin.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([maliciousUser])
        .rpc();
      assert.fail("Should have failed PDA seed constraint");
    } catch (e: any) {
      assert.include(e.message, "A seeds constraint was violated");
    }
  });
});
