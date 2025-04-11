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
        let provider = Provider::<Ws>::connect(provider_url).await?;
        let contract_address = cfg.get::<String>("ethereum.contract_address")?.parse()?;
        Ok(Self { storage, provider, contract_address })
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let abi_json = include_str!("../../../abis/CallBreakerEVM.json");
        let contract_abi: Abi = serde_json::from_str(abi_json)?;
        let user_event = contract_abi.event("UserObjectivePushed")?;

        let filter = Filter::new()
            .address(self.contract_address)
            .topic0(user_event.signature());

        let mut stream = self.provider.subscribe_logs(&filter).await?;

        while let Some(log) = stream.next().await {
            let raw_log = RawLog {
                topics: log.topics.clone(),
                data: log.data.clone().to_vec(),
            };

            let decoded_log = user_event.parse_log(raw_log)?;

            let user_event_proto = convert_to_user_event_proto(&decoded_log)?;
            let json_bytes = serde_json::to_vec(&user_event_proto)?;

            let sequence_id = self.storage.next_sequence_id().await?;
            let event_hash = calculate_hash(&json_bytes);

            self.storage.save_new_request(&sequence_id, &json_bytes).await?;

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
    let app_id_u256 = log.params[0].value.clone().into_uint().unwrap();
    let app_id = u256_to_bytes(app_id_u256);
    let chain_id = log.params[1].value.clone().into_uint().unwrap().as_u64();
    let block_number = log.params[2].value.clone().into_uint().unwrap().as_u64();

    let user_objective = match &log.params[3].value {
        Token::Tuple(fields) => {
            let app_id = u256_to_bytes(fields[0].clone().into_uint().unwrap());
            let nonce = fields[1].clone().into_uint().unwrap().as_u64();
            let chain_id = fields[4].clone().into_uint().unwrap().as_u64();

            let call_objects = fields[7].clone().into_array().unwrap().into_iter().map(|obj| {
                if let Token::Tuple(inner) = obj {
                    CallObjectProto {
                        id: 0,
                        chain_id,
                        salt: u256_to_bytes(inner[0].clone().into_uint().unwrap()),
                        amount: u256_to_bytes(inner[1].clone().into_uint().unwrap()),
                        gas: u256_to_bytes(inner[2].clone().into_uint().unwrap()),
                        address: inner[3].clone().into_address().unwrap().as_bytes().to_vec(),
                        skippable: inner[6].clone().into_bool().unwrap(),
                        verifiable: inner[7].clone().into_bool().unwrap(),
                        callvalue: inner[4].clone().into_bytes().unwrap(),
                        returnvalue: inner[5].clone().into_bytes().unwrap(),
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
