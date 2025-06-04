use crate::or::storage::Storage;
use crate::proto::chain_selection_proto::Chain;
use crate::proto::orchestrator_service_server::{OrchestratorService, OrchestratorServiceServer};
use crate::proto::{
    AppChainPayloadProto, AppChainResultProto, AppChainResultStatus, ChainSelectionProto,
    LatestBlockHashRequestProto, LatestBlockHashResponseProto, SolanaCluster,
    SubmitSolutionRequestProto, SubmitSolutionResponseProto,
};

use std::fs;

use postcard;
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::transaction::Transaction;
use tonic::{Request, Response, Status, transport::Server};
use tonic_reflection::server::Builder as ReflectionBuilder;

#[derive(Clone)]
pub struct OrchestratorGrpcService {
    storage: Storage,
    solana_devnet_url: String,
    solana_mainnet_url: String,
    solana_default_url: String,
}

const DEVNET: i32 = SolanaCluster::Devnet as i32;
const MAINNET: i32 = SolanaCluster::Mainnet as i32;

impl OrchestratorGrpcService {
    fn get_solana_url(&self, chain: Option<ChainSelectionProto>) -> Result<String, Status> {
        if let Some(chain) = chain {
            if let Some(chain) = chain.chain {
                match chain {
                    Chain::EvmChainId(_) => {
                        return Err(Status::invalid_argument("EVM chain not supported yet"));
                    }
                    Chain::SolanaCluster(cluster) => {
                        match cluster {
                            DEVNET => {
                                return Ok(self.solana_devnet_url.clone());
                            }
                            MAINNET => {
                                return Ok(self.solana_mainnet_url.clone());
                            }
                            _ => {
                                return Err(Status::invalid_argument(format!(
                                    "Unsupported Solana cluster: {}",
                                    cluster
                                )));
                            }
                        }
                    }
                }
            }
        }
        Ok(self.solana_default_url.clone())
    }
}

#[tonic::async_trait]
impl OrchestratorService for OrchestratorGrpcService {
    async fn submit_solution(
        &self,
        request: Request<SubmitSolutionRequestProto>,
    ) -> Result<Response<SubmitSolutionResponseProto>, Status> {
        let metadata = request.metadata();
        log::info!(
            "Incoming gRPC SubmitSolution request: metadata = {:?}, remote_addr = {:?}",
            metadata,
            request.remote_addr()
        );
        let req = request.into_inner();
        log::info!("Request payload: sequence_id = {}", req.request_sequence_id,);

        // Fetch request from storage, only if state is New
        match self
            .storage
            .get_intent_in_state_new(req.request_sequence_id)
            .await
            .map_err(|e| Status::internal(format!("failed to fetch request from redis: {}", e)))?
        {
            Some(_stored_request) => {
                // Forward request data to Solana program
                log::info!(
                    "Submitting solution to Solana program for sequence_id: {}",
                    req.request_sequence_id
                );

                let transaction: Transaction =
                    postcard::from_bytes(&req.transaction).map_err(|e| {
                        Status::internal(format!("Failed to deserialize transaction: {}", e))
                    })?;

                // TODO: Add the chain selection logic here
                let solana_url = self.get_solana_url(req.chain)?;
                let client = RpcClient::new_with_commitment(
                    solana_url.clone(),
                    CommitmentConfig::confirmed(),
                );
                let tx_sig = client
                    .send_and_confirm_transaction(&transaction)
                    .map_err(|e| Status::internal(format!("Failed to send transaction: {}", e)))?;
                log::info!("Transaction submitted: {}", tx_sig);

                // Update state to UnderExecution
                log::info!(
                    "Updating request state to UnderExecution for sequence_id: {}",
                    req.request_sequence_id
                );
                self.storage
                    .update_request_state_to_under_execution(req.request_sequence_id)
                    .await
                    .map_err(|e| {
                        Status::internal(format!("failed to update request state: {}", e))
                    })?;

                Ok(Response::new(SubmitSolutionResponseProto {
                    result: AppChainResultProto {
                        status: AppChainResultStatus::Ok.into(),
                        message: None,
                    }
                    .into(),
                    payload: Some(AppChainPayloadProto {
                        solana_txid: tx_sig.to_string(),
                    }),
                }))
            }
            None => Ok(Response::new(SubmitSolutionResponseProto {
                result: AppChainResultProto {
                    status: AppChainResultStatus::EventNotFound.into(),
                    message: format!(
                        "Request with sequence_id {} is not in 'New' state or does not exist",
                        req.request_sequence_id
                    )
                    .into(),
                }
                .into(),
                payload: None,
            })),
        }
    }

    async fn get_latest_block_hash(
        &self,
        request: Request<LatestBlockHashRequestProto>,
    ) -> Result<Response<LatestBlockHashResponseProto>, Status> {
        let req = request.into_inner();
        // TODO: Add the chain selection logic here
        let client = RpcClient::new_with_commitment(
            self.get_solana_url(req.chain)?,
            CommitmentConfig::confirmed(),
        );
        let blockhash = client
            .get_latest_blockhash()
            .map_err(|e| Status::internal(format!("Failed to get latest blockhash: {}", e)))?;
        Ok(Response::new(LatestBlockHashResponseProto {
            result: Some(AppChainResultProto {
                status: AppChainResultStatus::Ok.into(),
                message: None,
            }),
            block_hash: blockhash.to_bytes().to_vec(),
        }))
    }
}

pub async fn start_grpc_server(storage: Storage, cfg: &config::Config) -> anyhow::Result<()> {
    let addr: String = cfg.get("grpc.address")?;
    let addr = addr.parse()?;
    let solana_devnet_url = cfg.get("solana.devnet_url")?;
    let solana_mainnet_url = cfg.get("solana.mainnet_url")?;
    let solana_default_url = cfg.get("solana.default_url")?;

    let service = OrchestratorGrpcService {
        storage,
        solana_devnet_url,
        solana_mainnet_url,
        solana_default_url,
    };

    log::info!("Reading the proto descriptor");
    let descriptor_bytes = fs::read("src/generated/user_descriptor.pb")?;

    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(&*descriptor_bytes)
        .build_v1()?;

    log::info!("Starting gRPC server on {}", addr);
    Server::builder()
        .add_service(OrchestratorServiceServer::new(service))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
