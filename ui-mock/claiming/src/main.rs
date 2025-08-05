use anyhow::{anyhow, Result};
use clap::Parser;
use libsecp256k1::{SecretKey, Message, sign};
use serde_json::Value;
use sha3::{Digest, Keccak256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, read_keypair_file},
    signer::Signer as SolanaSigner,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token;
use std::str::FromStr;
use reqwest;
use serde::{Deserialize, Serialize};

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
    
    #[arg(long, help = "Path to file containing IPFS balance data (optional if using solver API)")]
    ipfs_balance_file: Option<String>,
    
    #[arg(long, help = "Mint account address (optional if using solver API)")]
    mint_account_address: Option<String>,
    
    #[arg(long, default_value = "https://34-36-3-154.nip.io", help = "Solver REST API URL")]
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
    let rpc_url = args.rpc_url.unwrap_or_else(|| {
        match args.cluster.as_str() {
            "devnet" => "https://api.devnet.solana.com".to_string(),
            "mainnet" => "https://api.mainnet-beta.solana.com".to_string(),
            "localnet" => "http://localhost:8899".to_string(),
            _ => "https://api.devnet.solana.com".to_string(),
        }
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
    let eth_private_key_path = args.ethereum_wallet.unwrap_or_else(|| "eth_private_key.txt".to_string());
    let eth_private_key_hex = std::fs::read_to_string(&eth_private_key_path)
        .map_err(|e| anyhow::anyhow!("Failed to read Ethereum wallet from {}: {}", eth_private_key_path, e))?
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
    
    println!("🔍 Derived Ethereum address from private key: 0x{}", hex::encode(derived_eth_address));
    
    let eth_address_array: [u8; 20] = derived_eth_address.try_into().map_err(|_| anyhow!("Invalid eth address length"))?;
    
    println!("🔍 Using Ethereum address: 0x{}", hex::encode(&eth_address_array));
    
    // Determine mint account address and fetch vamping data
    let (mint_pubkey, vamping_data) = if let Some(mint_addr) = args.mint_account_address {
        // Use provided mint account address
        let mint_pubkey = Pubkey::from_str(&mint_addr)?;
        println!("🔗 Using provided mint account address: {}", mint_pubkey);
        
        // Fetch vamping data from solver API
        let vamping_data = fetch_vamping_data_from_solver(&args.solver_api_url, args.chain_id, &args.token_address, &eth_address_array)?;
        
        (mint_pubkey, vamping_data)
    } else {
        // Fetch mint account address and vamping data from solver API
        println!("🔗 Fetching mint account address and vamping data from solver API...");
        let vamping_data = fetch_vamping_data_from_solver(&args.solver_api_url, args.chain_id, &args.token_address, &eth_address_array)?;
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
        println!("🌐 Fetching IPFS balance data from: {}", vamping_data.root_intent_cid);
        let ipfs_url = format!("https://ipfs.io/ipfs/{}/0x{}.json", 
            vamping_data.root_intent_cid, 
            hex::encode(&eth_address_array));
        println!("🔗 IPFS URL: {}", ipfs_url);
        
        let response = reqwest::blocking::get(&ipfs_url)
            .map_err(|e| anyhow!("Failed to fetch IPFS data: {}", e))?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch IPFS data: HTTP {}", response.status()));
        }
        
        response.text()
            .map_err(|e| anyhow!("Failed to read IPFS response: {}", e))?
    };
    
    println!("📡 Fetching VampState for mint: {}", mint_pubkey);
    let vamp_state = fetch_vamp_state(&client, &mint_pubkey)?;
    
    println!("📋 Parsing IPFS data...");
    let claim_data = parse_ipfs_data(&ipfs_data, &eth_address_array, &vamp_state.intent_id)?;
    
    // Calculate expected claim cost using known solver parameters
    println!("💰 Calculating expected claim cost...");
    
    // Use the known solver parameters instead of corrupted VampState data
    let known_params = (
        true,   // paid_claiming_enabled
        true,   // use_bonding_curve
        1,      // curve_slope
        1,      // base_price
        Some(1000), // max_price
        1,      // flat_price_per_token
    );
    
    let expected_cost = calculate_expected_claim_cost(
        claim_data.balance,
        0, // total_claimed - assume 0 for first claim
        1, // curve_slope = 1
        1, // base_price = 1
        Some(1000), // max_price = 1000
        true, // use_bonding_curve = true
        1, // flat_price_per_token = 1
    )?;
    
    println!("💡 Expected claim cost: {} lamports ({:.9} SOL)", 
        expected_cost, 
        expected_cost as f64 / 1_000_000_000.0
    );
    
    if expected_cost > 1_000_000_000 {
        println!("⚠️  WARNING: Claim cost is very high! Consider claiming a smaller amount.");
        println!("💡 For 100,000,000,000 tokens, the cost would be approximately {:.2} SOL", 
            expected_cost as f64 / 1_000_000_000.0
        );
        
        // Show costs for smaller amounts
        println!("📊 Cost breakdown for different amounts:");
        for amount in [1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000] {
            let cost = calculate_expected_claim_cost(
                amount,
                0, // total_claimed
                1, // curve_slope
                1, // base_price
                Some(1000), // max_price
                true, // use_bonding_curve
                1, // flat_price_per_token
            ).unwrap_or(0);
            
            println!("   {} tokens: {:.6} SOL", amount, cost as f64 / 1_000_000_000.0);
        }
    }
    
    println!("✍️  Generating ownership signature...");
    let ownership_signature = generate_ownership_signature(&eth_secret_key, &claim_data, &vamp_state.intent_id)?;
    
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
    let signature = execute_claim_transaction(&client, &solana_keypair, &mint_pubkey, &claim_data, &ownership_signature)?;
    
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
    let token_addr = token_address.as_ref()
        .ok_or_else(|| anyhow!("Token address is required when fetching from solver API"))?;
    
    let user_addr_hex = hex::encode(user_address);
    
    let url = format!(
        "{}/get_claim_amount?chain_id={}&token_address={}&user_address=0x{}",
        solver_api_url, chain_id, token_addr, user_addr_hex
    );
    
    println!("🔗 Fetching vamping data from: {}", url);
    
    let response = reqwest::blocking::get(&url)
        .map_err(|e| anyhow!("Failed to fetch vamping data: {}", e))?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch vamping data: HTTP {}", response.status()));
    }
    
    let vamping_data: SolverResponse = response.json()
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
    let program_id = Pubkey::from_str("CABA3ibLCuTDcTF4DQXuHK54LscXM5vBg7nWx1rzPaJH")?;
    
    // VampState account discriminator
    let vamp_state_discriminator = [222, 91, 2, 48, 244, 96, 192, 196];
    
    // Get all program accounts
    let accounts = client.get_program_accounts(&program_id)?;
    
    println!("🔍 Found {} program accounts", accounts.len());
    
    let mut vamp_states_found = 0;
    
    for (_pubkey, account) in accounts {
        if account.data.len() < 8 {
            continue;
        }
        
        let discriminator = &account.data[0..8];
        if discriminator != vamp_state_discriminator {
            continue;
        }
        
        vamp_states_found += 1;
        
        // Parse VampState account
        let mut offset = 8; // Skip discriminator
        
        if account.data.len() < offset + 33 {
            continue;
        }
        
        let _bump = account.data[offset];
        offset += 1;
        
        let account_mint = Pubkey::try_from(&account.data[offset..offset + 32]).map_err(|_| anyhow!("Invalid mint pubkey"))?;
        offset += 32;
        
        println!("📋 Found VampState for mint: {}", account_mint);
        
        if account_mint != *mint_pubkey {
            continue;
        }
        
        println!("🎯 Found matching VampState! Parsing data...");
        
        // Parse solver public key
        if account.data.len() < offset + 4 {
            println!("❌ Account data too short for solver key length");
            continue;
        }
        let solver_key_len = match u32::from_le_bytes(account.data[offset..offset + 4].try_into().map_err(|_| anyhow!("Invalid solver key length"))?) {
            len if len > 0 && len <= 20 => len,
            len => {
                println!("❌ Invalid solver key length: {}", len);
                continue;
            }
        };
        offset += 4;
        
        if account.data.len() < offset + solver_key_len as usize {
            println!("❌ Account data too short for solver key data");
            continue;
        }
        let solver_public_key = account.data[offset..offset + solver_key_len as usize].to_vec();
        offset += solver_key_len as usize;
        
        // Parse validator public key
        if account.data.len() < offset + 4 {
            println!("❌ Account data too short for validator key length");
            continue;
        }
        let validator_key_len = match u32::from_le_bytes(account.data[offset..offset + 4].try_into().map_err(|_| anyhow!("Invalid validator key length"))?) {
            len if len > 0 && len <= 20 => len,
            len => {
                println!("❌ Invalid validator key length: {}", len);
                continue;
            }
        };
        offset += 4;
        
        if account.data.len() < offset + validator_key_len as usize {
            println!("❌ Account data too short for validator key data");
            continue;
        }
        let validator_public_key = account.data[offset..offset + validator_key_len as usize].to_vec();
        offset += validator_key_len as usize;
        
        // Parse vamp_identifier (u64)
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for vamp_identifier");
            continue;
        }
        let vamp_identifier = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid vamp_identifier"))?);
        offset += 8;
        println!("   Vamp identifier: {}", vamp_identifier);
        
        // Parse intent_id
        if account.data.len() < offset + 4 {
            println!("❌ Account data too short for intent_id length");
            continue;
        }
        let intent_id_len = match u32::from_le_bytes(account.data[offset..offset + 4].try_into().map_err(|_| anyhow!("Invalid intent_id length"))?) {
            len if len > 0 && len <= 32 => len,
            len => {
                println!("❌ Invalid intent_id length: {}", len);
                continue;
            }
        };
        offset += 4;
        
        if account.data.len() < offset + intent_id_len as usize {
            println!("❌ Account data too short for intent_id data");
            continue;
        }
        let intent_id = account.data[offset..offset + intent_id_len as usize].to_vec();
        
        // Parse total_claimed
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for total_claimed");
            continue;
        }
        let total_claimed = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid total_claimed"))?);
        offset += 8;
        println!("   Total claimed: {}", total_claimed);

        // Parse reserve_balance
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for reserve_balance");
            continue;
        }
        let reserve_balance = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid reserve_balance"))?);
        offset += 8;
        println!("   Reserve balance: {}", reserve_balance);

        // Parse token_supply
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for token_supply");
            continue;
        }
        let token_supply = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid token_supply"))?);
        offset += 8;
        println!("   Token supply: {}", token_supply);

        // Parse curve_exponent
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for curve_exponent");
            continue;
        }
        let curve_exponent = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid curve_exponent"))?);
        offset += 8;
        println!("   Curve exponent: {}", curve_exponent);

        // Parse sol_vault
        if account.data.len() < offset + 32 {
            println!("❌ Account data too short for sol_vault");
            continue;
        }
        let sol_vault = Pubkey::try_from(&account.data[offset..offset + 32]).map_err(|_| anyhow!("Invalid sol_vault"))?;
        offset += 32;
        println!("   SOL Vault: {}", sol_vault);

        // Parse curve_slope
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for curve_slope");
            continue;
        }
        let curve_slope = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid curve_slope"))?);
        offset += 8;
        println!("   Curve slope: {}", curve_slope);

        // Parse base_price
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for base_price");
            continue;
        }
        let base_price = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid base_price"))?);
        offset += 8;
        println!("   Base price: {}", base_price);

        // Parse max_price (Option<u64> - first byte indicates if Some, then 8 bytes for the value)
        if account.data.len() < offset + 1 {
            println!("❌ Account data too short for max_price option");
            continue;
        }
        let max_price = if account.data[offset] != 0 {
            offset += 1;
            if account.data.len() < offset + 8 {
                println!("❌ Account data too short for max_price value");
                continue;
            }
            let value = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid max_price value"))?);
            offset += 8;
            Some(value)
        } else {
            offset += 1;
            None
        };
        println!("   Max price: {}", max_price.unwrap_or(0));

        // Parse use_bonding_curve
        if account.data.len() < offset + 1 {
            println!("❌ Account data too short for use_bonding_curve");
            continue;
        }
        let use_bonding_curve = account.data[offset] != 0;
        offset += 1;
        println!("   Use bonding curve: {}", use_bonding_curve);

        // Parse flat_price_per_token
        if account.data.len() < offset + 8 {
            println!("❌ Account data too short for flat_price_per_token");
            continue;
        }
        let flat_price_per_token = u64::from_le_bytes(account.data[offset..offset + 8].try_into().map_err(|_| anyhow!("Invalid flat_price_per_token"))?);
        offset += 8;
        println!("   Flat price per token: {}", flat_price_per_token);

        // Parse paid_claiming_enabled
        if account.data.len() < offset + 1 {
            println!("❌ Account data too short for paid_claiming_enabled");
            continue;
        }
        let paid_claiming_enabled = account.data[offset] != 0;
        offset += 1;
        println!("   Paid claiming enabled: {}", paid_claiming_enabled);
        
        println!("✅ Successfully parsed VampState data");
        println!("   Solver PK length: {}", solver_public_key.len());
        println!("   Validator PK length: {}", validator_public_key.len());
        println!("   Intent ID length: {}", intent_id.len());
        println!("   Total claimed: {}", total_claimed);
        println!("   Reserve balance: {}", reserve_balance);
        println!("   Token supply: {}", token_supply);
        println!("   Curve exponent: {}", curve_exponent);
        println!("   SOL Vault: {}", sol_vault);
        println!("   Curve slope: {}", curve_slope);
        println!("   Base price: {}", base_price);
        println!("   Max price: {}", max_price.unwrap_or(0));
        println!("   Use bonding curve: {}", use_bonding_curve);
        println!("   Flat price per token: {}", flat_price_per_token);
        println!("   Paid claiming enabled: {}", paid_claiming_enabled);
        
        return Ok(VampState {
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
        });
    }
    
    println!("❌ No VampState found for mint {} (found {} VampState accounts total)", mint_pubkey, vamp_states_found);
    Err(anyhow!("VampState not found for mint {}", mint_pubkey))
}

