use std::{error::Error, future::Future, marker::Send, sync::Arc, time::{Duration, SystemTime}};

use ethers::utils::keccak256;
use log::{error, info};
use mysql::{Pool, PooledConn, prelude::Queryable};
use tokio::{spawn, sync::Mutex, time::sleep};
use tonic::{Request, transport::Channel};

use crate::{request_handlers::DeployTokenHandler, use_proto::proto::{
    request_registrator_service_client::RequestRegistratorServiceClient, PollRequestProto, UserEventProto
}};

const TICK_FREQUENCY: &str = "500ms";
const DEPLOY_TOKEN_APP_ID: &str = "VampFunDeployToken";
const CLONE_APP_ID: &str = "VampFunClone";

pub struct RequestRegistratorListener {
    client: RequestRegistratorServiceClient<Channel>,
    poll_frequency: Duration,
    mysql_host: String,
    mysql_port: u16,
    mysql_user: String,
    mysql_password: String,
    mysql_database: String,
}

/// A polling client that pings the request registrator for new UserEventProto events.
impl RequestRegistratorListener {
    pub async fn new(
        request_registrator_url: String,
        poll_frequency: Duration,
        mysql_host: String,
        mysql_port: u16,
        mysql_user: String,
        mysql_password: String,
        mysql_database: String,
    ) -> Result<Self, Box<dyn Error>> {
        let client = RequestRegistratorServiceClient::connect(request_registrator_url.clone()).await?;
        info!("Connected to request registrator at {}", request_registrator_url);
        Ok(Self {
            client,
            poll_frequency,
            mysql_host,
            mysql_port,
            mysql_user,
            mysql_password,
            mysql_database,
        })
    }

    /// Listens for events on the stream and calls the handler for each event.
    /// The handler is expected to be a function that takes a single argument of the event type.
    /// The event type is expected to correspond the type E that is specified by the caller.
    pub async fn listen(&mut self, deploy_token_handler: Arc<DeployTokenHandler>) -> Result<(), Box<dyn Error>> {
        let tick_frequency = parse_duration::parse(TICK_FREQUENCY)?;
        let deploy_token_app_id = keccak256(DEPLOY_TOKEN_APP_ID.as_bytes());
        let clone_app_id = keccak256(CLONE_APP_ID.as_bytes());
        let mut last_timestamp = 0u64;
        loop {
            let time_now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
            if time_now.as_secs() == last_timestamp || time_now.as_secs() % self.poll_frequency.as_secs() != 0 {
                sleep(tick_frequency).await;
                continue;
            }
            info!("Poll triggered");
            last_timestamp = time_now.as_secs();
            let last_request_id = self.read_last_request_id()?;
            let mut request_proto = PollRequestProto::default();
            if let Some(last_request_id) = last_request_id {
                request_proto.last_request_id = last_request_id.to_vec();
            }
            let request = Request::new(request_proto);
            let response = self.client.poll(request).await;
            match response {
                Ok(res) => {
                    let response_proto = res.into_inner();
                    info!("Poll response: {:?}", response_proto);
                    if response_proto.is_new_request {
                        let request_id = response_proto.request_id;
                        if let Some(event) = response_proto.event {
                            if event.app_id.as_slice() == deploy_token_app_id {
                                let handler = deploy_token_handler.clone();
                                spawn(async move {
                                    handler.handle(event).await;
                                });
                            }
                            self.write_request_id(request_id)?;
                        } else {
                            error!("Malformed request: the event is None while the is_new_request is true");
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to poll request registrator: {:?}", err);
                }
            }
            
            sleep(tick_frequency).await;
        }
    }

    fn create_db_conn(&self) -> Result<PooledConn, Box<dyn Error>> {
        let mysql_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            self.mysql_user,
            self.mysql_password,
            self.mysql_host,
            self.mysql_port,
            self.mysql_database
        );
        let db_conn = Pool::new(mysql_url.as_str())?.get_conn()?;
        Ok(db_conn)
    }

    fn read_last_request_id(&self) -> Result<Option<[u8; 32]>, Box<dyn Error>> {
        let mut conn = self.create_db_conn()?;

        let stmt = "SELECT user_event_id FROM request_logs ORDER BY ts DESC LIMIT 1";
        let last_request_id: Option<[u8; 32]> = conn.exec_first(stmt, ())?;

        Ok(last_request_id)
    }

    fn write_request_id(
        &self,
        request_id: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        let mut conn = self.create_db_conn()?;

        let stmt = "INSERT INTO request_logs (user_event_id) VALUES (?)";
        conn.exec_drop(stmt, (request_id.as_slice(),))?;

        Ok(())
    }
}
