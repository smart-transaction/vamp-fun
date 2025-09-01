use anyhow::Result;
use clap::Parser;
use hex::FromHex;
use log::Level;
use sha3::{Digest, Keccak256};
use std::time::Duration;
use tokio::time::sleep;
use tonic::Request;

pub mod proto { tonic::include_proto!("stxn.io"); }

use proto::{
    request_registrator_service_client::RequestRegistratorServiceClient,
    orchestrator_service_client::OrchestratorServiceClient,
    AppChainResultStatus,
    PollRequestProto,
    SubmitSolutionRequest2Proto, MultiChainTransactionProto, DestinationChainIdProto,
};

#[derive(Parser, Debug, Clone)]
struct Args {
    #[arg(long)]
    request_registrator_url: String,
    #[arg(long)]
    orchestrator_url: String,
    #[arg(long, default_value = "5s")]
    poll_frequency: String,
    // EVM signer private key (hex)
    #[arg(long)]
    evm_private_key_hex: String,
    // EVM erc20 token address (hex 0x...)
    #[arg(long)]
    erc20_token_address: String,
    // Amount in wei (decimal string)
    #[arg(long, default_value = "0")] 
    amount_wei: String,
    // Destination chain reference (decimal chainId) for eip155
    #[arg(long)]
    eip155_chain_ref: String,
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    stderrlog::new().verbosity(Level::Info).init().unwrap();
    let poll = parse_duration::parse(&args.poll_frequency)?;

    let mut rr = RequestRegistratorServiceClient::connect(args.request_registrator_url.clone()).await?;
    let mut orch = OrchestratorServiceClient::connect(args.orchestrator_url.clone()).await?;

    use ethers_signers::Signer;
    let signer = ethers_signers::LocalWallet::from_bytes(&<[u8; 32]>::from_hex(args.evm_private_key_hex.trim_start_matches("0x"))?)?;

    let mut last_sequence_id: u64 = 0;
    loop {
        let poll_req = PollRequestProto { last_sequence_id };
        let resp = rr.poll(Request::new(poll_req)).await?.into_inner();
        if let Some(res) = resp.result.as_ref() {
            if res.status() == AppChainResultStatus::Ok {
                let seq = resp.sequence_id;
                if let Some(event) = resp.event {
                    if let Some(_obj) = event.user_objective {
                        // Fallback to zero address if not provided; orchestrator/RPC will reject if invalid
                        let to = ethers_core::types::Address::zero();
                        let token = args.erc20_token_address.parse::<ethers_core::types::Address>()?;
                        let amount = ethers_core::types::U256::from_dec_str(&args.amount_wei)?;

                        // Build ERC20 transfer data: transfer(address,uint256)
                        let selector = &keccak256(b"transfer(address,uint256)")[..4];
                        let mut calldata = Vec::with_capacity(4 + 32 + 32);
                        calldata.extend_from_slice(selector);
                        // pad address left to 32 bytes
                        let mut addr_pad = [0u8; 32];
                        addr_pad[12..].copy_from_slice(to.as_bytes());
                        calldata.extend_from_slice(&addr_pad);
                        // amount 32 bytes big endian
                        let mut amt_pad = [0u8; 32];
                        amount.to_big_endian(&mut amt_pad);
                        calldata.extend_from_slice(&amt_pad);

                        // Build typed EVM tx and sign to raw RLP
                        use ethers_core::types::{TransactionRequest, NameOrAddress, Bytes, U256};
                        use ethers_core::types::transaction::eip2718::TypedTransaction;
                        let chain_u64 = args.eip155_chain_ref.parse::<u64>()?;
                        let tx_req = TransactionRequest::new()
                            .to(NameOrAddress::Address(token))
                            .data(Bytes::from(calldata))
                            .chain_id(chain_u64)
                            .gas(U256::from(100000u64))
                            .gas_price(U256::from(2_000_000_000u64))
                            .nonce(U256::from(0u64));
                        let typed: TypedTransaction = tx_req.into();
                        let sig = signer.sign_transaction(&typed).await?;
                        let raw = typed.rlp_signed(&sig);
                        let raw_vec = raw.to_vec();

                        // Submit via SubmitSolution2
                        let dest = DestinationChainIdProto { namespace: "eip155".to_string(), reference: args.eip155_chain_ref.clone() };
                        let mtx = MultiChainTransactionProto { step: 1, destination: Some(dest), transaction: raw_vec };
                        let req = SubmitSolutionRequest2Proto { request_sequence_id: seq, txs: vec![mtx] };
                        let submit = orch.submit_solution2(Request::new(req)).await?.into_inner();
                        if let Some(res2) = submit.result { if res2.status() != AppChainResultStatus::Ok { log::warn!("SubmitSolution2 failed: {:?}", res2.message); }}
                        last_sequence_id = seq;
                        continue;
                    }
                }
            }
        }
        sleep(Duration::from_millis(poll.as_millis() as u64)).await;
    }
} 