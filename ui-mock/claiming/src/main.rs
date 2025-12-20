use anyhow::{anyhow, Result};
use clap::Parser;
use libsecp256k1::{sign, Message, SecretKey};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha3::{Digest, Keccak256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer as SolanaSigner,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token;
use std::str::FromStr;

// Embed IDL for dynamic error decoding
const VAMP_IDL_JSON: &str = include_str!("../../../idls/solana_vamp_program.json");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "devnet")]
    cluster: String,

    #[arg(long)]
    rpc_url: Option<String>,

    #[arg(long, help = "Path to Solana wallet keypair file")]
    solana_wallet: Option<String>,

    #[arg(long, help = "Path to Ethereum private key file")]
    ethereum_wallet: Option<String>,

    #[arg(
        long,
        help = "Path to file containing IPFS balance data (optional if using solver API)"
    )]
    ipfs_balance_file: Option<String>,

    #[arg(long, help = "Mint account address (optional if using solver API)")]
    mint_account_address: Option<String>,

    #[arg(
        long,
        default_value = "https://34-36-3-154.nip.io",
        help = "Solver REST API URL"
    )]
    solver_api_url: String,

    #[arg(long, help = "Token address (ERC20 contract address)")]
    token_address: Option<String>,

    #[arg(long, default_value = "21363", help = "Chain ID")]
    chain_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SolverResponse {
    token_address: String,
    user_address: String,
    amount: String,
    decimals: u8,
    target_txid: String,
    solver_signature: String,
    validator_signature: String,
    mint_account_address: String,
    token_spl_address: String,
    root_intent_cid: String,
    intent_id: String,
}

#[derive(Debug)]
struct ClaimData {
    eth_address: [u8; 20],
    balance: u64,
    solver_signature: [u8; 65],
    validator_signature: [u8; 65],
}

#[allow(dead_code)]
#[derive(Debug)]
struct VampState {
    solver_public_key: Vec<u8>,
    validator_public_key: Vec<u8>,
    intent_id: Vec<u8>,
    total_claimed: u64,
    reserve_balance: u64,
    token_supply: u64,
    curve_exponent: u64,
    sol_vault: Pubkey,
    curve_slope: u64,
    base_price: u64,
    max_price: Option<u64>,
    use_bonding_curve: bool,
    flat_price_per_token: u64,
    paid_claiming_enabled: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Setup Solana client
    let rpc_url = args.rpc_url.unwrap_or_else(|| match args.cluster.as_str() {
        "devnet" => "https://api.devnet.solana.com".to_string(),
        "mainnet" => "https://api.mainnet-beta.solana.com".to_string(),
        "localnet" => "http://localhost:8899".to_string(),
        _ => "https://api.devnet.solana.com".to_string(),
    });

    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Generate or load Solana keypair
    let solana_keypair = if let Some(path) = args.solana_wallet {
        read_keypair_file(&path).map_err(|e| anyhow!("Failed to read Solana wallet file: {}", e))?
    } else {
        Keypair::new() // Generate new keypair
    };

    println!("🔑 Solana wallet: {}", solana_keypair.pubkey());

