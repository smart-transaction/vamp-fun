use std::sync::Arc;

use alloy::sol_types::SolEvent;
use alloy_primitives::Log;
use anyhow::Result;
use cleanapp_rustlib::rabbitmq::publisher::Publisher;
use tracing::info;

use crate::{cfg::Cfg, vamper_event::VampTokenIntent};

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

    pub async fn publish(&self, event: &Log) -> Result<()> {
        // decoding a raw log -> typed
        let typed = VampTokenIntent::decode_log(event)?;
        let ev: &VampTokenIntent = &typed.data;
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
