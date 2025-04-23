use crate::rr::storage::Storage;
use crate::utils::crypto::calculate_hash;
use crate::proto::{UserEventProto, AdditionalDataProto, UserObjectiveProto, CallObjectProto};
use ethers::abi::{Abi, RawLog, Token};
use ethers::providers::{Provider, Ws};
use ethers::prelude::*;
use ethers::types::U256;
use futures_util::StreamExt;

pub struct EthereumListener {
    storage: Storage,
    provider: Provider<Ws>,
    contract_address: Address,
}

impl EthereumListener {
    pub async fn new(storage: Storage, cfg: &config::Config) -> anyhow::Result<Self> {
        let provider_url: String = cfg.get("ethereum.rpc_url")?;
        log::info!("Connecting to Ethereum provider at {}", provider_url);
        let provider = Provider::<Ws>::connect(provider_url).await?;
        let contract_address = cfg.get::<String>("ethereum.contract_address")?.parse()?;
        Ok(Self { storage, provider, contract_address })
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let abi_json = include_str!("../../../abis/CallBreakerEVM.json");
        let contract_abi: Abi = serde_json::from_str(abi_json)?;
        let user_event = contract_abi.event("UserObjectivePushed")?;
        let event_sig = user_event.signature();

        log::info!("Starting Ethereum listener for UserObjectivePushed...");

        let last_processed_block = self.storage.get_last_processed_block().await.unwrap_or(0);
        log::info!("Last processed block from Redis: {}", last_processed_block);

        let latest_block = self.provider.get_block_number().await?.as_u64();
        log::info!("Current chain head block: {}", latest_block);

        // Catch-up mode (batch fetch)
        let logs = self.provider.get_logs(&Filter::new()
            .address(self.contract_address)
            .topic0(event_sig)
            .from_block(last_processed_block + 1)
            .to_block(latest_block)).await?;

        for log in logs {
            let block_number = log.block_number.unwrap_or_default().as_u64();
            log::info!("[Reindex] Found event at block {}", block_number);

            let raw_log = RawLog {
                topics: log.topics.clone(),
                data: log.data.to_vec(),
            };
            let decoded_log = user_event.parse_log(raw_log)?;
            let user_event_proto = convert_to_user_event_proto(&decoded_log)?;
            let json_string = serde_json::to_string(&user_event_proto)?;

            let sequence_id = self.storage.next_sequence_id().await?;
            let event_hash = calculate_hash(json_string.as_bytes());

            self.storage.save_new_request(&sequence_id, &json_string).await?;
            log::info!("Stored event seq_id={} hash={}", sequence_id, event_hash);
        }

        self.storage.set_last_processed_block(latest_block).await?;

        // Live subscription
        let mut block_stream = self.provider.subscribe_blocks().await?;
        let mut last_processed_block = latest_block;
        while let Some(block) = block_stream.next().await {
            let new_block_number = block.number.ok_or_else(|| anyhow::anyhow!("Block has no number"))?.as_u64();

            for block_number in (last_processed_block + 1)..=new_block_number {
                self.process_block(&user_event, event_sig, block_number).await?;
                self.storage.set_last_processed_block(block_number).await?;
                last_processed_block = block_number;
            }
        }

        Ok(())
    }

    async fn process_block(
        &self,
        user_event: &ethers::abi::Event,
        event_sig: H256,
        block_number: u64,
    ) -> anyhow::Result<()> {
        log::info!("Processing block: {}", block_number);

        let logs = self.provider.get_logs(&Filter::new()
            .address(self.contract_address)
            .topic0(event_sig)
            .from_block(block_number)
            .to_block(block_number)).await?;

        for log in logs {
            log::info!("Found UserObjectivePushed event at block: {}", block_number);

            let raw_log = RawLog {
                topics: log.topics.clone(),
                data: log.data.to_vec(),
            };

            let decoded_log = user_event.parse_log(raw_log)?;
            let user_event_proto = convert_to_user_event_proto(&decoded_log)?;
            let json_string = serde_json::to_string(&user_event_proto)?;

            let sequence_id = self.storage.next_sequence_id().await?;
            let event_hash = calculate_hash(json_string.as_bytes());

            self.storage.save_new_request(&sequence_id, &json_string).await?;
            log::info!("Stored event seq_id={} hash={}", sequence_id, event_hash);
        }

        Ok(())
    }
}

fn u256_to_bytes(val: U256) -> Vec<u8> {
    let mut buf = [0u8; 32];
    val.to_big_endian(&mut buf);
    buf.to_vec()
}

fn convert_to_user_event_proto(log: &ethers::abi::Log) -> anyhow::Result<UserEventProto> {
    let app_id = match &log.params[0].value {
        Token::FixedBytes(b) => b.clone(),
        other => {
            log::error!("Expected bytes32 for top-level appId but got: {:?}", other);
            return Err(anyhow::anyhow!("Invalid appId type"));
        }
    };
    let chain_id = log.params[1].value.clone().into_uint().unwrap().as_u64();
    let block_number = log.params[2].value.clone().into_uint().unwrap().as_u64();

    let user_objective = match &log.params[3].value {
        Token::Tuple(fields) => {
            let app_id = match &fields[0] {
                Token::Bytes(b) => b.clone(),
                other => {
                    log::error!("Expected bytes for userObjective.appId but got: {:?}", other);
                    return Err(anyhow::anyhow!("Invalid appId type in userObjective"));
                }
            };
            let nonce = fields[1].clone().into_uint().unwrap().as_u64();
            let chain_id = fields[3].clone().into_uint().unwrap().as_u64();

            let call_objects = fields[7].clone().into_array().unwrap().into_iter().map(|obj| {
                if let Token::Tuple(inner) = obj {
                    CallObjectProto {
                        id: 0,
                        chain_id,
                        salt: u256_to_bytes(inner[0].clone().into_uint().unwrap()),
                        amount: u256_to_bytes(inner[1].clone().into_uint().unwrap()),
                        gas: u256_to_bytes(inner[2].clone().into_uint().unwrap()),
                        address: inner[3].clone().into_address().unwrap().as_bytes().to_vec(),
                        callvalue: inner[4].clone().into_bytes().unwrap(),
                        returnvalue: inner[5].clone().into_bytes().unwrap(),
                        skippable: inner[6].clone().into_bool().unwrap(),
                        verifiable: inner[7].clone().into_bool().unwrap(),
                    }
                } else {
                    panic!("Unexpected callObject format")
                }
            }).collect();

            UserObjectiveProto {
                app_id,
                nonse: nonce,
                chain_id,
                call_objects,
            }
        }
        _ => return Err(anyhow::anyhow!("Invalid userObjective format")),
    };

    let additional_data = match &log.params[4].value {
        Token::Array(arr) => arr.iter().map(|entry| {
            if let Token::Tuple(kv) = entry {
                AdditionalDataProto {
                    key: kv[0].clone().into_fixed_bytes().unwrap(),
                    value: kv[1].clone().into_bytes().unwrap(),
                }
            } else {
                panic!("Unexpected additionalData entry")
            }
        }).collect(),
        _ => return Err(anyhow::anyhow!("Invalid additionalData format")),
    };

    Ok(UserEventProto {
        app_id,
        chain_id,
        block_number,
        user_objective: Some(user_objective),
        additional_data,
    })
}

