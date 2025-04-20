import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";
import { SolanaVampProgram } from "../target/types/solana_vamp_program";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";

const TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

describe("solana-vamp-program", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaVampProgram as Program<SolanaVampProgram>;
  const authority = provider.wallet.publicKey;

  it("Initialize Vamp State and mint token", async () => {
    // Find mint account PDA
    const [mintAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint")],
      program.programId
    );

    // Find mint authority PDA
    const [mintAuthority] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint")],
      program.programId
    );

    // Find metadata account PDA
    const [metadataAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mintAccount.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID
    );

    // Find vamp state PDA
    const [vampState] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vamp"), authority.toBuffer()],
      program.programId
    );

    // Create a keypair for the vault
    const vault = await getAssociatedTokenAddress(
      mintAccount,
      authority,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );    

    // Create token mappings
    const tokenMappings = [{
      tokenAddress: new anchor.web3.PublicKey("11111111111111111111111111111111"),
      tokenAmount: new BN(1000000),
      ethAddress: Array.from(Buffer.alloc(20, 1)) // Example Ethereum address
    }];

    // Create vamping data (example buffer)
    const vampingData = Buffer.from("example vamping data");

    try {
      const tx = await program.methods
        .createTokenMint(vampingData, tokenMappings)
        .accounts({
          authority,
          mint_account: mintAccount,
          metadataAccount,
          vampState,
          vault: vault.publicKey,
          mintAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([authority])
        .rpc();

      console.log("Transaction signature", tx);
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });
});