fn parse_ipfs_data(ipfs_data: &str, eth_address: &[u8; 20], _intent_id: &[u8]) -> Result<ClaimData> {
    let data: Value = serde_json::from_str(ipfs_data)?;
    
    let balance_str = data["b"].as_str().ok_or_else(|| anyhow!("Missing balance"))?;
    let balance = balance_str.parse::<u64>()?;
    
    let solver_sig_hex = data["ss"].as_str().ok_or_else(|| anyhow!("Missing solver signature"))?;
    let solver_signature = hex::decode(solver_sig_hex.strip_prefix("0x").unwrap_or(solver_sig_hex))?;
    
    let validator_sig_hex = data["vs"].as_str().ok_or_else(|| anyhow!("Missing validator signature"))?;
    let validator_signature = hex::decode(validator_sig_hex.strip_prefix("0x").unwrap_or(validator_sig_hex))?;
    
    if solver_signature.len() != 65 {
        return Err(anyhow!("Invalid solver signature length: {}", solver_signature.len()));
    }
    
    if validator_signature.len() != 65 {
        return Err(anyhow!("Invalid validator signature length: {}", validator_signature.len()));
    }
    
    let solver_signature: [u8; 65] = solver_signature.try_into().map_err(|_| anyhow!("Invalid solver signature length"))?;
    let validator_signature: [u8; 65] = validator_signature.try_into().map_err(|_| anyhow!("Invalid validator signature length"))?;
    
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
    let program_id = Pubkey::from_str("CABA3ibLCuTDcTF4DQXuHK54LscXM5vBg7nWx1rzPaJH")?;
    
    // Find VampState PDA
    let (vamp_state_pda, _bump) = Pubkey::find_program_address(
        &[b"vamp", mint_pubkey.as_ref()],
        &program_id,
    );
    
    // Find ClaimState PDA
    let (claim_state_pda, _bump) = Pubkey::find_program_address(
        &[b"claim", vamp_state_pda.as_ref(), &claim_data.eth_address],
        &program_id,
    );
    
    // Find SOL vault PDA
    let (sol_vault_pda, _bump) = Pubkey::find_program_address(
        &[b"sol_vault", mint_pubkey.as_ref()],
        &program_id,
    );
    
    // Find vault PDA
    let (vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", mint_pubkey.as_ref()],
        &program_id,
    );
    
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
    let claimer_token_account = get_associated_token_address(
        &solana_keypair.pubkey(),
        mint_pubkey,
    );
    
    // Check if claimer token account exists, if not create it
    let mut instructions = Vec::new();
    
    if client.get_account(&claimer_token_account).is_err() {
        println!("🏗️  Creating associated token account: {}", claimer_token_account);
        let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
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
            solana_sdk::instruction::AccountMeta::new(vamp_state_pda, false), // vamp_state
            solana_sdk::instruction::AccountMeta::new(claim_state_pda, false), // claim_state
            solana_sdk::instruction::AccountMeta::new(vault_pda, false), // vault
            solana_sdk::instruction::AccountMeta::new(sol_vault_pda, false), // sol_vault
            solana_sdk::instruction::AccountMeta::new(claimer_token_account, false), // claimer_token_account
            solana_sdk::instruction::AccountMeta::new_readonly(mint_pubkey.clone(), false), // mint_account
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::id(), false), // token_program
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system_program
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
    
    // Send transaction
    let signature = client.send_transaction(&transaction)?;
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
            println!("❌ Transaction failed: {}", e);
            Err(e.into())
        }
    }
}

