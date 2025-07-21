use crate::validator_vamp::config;
use std::collections::HashMap;
use std::fs;
use ethers::core::k256::sha2::Digest;
use ethers::signers::{LocalWallet, Signer};
use prost::Message;
use serde_json::json;
use sha3::Keccak256;
use tonic::{Request, Response, Status};
use tonic::transport::Server;
use crate::proto::{SubmitSolutionForValidationRequestProto, SubmitSolutionForValidationResponseProto, validator_service_server::{ValidatorService}, AppChainResultProto, AppChainResultStatus};
use crate::proto::{VampSolutionForValidationProto, VampSolutionValidatedDetailsProto};
use crate::proto::validator_service_server::ValidatorServiceServer;
use crate::validator_vamp::ipfs_service::IpfsService;
use crate::validator_vamp::storage::Storage;
use tonic_reflection::server::Builder as ReflectionBuilder;

pub struct ValidatorGrpcService {
    pub storage: Storage,
    pub ipfs_service: IpfsService,
    pub validator_wallet: LocalWallet,
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

        let mut solution = VampSolutionForValidationProto::decode(req.solution_for_validation.as_slice())
            .map_err(|e| {
                log::warn!("Protobuf decode error for intent_id: {} - {}", req.intent_id, e);
                Status::internal(format!("Protobuf decode error: {e}"))
            })?;

