use std::{env, process};
use ethers::signers::LocalWallet;
use validator_vamp::validator_vamp::config::load_config;
use validator_vamp::validator_vamp::ipfs_service::IpfsService;
use validator_vamp::validator_vamp::storage::Storage;
use validator_vamp::validator_vamp::validator_grpc_service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info"); }
    }
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <config-file-path>", args[0]);
        process::exit(1);
    }

    let config_file_path = &args[1];
    let config = load_config(config_file_path);

    let storage = Storage::new(&config.storage).await?;
    let ipfs_service = IpfsService::new(&config.ipfs);

    let validator_wallet = env::var("VALIDATOR_PRIVATE_KEY")
        .expect("VALIDATOR_PRIVATE_KEY not set")
        .parse::<LocalWallet>()?;

    validator_grpc_service::start_grpc_server(config.clone(),storage,ipfs_service,validator_wallet).await?;

    Ok(())
}
