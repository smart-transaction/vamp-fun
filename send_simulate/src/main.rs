use std::str::FromStr;

use ethers::types::{Address, U256};
use ethers::utils::{keccak256, parse_units};
use prost::Message as _;
use rabbitmq_stream_client::error::StreamCreateError;
use rabbitmq_stream_client::Environment;
use rabbitmq_stream_client::types::{ByteCapacity, Message, ResponseCode};

mod proto {
  tonic::include_proto!("vamp.fun");
}

use proto::{AdditionalDataProto, CallObjectProto, UserEventProto, UserObjectiveProto};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment = Environment::builder().build().await?;
    let stream = "DeployToken";
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

    let amount: U256 = parse_units(100, "ether").unwrap().into();
    let gas: U256 = parse_units(1000000, "gwei").unwrap().into();
    let mut amount_bytes: [u8; 32] = [0; 32];
    amount.to_little_endian(&mut amount_bytes);
    let mut gas_bytes: [u8; 32] = [0; 32];
    gas.to_little_endian(&mut gas_bytes);

    let token_deploy_request = UserEventProto {
        app_id: keccak256("DeployToken".as_bytes()).to_vec(),
        chain_id: 21363,
        block_number: 1153818,
        user_objective: Some(UserObjectiveProto {
            app_id: keccak256("DeployToken".as_bytes()).to_vec(),
            nonse: 1,
            chain_id: 21363,
            call_objects: vec![
                CallObjectProto {
                    id: 1,
                    chain_id: 21363,
                    salt: keccak256("TheSalt".as_bytes()).to_vec(),
                    amount: amount_bytes.to_vec(),
                    gas: gas_bytes.to_vec(),
                    address: Address::from_str("0xF821AdA310c3c7DA23aBEa279bA5Bf22B359A7e1").unwrap().as_bytes().to_vec(),
                    skippable: true,
                    verifiable: true,
                    callvalue: vec![],
                    returnvalue: vec![],
                },
            ],
        }),
        additional_data: vec![
            AdditionalDataProto {
                key: keccak256("ERC20ContractAddress".as_bytes()).to_vec(),
                value: Address::from_str("0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966").unwrap().as_bytes().to_vec(),
            },
        ],
    };

    let mut buf: Vec<u8> = Vec::new();
    UserEventProto::encode(&token_deploy_request, &mut buf).unwrap();

    producer
        .send_with_confirm(Message::builder().body(buf).build())
        .await?;
    println!("Sent message to stream: {}", stream);
    producer.close().await?;
    Ok(())
}