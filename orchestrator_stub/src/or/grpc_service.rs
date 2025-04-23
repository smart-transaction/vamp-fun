use crate::or::solana_orchestrator::SolanaOrchestrator;
use crate::or::storage::Storage;
use crate::proto::orchestrator_service_server::{OrchestratorService, OrchestratorServiceServer};
use crate::proto::{
    AppChainResultProto, AppChainResultStatus, SubmitSolutionRequestProto,
    SubmitSolutionResponseProto,
};
use std::fs;
use tonic::{Request, Response, Status, transport::Server};
use tonic_reflection::server::Builder as ReflectionBuilder;

#[derive(Clone)]
pub struct OrchestratorGrpcService {
    storage: Storage,
}

#[tonic::async_trait]
impl OrchestratorService for OrchestratorGrpcService {
    async fn solver_decision(
        &self,
        request: Request<SubmitSolutionRequestProto>,
    ) -> Result<Response<SubmitSolutionResponseProto>, Status> {
        let req = request.into_inner();

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
                SolanaOrchestrator::submit_to_solana(
                    req.generic_solution,
                    req.token_ers20_address,
                    req.chain_id,
                    req.salt,
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
            })),
        }
    }
}

pub async fn start_grpc_server(storage: Storage, cfg: &config::Config) -> anyhow::Result<()> {
    let addr: String = cfg.get("grpc.address")?;
    let addr = addr.parse()?;

    let service = OrchestratorGrpcService { storage };

    let descriptor_bytes = fs::read("src/generated/user_descriptor.pb")?;

    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(&*descriptor_bytes)
        .build_v1()?;

    Server::builder()
        .add_service(OrchestratorServiceServer::new(service))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
