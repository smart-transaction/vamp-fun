use std::str::FromStr;

use clap::Parser;
use ethers::types::{Address, U256};
use ethers::utils::{keccak256, parse_units};
use prost::Message as _;

mod proto {
    tonic::include_proto!("vamp.fun");
}

use proto::{
    AdditionalDataProto, CallObjectProto, PollRequestProto, PollResponseProto, UserEventProto,
    UserObjectiveProto,
    request_registrator_service_server::{
        RequestRegistratorService, RequestRegistratorServiceServer,
    },
};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct MyRequestRegistratorService {
    start_seq_id: u64,
}

const VAMPING_APP_ID: &str = "VampFunVamping";
const CONTRACT_ADDRESS_NAME: &str = "ERC20ContractAddress";

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub start_sequence_id: u64,
}

impl MyRequestRegistratorService {
    pub fn new(start_sequence_id: u64) -> Self {
        Self {
            start_seq_id: start_sequence_id,
        }
    }
}

#[tonic::async_trait]
impl RequestRegistratorService for MyRequestRegistratorService {
    async fn poll(
        &self,
        request: Request<PollRequestProto>,
    ) -> Result<Response<PollResponseProto>, Status> {
        println!("Received request: {:?}", request);
        let amount: U256 = parse_units(200, "ether").unwrap().into();
        let gas: U256 = parse_units(1000000, "gwei").unwrap().into();
        let mut amount_bytes: [u8; 32] = [0; 32];
        amount.to_little_endian(&mut amount_bytes);
        let mut gas_bytes: [u8; 32] = [0; 32];
        gas.to_little_endian(&mut gas_bytes);

        let token_deploy_event = UserEventProto {
            app_id: keccak256(VAMPING_APP_ID.as_bytes()).to_vec(),
            chain_id: 84532,
            block_number: 24221560,
            user_objective: Some(UserObjectiveProto {
                app_id: keccak256(VAMPING_APP_ID.as_bytes()).to_vec(),
                nonse: 1,
                chain_id: 84532,
                call_objects: vec![CallObjectProto {
                    id: 1,
                    chain_id: 84532,
                    salt: keccak256("TheSalt".as_bytes()).to_vec(),
                    amount: amount_bytes.to_vec(),
                    gas: gas_bytes.to_vec(),
                    address: Address::from_str("0xF821AdA310c3c7DA23aBEa279bA5Bf22B359A7e1")
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                    skippable: true,
                    verifiable: true,
                    callvalue: vec![],
                    returnvalue: vec![],
                }],
            }),
            additional_data: vec![AdditionalDataProto {
                key: keccak256(CONTRACT_ADDRESS_NAME.as_bytes()).to_vec(),
                value: Address::from_str("0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966")
                    .unwrap()
                    .as_bytes()
                    .to_vec(),
            }],
        };

        let mut poll_response = PollResponseProto::default();

        let mut token_deploy_buff = Vec::new();
        UserEventProto::encode(&token_deploy_event, &mut token_deploy_buff).unwrap();

        let request_id = keccak256(&token_deploy_buff);
        poll_response.sequence_id = self.start_seq_id;
        poll_response.request_id = request_id.to_vec();
        poll_response.event = Some(token_deploy_event);

        Ok(Response::new(poll_response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr = "[::1]:50051".parse()?;
    let request_registrator = MyRequestRegistratorService::new(args.start_sequence_id);

    Server::builder()
        .add_service(RequestRegistratorServiceServer::new(request_registrator))
        .serve(addr)
        .await?;
    Ok(())
}
