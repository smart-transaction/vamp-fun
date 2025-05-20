use std::{
    error::Error,
    sync::Arc,
    time::{Duration, SystemTime},
};

use ethers::utils::keccak256;
use log::{error, info};
use mysql::prelude::Queryable;
use tokio::time::sleep;
use tonic::{Request, transport::Channel};

use crate::{
    mysql_conn::DbConn,
    request_handler::DeployTokenHandler,
    use_proto::proto::{
        AppChainResultStatus, PollRequestProto,
        request_registrator_service_client::RequestRegistratorServiceClient,
    },
};

const TICK_FREQUENCY: &str = "500ms";
pub const VAMPING_APP_ID: &str = "VampFunVamping";

pub struct RequestRegistratorListener {
    client: RequestRegistratorServiceClient<Channel>,
    poll_frequency: Duration,
    db_conn: DbConn,
}

/// A polling client that pings the request registrator for new UserEventProto events.
impl RequestRegistratorListener {
    pub async fn new(
        request_registrator_url: String,
        poll_frequency: Duration,
        db_conn: DbConn,
    ) -> Result<Self, Box<dyn Error>> {
        info!(
            "Connecting to request registrator at {}",
            request_registrator_url
        );
        let client =
            RequestRegistratorServiceClient::connect(request_registrator_url.clone()).await?;
        info!(
            "Connected successfully to request registrator at {}",
            request_registrator_url
        );
        Ok(Self {
            client,
            poll_frequency,
            db_conn,
        })
    }

    /// Listens for events on the stream and calls the handler for each event.
    /// The handler is expected to be a function that takes a single argument of the event type.
    pub async fn listen(
        &mut self,
        deploy_token_handler: Arc<DeployTokenHandler>,
    ) -> Result<(), Box<dyn Error>> {
        let tick_frequency = parse_duration::parse(TICK_FREQUENCY)?;
        let vamping_app_id = keccak256(VAMPING_APP_ID.as_bytes());
        let mut last_timestamp = 0u64;
        loop {
            sleep(tick_frequency).await;
            let time_now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
            if time_now.as_secs() == last_timestamp
                || time_now.as_secs() % self.poll_frequency.as_secs() != 0
            {
                continue;
            }
            last_timestamp = time_now.as_secs();
            let ids = self.read_last_sequence_id()?;
            let mut request_proto = PollRequestProto::default();
            if let Some(last_id) = ids {
                request_proto.last_sequence_id = last_id;
            }
            let last_sequence_id = request_proto.last_sequence_id;
            let request = Request::new(request_proto);
            let response = self.client.poll(request).await;
            if let Err(err) = response {
                error!("Failed to send request: {:?}", err);
                continue;
            }
            let res = response.unwrap();
            let response_proto = res.into_inner();
            if let Some(result) = response_proto.result {
                if let Err(err) = AppChainResultStatus::try_from(result.status) {
                    error!("Failed to parse result status: {:?}", err);
                    continue;
                }
                let status = AppChainResultStatus::try_from(result.status).unwrap();
                match status {
                    AppChainResultStatus::Ok => {
                        let sequence_id = response_proto.sequence_id;
                        if last_sequence_id >= sequence_id {
                            // The message was already received, skipping it
                            continue;
                        }
                        info!("Received new event with sequence ID: {}", sequence_id);
                        if let None = response_proto.event {
                            error!("Malformed request: the event is None");
                            continue;
                        }
                        let event = response_proto.event.unwrap();
                        if let None = event.user_objective {
                            error!("Malformed request: the user objective is None");
                            continue;
                        }
                        let event_to_handle = event.clone();
                        let user_objective = event.user_objective.unwrap();
                        if user_objective.app_id.as_slice() == vamping_app_id {
                            let handler = deploy_token_handler.clone();
                            if let Err(err) = handler.handle(sequence_id, event_to_handle).await {
                                error!("Failed to handle event: {:?}", err);
                            }
                        }
                        self.write_sequence_id(sequence_id)?;
                    }
                    AppChainResultStatus::EventNotFound => {
                        // No new event, just skip
                        continue;
                    }
                    AppChainResultStatus::Error => {
                        error!("Request failed with result: {:?}", result);
                        continue;
                    }
                }
            }
        }
    }

    fn read_last_sequence_id(&self) -> Result<Option<u64>, Box<dyn Error>> {
        let mut conn = self.db_conn.create_db_conn()?;

        let stmt = "SELECT sequence_id FROM request_logs ORDER BY ts DESC LIMIT 1";
        let seq_id: Option<u64> = conn.exec_first(stmt, ())?;

        Ok(seq_id)
    }

    fn write_sequence_id(&self, sequence_id: u64) -> Result<(), Box<dyn Error>> {
        let mut conn = self.db_conn.create_db_conn()?;

        let stmt = "INSERT INTO request_logs (sequence_id) VALUES (?)";
        conn.exec_drop(stmt, (sequence_id,))?;

        Ok(())
    }
}
