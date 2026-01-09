use std::{fmt::Debug, sync::Arc};

use alloy::sol_types::SolEvent;
use alloy_primitives::Log;
use anyhow::Result;
use cleanapp_rustlib::rabbitmq::publisher::Publisher;
use serde::Serialize;
use tracing::info;

use crate::cfg::Cfg;

pub struct EventPublisher {
    publisher: Publisher
}

impl EventPublisher {
    pub async fn new(cfg: Arc<Cfg>) -> Result<Self> {
        let amqp_url = Self::amqp_url(&cfg);
        Ok(Self {
            publisher: Publisher::new(&amqp_url, &cfg.exchange_name, &cfg.routing_key).await?
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
