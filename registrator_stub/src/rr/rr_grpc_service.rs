use std::fs;
use crate::proto::{
    request_registrator_service_server::{RequestRegistratorService, RequestRegistratorServiceServer},
    PollRequestProto, PollResponseProto, AppChainResultProto, AppChainResultStatus, UserEventProto,
    PollNextAvailableRequestProto, PollNextAvailableResponseProto,
    PushRequestProto, PushResponseProto,
};
use crate::rr::storage::Storage;
use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection::server::Builder as ReflectionBuilder;
use prost::Message;

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
        match self.storage.get_request_by_sequence_id(next_sequence_id).await {
            Ok(stored_request) => {
                // Calculate hash of the event
                let intent_id = stored_request.intent_id;

                let proto_hex = stored_request.proto_data.as_deref()
                    .ok_or_else(|| Status::internal("Missing proto_data for event"))?;
                let proto_bytes = hex::decode(proto_hex)
                    .map_err(|e| Status::internal(format!("Failed to decode hex data from Redis: {}", e)))?;
                let user_event = UserEventProto::decode(&proto_bytes[..])
                    .map_err(|e| Status::internal(format!("Failed to decode UserEventProto from Redis: {}", e)))?;

                Ok(Response::new(PollResponseProto {
                    result: AppChainResultProto {
                        status: AppChainResultStatus::Ok.into(),
                        message: None
                    }.into(),
                    sequence_id: stored_request.sequence_id,
                    event: Some(user_event),
                    intent_id: Some(intent_id.as_bytes().to_vec()),
                }))
            }
            Err(_) => Ok(Response::new(PollResponseProto {
                result: AppChainResultProto {
                    status: AppChainResultStatus::EventNotFound.into(),
                    message: format!("sequence_id {} does not exist", next_sequence_id).into(),
                }.into(),
                sequence_id: next_sequence_id.into(),
                event: None,
                intent_id: None,
            })),
        }
    }

    async fn poll_next_available(
        &self,
        request: Request<PollNextAvailableRequestProto>,
    ) -> Result<Response<PollNextAvailableResponseProto>, Status> {
        let req = request.into_inner();
        let mut seq = req.last_seen_sequence_id + 1;
        let max_scan = 1000u64;
        let mut scanned = 0u64;
        loop {
            if scanned >= max_scan { break; }
            scanned += 1;
            match self.storage.get_request_by_sequence_id(seq).await {
                Ok(stored_request) => {
                    let actionable = matches!(stored_request.state,
                        appchain_core::types::RequestState::New | appchain_core::types::RequestState::Validated);
                    if actionable {
                        let proto_hex = stored_request.proto_data.as_deref()
                            .ok_or_else(|| Status::internal("Missing proto_data for event"))?;
                        let proto_bytes = hex::decode(proto_hex)
                            .map_err(|e| Status::internal(format!("Failed to decode hex data from Redis: {}", e)))?;
                        let user_event = UserEventProto::decode(&proto_bytes[..])
                            .map_err(|e| Status::internal(format!("Failed to decode UserEventProto from Redis: {}", e)))?;
                        return Ok(Response::new(PollNextAvailableResponseProto {
                            result: AppChainResultProto { status: AppChainResultStatus::Ok.into(), message: None }.into(),
                            sequence_id: stored_request.sequence_id,
                            event: Some(user_event),
                            intent_id: Some(stored_request.intent_id.as_bytes().to_vec()),
                        }));
                    }
                }
                Err(_) => { /* gap or beyond tip; continue scanning */ }
            }
            seq += 1;
        }
        Ok(Response::new(PollNextAvailableResponseProto {
            result: AppChainResultProto { status: AppChainResultStatus::EventNotFound.into(), message: None }.into(),
            sequence_id: req.last_seen_sequence_id,
            event: None,
            intent_id: None,
        }))
    }
    async fn push(
        &self,
        request: Request<PushRequestProto>,
    ) -> Result<Response<PushResponseProto>, Status> {
        let metadata = request.metadata();
        log::info!("Incoming gRPC Push request: metadata = {:?}, remote_addr = {:?}", metadata, request.remote_addr());
        let req = request.into_inner();
        let Some(event) = req.event else {
            return Err(Status::invalid_argument("Missing event"));
        };

        // Encode event to bytes and JSON for storage
        let mut proto_bytes = Vec::new();
        event
            .encode(&mut proto_bytes)
            .map_err(|e| Status::internal(format!("Failed to encode event: {}", e)))?;
        let json_string = serde_json::to_string(&event)
            .map_err(|e| Status::internal(format!("Failed to serialize event to JSON: {}", e)))?;

        // Compute sequence id and persist
        let sequence_id = self.storage
            .next_sequence_id()
            .await
            .map_err(|e| Status::internal(format!("Failed to allocate sequence id: {}", e)))?;

        // Use hex-encoded proto bytes
        let intent_id_hex = hex::encode(&event.intent_id);
        self.storage
            .save_new_intent(&intent_id_hex, sequence_id, &json_string, &hex::encode(&proto_bytes))
            .await
            .map_err(|e| Status::internal(format!("Failed to store event: {}", e)))?;

        log::info!("Stored pushed event seq_id={} intent_id={} bytes={}", sequence_id, intent_id_hex, proto_bytes.len());

        Ok(Response::new(PushResponseProto {
            result: AppChainResultProto { status: AppChainResultStatus::Ok.into(), message: None }.into(),
            sequence_id,
        }))
    }
}

pub async fn start_grpc_server(storage: Storage, cfg: &config::Config) -> anyhow::Result<()> {
    let addr: String = cfg.get("grpc.address")?;
    let addr = addr.parse()?;
    let service = RRService { storage };

    log::info!("Reading the proto descriptor");
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
