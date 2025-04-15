# SMART TRANSACTION SOLANA VAMP PROGRAM

This Solana program enables **secure token airdrops using Merkle roots**. Eligible users can claim their tokens by submitting a valid Merkle proof. 

---

## 🛠️ Deployment on Devnet

### 1. Install Dependencies

```bash
git clone https://github.com/your-org/solana-merkle-airdrop.git
cd solana-merkle-airdrop
npm install
```

### 2. Configure Anchor
Set the provider to Devnet:

```bash
anchor config set --cluster devnet
anchor config set --provider.wallet ~/.config/solana/id.json
```

### 3. Build and Deploy
```bash
anchor build
anchor deploy
```
After successful deployment, copy the program ID shown in the terminal output.

### 🧪 Test Locally
You can run tests with:

```bash
anchor test
```

### 🔌 User Integration Guide
#### 1. Get the IDL
Once the program is built, the IDL is located in:

```bash
target/idl/solana_vamp_program.json
```

#### 2. Using the IDL for integration
```ts
import { AnchorProvider, Program, Idl } from "@project-serum/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import idl from "./idl/solana_vamp_program.json";

const connection = new Connection("https://api.devnet.solana.com");
const provider = new AnchorProvider(connection, window.solana, {});
const programId = new PublicKey("<REPLACE_WITH_PROGRAM_ID>");
const program = new Program(idl as Idl, programId, provider);
```
