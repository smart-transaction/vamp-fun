use std::{error::Error, future::Future, marker::Send, sync::Arc};

use futures::StreamExt;
use log::error;
use prost::Message;
use rabbitmq_stream_client::{
    Consumer, Environment,
    error::StreamCreateError,
    types::{ByteCapacity, OffsetSpecification, ResponseCode},
};
use tokio::{spawn, sync::Mutex};

pub struct RabbitMQListener {
    consumer: Consumer,
}

pub trait Handler<T> {
    fn handle(&mut self, event: T) -> impl Future<Output = ()> + Send;
}

/// A listener for RabbitMQ streams. Expects the event data to be an encoded protobuf message.
impl RabbitMQListener {
    pub async fn new(stream_name: &str, consumer_name: &str) -> Result<Self, Box<dyn Error>> {
        let environment = Environment::builder().build().await?;
        let create_response = environment
            .stream_creator()
            .max_length(ByteCapacity::GB(5))
            .create(&stream_name)
            .await;

        if let Err(e) = create_response {
            if let StreamCreateError::Create { stream, status } = e {
                match status {
                    // we can ignore this error because the stream already exists
                    ResponseCode::StreamAlreadyExists => {}
                    err => {
                        println!("Error creating stream: {:?} {:?}", stream, err);
                    }
                }
            }
        }

        let mut consumer = environment
            .consumer()
            .name(&consumer_name)
            .offset(OffsetSpecification::First)
            .build(&stream_name)
            .await?;

        let stored_offset: u64 = consumer.query_offset().await.unwrap_or_else(|_| 0);
        if stored_offset > 0 {
            consumer = environment
                .consumer()
                .name(&consumer_name)
                .offset(OffsetSpecification::Offset(stored_offset + 1))
                .build(&stream_name)
                .await?;
        }

        Ok(Self { consumer })
    }

    /// Listens for events on the stream and calls the handler for each event.
    /// The handler is expected to be a function that takes a single argument of the event type.
    /// The event type is expected to correspond the type E that is specified by the caller.
    pub async fn listen<E: Message + Default + 'static, H: Handler<E> + Send + 'static>(&mut self, handler: Arc<Mutex<H>>) {
        let handle = self.consumer.handle();
        while let Some(delivery) = self.consumer.next().await {
            let d = delivery.unwrap();
            let offset = d.offset();
            let event = E::decode(d.message().data().unwrap());
            if let Err(err) = event {
                error!("Error decoding event: {:?}", err);
                continue;
            }
            let h = handler.clone();
            let event = event.unwrap();
            spawn(async move {
                let mut h = h.lock().await;
                h.handle(event).await;
            });
            let _ = self
                .consumer
                .store_offset(offset)
                .await
                .unwrap_or_else(|e| println!("Err: {}", e));
        }

        handle.close().await.unwrap();
    }
}
