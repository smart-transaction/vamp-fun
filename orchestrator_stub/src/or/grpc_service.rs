use crate::or::solana_orchestrator::SolanaOrchestrator;
use crate::or::storage::Storage;
use crate::proto::orchestrator_service_server::{OrchestratorService, OrchestratorServiceServer};
use crate::proto::{
    AppChainPayloadProto, AppChainResultProto, AppChainResultStatus, SubmitSolutionRequestProto,
    SubmitSolutionResponseProto,
};
use std::fs;
use tonic::{Request, Response, Status, transport::Server};
use tonic_reflection::server::Builder as ReflectionBuilder;

#[derive(Clone)]
pub struct OrchestratorGrpcService {
    storage: Storage,
    solana_cluster: String,
    solana_private_key: String,
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
        log::info!(
            "Request payload: sequence_id = {}, solution_len = {}",
            req.request_sequence_id,
            req.generic_solution.len()
        );

        // Fetch request from storage, only if state is New
        match self
            .storage
            .get_new_request(req.request_sequence_id)
            .await
            .map_err(|e| Status::internal(format!("failed to fetch request from redis: {}", e)))?
        {
            Some(_stored_request) => {
                // Forward request data to Solana program
                log::info!(
                    "Submitting solution to Solana program for sequence_id: {}",
                    req.request_sequence_id
                );
                let solana_txid = SolanaOrchestrator::submit_to_solana(
                    req.generic_solution,
                    req.token_ers20_address,
                    self.solana_cluster.clone(),
                    self.solana_private_key.clone(),
                    req.chain_id,
                    req.salt,
                    req.request_id,
                )
                .await
                .map_err(|e| {
                    Status::internal(format!(
                        "Failed to execute solana transaction: \
                    {}",
                        e
                    ))
                })?;

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
                    payload: Some(AppChainPayloadProto { solana_txid }),
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
}

pub async fn start_grpc_server(
    storage: Storage,
    cfg: &config::Config,
    solana_private_key: String,
) -> anyhow::Result<()> {
    let addr: String = cfg.get("grpc.address")?;
    let addr = addr.parse()?;
    let solana_cluster = cfg.get("solana.cluster")?;

    let service = OrchestratorGrpcService {
        storage,
        solana_cluster,
        solana_private_key,
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
