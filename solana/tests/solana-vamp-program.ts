import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ComputeBudgetProgram, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";
import { SolanaVampProgram } from "../target/types/solana_vamp_program";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";

const TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
const PROGRAM_ID = new PublicKey("AdcKTPCt4egfRT7LryV5WbZajTSDf9Ncb7RavZ6dWLPi");

// Generate a keypair for the mint account (not a PDA)
const mintKeypair = anchor.web3.Keypair.generate();

describe("solana-vamp-project", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.solanaVampProgram as Program<SolanaVampProgram>;
  const authority = provider.wallet.publicKey;
  const salt = new BN(1);

  it("Initializes Vamp State and Mints Token", async () => {
    const mintAccount = mintKeypair.publicKey;

    // Find mint authority PDA
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

    const genericSolution = Buffer.from(
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
    );

    // Find vamp state PDA
    const [vampState, vampStateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vamp"), 
        mintAccount.toBuffer()
      ],
      PROGRAM_ID
    );
    
    // Find vault PDA
    const [vault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), mintAccount.toBuffer()],
      PROGRAM_ID
    );

    console.log("Authority:", authority.toBase58());
    console.log("Mint Account:", mintAccount.toBase58());
    console.log("Metadata Account:", metadataAccount.toBase58());
    console.log("Mint Authority:", mintAuthority.toBase58());
    console.log("Vamp State:", vampState.toBase58());
    console.log("Vault:", vault.toBase58());

    try {
      const tx = await program.methods
        .createTokenMint(genericSolution)
        .accounts({
          authority,
          mintAccount,
          metadataAccount,
          vampState,
          vault,
          mintAuthority,
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

      console.log("Transaction signature:", tx);

      // Fetch and verify the vamp state account
      const vampStateAccount = await program.account.vampState.fetch(vampState);
      console.log("Vamp State Account:", vampStateAccount);

      // Verify the mint account matches
      assert.equal(vampStateAccount.mint.toBase58(), mintAccount.toBase58(), "Mint account mismatch");
      
      // Verify the authority matches
      assert.equal(vampStateAccount.authority.toBase58(), authority.toBase58(), "Authority mismatch");
      
      // Verify the bump matches
      assert.equal(vampStateAccount.bump, vampStateBump, "Bump mismatch");

      // Verify token mappings
      assert.isArray(vampStateAccount.tokenMappings, "Token mappings should be an array");
      console.log("Token Mappings:", vampStateAccount.tokenMappings);
    } catch(error) {
      console.error("Transaction error:", error);
      throw error;
    }
  });
});
