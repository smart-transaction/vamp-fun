use std::{fmt::Debug, sync::Arc};

use alloy::sol_types::SolEvent;
use alloy_primitives::Log;
use anyhow::Result;
use cleanapp_rustlib::rabbitmq::publisher::Publisher;
use serde::Serialize;
use tracing::info;

use crate::{app_state::AppState, cfg::Cfg};

pub struct EventPublisher {
    publisher: Publisher
}

impl EventPublisher {
    pub async fn new(state: Arc<AppState>, routing_key: &str) -> Result<Self> {
        let amqp_url = Self::amqp_url(&state.cfg);
        Ok(Self {
            publisher: Publisher::new(&amqp_url, &state.cfg.exchange_name, routing_key).await?
        })
    }

    pub async fn publish<Event>(&self, event: &Log) -> Result<()>
    where Event: Debug + SolEvent + Serialize {
        // decoding a raw log -> typed
        let typed = Event::decode_log(event)?;
        let ev = &typed.data;
        info!("Publishing the event: {:?}", ev);

        self.publisher.publish(&ev).await?;

        Ok(())
    }

    fn amqp_url(cfg: &Cfg) -> String {
        format!("amqp://{}:{}@{}:{}", 
            cfg.amqp_user, 
            cfg.amqp_password, 
            cfg.amqp_host, 
            cfg.amqp_port)
    }

}
