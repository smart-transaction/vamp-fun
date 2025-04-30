use std::error::Error;

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
    let db_conn = Pool::new(mysql_url.as_str())?.get_conn()?;
    Ok(db_conn)
}
