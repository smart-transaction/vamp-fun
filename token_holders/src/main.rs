use ethers::prelude::*;
use ethers::utils::keccak256;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

// // Generate the ERC20 interface (includes Transfer event)
// abigen!(
//     ERC20,
//     r#"[
//         event Transfer(address indexed from, address indexed to, uint256 value)
//     ]"#
// );


// SECONDARY_HTTP_CHAIN_URL="https://sepolia.base.org"
// SECONDARY_ERC20="0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966"

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let provider = Provider::<Http>::try_from("https://sepolia.base.org")?;
    let provider = Provider::<Http>::try_from("https://service.lestnet.org")?;
    println!("Successfully connected to the chain");
    let client = Arc::new(provider);

    // let contract_address: Address = "0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966".parse()?;
    // let contract_address: Address = "0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966".parse()?;
    let contract_address: Address = "0x2D29ee5D409e66482EB5C4FBCaF092CeC4e57A8c".parse()?;

    let mut holders: HashMap<Address, U256> = HashMap::new();

    let blocks_step = 10000;
    let first_block = 0;
    let latest_block = 1145842;

    let event_signature = H256::from_slice(
        &keccak256("Transfer(address,address,uint256)")
    );

    for b in (first_block..latest_block).step_by(blocks_step) {
        // println!("Processing blocks from {} to {}", b, b + blocks_step);
        let filter = Filter::new().from_block(BlockNumber::from(b))
            .to_block(BlockNumber::from(b + blocks_step))
            .topic0(event_signature)
            .address(contract_address);

        let logs = client.get_logs(&filter).await?;
        for log in logs {
            let from = log.topics.get(1).unwrap();
            let to = log.topics.get(2).unwrap();
            let value = U256::from(log.data.0.to_vec().as_slice());
            let from_address = Address::from_slice(&from[12..]);
            let to_address = Address::from_slice(&to[12..]);
            println!("From: {:?}, To: {:?}, Value: {:?}", from_address, to_address, value);
            if from_address != Address::zero() {
                if let Some(v) = holders.get(&from_address) {
                    holders.insert(from_address, v.checked_sub(value).unwrap());
                }
            }
            if to_address != Address::zero() {
                match holders.get(&to_address) {
                    Some(v) => {
                        holders.insert(to_address, v.checked_add(value).unwrap());
                    }
                    None => {
                        holders.insert(to_address, value);
                    }
                }
            }
        }
    }

    println!("{:#?}", holders);

    Ok(())
}
