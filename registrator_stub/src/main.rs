use std::env;

mod rr;
mod utils;
mod proto;

use ethers::providers::{Middleware, Provider, Ws};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info"); }
    }
    env_logger::init();

    let cfg = config::Config::builder()
        .add_source(config::File::with_name("config/config.toml"))
        .build()?;

    log::info!("Retrieving the chain_id from: {:?}", cfg.get::<String>("ethereum.rpc_url"));
    let chain_id = fetch_chain_id(&cfg).await?;
    log::info!("Connected chain_id: {}", chain_id);

    let storage = rr::storage::Storage::new(&cfg, chain_id).await?;
    let listener = rr::ethereum_listener::EthereumListener::new(storage.clone(), &cfg).await?;

    tokio::spawn(async move { listener.listen().await.unwrap() });

    rr::rr_grpc_service::start_grpc_server(storage, &cfg).await?;
    Ok(())
}

async fn fetch_chain_id(cfg: &config::Config) -> anyhow::Result<u64> {
    let provider_url: String = cfg.get("ethereum.rpc_url")?;
    let provider = Provider::<Ws>::connect(provider_url).await?;
    let chain_id = provider.get_chainid().await?.as_u64();
    Ok(chain_id)
}
