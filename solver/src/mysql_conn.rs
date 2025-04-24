use std::error::Error;

use mysql::{Pool, PooledConn};

pub fn create_db_conn(
    mysql_host: &str,
    mysql_port: &str,
    mysql_user: &str,
    mysql_password: &str,
    mysql_database: &str,
) -> Result<PooledConn, Box<dyn Error>> {
    let mysql_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        mysql_user, mysql_password, mysql_host, mysql_port, mysql_database
    );
    let db_conn = Pool::new(mysql_url.as_str())?.get_conn()?;
    Ok(db_conn)
}
