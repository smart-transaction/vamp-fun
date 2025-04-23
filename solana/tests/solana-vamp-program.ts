import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ComputeBudgetProgram, PublicKey } from "@solana/web3.js";
import { assert } from "chai";
import { SolanaVampProgram } from "../target/types/solana_vamp_program";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import { keccak_256 } from "js-sha3";
import * as protobuf from "protobufjs";

const TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
const PROGRAM_ID = new PublicKey("5zKTcVqXKk1vYGZpK47BvMo8fwtUrofroCdzSK931wVc");
const TokenMappingProto = `
  syntax = "proto3";
  message TokenMappingProto {
    repeated bytes addresses = 1;
    repeated uint64 amounts = 2;
  }

  message TokenVampingInfoProto {
    bytes merkle_root = 1;
    string token_name = 2;
    string token_symbol = 3;
    string token_uri = 4;
    uint64 amount = 5;
    uint32 decimal = 6;
    TokenMappingProto token_mapping = 7;
  }
`;

// Generate a keypair for the mint account (not a PDA)
const mintKeypair = anchor.web3.Keypair.generate();
const mintKeypair2 = anchor.web3.Keypair.generate();

describe("solana-vamp-project", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.solanaVampProgram as Program<SolanaVampProgram>;
  const authority = provider.wallet.publicKey;

  it("Initializes Vamp State and Mints Token", async () => {
    const mintAccount1 = mintKeypair.publicKey;
    console.log("Mint Account 1:", mintAccount1.toBase58());
    const accounts = await setupAccounts(mintAccount1);
    const vampingData = await getVampingData();

    // logAccountInfo(accounts);

    try {
      const tx = await createTokenMintTransaction(accounts, vampingData);
      await verifyVampState(accounts.vampState, accounts.vampStateBump, mintAccount1);
    } catch(error) {
      console.error("Transaction error:", error);
      throw error;
    }

    // create a new mint account for another vamping token
    const mintAccount2 = mintKeypair2.publicKey;
    console.log("Mint Account 2:", mintAccount2.toBase58());
    const accounts2 = await setupAccounts(mintAccount2);
    try {
      // const tx = await createTokenMintTransaction(accounts2, vampingData);
      // await verifyVampState(accounts2.vampState, accounts2.vampStateBump, mintAccount2);
    } catch(error) {
      console.error("Transaction error:", error);
      throw error;
    }
  });

  async function setupAccounts(mintAccount: PublicKey) {
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
  
  function logAccountInfo(accounts: any) {
    console.log("Authority:", authority.toBase58());
    console.log("Mint Account:", mintKeypair.publicKey.toBase58());
    console.log("Metadata Account:", accounts.metadataAccount.toBase58());
    console.log("Mint Authority:", accounts.mintAuthority.toBase58());
    console.log("Vamp State:", accounts.vampState.toBase58());
    console.log("Vault:", accounts.vault.toBase58());
  }

  async function createTokenMintTransaction(accounts: any, genericSolution: Buffer) {
    return await program.methods
      .createTokenMint(genericSolution)
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
    // const root = protobuf.parse(TokenMappingProto).root;
    // const TokenVampingInfoProto = root.lookupType("TokenVampingInfoProto");
    // const TokenMapping = root.lookupType("TokenMappingProto");
  
    // const ethAddresses = [
    //   Buffer.from("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "hex"), // 20 bytes
    //   Buffer.from("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "hex"), // 20 bytes
    // ];
  
    // const amounts = [100, 200];
  
    // // Create merkle root (mocked for now)
    // const leaves = ethAddresses.map((addr, i) => {
    //   const amountBuffer = Buffer.alloc(8);
    //   amountBuffer.writeBigUInt64LE(BigInt(amounts[i]));
    //   return Buffer.from(keccak_256(Buffer.concat([addr, amountBuffer])), 'hex');
    // });
    // const dummyRoot = Buffer.from(keccak_256(Buffer.concat(leaves)), 'hex'); // Mock root
  
    // const tokenMappingPayload = TokenMapping.create({
    //   addresses: ethAddresses,
    //   amounts,
    // });
  
    // const vampingInfoPayload = TokenVampingInfoProto.create({
    //   merkle_root: dummyRoot,
    //   token_name: "MyToken",
    //   token_symbol: "MYT",
    //   token_uri: "https://example.com/token.json",
    //   amount: 1_000_000,
    //   decimal: 9,
    //   token_mapping: tokenMappingPayload,
    // });
  
    // const encoded = TokenVampingInfoProto.encode(vampingInfoPayload).finish();
    // return Buffer.from(encoded);
  }
});
