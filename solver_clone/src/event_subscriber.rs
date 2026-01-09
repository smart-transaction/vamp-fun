use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use cleanapp_rustlib::rabbitmq::subscriber::{Callback, Message, Subscriber};
use tokio::spawn;
use tracing::{error, info};

use crate::{cfg::Cfg, event_handler::DeployTokenHandler, events::VampTokenIntent};

pub struct SubscriberCallback {
    handler: Arc<DeployTokenHandler>,
}

impl SubscriberCallback {
    pub fn new(handler: Arc<DeployTokenHandler>) -> Self {
        Self { handler }
    }
}

impl Callback for SubscriberCallback {
    fn on_message(&self, message: &Message) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let event: VampTokenIntent = message.unmarshal_to()?;
        info!("Decoded event: {:?}", event);
        let handler = self.handler.clone();
        spawn(async move {
            match handler.handle(event).await {
                Ok(_) => {}
                Err(err) => {
                    error!("Error handling the vamping request: {}", err);
                }
            }
        });
        Ok(())
    }
}

pub struct EventSubscriber {
    cfg: Arc<Cfg>,
    subscriber: Subscriber,
}

/// A RabbitMQ event subscriber. Receives events published by the event listener.
impl EventSubscriber {
    pub async fn new(cfg: Arc<Cfg>) -> Result<Self> {
        let url = Self::amqp_url(&cfg);
        info!(url, "Connecting to RabbitMQ...");
        let subscriber = Subscriber::new(&url, &cfg.exchange_name, &cfg.queue_name).await?;
        info!(url, "Connected to RabbitMQ.");
        Ok(Self { cfg, subscriber })
    }

    /// Listens for events on the stream and calls the handler for each event.
    /// The handler is expected to be a function that takes a single argument of the event type.
    pub async fn listen(&mut self, deploy_token_handler: Arc<DeployTokenHandler>) -> Result<()> {
        let mut callbacks: HashMap<String, Arc<dyn Callback + Send + Sync + 'static>> =
            HashMap::new();

        callbacks.insert(
            self.cfg.routing_key.clone(),
            Arc::new(SubscriberCallback::new(deploy_token_handler)),
        );

        self.subscriber.start(callbacks).await?;

        Ok(())
    }

    fn amqp_url(cfg: &Cfg) -> String {
        format!(
            "amqp://{}:{}@{}:{}",
            cfg.amqp_user, cfg.amqp_password, cfg.amqp_host, cfg.amqp_port
        )
    }
}
