mod proto;

use prost::Message;
use proto::{
    AppChainResultProto, AppChainResultStatus, SubmitSolutionRequestProto,
    SubmitSolutionResponseProto, TokenVampingInfoProto,
    orchestrator_service_server::{OrchestratorService, OrchestratorServiceServer},
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

fn prepare_and_output_request_proto(
    request: &SubmitSolutionRequestProto,
    token_vamping_info: &TokenVampingInfoProto,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut token_vamping_info = token_vamping_info.clone();
    prepare_and_output_token_vamping_info(&mut token_vamping_info)?;
    let generic_data = token_vamping_info.encode_to_vec();

    let mut out_request = request.clone();
    out_request.generic_solution = generic_data;

    let json_string = serde_json::to_string(&out_request)?;
    println!("Request JSON:\n{}", json_string);
    Ok(())
}

fn prepare_and_output_token_vamping_info(
    token_vamping_info: &mut TokenVampingInfoProto,
) -> Result<(), Box<dyn std::error::Error>> {
    // Truncating the data for demonstration purposes
    let data = &token_vamping_info.token_mapping;
    if let Some(mut data) = data.clone() {
        data.addresses = data.addresses[..5].to_vec();
        data.amounts = data.amounts[..5].to_vec();
        token_vamping_info.token_mapping = Some(data);
    }
    let json_string = serde_json::to_string(&token_vamping_info)?;
    println!("Token Vamping Info JSON:\n{}", json_string);
    Ok(())
}

#[tonic::async_trait]
impl OrchestratorService for MyOrchestratorService {
    async fn solver_decision(
        &self,
        request: Request<SubmitSolutionRequestProto>,
    ) -> Result<Response<SubmitSolutionResponseProto>, Status> {
        let solver_decision_request = request.into_inner();
        let out_solver_decision_request = solver_decision_request.clone();
        match TokenVampingInfoProto::decode(&out_solver_decision_request.generic_solution[..]) {
            Ok(token_vamping_info) => {
                if let Err(err) = prepare_and_output_request_proto(
                    &out_solver_decision_request,
                    &token_vamping_info,
                ) {
                    return Err(Status::internal(format!(
                        "Failed to prepare and output request proto: {}",
                        err
                    )));
                }
            }
            Err(e) => {
                return Err(Status::invalid_argument(format!(
                    "Failed to decode TokenVampingInfo: {}",
                    e
                )));
            }
        }

        let result = AppChainResultProto {
            status: AppChainResultStatus::Ok as i32,
            message: None,
        };
        let solver_decision_response = SubmitSolutionResponseProto {
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
