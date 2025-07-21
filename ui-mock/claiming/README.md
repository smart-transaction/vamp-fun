# Claiming Client

A Rust client for claiming tokens from the vamp.fun Solana program.

## Features

- Fetches VampState from Solana to get intent_id and public keys
- Parses IPFS data for solver and validator signatures
- Generates ownership signature using Ethereum private key
- Executes claim transaction on Solana
- Automatically creates associated token account if needed

## Prerequisites

Before running the client, ensure you have:

1. **Solana Wallet**: A funded Solana wallet keypair file
2. **Ethereum Private Key**: The private key for the Ethereum address that owns the tokens
3. **IPFS Balance File**: JSON file containing the balance data and signatures
4. **Mint Account Address**: The SPL token mint address for the vamping

## Usage

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run -- \
  --solana-wallet /path/to/solana/keypair.json \
  --ethereum-wallet /path/to/ethereum/private_key.txt \
  --ipfs-balance-file /path/to/ipfs_balance.json \
  --mint-account-address <SPL_TOKEN_MINT_ADDRESS>
```

### Example

```bash
cargo run -- \
  --solana-wallet ~/.config/solana/vampfun-claimer.json \
  --ethereum-wallet ~/.ethereum/keystore/eth_token_holder_key_0x3b819cca456a577f75378787cdafd46f1d540101.txt \
  --ipfs-balance-file config/0x3b819cca456a577f75378787cdafd46f1d540101.json \
  --mint-account-address 3GVz1Scw31aggLNKAhGa489rfFYpy6A6Y7MWudf9EX3U
```

### Optional Parameters

```bash
# Custom RPC URL
cargo run -- --rpc-url "https://api.devnet.solana.com" --solana-wallet ... --ethereum-wallet ... --ipfs-balance-file ... --mint-account-address ...

# Different cluster (mainnet, localnet)
cargo run -- --cluster mainnet --solana-wallet ... --ethereum-wallet ... --ipfs-balance-file ... --mint-account-address ...
```

## File Formats

### Ethereum Private Key File
Plain text file containing the hex-encoded private key (without 0x prefix):
```
a2386c0246a2a8b28d03eaf6109b12c6a994fe2cc20029f6143569a9ac5362ac
```

### IPFS Balance File
JSON file containing balance and signatures:
```json
{
  "b": "100000000000",
  "ss": "d88b4c604a265a8b807ff653af76bb137c6223c5748ceebea498819daee5c3187bca67adedb0cb33518c74c2301a1e1471038008ce92d576187503047a79e0481c",
  "vs": "5d58539f4503033353bea72f87202ec724d3deb7502c20b5cdbb005a41e4b45841ac8817f0e8cb2ed16aff374026f05867115b9daa559798db1ca721abee462f1b"
}
```

Where:
- `b`: Balance amount (string)
- `ss`: Solver signature (hex string)
- `vs`: Validator signature (hex string)

## How it works

1. **Load Keys**: Reads Solana keypair and Ethereum private key from files
2. **Fetch VampState**: Connects to Solana and finds the VampState account for the given mint
3. **Parse IPFS Data**: Extracts solver and validator signatures from the balance file
4. **Generate Ownership Signature**: Creates signature using the Ethereum private key
5. **Create Token Account**: Creates associated token account if it doesn't exist
6. **Execute Claim**: Sends the claim transaction to Solana

## Error Handling

The client includes comprehensive error handling for:
- Network connection issues
- Invalid account data
- Missing signatures
- Transaction failures
- Signature verification errors
- Insufficient SOL balance

## Security Notes

- **Never commit private keys**: Ensure private key files are in `.gitignore`
- **Use secure file permissions**: Set appropriate permissions on key files
- **Validate addresses**: Double-check mint addresses and Ethereum addresses
- **Test on devnet first**: Always test with small amounts before mainnet

## Example Output

```
🔑 Solana wallet: EfUdSis3Cbxaknq1rd94aFCABf95isK4wJQ6TkpUH2nd
🔍 Derived Ethereum address from private key: 0x3b819cca456a577f75378787cdafd46f1d540101
🔍 Using Ethereum address: 0x3b819cca456a577f75378787cdafd46f1d540101
📡 Fetching VampState for mint: 3GVz1Scw31aggLNKAhGa489rfFYpy6A6Y7MWudf9EX3U
✅ Found VampState:
   Solver PK: 0x4f81fbaa07c6eba9b2c9e7dedefe7984b565909c
   Validator PK: 0xde236f0dc86b8fedd4cbdaee642f5d95358a35d2
   Intent ID: 0x9984ac367455a0d1aad7ba2c89e69462bfb6026ec6c060c211eab5361d7cb44e
✅ Parsed IPFS data:
   Balance: 100000000000
   Solver sig: 0xd88b4c604a265a8b807ff653af76bb137c6223c5748ceebea498819daee5c3187bca67adedb0cb33518c74c2301a1e1471038008ce92d576187503047a79e0481c
   Validator sig: 0x5d58539f4503033353bea72f87202ec724d3deb7502c20b5cdbb005a41e4b45841ac8817f0e8cb2ed16aff374026f05867115b9daa559798db1ca721abee462f1b
✅ Generated ownership signature: 0xdd31e33516ea73aa3c226c645c6598724934667763c3c4dd23fd7c30a53c224310fce511ab9af2889f8c357663d4ccabdc78f00749a5c813160f101f552290301b
💰 SOL balance: 100000000 lamports
✅ Sufficient SOL balance for transaction
🚀 Executing claim transaction...
📋 Account addresses:
   VampState: 8UcLYnQT8tpCnki1JoAyPsM7BdDwrEtWz87yFk6pbhoV
   ClaimState: C6BVYySDNhewfLgzUtyTfqruQ4Yg3ASovLAffLydX9fj
   SOL Vault: 2kEzXcRSPBHHQ1zxwMYfd5Y6h8g9SvjBsC9RSyS8tJU2
   Vault: GxuAvETb8cUb4hhMHpgc1bLtBtKSsq2SiP1UQcojnC2Q
   Mint: 3GVz1Scw31aggLNKAhGa489rfFYpy6A6Y7MWudf9EX3U
🏗️  Creating associated token account: Dkf34REMpVrpKEDUmtS7KFniP56LiicGw3wVycPYDRQm
✅ ClaimState does not exist yet - will be created
Error: RPC response error -32002: Transaction simulation failed: Error processing Instruction 1: custom program error: 0x177a [28 log messages]
```

## Troubleshooting

### Common Errors

- **0x1774 (InvalidOwnerSignature)**: Check that the Ethereum private key matches the address in the IPFS data
- **0x1775 (InvalidSolverSignature)**: The solver signature in the IPFS data is invalid
- **0x1776 (InvalidValidatorSignature)**: The validator signature in the IPFS data is invalid
- **0x177a (ArithmeticOverflow)**: The balance amount is too large for the bonding curve calculation
- **0x1779 (TokensAlreadyClaimed)**: Tokens have already been claimed for this address

### Debugging Tips

1. Verify the mint address matches the VampState on-chain
2. Check that the IPFS data contains valid signatures
3. Ensure the Ethereum private key corresponds to the address in the IPFS data
4. Verify the Solana wallet has sufficient SOL for transaction fees
5. Check that the intent_id in VampState matches the one used to generate signatures 