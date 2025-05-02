use std::error::Error;

use mysql::{Pool, PooledConn};
use urlencoding::encode;

#[derive(Clone, Debug)]
pub struct DbConn {
    mysql_host: String,
    mysql_port: String,
    mysql_user: String,
    mysql_password: String,
    mysql_database: String,
}

impl DbConn {
    pub fn new(
        mysql_host: String,
        mysql_port: String,
        mysql_user: String,
        mysql_password: String,
        mysql_database: String,
    ) -> Self {
        Self {
            mysql_host,
            mysql_port,
            mysql_user,
            mysql_password,
            mysql_database,
        }
    }

    pub fn create_db_conn(&self) -> Result<PooledConn, Box<dyn Error>> {
        let encoded_password = encode(&self.mysql_password);
        let mysql_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            self.mysql_user, encoded_password, self.mysql_host, self.mysql_port, self.mysql_database
        );
        let db_conn = Pool::new(mysql_url.as_str())?.get_conn()?;
        Ok(db_conn)
    }
}
