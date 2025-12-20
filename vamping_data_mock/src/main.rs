use std::str::FromStr;

use ethers::middleware::signer;
use ethers::signers::Signer;
use ethers::signers::LocalWallet;
use prost::Message;

pub mod vamp_fun {
  include!(concat!(env!("OUT_DIR"), "/vamp.fun.rs"));
}

const SOLVER_PRIVATE_KEY: &str = "9aa4451744ed6f2e3eeee95923c8c5323d86a41315114961e5cabac111719c64";
const VALIDATOR_PRIVATE_KEY: &str = "c9927bc21d1c962ee9a4f0634b49868ab80cb1b3f3522881849a5e81ca21edb0";
const BALANCE_ACCOUNT_PRIVATE_KEY: &str = "fc813315c55817d4fb1396dcf772e1dfa84b8d3328713a6647930ec1983a67cf";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vamping_data = vamp_fun::TokenVampingInfoProto::default();
    vamping_data.token_name = "My Memetoken".to_string();
    vamping_data.token_symbol = "MEME".to_string();
    vamping_data.token_erc20_address = hex::decode("0a0b5506644f9173eca50d1d7d2cace596a5e555")?;
    vamping_data.token_uri = Some("https://example.com/token/1".to_string());
    vamping_data.amount = 312_012_000_000_000;
    vamping_data.decimal = 9;
    vamping_data.chain_id = 1;
    vamping_data.salt = 1234567890;
    vamping_data.solver_public_key = hex::decode("f98b828b389bef4ebbb5911ca17e4f7989c9068d")?;
    vamping_data.validator_public_key = hex::decode("8b25ed06e216553f8d4265996061b0a065afa35c")?;
    vamping_data.intent_id = hex::decode("1111111111111111222222222222222277777777777777779999999999999999")?;

    let mut encoded_vamping_data = Vec::new();
    vamping_data.encode(&mut encoded_vamping_data)?;

    println!("Vamping data mock:");
    println!("==================");
    println!("token_name = My Memetoken");
    println!("token_symbol = MEME");
    println!("token_erc20_address = 0x0a0b5506644f9173eca50d1d7d2cace596a5e555");
    println!("token_uri = https://example.com/token/1");
    println!("amount = 312_012_000_000_000");
    println!("decimal = 9");
    println!("chain_id = 1");
    println!("salt = 1234567890");
    println!("solver_public_key = 0xf98b828b389bef4ebbb5911ca17e4f7989c9068d");
    println!("validator_public_key = 0x8b25ed06e216553f8d4265996061b0a065afa35c");
    println!("intent_id = 0x1111111111111111222222222222222277777777777777779999999999999999");

    println!("Encoded Vamping Info: {:?}", encoded_vamping_data);

    println!("Folded Intent ID: {:?}", fold_intent_id(&vamping_data.intent_id)?);

    let balance_address = "8ebd059f9acef4758a8ac8d6e017d6c76b248c82";
    let balance_amount = 1_000_000_000u64;
    println!("Balance Address: 0x{}", balance_address);
    println!("Balance Amount: {}", balance_amount);

    // Add signatures
    let balance_hash = balance_util::get_balance_hash(
        &hex::decode(balance_address)?,
        balance_amount,
        &vamping_data.intent_id,
    )?;

    println!("Balance hash: {:?}", balance_hash);

    let signer = LocalWallet::from_str(SOLVER_PRIVATE_KEY)?;
    println!("Solver public key: {:?}", signer.address().as_bytes());
    let solver_signature = signer.sign_message(&balance_hash).await?;
    println!("Solver Signature: {:?}", solver_signature.to_vec());

    let signer = LocalWallet::from_str(VALIDATOR_PRIVATE_KEY)?;
    let validator_signature = signer.sign_message(&balance_hash).await?;
    println!("Validator Signature: {:?}", validator_signature.to_vec());

    let signer = LocalWallet::from_str(BALANCE_ACCOUNT_PRIVATE_KEY)?;
    let owner_signature = signer.sign_message(&balance_hash).await?;
    println!("Owner Signature: {:?}", owner_signature.to_vec());

    Ok(())
}

fn fold_intent_id(intent_id: &[u8]) -> Result<u64, Box<dyn std::error::Error>> {
    let mut hash64 = 0u64;
    for chunk in intent_id.chunks(8) {
        let chunk_value = u64::from_le_bytes(chunk.try_into()?);
        hash64 ^= chunk_value; // XOR the chunks to reduce to 64 bits
    }
    Ok(hash64)
}
