use std::error::Error;

use log::info;
use mysql::{Pool, PooledConn};
use urlencoding::encode;

pub fn create_db_conn(
    mysql_host: &str,
    mysql_port: &str,
    mysql_user: &str,
    mysql_password: &str,
    mysql_database: &str,
) -> Result<PooledConn, Box<dyn Error>> {
    let encoded_password = encode(mysql_password);
    let mysql_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        mysql_user, encoded_password, mysql_host, mysql_port, mysql_database
    );
    info!("Connecting to mysql with the URL {}", mysql_url);
    let db_conn = Pool::new(mysql_url.as_str())?.get_conn()?;
    info!("CMYSQL connection successful");
    Ok(db_conn)
}