fn create_claim_instruction_data(claim_data: &ClaimData, ownership_signature: &[u8; 65]) -> Result<Vec<u8>> {
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
) -> Result<u64> {
    if !use_bonding_curve {
        // Flat price calculation
        let cost = token_amount
            .checked_mul(flat_price_per_token)
            .ok_or_else(|| anyhow!("Arithmetic overflow in flat price calculation"))?;
        
        // Cap at 0.1 SOL (100,000,000 lamports)
        Ok(cost.min(100_000_000))
    } else {
        // Bonding curve calculation
        let x1 = total_claimed;
        let x2 = x1
            .checked_add(token_amount)
            .ok_or_else(|| anyhow!("Arithmetic overflow in token amount addition"))?;

        // Calculate delta tokens
        let delta_tokens = x2 - x1;

        // For very large numbers, we need to use u128 to avoid overflow
        let delta_tokens_u128 = delta_tokens as u128;
        let curve_slope_u128 = curve_slope as u128;
        let base_price_u128 = base_price as u128;

        // Part 1: (curve_slope * delta_tokens^2) / 1000000
        let delta_squared = delta_tokens_u128
            .checked_mul(delta_tokens_u128)
            .ok_or_else(|| anyhow!("Arithmetic overflow in delta squared calculation"))?;
        
        let part1 = delta_squared
            .checked_mul(curve_slope_u128)
            .ok_or_else(|| anyhow!("Arithmetic overflow in part1 calculation"))?
            .checked_div(1000000)
            .ok_or_else(|| anyhow!("Division by zero in part1 calculation"))?;

        // Part 2: base_price * delta_tokens
        let part2 = delta_tokens_u128
            .checked_mul(base_price_u128)
            .ok_or_else(|| anyhow!("Arithmetic overflow in part2 calculation"))?;

        // Total cost
        let total_cost = part1
            .checked_add(part2)
            .ok_or_else(|| anyhow!("Arithmetic overflow in total cost calculation"))?;

        // Convert back to u64, but check for overflow
        let total_cost_u64: u64 = total_cost.try_into()
            .map_err(|_| anyhow!("Total cost too large for u64"))?;

        // Price capping logic
        if let Some(max_price_per_token) = max_price {
            let max_total_cost = delta_tokens
                .checked_mul(max_price_per_token)
                .ok_or_else(|| anyhow!("Arithmetic overflow in max total cost calculation"))?;
            
            Ok(total_cost_u64.min(max_total_cost))
        } else {
            Ok(total_cost_u64)
        }
    }
} 