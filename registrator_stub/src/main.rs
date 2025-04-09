mod rr;
mod utils;
mod proto;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cfg = config::Config::builder()
        .add_source(config::File::with_name("config/config.toml"))
        .build()?;

    let storage = rr::storage::Storage::new(&cfg).await?;
    let listener = rr::ethereum_listener::EthereumListener::new(storage.clone(), &cfg).await?;

    tokio::spawn(async move { listener.listen().await.unwrap() });

    rr::grpc_service::start_grpc_server(storage, &cfg).await?;
    Ok(())
}
