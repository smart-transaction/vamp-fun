use std::str::FromStr;

use ethers::types::{Address, U256};
use ethers::utils::parse_units;
use prost::Message as _;
use rabbitmq_stream_client::error::StreamCreateError;
use rabbitmq_stream_client::Environment;
use rabbitmq_stream_client::types::{ByteCapacity, Message, ResponseCode};

mod proto {
  tonic::include_proto!("vamp.fun");
}

use proto::StateSnapshotProto;

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

    let accounts = vec![
        Address::from_str("0x730090427eC82db786f3A481a64aBA6792f895f7").unwrap(),
        Address::from_str("0x143Eb26EF67448761a97a12dd4f011Bf4389064e").unwrap(),
        Address::from_str("0x07FcC5862EB168711fb0A8fD259b4318E5b94B1b").unwrap(),
    ];

    let amounts: Vec<U256> = vec![
        parse_units(1, "ether").unwrap().into(),
        parse_units(2, "ether").unwrap().into(),
        parse_units(3, "ether").unwrap().into(),
    ];

    let state_snapshot = StateSnapshotProto{
        accounts: accounts.iter().map(|a| a.as_ref().to_vec()).collect(),
        amounts: amounts.iter().map(|a| {
            let mut bytes: Vec<u8> = vec![0; 32];
            a.to_little_endian(bytes.as_mut_slice());
            bytes
        }).collect(),
    };

    let mut buf: Vec<u8> = Vec::new();
    StateSnapshotProto::encode(&state_snapshot, &mut buf).unwrap();

    producer
        .send_with_confirm(Message::builder().body(buf).build())
        .await?;
    println!("Sent message to stream: {}", stream);
    producer.close().await?;
    Ok(())
}