    // Read Ethereum private key from file
    let eth_private_key_path = args
        .ethereum_wallet
        .unwrap_or_else(|| "eth_private_key.txt".to_string());
    let eth_private_key_hex = std::fs::read_to_string(&eth_private_key_path)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to read Ethereum wallet from {}: {}",
                eth_private_key_path,
                e
            )
        })?
        .trim()
        .to_string();

    let eth_secret_key = SecretKey::parse_slice(&hex::decode(&eth_private_key_hex)?)?;

    // Derive the Ethereum address from the private key
    let public_key = libsecp256k1::PublicKey::from_secret_key(&eth_secret_key);
    let public_key_bytes = public_key.serialize();
    let mut hasher = Keccak256::new();
    hasher.update(&public_key_bytes[1..]); // Skip the first byte (compression flag)
    let hash = hasher.finalize();
    let derived_eth_address = &hash[12..]; // Last 20 bytes

    println!(
        "🔍 Derived Ethereum address from private key: 0x{}",
        hex::encode(derived_eth_address)
    );

    let eth_address_array: [u8; 20] = derived_eth_address
        .try_into()
        .map_err(|_| anyhow!("Invalid eth address length"))?;

    println!(
        "🔍 Using Ethereum address: 0x{}",
        hex::encode(&eth_address_array)
    );

    // Determine mint account address and fetch vamping data
    let (mint_pubkey, vamping_data) = if let Some(mint_addr) = args.mint_account_address {
        // Use provided mint account address
        let mint_pubkey = Pubkey::from_str(&mint_addr)?;
        println!("🔗 Using provided mint account address: {}", mint_pubkey);

        // Fetch vamping data from solver API
        let vamping_data = fetch_vamping_data_from_solver(
            &args.solver_api_url,
            args.chain_id,
            &args.token_address,
            &eth_address_array,
        )?;

        (mint_pubkey, vamping_data)
    } else {
        // Fetch mint account address and vamping data from solver API
        println!("🔗 Fetching mint account address and vamping data from solver API...");
        let vamping_data = fetch_vamping_data_from_solver(
            &args.solver_api_url,
            args.chain_id,
            &args.token_address,
            &eth_address_array,
        )?;
        let mint_pubkey = Pubkey::from_str(&vamping_data.mint_account_address)?;
        println!("🔗 Fetched mint account address: {}", mint_pubkey);

        (mint_pubkey, vamping_data)
    };

    // Fetch IPFS balance data
    let ipfs_data = if let Some(path) = args.ipfs_balance_file {
        // Use provided IPFS balance file
        println!("📁 Reading IPFS balance data from file: {}", path);
        std::fs::read_to_string(&path)
            .map_err(|e| anyhow!("Failed to read IPFS balance file from {}: {}", path, e))?
            .trim()
            .to_string()
    } else {
        // Fetch IPFS balance data dynamically
        println!(
            "🌐 Fetching IPFS balance data from: {}",
            vamping_data.root_intent_cid
        );
        let ipfs_url = format!(
            "https://ipfs.io/ipfs/{}/0x{}.json",
            vamping_data.root_intent_cid,
            hex::encode(&eth_address_array)
        );
        println!("🔗 IPFS URL: {}", ipfs_url);

        let response = reqwest::blocking::get(&ipfs_url)
            .map_err(|e| anyhow!("Failed to fetch IPFS data: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch IPFS data: HTTP {}",
                response.status()
            ));
        }

        response
            .text()
            .map_err(|e| anyhow!("Failed to read IPFS response: {}", e))?
    };

    println!("📡 Fetching VampState for mint: {}", mint_pubkey);
    let vamp_state = fetch_vamp_state(&client, &mint_pubkey)?;

    println!("📋 Parsing IPFS data...");
    let claim_data = parse_ipfs_data(&ipfs_data, &eth_address_array, &vamp_state.intent_id)?;

    // Calculate expected claim cost using on-chain formula
    println!("💰 Calculating expected claim cost...");
    let human_amount = claim_data.balance / 10u64.pow(vamping_data.decimals as u32);
    let expected_cost = calculate_expected_claim_cost(
        human_amount,
        vamp_state.total_claimed,
        vamp_state.curve_slope,
        vamp_state.base_price,
        vamp_state.max_price,
        vamp_state.use_bonding_curve,
        vamp_state.flat_price_per_token,
        vamp_state.paid_claiming_enabled,
    )?;

    println!(
        "💡 Expected claim cost: {} lamports ({:.9} SOL)",
        expected_cost,
        expected_cost as f64 / 1_000_000_000.0
    );

    println!("✍️  Generating ownership signature...");
    let ownership_signature =
        generate_ownership_signature(&eth_secret_key, &claim_data, &vamp_state.intent_id)?;

    // Check SOL balance
    let balance = client.get_balance(&solana_keypair.pubkey())?;
    println!("💰 SOL balance: {} lamports", balance);

    if balance < 50_000_000 {
        println!("⚠️  Insufficient SOL balance. Please manually fund the wallet with SOL.");
        println!("   Wallet address: {}", solana_keypair.pubkey());
        return Err(anyhow!("Insufficient SOL balance"));
    } else {
        println!("✅ Sufficient SOL balance for transaction");
    }

    println!("🚀 Executing claim transaction...");
    let signature = execute_claim_transaction(
        &client,
        &solana_keypair,
        &mint_pubkey,
        &claim_data,
        &ownership_signature,
    )?;

    println!("✅ Claim transaction successful!");
    println!("📝 Transaction signature: {}", signature);

    Ok(())
}

