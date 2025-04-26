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

const TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
const PROGRAM_ID = new PublicKey("5zKTcVqXKk1vYGZpK47BvMo8fwtUrofroCdzSK931wVc");

// Generate a keypair for the mint account (not a PDA)
const mintKeypair = anchor.web3.Keypair.generate();
const mintKeypair2 = anchor.web3.Keypair.generate();
const claimerKeypair = anchor.web3.Keypair.generate();

describe("solana-vamp-project", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.solanaVampProgram as Program<SolanaVampProgram>;
  const authority = provider.wallet.publicKey;

  it("Initializes Vamp State and Mints Token", async () => {
    const mintAccount1 = mintKeypair.publicKey;
    const accounts = await setupInitAccounts(mintAccount1);
    const vampingData = await getVampingData();

    try {
      const tx = await program.methods
        .createTokenMint(vampingData)
        .accounts({
          authority,
          mintAccount: mintKeypair.publicKey,
          metadataAccount: accounts.metadataAccount,
          vampState: accounts.vampState,
          vault: accounts.vault,
          mintAuthority: accounts.mintAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([provider.wallet.payer, mintKeypair])
        .preInstructions([
          ComputeBudgetProgram.setComputeUnitLimit({
            units: 2_000_000,
          })
        ])
        .rpc();
      await verifyVampState(accounts.vampState, accounts.vampStateBump, mintAccount1);
    } catch (error) {
      console.error("Transaction error:", error);
      throw error;
    }
  });

  it("Claims tokens for a user based on ETH address mapping", async () => {
    const mintAccount2 = mintKeypair2.publicKey;

    const accounts = await setupInitAccounts(mintAccount2);
    const vampingData = await getVampingData();

    await program.methods
      .createTokenMint(vampingData)
      .accounts({
        authority,
        mintAccount: mintKeypair2.publicKey,
        metadataAccount: accounts.metadataAccount,
        vampState: accounts.vampState,
        vault: accounts.vault,
        mintAuthority: accounts.mintAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([provider.wallet.payer, mintKeypair2])
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({
          units: 2_000_000,
        })
      ])
      .rpc();

    // Verify the vamp state was created correctly
    const vampStateAccount = await program.account.vampState.fetch(accounts.vampState);

    // Use the first address from the token mappings

    const amount = 1000000000;
    const [ethAddress, ethSignature] = await signMessage(amount.toString(), "94eb3102993b41ec55c241060f47daa0f6372e2e3ad7e91612ae36c364042e44");

    const claimerTokenAccount = await getAssociatedTokenAddress(mintAccount2, claimerKeypair.publicKey);

    // Airdrop SOL to the claimer and create the ATA
    const sig = await provider.connection.requestAirdrop(claimerKeypair.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig);

    const ataIx = createAssociatedTokenAccountInstruction(
      claimerKeypair.publicKey,
      claimerTokenAccount,
      claimerKeypair.publicKey,
      mintAccount2
    );
    const ataTx = new anchor.web3.Transaction().add(ataIx);
    await provider.sendAndConfirm(ataTx, [claimerKeypair]);

    // Call claim
    const ethAddressBytes: number[] = hexToBytes(ethAddress);
    const ethSignatureBytes = hexToBytes(ethSignature);
    const tx = await program.methods
      .claim(new BN(amount), ethAddressBytes, ethSignatureBytes)
      .accounts({
        authority: claimerKeypair.publicKey,
        vampState: accounts.vampState,
        vault: accounts.vault,
        claimerTokenAccount: claimerTokenAccount,
        mintAccount: mintAccount2,
        token_program: TOKEN_PROGRAM_ID,
      } as any)
      .signers([claimerKeypair])
      .rpc();

    // Check if tokens were transferred
    const claimerAta = await provider.connection.getTokenAccountBalance(claimerTokenAccount);
    console.log("Claimer token balance:", claimerAta.value.amount);
    assert.equal(claimerAta.value.amount, amount.toString(), "Token amount mismatch");
  });

  async function setupInitAccounts(mintAccount: PublicKey) {
    const [mintAuthority] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint_authority")],
      PROGRAM_ID
    );

    const [metadataAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mintAccount.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID
    );

    const [vampState, vampStateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vamp"), mintAccount.toBuffer()],
      PROGRAM_ID
    );

    const [vault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), mintAccount.toBuffer()],
      PROGRAM_ID
    );

    return {
      mintAuthority,
      metadataAccount,
      vampState,
      vampStateBump,
      vault
    };
  }

  async function signMessage(message: string, privateKey: string) {
    try {
      // Create a wallet instance from the private key
      const wallet = new ethers.Wallet(privateKey);

      // Sign the message using Ethereum's prefixed message hashing
      const signature = await wallet.signMessage(message);
      return [wallet.address, signature];
    } catch (error) {
      console.error('Error signing message:', error);
      throw error;
    }
  }

  function hexToBytes(hex: string) {
    if (hex.length % 2 !== 0) {
      throw new Error("Invalid hex string");
    }
    const bytes = [];
    for (let i = 2; i < hex.length; i += 2) {
      bytes[(i - 2) / 2] = parseInt(hex.substr(i, 2), 16);
    }
    return bytes;
  }

  async function verifyVampState(vampState: PublicKey, vampStateBump: number, mintAccount: PublicKey) {
    const vampStateAccount = await program.account.vampState.fetch(vampState);

    assert.equal(vampStateAccount.mint.toBase58(), mintAccount.toBase58(), "Mint account mismatch");
    assert.equal(vampStateAccount.authority.toBase58(), authority.toBase58(), "Authority mismatch");
    assert.equal(vampStateAccount.bump, vampStateBump, "Bump mismatch");
    assert.isArray(vampStateAccount.tokenMappings, "Token mappings should be an array");
  }

  async function getVampingData() {
    return Buffer.from(
      [
        10, 32, 90, 160, 56, 251, 44, 118, 170, 168, 64, 47, 17, 121, 171, 204, 191, 209, 4, 159,
        217, 18, 75, 33, 146, 241, 243, 172, 228, 148, 215, 40, 55, 212, 18, 13, 86, 97, 109, 112,
        105, 110, 103, 32, 84, 111, 107, 101, 110, 26, 4, 86, 65, 77, 80, 34, 20, 182, 154, 101,
        107, 43, 232, 170, 11, 56, 89, 178, 78, 237, 60, 34, 219, 32, 110, 233, 102, 42, 27, 104,
        116, 116, 112, 115, 58, 47, 47, 101, 120, 97, 109, 112, 108, 101, 46, 99, 111, 109, 47,
        116, 111, 107, 101, 110, 47, 49, 48, 128, 172, 199, 240, 55, 56, 9, 66, 137, 1, 10, 20,
        195, 145, 61, 77, 139, 171, 73, 20, 50, 134, 81, 194, 234, 232, 23, 200, 183, 142, 31, 76,
        10, 20, 101, 208, 138, 5, 108, 23, 174, 19, 55, 5, 101, 176, 76, 247, 125, 42, 250, 28, 185,
        250, 10, 20, 89, 24, 178, 230, 71, 70, 77, 71, 67, 96, 26, 134, 87, 83, 230, 76, 128, 89,
        220, 79, 10, 20, 245, 80, 76, 226, 188, 197, 38, 20, 241, 33, 175, 249, 185, 59, 32, 1, 217,
        39, 21, 202, 10, 20, 253, 206, 66, 17, 111, 84, 31, 200, 247, 176, 119, 110, 43, 48, 131,
        43, 213, 98, 28, 133, 18, 25, 128, 148, 235, 220, 3, 128, 168, 214, 185, 7, 128, 188, 193,
        150, 11, 128, 208, 172, 243, 14, 128, 228, 151, 208, 18
      ]
    )
  }
});
