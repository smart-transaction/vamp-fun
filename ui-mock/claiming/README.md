# Claiming Client

A command-line tool for claiming tokens from the Vamp protocol.

## Features

- **Automatic Data Fetching**: Automatically fetches vamping data from the solver API and IPFS balance files
- **Flexible Configuration**: Can use provided files or fetch data dynamically
- **Ethereum Address Derivation**: Automatically derives Ethereum address from private key
- **SOL Balance Checking**: Verifies sufficient SOL balance before claiming

## Usage

### Basic Usage (Automatic Fetching)

```bash
# Minimal usage - automatically fetches all required data
cargo run -- \
  --ethereum-wallet ~/.ethereum/keystore/eth_token_holder_key_0x3b819cca456a577f75378787cdafd46f1d540101.txt \
  --token-address 0xd60c5Cfea0407c6156690B15539Cad34Ab0A1DA6

# With custom solver API
cargo run -- \
  --ethereum-wallet ~/.ethereum/keystore/eth_token_holder_key_0x3b819cca456a577f75378787cdafd46f1d540101.txt \
  --token-address 0xd60c5Cfea0407c6156690B15539Cad34Ab0A1DA6 \
  --solver-api-url https://your-solver-api.com
```

### Manual Usage (Providing Files)

```bash
# Using provided files
cargo run -- \
  --solana-wallet ~/.config/solana/vampfun-claimer.json \
  --ethereum-wallet ~/.ethereum/keystore/eth_token_holder_key_0x3b819cca456a577f75378787cdafd46f1d540101.txt \
  --ipfs-balance-file config/0x3b819cca456a577f75378787cdafd46f1d540101.json \
  --mint-account-address 8ML5qZnT7S8i9GnBXTXibSg3EgXNzFpEVkBGtd8ywg7F
```

### Advanced Usage

```bash
# Custom cluster and chain ID
cargo run -- \
  --cluster mainnet \
  --chain-id 1 \
  --ethereum-wallet ~/.ethereum/keystore/eth_token_holder_key_0x3b819cca456a577f75378787cdafd46f1d540101.txt \
  --token-address 0xd60c5Cfea0407c6156690B15539Cad34Ab0A1DA6 \
  --solver-api-url https://mainnet-solver-api.com
```

## Command Line Arguments

| Argument | Required | Default | Description |
|----------|----------|---------|-------------|
| `--ethereum-wallet` | Yes | - | Path to Ethereum private key file |
| `--token-address` | No* | - | ERC20 token contract address (required for auto-fetching) |
| `--solana-wallet` | No | Generated | Path to Solana wallet keypair file |
| `--ipfs-balance-file` | No* | Auto-fetched | Path to IPFS balance data file |
| `--mint-account-address` | No* | Auto-fetched | Solana mint account address |
| `--solver-api-url` | No | `https://34-36-3-154.nip.io` | Solver REST API URL |
| `--chain-id` | No | `21363` | Chain ID for the token |
| `--cluster` | No | `devnet` | Solana cluster (devnet/mainnet/localnet) |
| `--rpc-url` | No | Auto | Custom Solana RPC URL |

*Required when not using auto-fetching mode

## How It Works

### Automatic Mode (Recommended)

1. **Derives Ethereum Address**: Extracts public key from private key and derives Ethereum address
2. **Fetches Vamping Data**: Calls solver API to get mint account address, root intent CID, and other metadata
3. **Fetches IPFS Balance**: Downloads balance data from IPFS using the root intent CID
4. **Fetches VampState**: Gets intent ID and public keys from Solana program
5. **Generates Signatures**: Creates ownership signature for the claim
6. **Executes Claim**: Submits the claim transaction to Solana

### Manual Mode

1. **Reads Files**: Uses provided IPFS balance file and mint account address
2. **Fetches VampState**: Gets intent ID and public keys from Solana program
3. **Generates Signatures**: Creates ownership signature for the claim
4. **Executes Claim**: Submits the claim transaction to Solana

## API Endpoints

The tool expects the solver API to provide the following endpoint:

```
GET /get_claim_amount?chain_id={chain_id}&token_address={token_address}&user_address={user_address}
```

Response format:
```json
{
  "token_address": "0x...",
  "user_address": "0x...",
  "amount": "100000000000",
  "decimals": 9,
  "target_txid": "...",
  "solver_signature": "0x...",
  "validator_signature": "0x...",
  "mint_account_address": "...",
  "token_spl_address": "...",
  "root_intent_cid": "Qm...",
  "intent_id": "..."
}
```

## IPFS Data Format

The IPFS balance file should contain the user's token balance and signatures:

```json
{
  "balance": "100000000000",
  "solver_signature": "0x...",
  "validator_signature": "0x..."
}
```

## Requirements

- Sufficient SOL balance in the Solana wallet (minimum 0.05 SOL recommended)
- Valid Ethereum private key file
- Internet connection for API calls and IPFS access 