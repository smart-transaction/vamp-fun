mod proto {
    tonic::include_proto!("vamp.fun");
}

use proto::{
    orchestrator_service_server::{
        OrchestratorService, OrchestratorServiceServer,
    },
    AppChainResultProto, AppChainResultStatus, SolverDecisionRequestProto, SolverDecisionResponseProto,
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
        println!("Received request: {:?}", request);

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
