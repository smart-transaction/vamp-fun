mod proto {
    tonic::include_proto!("vamp.fun");
}

use ethers::utils::keccak256;
use prost::Message;
use proto::{
    orchestrator_service_server::{
        OrchestratorService, OrchestratorServiceServer,
    },
    AppChainResultProto, AppChainResultStatus, SolverDecisionRequestProto, SolverDecisionResponseProto, TokenVampingInfoProto
};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct MyOrchestratorService {}

impl MyOrchestratorService {
    pub fn new() -> Self {
        Self {}
    }
}

#[tonic::async_trait]
impl OrchestratorService for MyOrchestratorService {
    async fn solver_decision(
        &self,
        request: Request<SolverDecisionRequestProto>,
    ) -> Result<Response<SolverDecisionResponseProto>, Status> {
        let solver_decision_request = request.into_inner();
        if let Some(user_event) = solver_decision_request.event {
            for data in user_event.additional_data {
                if data.key == keccak256("TokenVampingInfo".as_bytes()).to_vec() {
                    match TokenVampingInfoProto::decode(&data.value[..]) {
                        Ok(token_vamping_info) => {
                            println!("Token Vamping Info: {:?}", token_vamping_info);
                        }
                        Err(e) => {
                            return Err(Status::invalid_argument(format!(
                                "Failed to decode TokenVampingInfo: {}",
                                e
                            )));
                        }
                    }
                } else {
                    return Err(Status::invalid_argument("Unknown additional data key"));
                }
            }
        } else {
            return Err(Status::invalid_argument("No event found in request"));
        }

        let result = AppChainResultProto {
            status: AppChainResultStatus::Ok as i32,
            message: None,
        };
        let solver_decision_response = SolverDecisionResponseProto {
            result: Some(result),
        };

        Ok(Response::new(solver_decision_response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50052".parse()?;
    let orchestrator = MyOrchestratorService::new();

    println!("Starting orchestrator simulation server on {}", addr);
    Server::builder()
        .add_service(OrchestratorServiceServer::new(orchestrator))
        .serve(addr)
        .await?;
    Ok(())
}
