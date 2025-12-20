import * as anchor from "@coral-xyz/anchor";
import { ethers } from "ethers";
import { Program } from "@coral-xyz/anchor";
import { ComputeBudgetProgram, PublicKey } from "@solana/web3.js";
import { assert } from "chai";
import { SolanaVampProgram } from "../target/types/solana_vamp_program";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction
} from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";

// Constants
const TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

// Program setup
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
const program = anchor.workspace.solanaVampProgram as Program<SolanaVampProgram>;
const PROGRAM_ID = program.programId;
const claimerKeypair = anchor.web3.Keypair.generate();

describe("solana-vamp-project", () => {
  const authority = provider.wallet.publicKey;

  // Helper functions
  async function setupInitAccounts(authority: PublicKey) {
    let count = new BN(0);
    const [mintAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from('mint'), authority.toBuffer(), count.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    count = new BN(1);
    const [mintAccount2] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from('mint'), authority.toBuffer(), count.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    count = new BN(2);
    const [mintAccount3] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from('mint'), authority.toBuffer(), count.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const [metadataAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBuffer(), mintAccount.toBuffer()],
      TOKEN_METADATA_PROGRAM_ID
    );

    const [metadataAccount2] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBuffer(), mintAccount2.toBuffer()],
      TOKEN_METADATA_PROGRAM_ID
    );

    const [metadataAccount3] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBuffer(), mintAccount3.toBuffer()],
      TOKEN_METADATA_PROGRAM_ID
    );

    const [vampState, vampStateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vamp"), mintAccount.toBuffer()],
      PROGRAM_ID
    );

    const [vampState2, vampStateBump2] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vamp"), mintAccount2.toBuffer()],
      PROGRAM_ID
    );

    const [vampState3, vampStateBump3] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vamp"), mintAccount3.toBuffer()],
      PROGRAM_ID
    );

    const [vault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), mintAccount.toBuffer()],
      PROGRAM_ID
    );

    const [vault2] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), mintAccount2.toBuffer()],
      PROGRAM_ID
    );

    const [vault3] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), mintAccount3.toBuffer()],
      PROGRAM_ID
    );

    const [solVault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("sol_vault"), mintAccount.toBuffer()],
      PROGRAM_ID
    );

    const [solVault2] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("sol_vault"), mintAccount2.toBuffer()],
      PROGRAM_ID
    );

    const [solVault3] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("sol_vault"), mintAccount3.toBuffer()],
      PROGRAM_ID
    );

    return {
      metadataAccount,
      vampState,
      vampStateBump,
      vault,
      solVault,
      mintAccount,
      mintAccount2,
      vampState2,
      vampStateBump2,
      vault2,
      solVault2,
      metadataAccount2,
      mintAccount3,
      vampState3,
      vampStateBump3,
      vault3,
      solVault3,
      metadataAccount3
    };
  }

  async function signMessage(message: string, privateKey: string): Promise<[string, string]> {
    try {
      const wallet = new ethers.Wallet(privateKey);
      const signature = await wallet.signMessage(message);
      return [wallet.address, signature];
    } catch (error) {
      console.error('Error signing message:', error);
      throw error;
    }
  }

  function hexToBytes(hex: string): number[] {
    if (hex.length % 2 !== 0) {
      throw new Error("Invalid hex string");
    }
    const bytes: number[] = [];
    for (let i = 2; i < hex.length; i += 2) {
      bytes[(i - 2) / 2] = parseInt(hex.substr(i, 2), 16);
    }
    return bytes;
  }

  async function verifyVampState(vampState: PublicKey, vampStateBump: number, mintAccount: PublicKey) {
    const vampStateAccount = await program.account.vampState.fetch(vampState);
    assert.equal(vampStateAccount.mint.toBase58(), mintAccount.toBase58(), "Mint account mismatch");
    assert.equal(vampStateAccount.bump, vampStateBump, "Bump mismatch");
  }

  async function getVampingData(): Promise<Buffer> {
    return Buffer.from([18, 12, 77, 121, 32, 77, 109, 101, 109, 116, 111, 107, 101, 110, 26, 4, 77, 69, 77, 69, 34, 20, 10, 11, 85, 6, 100, 79, 145, 115, 236, 165, 13, 29, 125, 44, 172, 229, 150, 165, 229, 85, 42, 27, 104, 116, 116, 112, 115, 58, 47, 47, 101, 120, 97, 109, 112, 108, 101, 46, 99, 111, 109, 47, 116, 111, 107, 101, 110, 47, 49, 48, 128, 240, 179, 163, 223, 248, 70, 56, 9, 72, 1, 80, 210, 133, 216, 204, 4, 90, 20, 249, 139, 130, 139, 56, 155, 239, 78, 187, 181, 145, 28, 161, 126, 79, 121, 137, 201, 6, 141, 98, 20, 139, 37, 237, 6, 226, 22, 85, 63, 141, 66, 101, 153, 96, 97, 176, 160, 101, 175, 163, 92, 106, 32, 17, 17, 17, 17, 17, 17, 17, 17, 34, 34, 34, 34, 34, 34, 34, 34, 119, 119, 119, 119, 119, 119, 119, 119, 153, 153, 153, 153, 153, 153, 153, 153]);
  }

  function getEthAddress() {
    return "0x8ebd059f9acef4758a8ac8d6e017d6c76b248c82";
  }

  function getEthAddressBytes() {
    return hexToBytes(getEthAddress());
  }

  async function getSignatures() {
    const solverSignature = [251, 190, 51, 170, 61, 104, 94, 173, 134, 86, 195, 233, 114, 39, 131, 218, 205, 35, 184, 80, 233, 53, 220, 244, 27, 165, 216, 133, 6, 251, 209, 206, 62, 148, 200, 51, 176, 66, 113, 38, 158, 246, 60, 234, 141, 183, 42, 176, 53, 65, 143, 195, 84, 99, 162, 156, 57, 192, 188, 82, 3, 23, 55, 169, 27];

    const validatorSignature = [132, 102, 82, 207, 139, 9, 105, 132, 111, 194, 73, 232, 249, 93, 122, 112, 80, 215, 153, 195, 146, 169, 161, 84, 195, 61, 80, 124, 160, 220, 174, 148, 91, 127, 181, 185, 19, 26, 125, 186, 208, 87, 72, 6, 210, 252, 242, 117, 76, 4, 174, 63, 192, 211, 223, 144, 225, 206, 40, 241, 224, 119, 94, 225, 27];
  
    const ownerSignature = [140, 92, 134, 184, 0, 228, 15, 165, 64, 112, 11, 199, 184, 110, 96, 93, 125, 4, 68, 147, 124, 176, 160, 51, 76, 86, 4, 248, 101, 100, 3, 147, 9, 252, 21, 198, 4, 40, 200, 2, 43, 44, 193, 163, 224, 105, 113, 21, 65, 218, 235, 207, 125, 43, 216, 68, 106, 155, 15, 99, 210, 221, 127, 220, 28];

    return {
      solverSignature,
      validatorSignature,
      ownerSignature,
    };
  }

  // Test cases
  it("Initializes Vamp State and Mints Token", async () => {
    const accounts = await setupInitAccounts(authority);
    const vampingData = await getVampingData();

    try {
      const tx = await program.methods
        .createTokenMint(new BN(0), 9, vampingData)
        .accounts({
          authority,
          mintAccount: accounts.mintAccount,
          metadataAccount: accounts.metadataAccount,
          vampState: accounts.vampState,
          vault: accounts.vault,
          solVault: accounts.solVault,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([provider.wallet.payer])
        .preInstructions([
          ComputeBudgetProgram.setComputeUnitLimit({
            units: 2_000_000,
          })
        ])
        .rpc();

      const details = await provider.connection.getParsedTransaction(tx, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });

      if (details?.meta) {
        const fee = details.meta.fee;
        console.log(`Transaction Fee: ${fee} lamports`);
        const SOL = fee / anchor.web3.LAMPORTS_PER_SOL;
        console.log(`Transaction Fee in SOL: ${SOL.toFixed(9)} SOL`);
      } else {
        console.warn("Transaction metadata unavailable.");
      }
      await verifyVampState(accounts.vampState, accounts.vampStateBump, accounts.mintAccount);
    } catch (error) {
      console.error("Transaction error:", error);
      throw error;
    }
  });

  it("Claims tokens for a user based on ETH address mapping", async () => {
    const accounts = await setupInitAccounts(authority);
    const mintAccount2 = accounts.mintAccount2;
    const vampingData = await getVampingData();

    const [claimState] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("claim"), accounts.vampState2.toBuffer(), Buffer.from(getEthAddress().slice(2), "hex")],
      PROGRAM_ID
    );

    // Initialize token mint
    await program.methods
      .createTokenMint(new BN(1), 9, vampingData)
      .accounts({
        authority,
        mintAccount: mintAccount2,
        metadataAccount: accounts.metadataAccount2,
        vampState: accounts.vampState2,
        vault: accounts.vault2,
        solVault: accounts.solVault2,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([provider.wallet.payer])
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({
          units: 2_000_000,
        })
      ])
      .rpc();

    // Setup claimer account
    const claimerTokenAccount = await getAssociatedTokenAddress(mintAccount2, claimerKeypair.publicKey);
    const sig = await provider.connection.requestAirdrop(claimerKeypair.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig);

    const ataIx = createAssociatedTokenAccountInstruction(
      claimerKeypair.publicKey,
      claimerTokenAccount,
      claimerKeypair.publicKey,
      mintAccount2
    );
    const ataTx = new anchor.web3.Transaction().add(ataIx);
    await provider.sendAndConfirm(ataTx, [claimerKeypair]);

    // Get initial SOL balance of claimer and SOL vault
    const initialClaimerBalance = await provider.connection.getBalance(claimerKeypair.publicKey);
    const initialSolVaultBalance = await provider.connection.getBalance(accounts.solVault2);

    // Execute claim
    const {solverSignature, validatorSignature, ownerSignature} = await getSignatures();

    await program.methods
      .claim(getEthAddressBytes(), new BN(1_000_000_000), solverSignature, validatorSignature, ownerSignature)
      .accounts({
        authority: claimerKeypair.publicKey,
        vampState: accounts.vampState2,
        claimState,
        vault: accounts.vault2,
        solVault: accounts.solVault2,
        claimerTokenAccount,
        mintAccount: mintAccount2,
        token_program: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([claimerKeypair])
      .rpc();

    // Verify token balance
    const claimerData = await provider.connection.getTokenAccountBalance(claimerTokenAccount);
    assert.equal(claimerData.value.amount, "1000000000", "Token amount mismatch");

    // Verify SOL was deposited to SOL vault
    const finalSolVaultBalance = await provider.connection.getBalance(accounts.solVault2);
    const finalClaimerBalance = await provider.connection.getBalance(claimerKeypair.publicKey);
    
    console.log(`Initial SOL vault balance: ${initialSolVaultBalance} lamports`);
    console.log(`Final SOL vault balance: ${finalSolVaultBalance} lamports`);
    console.log(`SOL deposited to vault: ${finalSolVaultBalance - initialSolVaultBalance} lamports`);
    console.log(`SOL deducted from claimer: ${initialClaimerBalance - finalClaimerBalance} lamports`);
    
    // Verify SOL was transferred (should be greater than 0)
    assert.isTrue(finalSolVaultBalance > initialSolVaultBalance, "SOL was not deposited to vault");
    assert.isTrue(initialClaimerBalance > finalClaimerBalance, "SOL was not deducted from claimer");

    // Verify double claim prevention
    try {
      await program.methods
        .claim(getEthAddressBytes(), new BN(1_000_000_000), solverSignature, validatorSignature, ownerSignature)
        .accounts({
          authority: claimerKeypair.publicKey,
          vampState: accounts.vampState2,
          claimState,
          vault: accounts.vault2,
          solVault: accounts.solVault2,
          claimerTokenAccount,
          mintAccount: mintAccount2,
          token_program: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([claimerKeypair])
        .rpc();

      assert.fail("Second claim should have failed but succeeded");
    } catch (err) {
      assert.include(err.message, "Simulation failed");
    }
  });

  it("Tests bonding curve SOL calculation and deposition", async () => {
    const accounts = await setupInitAccounts(authority);
    const vampingData = await getVampingData();

    // Initialize token mint
    await program.methods
      .createTokenMint(new BN(2), 9, vampingData)
      .accounts({
        authority,
        mintAccount: accounts.mintAccount3,
        metadataAccount: accounts.metadataAccount3,
        vampState: accounts.vampState3,
        vault: accounts.vault3,
        solVault: accounts.solVault3,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([provider.wallet.payer])
      .rpc();

    // Setup claimer account
    const claimerTokenAccount = await getAssociatedTokenAddress(accounts.mintAccount3, claimerKeypair.publicKey);
    const sig = await provider.connection.requestAirdrop(claimerKeypair.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig);

    const ataIx = createAssociatedTokenAccountInstruction(
      claimerKeypair.publicKey,
      claimerTokenAccount,
      claimerKeypair.publicKey,
      accounts.mintAccount3
    );
    const ataTx = new anchor.web3.Transaction().add(ataIx);
    await provider.sendAndConfirm(ataTx, [claimerKeypair]);

    // Get initial balances
    const initialClaimerBalance = await provider.connection.getBalance(claimerKeypair.publicKey);
    const initialSolVaultBalance = await provider.connection.getBalance(accounts.solVault3);

    // Claim a small amount first (should cost less)
    const smallAmount = new BN(1_000_000_000); // 1 token
    const [claimState1] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("claim"), accounts.vampState3.toBuffer(), Buffer.from(getEthAddress().slice(2), "hex")],
      PROGRAM_ID
    );

    const {solverSignature, validatorSignature, ownerSignature} = await getSignatures();

    await program.methods
      .claim(getEthAddressBytes(), smallAmount, solverSignature, validatorSignature, ownerSignature)
      .accounts({
        authority: claimerKeypair.publicKey,
        vampState: accounts.vampState3,
        claimState: claimState1,
        vault: accounts.vault3,
        solVault: accounts.solVault3,
        claimerTokenAccount,
        mintAccount: accounts.mintAccount3,
        token_program: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([claimerKeypair])
      .rpc();

    const balanceAfterFirstClaim = await provider.connection.getBalance(claimerKeypair.publicKey);
    const solVaultBalanceAfterFirstClaim = await provider.connection.getBalance(accounts.solVault3);
    
    const firstClaimCost = initialClaimerBalance - balanceAfterFirstClaim;
    const firstClaimDeposited = solVaultBalanceAfterFirstClaim - initialSolVaultBalance;
    
    console.log(`First claim cost: ${firstClaimCost} lamports (${firstClaimCost / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
    console.log(`First claim deposited: ${firstClaimDeposited} lamports`);

    // // Claim a larger amount (should cost more due to bonding curve)
    // const nextAmount = new BN(1_000_000_000); // 1 token
    // const [claimState2] = anchor.web3.PublicKey.findProgramAddressSync(
    //   [Buffer.from("claim"), accounts.vampState3.toBuffer(), Buffer.from("0x1234567890123456789012345678901234567890".slice(2), "hex")],
    //   PROGRAM_ID
    // );

    // await program.methods
    //   .claim(getEthAddressBytes(), nextAmount, solverSignature, validatorSignature, ownerSignature)
    //   .accounts({
    //     authority: claimerKeypair.publicKey,
    //     vampState: accounts.vampState3,
    //     claimState: claimState2,
    //     vault: accounts.vault3,
    //     solVault: accounts.solVault3,
    //     claimerTokenAccount,
    //     mintAccount: accounts.mintAccount3,
    //     token_program: TOKEN_PROGRAM_ID,
    //     systemProgram: anchor.web3.SystemProgram.programId,
    //   })
    //   .signers([claimerKeypair])
    //   .rpc();

    // const finalClaimerBalance = await provider.connection.getBalance(claimerKeypair.publicKey);
    // const finalSolVaultBalance = await provider.connection.getBalance(accounts.solVault3);
    
    // const secondClaimCost = balanceAfterFirstClaim - finalClaimerBalance;
    // const secondClaimDeposited = finalSolVaultBalance - solVaultBalanceAfterFirstClaim;
    
    // console.log(`Second claim cost: ${secondClaimCost} lamports (${secondClaimCost / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
    // console.log(`Second claim deposited: ${secondClaimDeposited} lamports`);

    // // Verify bonding curve behavior: second claim should cost more per token
    // const firstCostPerToken = firstClaimCost / smallAmount.toNumber();
    // const secondCostPerToken = secondClaimCost / nextAmount.toNumber();
    
    // console.log(`First claim cost per token: ${firstCostPerToken} lamports`);
    // console.log(`Second claim cost per token: ${secondCostPerToken} lamports`);
    
    // // The second claim should cost more per token due to bonding curve
    // assert.isTrue(secondCostPerToken > firstCostPerToken, "Bonding curve not working - second claim should cost more per token");
    
    // // Verify SOL was properly deposited
    // assert.isTrue(finalSolVaultBalance > initialSolVaultBalance, "SOL was not deposited to vault");
    // assert.isTrue(initialClaimerBalance > finalClaimerBalance, "SOL was not deducted from claimer");
    
    // // Verify the deposited amount matches the deducted amount (minus transaction fees)
    // const totalDeposited = finalSolVaultBalance - initialSolVaultBalance;
    // const totalDeducted = initialClaimerBalance - finalClaimerBalance;
    // const transactionFees = totalDeducted - totalDeposited;
    
    // console.log(`Total SOL deposited: ${totalDeposited} lamports`);
    // console.log(`Total SOL deducted: ${totalDeducted} lamports`);
    // console.log(`Transaction fees: ${transactionFees} lamports`);
    
    // assert.isTrue(transactionFees > 0, "Transaction fees should be positive");
  });
});