fn fetch_vamping_data_from_solver(
    solver_api_url: &str,
    chain_id: u64,
    token_address: &Option<String>,
    user_address: &[u8; 20],
) -> Result<SolverResponse> {
    let token_addr = token_address
        .as_ref()
        .ok_or_else(|| anyhow!("Token address is required when fetching from solver API"))?;

    let user_addr_hex = hex::encode(user_address);

    let url = format!(
        "{}/get_claim_amount?chain_id={}&token_address={}&user_address=0x{}",
        solver_api_url, chain_id, token_addr, user_addr_hex
    );

    println!("🔗 Fetching vamping data from: {}", url);

    let response =
        reqwest::blocking::get(&url).map_err(|e| anyhow!("Failed to fetch vamping data: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch vamping data: HTTP {}",
            response.status()
        ));
    }

    let vamping_data: SolverResponse = response
        .json()
        .map_err(|e| anyhow!("Failed to parse vamping data: {}", e))?;

    println!("✅ Fetched vamping data:");
    println!("   Token address: {}", vamping_data.token_address);
    println!("   User address: {}", vamping_data.user_address);
    println!("   Amount: {}", vamping_data.amount);
    println!("   Mint account: {}", vamping_data.mint_account_address);
    println!("   Root intent CID: {}", vamping_data.root_intent_cid);

    Ok(vamping_data)
}

fn fetch_vamp_state(client: &RpcClient, mint_pubkey: &Pubkey) -> Result<VampState> {
    let program_id = Pubkey::from_str("FAyBECn6ppQgRwb5R4LryAzNic3XwsCuHakVpD1X7hFW")?;

    // Derive PDA for VampState and fetch directly
    let (vamp_state_pda, _bump) =
        Pubkey::find_program_address(&[b"vamp", mint_pubkey.as_ref()], &program_id);
    let account = client.get_account(&vamp_state_pda)?;

    // Parse account data
    let data = account.data;
    if data.len() < 8 + 1 + 32 {
        return Err(anyhow!("VampState account data too short"));
    }

    // Discriminator check (optional)
    let discriminator = &data[0..8];
    let expected_discriminator = [222, 91, 2, 48, 244, 96, 192, 196];
    if discriminator != expected_discriminator {
        return Err(anyhow!("Invalid VampState discriminator"));
    }

    let mut offset = 8; // Skip discriminator

    // bump
    let _bump = data[offset];
    offset += 1;

    // mint
    let _mint =
        Pubkey::try_from(&data[offset..offset + 32]).map_err(|_| anyhow!("Invalid mint pubkey"))?;
    offset += 32;

    // solver_public_key (Vec<u8>)
    let solver_len = u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| anyhow!("Invalid solver key length"))?,
    ) as usize;
    offset += 4;
    let solver_public_key = data[offset..offset + solver_len].to_vec();
    offset += solver_len;

    // validator_public_key (Vec<u8>)
    let validator_len = u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| anyhow!("Invalid validator key length"))?,
    ) as usize;
    offset += 4;
    let validator_public_key = data[offset..offset + validator_len].to_vec();
    offset += validator_len;

    // vamp_identifier
    let _vamp_identifier = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid vamp_identifier"))?,
    );
    offset += 8;

    // intent_id (Vec<u8>)
    let intent_id_len = u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| anyhow!("Invalid intent_id length"))?,
    ) as usize;
    offset += 4;
    let intent_id = data[offset..offset + intent_id_len].to_vec();
    offset += intent_id_len;

    // total_claimed
    let total_claimed = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid total_claimed"))?,
    );
    offset += 8;

    // reserve_balance
    let reserve_balance = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid reserve_balance"))?,
    );
    offset += 8;

    // token_supply
    let token_supply = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid token_supply"))?,
    );
    offset += 8;

    // curve_exponent
    let curve_exponent = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid curve_exponent"))?,
    );
    offset += 8;

    // sol_vault
    let sol_vault =
        Pubkey::try_from(&data[offset..offset + 32]).map_err(|_| anyhow!("Invalid sol_vault"))?;
    offset += 32;

    // curve_slope
    let curve_slope = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid curve_slope"))?,
    );
    offset += 8;

    // base_price
    let base_price = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid base_price"))?,
    );
    offset += 8;

    // max_price (Option<u64>)
    let max_price = if data[offset] != 0 {
        offset += 1;
        let v = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| anyhow!("Invalid max_price"))?,
        );
        offset += 8;
        Some(v)
    } else {
        offset += 1;
        None
    };

    // use_bonding_curve (bool)
    let use_bonding_curve = data[offset] != 0;
    offset += 1;

    // flat_price_per_token (u64)
    let flat_price_per_token = u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .map_err(|_| anyhow!("Invalid flat_price_per_token"))?,
    );
    offset += 8;

    // paid_claiming_enabled (bool)
    let paid_claiming_enabled = data[offset] != 0;
    // do not advance offset further; not needed and avoids unused assignment warning

    Ok(VampState {
        solver_public_key,
        validator_public_key,
        intent_id,
        total_claimed,
        reserve_balance,
        token_supply,
        curve_exponent,
        sol_vault,
        curve_slope,
        base_price,
        max_price,
        use_bonding_curve,
        flat_price_per_token,
        paid_claiming_enabled,
    })
}

