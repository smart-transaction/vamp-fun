use std::error::Error;

use anyhow::Context;
use sqlx::MySqlPool;
use urlencoding::encode;

#[derive(Clone, Debug)]
pub struct DbConn {
    mysql_host: String,
    mysql_port: u16,
    mysql_user: String,
    mysql_password: String,
    mysql_database: String,
}

impl DbConn {
    pub fn new<T>(
        mysql_host: T,
        mysql_port: u16,
        mysql_user: T,
        mysql_password: T,
        mysql_database: T,
    ) -> Self
    where T: Into<String> {
        Self {
            mysql_host: mysql_host.into(),
            mysql_port,
            mysql_user: mysql_user.into(),
            mysql_password: mysql_password.into(),
            mysql_database: mysql_database.into(),
        }
    }

    pub async fn create_db_conn(&self) -> Result<MySqlPool, Box<dyn Error>> {
        let encoded_password = encode(&self.mysql_password);
        let mysql_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            self.mysql_user,
            encoded_password,
            self.mysql_host,
            self.mysql_port,
            self.mysql_database
        );
        let db_conn = MySqlPool::connect(&mysql_url)
            .await
            .context("connect mysql")?;
        Ok(db_conn)
    }
}
