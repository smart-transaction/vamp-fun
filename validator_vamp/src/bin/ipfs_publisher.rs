use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

use clap::Parser;
use rand::{Rng, RngCore};
use reqwest::multipart::{Form, Part};
use serde::Serialize;
use sha3::{Digest, Keccak256};
use walkdir::WalkDir;

#[derive(Serialize)]
struct BalanceEntry {
    balance: String,
    solver_individual_balance_sig: String,
    validator_individual_balance_sig: String,
}

#[derive(Parser, Debug)]
#[command(name = "IPFS Publisher")]
struct Args {
    #[arg(long, default_value_t = false)]
    publish: bool,

    #[arg(long, default_value_t = false)]
    pin: bool,

    #[arg(long, default_value_t = false)]
    mfs: bool,

    #[arg(long)]
    ipfs_url: String,
}

fn generate_intent_id(
    source_chain_id: u64,
    timestamp: u64,
    block_number: u64,
    sequence_counter: u64,
) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(source_chain_id.to_be_bytes());
    hasher.update(timestamp.to_be_bytes());
    hasher.update(block_number.to_be_bytes());
    hasher.update(sequence_counter.to_be_bytes());
    let result = hasher.finalize();
    format!("vamp-fun-hbe-by-int-{}", hex::encode(result))
}

fn random_eth_address() -> String {
    let mut buf = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut buf);
    format!("0x{}", hex::encode(buf))
}

fn random_signature() -> String {
    let mut buf = [0u8; 65];
    rand::thread_rng().fill_bytes(&mut buf);
    format!("0x{}", hex::encode(buf))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let source_chain_id = 1;
    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
    let block_number = rand::thread_rng().gen_range(0..12345678);
    let sequence_counter = rand::thread_rng().gen_range(0..10000);

    let intent_id = generate_intent_id(source_chain_id, timestamp, block_number, sequence_counter);
    let base_dir = PathBuf::from("ipfs-root").join(&intent_id);
    create_dir_all(&base_dir)?;

    let mut entries: HashMap<String, BalanceEntry> = HashMap::new();
    for _ in 0..3 {
        let address = random_eth_address();
        let balance: u128 = rand::thread_rng().gen_range(0..=1_000_000_000_000_000_000_000u128);
        entries.insert(
            address.clone(),
            BalanceEntry {
                balance: balance.to_string(),
                solver_individual_balance_sig: random_signature(),
                validator_individual_balance_sig: random_signature(),
            },
        );
    }

    for (addr, entry) in entries.iter() {
        let path = base_dir.join(format!("{addr}.json"));
        let json = serde_json::to_string_pretty(entry)?;
        let mut file = File::create(&path)?;
        file.write_all(json.as_bytes())?;
    }

    println!("Generated folder: {}", intent_id);
    println!("Folder contents:");
    for entry in WalkDir::new(&base_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        println!(" - {}", entry.path().display());
    }

    if args.publish {
        let client = reqwest::Client::new();
        let pin_flag = if args.pin { "true" } else { "false" };

        let ipfs_api = format!(
            "{}/api/v0/add?pin={}&wrap-with-directory=true&stream-channels=true",
            args.ipfs_url, pin_flag
        );

        println!("[HTTP] POST {}", ipfs_api);

        let mut form = Form::new();
        for entry in WalkDir::new(&base_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let rel_path = entry
                .path()
                .strip_prefix(&base_dir)?
                .to_string_lossy()
                .replace("\\", "/");

            let content = std::fs::read(entry.path())?;
            let part = Part::bytes(content).file_name(rel_path);
            form = form.part("file", part);
        }

        let res = client.post(&ipfs_api).multipart(form).send().await?;
        let text = res.text().await?;
        println!("Uploaded:\n{}", text);

        // We don't really need mounting in MFS for our vamp tasks. 
        // Just to be more visible on our local node.
        // Though later on we may theoretically propagate this MFS with IPNS
        if args.mfs {
            if let Some(cid_line) = text.lines().rev().find(|line| line.contains("\"Hash\"")) {
                if let Some(cid) = cid_line.split("\"Hash\":\"").nth(1).and_then(|s| s.split('\"').next()) {
                    let mfs_cp_url = format!(
                        "{}/api/v0/files/cp?arg=/ipfs/{}&arg=/{}",
                        args.ipfs_url, cid, intent_id
                    );
                    println!("[HTTP] POST {}", mfs_cp_url);
                    let mfs_res = client.post(&mfs_cp_url).send().await?;
                    if mfs_res.status().is_success() {
                        println!("Mounted in MFS at /{}", intent_id);
                    } else {
                        println!("Failed to mount in MFS: {}", mfs_res.text().await?);
                    }
                } else {
                    println!("Failed to extract CID from response");
                }
            } else {
                println!("No valid Hash line found in response");
            }
        }
    }

    Ok(())
}