fn parse_ipfs_data(
    ipfs_data: &str,
    eth_address: &[u8; 20],
    _intent_id: &[u8],
) -> Result<ClaimData> {
    let data: Value = serde_json::from_str(ipfs_data)?;

    let balance_str = data["b"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing balance"))?;
    let balance = balance_str.parse::<u64>()?;

    let solver_sig_hex = data["ss"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing solver signature"))?;
    let solver_signature =
        hex::decode(solver_sig_hex.strip_prefix("0x").unwrap_or(solver_sig_hex))?;

    let validator_sig_hex = data["vs"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing validator signature"))?;
    let validator_signature = hex::decode(
        validator_sig_hex
            .strip_prefix("0x")
            .unwrap_or(validator_sig_hex),
    )?;

    if solver_signature.len() != 65 {
        return Err(anyhow!(
            "Invalid solver signature length: {}",
            solver_signature.len()
        ));
    }

    if validator_signature.len() != 65 {
        return Err(anyhow!(
            "Invalid validator signature length: {}",
            validator_signature.len()
        ));
    }

    let solver_signature: [u8; 65] = solver_signature
        .try_into()
        .map_err(|_| anyhow!("Invalid solver signature length"))?;
    let validator_signature: [u8; 65] = validator_signature
        .try_into()
        .map_err(|_| anyhow!("Invalid validator signature length"))?;

    Ok(ClaimData {
        eth_address: *eth_address,
        balance,
        solver_signature,
        validator_signature,
    })
}

fn generate_ownership_signature(
    eth_secret_key: &SecretKey,
    claim_data: &ClaimData,
    intent_id: &[u8],
) -> Result<[u8; 65]> {
    // Create the message hash that needs to be signed
    let mut hasher = Keccak256::new();
    hasher.update(&claim_data.eth_address);
    hasher.update(&claim_data.balance.to_le_bytes()); // Little-endian as expected by Solana
    hasher.update(intent_id);
    let message_hash = hasher.finalize();

    // Add Ethereum message prefix like the Solana program does during verification
    const PREFIX: &str = "\x19Ethereum Signed Message:\n";
    let len = message_hash.len();
    let len_string = len.to_string();

    let mut eth_message = Vec::with_capacity(PREFIX.len() + len_string.len() + message_hash.len());
    eth_message.extend_from_slice(PREFIX.as_bytes());
    eth_message.extend_from_slice(len_string.as_bytes());
    eth_message.extend_from_slice(&message_hash);

    // Hash the message with prefix - this is what the Solana program will hash during verification
    let mut final_hasher = Keccak256::new();
    final_hasher.update(&eth_message);
    let final_message_hash = final_hasher.finalize();

    // Create Ethereum signature format - sign the message with prefix
    let message = Message::parse_slice(&final_message_hash)?;
    let (signature, recovery_id) = sign(&message, eth_secret_key);

    // Convert to Ethereum signature format (r, s, v)
    let mut eth_signature = [0u8; 65];
    eth_signature[0..32].copy_from_slice(&signature.r.b32());
    eth_signature[32..64].copy_from_slice(&signature.s.b32());
    eth_signature[64] = recovery_id.serialize() + 27; // Ethereum format

    Ok(eth_signature)
}

fn execute_claim_transaction(
    client: &RpcClient,
    solana_keypair: &Keypair,
    mint_pubkey: &Pubkey,
    claim_data: &ClaimData,
    ownership_signature: &[u8; 65],
) -> Result<String> {
    let program_id = Pubkey::from_str("FAyBECn6ppQgRwb5R4LryAzNic3XwsCuHakVpD1X7hFW")?;

    // Find VampState PDA
    let (vamp_state_pda, _bump) =
        Pubkey::find_program_address(&[b"vamp", mint_pubkey.as_ref()], &program_id);

    // Find ClaimState PDA
    let (claim_state_pda, _bump) = Pubkey::find_program_address(
        &[b"claim", vamp_state_pda.as_ref(), &claim_data.eth_address],
        &program_id,
    );

    // Find SOL vault PDA
    let (sol_vault_pda, _bump) =
        Pubkey::find_program_address(&[b"sol_vault", mint_pubkey.as_ref()], &program_id);

    // Find vault PDA
    let (vault_pda, _bump) =
        Pubkey::find_program_address(&[b"vault", mint_pubkey.as_ref()], &program_id);

    println!("📋 Account addresses:");
    println!("   VampState: {}", vamp_state_pda);
    println!("   ClaimState: {}", claim_state_pda);
    println!("   SOL Vault: {}", sol_vault_pda);
    println!("   Vault: {}", vault_pda);
    println!("   Mint: {}", mint_pubkey);

    // Create the instruction data for buy_claim_tokens
    let instruction_data = create_claim_instruction_data(claim_data, ownership_signature)?;

    // Create the transaction
    let recent_blockhash = client.get_latest_blockhash()?;

    // Create claimer token account
    let claimer_token_account = get_associated_token_address(&solana_keypair.pubkey(), mint_pubkey);

    // Check if claimer token account exists, if not create it
    let mut instructions = Vec::new();

    if client.get_account(&claimer_token_account).is_err() {
        println!(
            "🏗️  Creating associated token account: {}",
            claimer_token_account
        );
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &solana_keypair.pubkey(),
                &solana_keypair.pubkey(),
                mint_pubkey,
                &spl_token::id(),
            );
        instructions.push(create_ata_ix);
    }

    // Check if ClaimState already exists
    if let Ok(claim_state_account) = client.get_account(&claim_state_pda) {
        println!("⚠️  ClaimState already exists for this address!");
        println!("   ClaimState address: {}", claim_state_pda);
        println!("   Account data length: {}", claim_state_account.data.len());

        // Try to parse the ClaimState to see if it's already claimed
        if claim_state_account.data.len() >= 9 {
            let is_claimed = claim_state_account.data[8] != 0; // Skip 8-byte discriminator
            if is_claimed {
                println!("❌ Tokens already claimed for this address!");
                return Err(anyhow!("Tokens already claimed for this address"));
            } else {
                println!("ℹ️  ClaimState exists but tokens not yet claimed");
            }
        }
    } else {
        println!("✅ ClaimState does not exist yet - will be created");
    }

    let claim_instruction = solana_sdk::instruction::Instruction {
        program_id,
        accounts: vec![
            // Context accounts
            solana_sdk::instruction::AccountMeta::new(solana_keypair.pubkey(), true), // authority
            solana_sdk::instruction::AccountMeta::new(vamp_state_pda, false),         // vamp_state
            solana_sdk::instruction::AccountMeta::new(claim_state_pda, false),        // claim_state
            solana_sdk::instruction::AccountMeta::new(vault_pda, false),              // vault
            solana_sdk::instruction::AccountMeta::new(sol_vault_pda, false),          // sol_vault
            solana_sdk::instruction::AccountMeta::new(claimer_token_account, false), // claimer_token_account
            solana_sdk::instruction::AccountMeta::new_readonly(mint_pubkey.clone(), false), // mint_account
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::id(), false), // token_program
            solana_sdk::instruction::AccountMeta::new_readonly(
                solana_sdk::system_program::id(),
                false,
            ), // system_program
        ],
        data: instruction_data,
    };

    instructions.push(claim_instruction);

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&solana_keypair.pubkey()),
        &[solana_keypair],
        recent_blockhash,
    );

    // Send transaction with better error decoding
    let signature = match client.send_transaction(&transaction) {
        Ok(sig) => sig,
        Err(e) => {
            explain_client_error(e);
            return Err(anyhow!("Transaction send failed"));
        }
    };
    println!("📤 Transaction sent: {}", signature);

    // Wait for confirmation
    match client.confirm_transaction_with_commitment(&signature, CommitmentConfig::confirmed()) {
        Ok(confirmation) => {
            if confirmation.value {
                println!("✅ Transaction confirmed successfully!");
                Ok(signature.to_string())
            } else {
                println!("❌ Transaction failed to confirm");
                Err(anyhow!("Transaction failed to confirm"))
            }
        }
        Err(e) => {
            explain_client_error(e);
            Err(anyhow!("Transaction confirmation failed"))
        }
    }
}

