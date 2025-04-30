use std::fs;
use crate::proto::{
    request_registrator_service_server::{RequestRegistratorService, RequestRegistratorServiceServer},
    PollRequestProto, PollResponseProto, AppChainResultProto, AppChainResultStatus, UserEventProto,
};
use crate::rr::storage::Storage;
use crate::utils::crypto::calculate_hash;
use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection::server::Builder as ReflectionBuilder;

#[derive(Clone)]
pub struct RRService {
    storage: Storage,
}

#[tonic::async_trait]
impl RequestRegistratorService for RRService {
    async fn poll(
        &self,
        request: Request<PollRequestProto>,
    ) -> Result<Response<PollResponseProto>, Status> {
        let metadata = request.metadata();
        log::info!("Incoming gRPC PollRequest request: metadata = {:?}, remote_addr = {:?}", metadata, request.remote_addr());
        let req = request.into_inner();
        let last_sequence_id = req.last_sequence_id;
        log::info!("Request payload: last_sequence_id = {}", last_sequence_id);

        // Increment to check the next expected sequence id
        let next_sequence_id = last_sequence_id + 1;

        // Try fetching the next event from storage
        match self.storage.get_request_by_sequence_id(&next_sequence_id).await {
            Ok(stored_request) => {
                // Calculate hash of the event
                let request_id = calculate_hash(stored_request.data.as_bytes());

                let user_event: UserEventProto = serde_json::from_str(&stored_request.data)
                    .map_err(|e| Status::internal(format!("Failed to parse UserEventProto from Redis: {}", e)))?;

                Ok(Response::new(PollResponseProto {
                    result: AppChainResultProto {
                        status: AppChainResultStatus::Ok.into(),
                        message: None
                    }.into(),
                    sequence_id: stored_request.sequence_id,
                    event: Some(user_event),
                    request_id: Some(request_id.as_bytes().to_vec()),
                }))
            }
            Err(_) => Ok(Response::new(PollResponseProto {
                result: AppChainResultProto {
                    status: AppChainResultStatus::EventNotFound.into(),
                    message: format!("sequence_id {} does not exist", next_sequence_id).into(),
                }.into(),
                sequence_id: next_sequence_id.into(),
                event: None,
                request_id: None,
            })),
        }
    }
}

pub async fn start_grpc_server(storage: Storage, cfg: &config::Config) -> anyhow::Result<()> {
    let addr: String = cfg.get("grpc.address")?;
    let addr = addr.parse()?;
    let service = RRService { storage };

    log::info!("Readingthe proto descriptor");
    let descriptor_bytes = fs::read("src/generated/user_descriptor.pb")?;

    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(&*descriptor_bytes)
        .build_v1()?;

    log::info!("Starting gRPC server on {}", addr);

    Server::builder()
        .add_service(RequestRegistratorServiceServer::new(service))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
