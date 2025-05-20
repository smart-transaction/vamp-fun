use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info"); }
    }
    env_logger::init();

    let _cfg = config::Config::builder()
        .add_source(config::File::with_name("config/config.toml"))
        .build()?;
    Ok(())
}