fn create_claim_instruction_data(
    claim_data: &ClaimData,
    ownership_signature: &[u8; 65],
) -> Result<Vec<u8>> {
    // Anchor instruction discriminator for claim (from IDL)
    let discriminator = [62, 198, 214, 193, 213, 159, 108, 210];

    let mut data = Vec::new();
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&claim_data.eth_address);
    data.extend_from_slice(&claim_data.balance.to_le_bytes());
    data.extend_from_slice(&claim_data.solver_signature);
    data.extend_from_slice(&claim_data.validator_signature);
    data.extend_from_slice(ownership_signature);

    Ok(data)
}

fn calculate_expected_claim_cost(
    token_amount: u64,
    total_claimed: u64,
    curve_slope: u64,
    base_price: u64,
    max_price: Option<u64>,
    use_bonding_curve: bool,
    flat_price_per_token: u64,
    paid_claiming_enabled: bool,
) -> Result<u64> {
    if !use_bonding_curve {
        if !paid_claiming_enabled {
            return Ok(0);
        }
        // Mirror on-chain safety cap: min(flat_price_per_token, 1) and total cap 0.1 SOL
        let safe_flat = std::cmp::min(flat_price_per_token, 1);
        let cost = (token_amount as u128)
            .checked_mul(safe_flat as u128)
            .ok_or_else(|| anyhow!("Arithmetic overflow in flat price calculation"))?;
        let capped = std::cmp::min(cost, 100_000_000u128);
        return Ok(capped as u64);
    }

    let x1 = total_claimed as u128;
    let x2 = x1
        .checked_add(token_amount as u128)
        .ok_or_else(|| anyhow!("Arithmetic overflow in token amount addition"))?;

    let delta = x2
        .checked_sub(x1)
        .ok_or_else(|| anyhow!("Arithmetic overflow in delta calculation"))?;

    let part1 = delta
        .checked_mul(curve_slope as u128)
        .ok_or_else(|| anyhow!("Overflow mul slope"))?
        .checked_mul(delta)
        .ok_or_else(|| anyhow!("Overflow delta^2"))?
        .checked_div(100000u128)
        .ok_or_else(|| anyhow!("Div by zero"))?;

    let part2 = delta
        .checked_mul(base_price as u128)
        .ok_or_else(|| anyhow!("Overflow base mul"))?;

    let total = part1
        .checked_add(part2)
        .ok_or_else(|| anyhow!("Overflow total"))?;

    if let Some(_max_price_per_token) = max_price {
        // Client does not pre-assert; on-chain will enforce PriceTooHigh.
        let _avg = total
            .checked_div(delta)
            .ok_or_else(|| anyhow!("Div by zero avg"))?;
    }

    Ok(u64::try_from(total).map_err(|_| anyhow!("Total cost too large for u64"))?)
}

fn explain_client_error<E: std::fmt::Display>(err: E) {
    let s = err.to_string();
    // Look for pattern: custom program error: 0x####
    if let Some(idx) = s.find("custom program error: 0x") {
        let hex_start = idx + "custom program error: 0x".len();
        let hex_code = s[hex_start..]
            .split(|c: char| !c.is_ascii_hexdigit())
            .next()
            .unwrap_or("");
        if let Ok(decimal_code) = u64::from_str_radix(hex_code, 16) {
            if let Ok(idl) = serde_json::from_str::<Value>(VAMP_IDL_JSON) {
                if let Some(errors) = idl.get("errors").and_then(|e| e.as_array()) {
                    for e in errors {
                        if e.get("code").and_then(|c| c.as_u64()) == Some(decimal_code) {
                            let name = e.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            let msg = e.get("msg").and_then(|m| m.as_str()).unwrap_or("");
                            eprintln!("❌ Program error {} ({}): {}", hex_code, name, msg);
                            return;
                        }
                    }
                }
            }
            eprintln!(
                "❌ Program error {} (decimal {}), see IDL for details",
                hex_code, decimal_code
            );
            return;
        }
    }
    eprintln!("❌ {}", s);
}
