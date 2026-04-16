import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bitsave } from "../target/types/bitsave";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

describe("bitsave", () => {
  // Configure the client to use the local cluster.
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

    // Create stable coin mint
    stableCoinMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    // Create cs token mint
    csTokenMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    // Create user token account and mint some tokens
    const userAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user,
      stableCoinMint,
      user.publicKey
    );
    userStableTokenAccount = userAccount.address;

    await mintTo(
      provider.connection,
      admin,
      stableCoinMint,
      userStableTokenAccount,
      admin,
      1000_000_000 // 1000 tokens
    );
    
    // Determine vault token account (PDA)
    const vaultAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user,
      stableCoinMint,
      userVaultPDA,
      true // allowOwnerOffCurve since it's a PDA
    );
    vaultStableTokenAccount = vaultAccount.address;
  });

  it("Is initialized!", async () => {
    await program.methods
      .initialize(stableCoinMint, csTokenMint)
      .accounts({
        globalState: globalStatePDA,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([admin])
      .rpc();

    const state = await program.account.globalState.fetch(globalStatePDA);
    assert.equal(state.admin.toBase58(), admin.publicKey.toBase58());
    assert.equal(state.stableCoinMint.toBase58(), stableCoinMint.toBase58());
  });

  it("User joins bitsave", async () => {
    await program.methods
      .joinBitsave()
      .accounts({
        globalState: globalStatePDA,
        user_vault: userVaultPDA,
        user: user.publicKey,
        adminAccount: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user])
      .rpc();

    const vault = await program.account.userVault.fetch(userVaultPDA);
    assert.equal(vault.owner.toBase58(), user.publicKey.toBase58());
  });

  it("User creates a SOL saving", async () => {
    const amount = new anchor.BN(1 * LAMPORTS_PER_SOL);
    const maturityTime = new anchor.BN(Math.floor(Date.now() / 1000) + 10); // 10 seconds from now
    
    await program.methods
      .createSaving(solSavingName, maturityTime, 10, false, amount)
      .accounts({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: solSavingPDA,
        user: user.publicKey,
        adminAccount: admin.publicKey,
        tokenMint: PublicKey.default,
        userTokenAccount: user.publicKey, // Dummy, ignored for SOL
        vaultTokenAccount: userVaultPDA, // Dummy, ignored for SOL
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([user])
      .rpc();

    const saving = await program.account.saving.fetch(solSavingPDA);
    assert.equal(saving.amount.toNumber(), amount.toNumber());
    assert.equal(saving.tokenMint.toBase58(), PublicKey.default.toBase58());
    assert.isTrue(saving.isValid);
  });

  it("User creates a Token saving", async () => {
    const amount = new anchor.BN(100_000_000); // 100 tokens
    const maturityTime = new anchor.BN(Math.floor(Date.now() / 1000) + 10); // 10 seconds from now
    
    await program.methods
      .createSaving(tokenSavingName, maturityTime, 10, false, amount)
      .accounts({
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
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      } as any)
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
      .incrementSaving(incrementAmount)
      .accounts({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: solSavingPDA,
        user: user.publicKey,
        userTokenAccount: user.publicKey,
        vaultTokenAccount: userVaultPDA,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([user])
      .rpc();

    const postSaving = await program.account.saving.fetch(solSavingPDA);
    assert.equal(postSaving.amount.toNumber(), preSaving.amount.toNumber() + incrementAmount.toNumber());
  });

  it("User withdraws SOL saving prematurely (with penalty)", async () => {
    const preUserBalance = await provider.connection.getBalance(user.publicKey);
    
    await program.methods
      .withdrawSaving()
      .accounts({
        globalState: globalStatePDA,
        userVault: userVaultPDA,
        saving: solSavingPDA,
        user: user.publicKey,
        userTokenAccount: user.publicKey,
        vaultTokenAccount: userVaultPDA,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([user])
      .rpc();
      
    // Because the saving account is closed, fetching it should fail
    try {
      await program.account.saving.fetch(solSavingPDA);
      assert.fail("Saving account should have been closed");
    } catch (e) {
      // Expected
    }

    const postUserBalance = await provider.connection.getBalance(user.publicKey);
    assert.isTrue(postUserBalance > preUserBalance);
  });
});
