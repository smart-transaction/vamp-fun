import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";
import { SolanaVampProgram } from "../target/types/solana_vamp_program";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

const METADATA_PROGRAM_ID = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
describe("vamp_project", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.solanaVampProgram as Program<SolanaVampProgram>;

  it("Initializes Vamp State and Mints Token", async () => {
    const authority = Keypair.generate();
    const mintKeypair = Keypair.generate();

    const [vampState] = await PublicKey.findProgramAddressSync(
      [Buffer.from("vamp"), authority.publicKey.toBuffer()],
      program.programId
    );

    console.log("vamp_state:", vampState)
    const [metadataAccount] = await PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        new PublicKey(METADATA_PROGRAM_ID).toBuffer(),
        mintKeypair.publicKey.toBuffer(),
      ],
      new PublicKey(METADATA_PROGRAM_ID)
    );

    const vault = Keypair.generate();

    const genericSolutionBase64 = "CiDQrX3uaRDiQdcmkBz9lbF1rXGcXMzspCC77XFNuuvEVBIKU2hpbSBUb2tlbhoEU0hJTSIUtpplayvoqgs4WbJO7Twi2yBu6WYqFmh0dHBzOi8vdG9rZW4vdXJpL3NoaW0wwLP50P2PowI4CUKgAQoUHEvfYVnaNRvm9CmXZqF6RuE9UMgKFA2COESEMoDqXjBUivBjn3ksKBjPChSeTRFJcg5foBlSAd+XkXXVW0MnjQoUfqfB94BNUQdAy5W1MUkxpFfsi0gKFJzrYLvHLuKAQLpfkpbNyZ7OrY3sEggAvKBlAQAAABIIAMqaOwAAAAASCAAoa+4AAAAAEggAyBeoBAAAABIIAAx3QgMAAAA=";
    const genericSolution = Buffer.from(genericSolutionBase64, "base64");

    // Airdrop to authority (also maybe used for signing/fees)
    await provider.connection.requestAirdrop(authority.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);

    // Airdrop to mint account (which will need rent for mint account + possible transaction fees)
    await provider.connection.requestAirdrop(mintKeypair.publicKey, 1 * anchor.web3.LAMPORTS_PER_SOL);

    // Airdrop to vault
    await provider.connection.requestAirdrop(vault.publicKey, 1 * anchor.web3.LAMPORTS_PER_SOL);

    // Wait for airdrops to finalize
    await new Promise((resolve) => setTimeout(resolve, 2000));

    const tx = await program.methods
      .createTokenMint(genericSolution)
      .accountsPartial({
        authority: authority.publicKey,
        mintAccount: mintKeypair.publicKey,
        metadataAccount,
        vampState,
        vault: vault.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMetadataProgram: new PublicKey(METADATA_PROGRAM_ID),
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([mintKeypair, authority, vault])
      .rpc();

    console.log("Transaction signature:", tx);

    // Optional: Fetch vampState account and assert
    const state = await program.account.vampState.fetch(vampState);
    assert.ok(state.mint.equals(mintKeypair.publicKey));
    assert.ok(state.authority.equals(authority.publicKey));
  });
});
