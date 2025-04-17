use std::env;

mod or;
mod proto;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::var("RUST_LOG").is_err() {
        // unsafe { env::set_var("RUST_LOG", "debug"); }
        unsafe { env::set_var("RUST_LOG", "info"); }
    }
    env_logger::init();

    let cfg = config::Config::builder()
        .add_source(config::File::with_name("config/orchestrator.toml"))
        .build()?;

    let storage = or::storage::Storage::new(&cfg).await?;

    or::grpc_service::start_grpc_server(storage, &cfg).await?;
    Ok(())
}
