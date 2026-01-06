use std::error::Error;

use anyhow::Context;
use sqlx::MySqlPool;
use urlencoding::encode;

use crate::args::Args;

pub async fn create_db_conn(cfg: &Args) -> Result<MySqlPool, Box<dyn Error>> {
    let encoded_password = encode(&cfg.mysql_password);
    let mysql_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        cfg.mysql_user,
        encoded_password,
        cfg.mysql_host,
        cfg.mysql_port,
        cfg.mysql_database
    );
    let db_conn = MySqlPool::connect(&mysql_url)
        .await
        .context("connect mysql")?;
    Ok(db_conn)
}
