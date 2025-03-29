use prost::Message as _;
use rabbitmq_stream_client::error::StreamCreateError;
use rabbitmq_stream_client::Environment;
use rabbitmq_stream_client::types::{ByteCapacity, Message, ResponseCode};

mod proto {
  tonic::include_proto!("vamp.fun");
}

use proto::StateSnapshot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment = Environment::builder().build().await?;
    let stream = "StateSnapshot";
    let create_response = environment
        .stream_creator()
        .max_length(ByteCapacity::GB(5))
        .create(stream)
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

    let producer = environment.producer().build(stream).await?;

    let state_snapshot = StateSnapshot{
        accounts: vec![vec![0; 20], vec![1; 20]],
        amounts: vec![vec![0; 32], vec![1; 32]],
    };

    let mut buf: Vec<u8> = Vec::new();
    StateSnapshot::encode(&state_snapshot, &mut buf).unwrap();

    producer
        .send_with_confirm(Message::builder().body(buf).build())
        .await?;
    println!("Sent message to stream: {}", stream);
    producer.close().await?;
    Ok(())
}