        // Load from Redis
        // Fetch request from storage, only if state is New
        match self
            .storage
            .get_intent_in_state_new(req.intent_id.as_str())
            .await
            .map_err(|e| {
                log::warn!("Redis fetch error for intent_id: {} - {}", req.intent_id, e);
                Status::internal(format!("failed to fetch request from redis: {}", e))
            })?
        {
            Some(_stored_request) => {
                // Forward request data to Solana program
                log::info!(
                    "Solution submitted with intent_id: {}",
                    req.intent_id,
                );

                // Sign each entry with validator key
                for (addr, entry) in solution.individual_balance_entry_by_oth_address.iter_mut() {
                    let mut hasher = Keccak256::new();
                    
                    // Parse the Ethereum address from hex string to raw bytes
                    let eth_address = hex::decode(addr.strip_prefix("0x").unwrap_or(addr))
                        .map_err(|e| {
                            log::warn!("Invalid Ethereum address format for intent_id: {} - {}", req.intent_id, e);
                            Status::internal(format!("Invalid Ethereum address format: {e}"))
                        })?;
                    
                    // Parse the intent_id from hex string to raw bytes
                    let intent_id_bytes = hex::decode(req.intent_id.strip_prefix("0x").unwrap_or(&req.intent_id))
                        .map_err(|e| {
                            log::warn!("Invalid intent_id format for intent_id: {} - {}", req.intent_id, e);
                            Status::internal(format!("Invalid intent_id format: {e}"))
                        })?;
                    
                    // Use the same message format as the Solana program
                    hasher.update(&eth_address);
                    hasher.update(&entry.balance.to_le_bytes());
                    hasher.update(&intent_id_bytes);
                    let message_hash = hasher.finalize();
                    
                    // Add Ethereum message prefix like the Solana program does during verification
                    const PREFIX: &str = "\x19Ethereum Signed Message:\n";
                    let len = message_hash.len();
                    let len_string = len.to_string();
                    
                    let mut eth_message = Vec::with_capacity(PREFIX.len() + len_string.len() + message_hash.len());
                    eth_message.extend_from_slice(PREFIX.as_bytes());
                    eth_message.extend_from_slice(len_string.as_bytes());
                    eth_message.extend_from_slice(&message_hash);
                    
                    // Hash the message with prefix - this is what the Solana program will hash during verification
                    let mut final_hasher = sha3::Keccak256::new();
                    final_hasher.update(&eth_message);
                    let final_message_hash = final_hasher.finalize();
                    
                    let sig = self.validator_wallet.sign_hash(ethers::types::H256::from_slice(&final_message_hash))
                        .map_err(|e| {
                            log::warn!("Signing error for intent_id: {} - {}", req.intent_id, e);
                            Status::internal(format!("Signing error: {e}"))
                        })?;
                    entry.validator_individual_balance_sig = hex::encode(sig.to_vec());
                }

                // Serialize individual entries to minimized JSON
                let mut entry_by_oth_address = HashMap::new();
                for (addr, entry) in &solution.individual_balance_entry_by_oth_address {
                    let entry_json = json!({
                "b": entry.balance.to_string(),
                "ss": entry.solver_individual_balance_sig,
                "vs": entry.validator_individual_balance_sig,
                    });
                    let entry_str = serde_json::to_string(&entry_json)
                        .map_err(|e| {
                            log::warn!("JSON encode error for intent_id: {} - {}", req.intent_id, e);
                            Status::internal(format!("Entry JSON encode error: {e}"))
                        })?;
                    entry_by_oth_address.insert(addr.clone(), entry_str);
                }

                // Publish full directory and get root CID and per-address CIDs
                let intent_path = format!("vamp-fun-hbe-by-int-{}", req.intent_id);
                let (root_cid, cid_by_oth_address) = self.ipfs_service.publish_balance_map
                (&intent_path, 
                                                                            &entry_by_oth_address).await
                    .map_err(|e| {
                        log::warn!("IPFS publish error for intent_id: {} - {}", req.intent_id, e);
                        Status::internal(format!("IPFS publish map error: {e}"))
                    })?;

                //  Respond with validated details
                let validated_details = VampSolutionValidatedDetailsProto {
                    root_intent_cid: root_cid.clone(),
                    cid_by_oth_address,
                    validator_address: format!("{:#x}", self.validator_wallet.address()),
                };

                let mut validated_details_bytes = Vec::with_capacity(validated_details.encoded_len());
                validated_details.encode(&mut validated_details_bytes)
                    .map_err(|e| {
                        log::warn!("Protobuf encode error for intent_id: {} - {}", req.intent_id, e);
                        Status::internal(format!("Validated details encode error: {e}"))
                    })?;

                
                // Update lifecycle stage to Validated
                self.storage.update_request_state_to_validated(&req.intent_id, &solution
                    .solver_pubkey, &root_cid).await
                    .map_err(|e| {
                        log::warn!("Storage update error for intent_id: {} - {}", req.intent_id, e);
                        Status::internal(format!("Storage update error: {e}"))
                    })?;
                
                log::info!("Validation successful for intent_id: {}, Root CID: {}", req.intent_id, root_cid);
                
                Ok(Response::new(SubmitSolutionForValidationResponseProto {
                    result: AppChainResultProto {
                        status: AppChainResultStatus::Ok.into(),
                        message: None,
                    }
                        .into(),
                    solution_validated_details: validated_details_bytes,
                }))
            }
            
            None => {
                let error_msg = format!(
                    "Request with intent_id {} is not in 'New' state or does not exist",
                    req.intent_id
                );
                log::warn!("Validation failed for intent_id: {} - {}", req.intent_id, error_msg);
                
                Ok(Response::new(SubmitSolutionForValidationResponseProto {
                    result: AppChainResultProto {
                        status: AppChainResultStatus::EventNotFound.into(),
                        message: error_msg.into(),
                    }
                        .into(),
                    solution_validated_details: vec![],
                }))
            }
        }
    }
}
pub async fn start_grpc_server(config: config::Config, storage: Storage, ipfs_service: IpfsService, 
                               validator_wallet: LocalWallet
) -> anyhow::Result<()> {
    let addr: String = config.grpc.binding_url;
    let addr_parsed = addr.parse()?;

    log::info!("Reading the proto descriptor");
    let descriptor_bytes = fs::read("src/generated/user_descriptor.pb")?;

    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(&*descriptor_bytes)
        .build_v1()?;

    log::info!("Starting gRPC server on {}", addr);

    let validator_service = ValidatorGrpcService {
        storage,
        ipfs_service,
        validator_wallet,
    };

    Server::builder()
        .add_service(ValidatorServiceServer::new(validator_service))
        .add_service(reflection_service)
        .serve(addr_parsed)
        .await?;

    Ok(())
}