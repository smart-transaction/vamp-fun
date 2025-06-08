use tonic::{Request, Response, Status};
use crate::proto::{SubmitSolutionForValidationRequestProto, SubmitSolutionForValidationResponseProto, validator_service_server::{ValidatorService}, AppChainResultProto, AppChainResultStatus};
use crate::validator_vamp::storage::Storage;

pub struct ValidatorGrpcService {
    pub storage: Storage,
}

#[tonic::async_trait]
impl ValidatorService for ValidatorGrpcService {
    async fn submit_solution(
        &self,
        request: Request<SubmitSolutionForValidationRequestProto>,
    ) -> Result<Response<SubmitSolutionForValidationResponseProto>, Status> {
        let metadata = request.metadata();
        log::info!(
            "Incoming gRPC SubmitSolution request: metadata = {:?}, remote_addr = {:?}",
            metadata,
            request.remote_addr()
        );
        let req = request.into_inner();
        log::info!("Request payload: intent_id = {}", req.intent_id,);

        // 1. Load from Redis
        // Fetch request from storage, only if state is New
        match self
            .storage
            .get_intent_in_state_new(req.intent_id.as_str())
            .await
            .map_err(|e| Status::internal(format!("failed to fetch request from redis: {}", e)))?
        {
            Some(_stored_request) => {
                // Forward request data to Solana program
                log::info!(
                    "Solution submitted with intent_id: {}",
                    req.intent_id,
                );

                // Merkle checks
                // Perform validation (validator signature) 
                // Upload to IPFS
                // Save new state: Validated(ipfs_cid)
                
                Ok(Response::new(SubmitSolutionForValidationResponseProto {
                    result: AppChainResultProto {
                        status: AppChainResultStatus::Ok.into(),
                        message: None,
                    }
                        .into(),
                    solution_validated_details: vec![],
                }))
            }
            
            None => Ok(Response::new(SubmitSolutionForValidationResponseProto {
                result: AppChainResultProto {
                    status: AppChainResultStatus::EventNotFound.into(),
                    message: format!(
                        "Request with intent_id {} is not in 'New' state or does not exist",
                        req.intent_id
                    )
                        .into(),
                }
                    .into(),
                solution_validated_details: vec![],
            })),
        }

        
    }
}