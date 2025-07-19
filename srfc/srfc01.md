# srfc01 - vamp.fun - balance map storage, validation and usage

## Changes by stages

### Cloning Stage

1. We have our `intent_id` generated in our EntryPoint contract call. We just need to be sure that it will not repeat during the next call without actually checking it. Plus it should consider multiple intents in a single transaction.
```
   bytes32 intent_id = keccak256(abi.encodePacked(
   source_chain_id,
   block.timestamp,
   block.number,
   sequence_counter
   ));
```
`sequence_counter` is stored in EntryPoint contract state and updated after each id generation.
2. For each balance entry solver builds:
   `message = sha256(eth_address || balance || intent_id)`
   signs it using his `solver_private_key` and creates the `solver_individual_balance_sig` per balance entry.
Solver builds a `VampSolutionForValidationProto` structure:
```
   message VampSolutionForValidationProto {
      string intent_id = 1;
      string solver_pubkey = 2;
      map<string, IndividualBalanceEntry> individual_balance_entry_by_oth_address = 3;
   }
   
   message IndividualBalanceEntry {
      uint64 balance = 1;
      string solver_individual_balance_sig = 2;
      string validator_individual_balance_sig = 3;
   }
```
3. Validator (new component) receives the `VampSolutionForValidationProto` and checks the validity of each `IndividualBalanceEntry`.
After that it build the `balance_map` file structure:
```
   <intent_id>
   |
   +---<holder_etn_account_1>
   |   |
   |   +---balance_entry.json
   |
   +---<holder_etn_account_2>
   |
   +---balance_entry.json
```

Each balance entry should contain the following:
```
   {
   "balance": "10000000000",
   "solver_individual_balance_sig": "...",
   "validator_individual_balance_sig": "..."
   }
```
After that validator uploads this `balance_map` to IPFS and gets a root folder CID for it (`root_intent_cid`).
And return a VampSolutionValidatedDetailsProto response to the solver:
```
// The partial case of SolutionValidatedDetails for vamp.fun
message VampSolutionValidatedDetailsProto {
  // The root folder CID (IPFS unique ID).
  // For the flow when Validator submits the IPFS mapping himself.
  string root_intent_cid = 1;
  // The mapping of the CID (IPFS unique ID) for individual balance entry by the OriginalTokenHolders (OTH Address)
  map<string, string> cid_by_oth_address = 2;
}
```
Validator is a vamp.fun context aware component, so it can use our `vamp_fun.proto` protocol. And the request and response details are encapsulated inside 
more generic `SubmitSolutionForValidationRequestProto` and `SubmitSolutionForValidationResponseProto` messages.

3a. (alternative, temporary). 
   Solver uploads this `balance_map_json` and gets a CID for it (`vamp_root_cid`). In the early stage of centralized storage 
   solver can just keep some sha256 of the map there, just to keep the protocol clear. 
   That however leaves us the vulnerability of solver attack in case he will be the holder of some minimum amount in the source network and decides to abuse claiming and faking his own ownership balance by recreating the different `solver_individual_balance_sig`.

4. Solver adds params into solution to be stored in the destination SPL program:
   root_intent_cid: [u8; 32] // In case of A4.
   intent_id: [u8; N]
   solver_public_key: Pubkey

5. Solver executes the solution himself by calling the SPL factory or, more generic, the CallBreaker in according destination network.  
  
5a. (alternative, temporary)
   Solver submits the solution to the  good old orchestrator, it receives the solution and tries to execute it.
   Probably it will be moved to the solver itself and orchestrator will just inspect the execution attempts and flow updating the appchain state of the intent.

## Claiming stage

1. Claiming frontend retrieves the CID from the SPL program via non-modal view request. It can also be the part of that CloningFinishedEvent and travel through those url params, not that critical.
2. The claiming frontend retrieves the IPFS file with the path based on the cloning ID and the holder ETH account.
   The format will be
   https://{ipfs_public_endpoint}/ipfs/{vamp_root_cid}/{owner_address}.json
   E.g:
   https://ipfs.io/ipfs/QmSczc1erdsxBc9hYyzWTpiB1k4Q8vuMeMFD6C6uucyJrA/0x4dd48f9168c0e4e77c0c3f37b4576d18dac32bab.json
   Client retrieves only the entry for the holder ETH account, not the whole `balance_map` which can be 100+Mb.
3. Parses their entry: `eth_address`, `balance`, `solver_individual_balance_sig`, `validator_individual_balance_sig`.
4. Recomputes same message: `message = sha256(eth_address || balance || intent_id)`.
5. Signs it with their Ethereum private key: `ownership_sig = sign(message)`.
6. Sends to SPL program `claim` method with the following params:
```
   {
      eth_address,
      balance,
      solver_individual_balance_sig,
      validator_individual_balance_sig,
      ownership_sig
   }
```
7. SPL program makes checks and executes the claiming. Approximately something like that:
   pub fn claim(
   ctx: Context<Claim>,
   eth_address: [u8; 20],
   balance: u64,
   solver_individual_balance_sig: [u8; 65],
   validator_individual_balance_sig: [u8; 65],
   ownership_sig: [u8; 65],
   // ...
   ) -> Result<()> {
   let vamp_state = &ctx.accounts.vamp_state;

   // Build message
   let message = [
   eth_address.as_ref(),
   &balance.to_le_bytes(), // Or however we encode it on the solver side
   vamp_state.intent_id.as_ref(),
   ]
   .concat();

   // Recover solver signer from `solver_individual_balance_sig`
   let recovered_solver_pubkey = secp256k1_recover_pubkey(&message, &solver_individual_balance_sig)?;
   require!(
   recovered_solver_pubkey == vamp_state.solver_public_key,
   CustomError::InvalidSolverIndividualBalanceSignature
   );

   // Recover validator signer from `validator_individual_balance_sig`
   let recovered_validator_pubkey = secp256k1_recover_pubkey(&message, &validator_individual_balance_sig)?;
   require!(
   recovered_validator_pubkey == vamp_state.validator_public_key,
   CustomError::InvalidValidatorIndividualBalanceSignature
   );

   // Recover eth_address from ownership sig ===
   let recovered_eth_address = secp256k1_recover_eth_address(&message, &ownership_sig)?;
   require!(
   recovered_eth_address == eth_address,
   CustomError::InvalidOwnershipSignature
   );

   // Validate double claiming (which we already do via PDA's)
   // ...

   // Do the transfer

   //
   Ok(())
   }
   Notes
1. Technically we are not bounded onto IPFS CID cryptographycally, it is just some decentralized fixed pointer for us. So we can later store this object with signatures in any other decentralized storage e.g. Atelerix or perhaps some rollup. In fact we considered using EVM-like L2 chain with some smart-contract per address indexed storage. But it will cost like 30-50 USD per 100Mb. This is for like 600.000 holders. Not THAT much if you are cloning a 600k holders contract, but still much more expensive than IPFS, which would be near 1 dollar for 10 years of pinned storage of the same data.
2. As I mentioned in claim.2 - we can make some helper service that will split and index the `balance_map_json` file. It is not a security issue anymore, since the `individual_balance_sig` content inside it matters, rather than the whole file and will be checked nice and fast on-chain.
   UPD: Apparently we can use directories or even naming patterns (not only CID) for addressing the entries in IPFS individually. So it seems like we might avoid forcing the user to retrieve the whole snapshot or making a helper service as a workaround.
3. As I mentioned in cloning.6 - we may and we probably will add some balance map layer either in appchain or on-chain in destination chain on claiming stage. But now it's a modular task already.
4. Depending on using A4 workaround or implementing normal 4 along with solution validator layer we either stay trusting the solver or make ourselves malicious solver proof for future adoption of external solvers.
5. There is a minor concern with this approach. When we move from centelerix to atelerix - we will not be able to store the private key of the solution validator anymore. BUT we will be able to (at least in principle) introduce something similar to PDA -  some deterministic authority. Or actually consensus authority with N from M atelerix validators signatures required instead just a single `validator_individual_balance_sig` (since our malicious solver can also raise his own atelerix validator).

##  Required changes per component

### CallBreaker

   - Create the intent_id inside the CallBreaker contract function, as
   keccak256(abi.encodePacked(tx_id, chain_id, block.timestamp, block.difficulty));
   - Add the intent_id to the UserObjectivePushed event;
   - Return the intent_id from the pushUserObjective function; 

### Request registrator

   + read intent_id from UserObjectivePushed.intentId
                ```            
                "inputs": [
                {
                    "indexed": true,
                    "internalType": "bytes32",
                    "name": "intentId",
                    "type": "bytes32"
                },
                ...
                ```
   + change the map storage in Redis from "sequance_id to intent" into "intent_id intent"
   + also now store in Redis the mapping of "sequance_id to intent_id", as a long-term but temporarry workaround until we introduce the numerated blocks (or numerated transactions in numerated blocks) in app-chain to actually keep the same PollRequestProto with last_sequence_id.

### Validator vamp

   + create new component (validator_vamp)
   + protocol changes - validator_vamp will be application aware so it will use both our `user_objective.proto` - 
      with generic intent network protocol as well as `vamp_fun.proto` - vamping specific protocol
        + add SubmitSolutionForValidationRequestProto into `user_objective.proto`
            message SubmitSolutionForValidationRequestProto {
              uint64 intent_id = 1;
              // The solution to be validated in the specific encoding.
              // In case of vamp.fun VampSolutionForValidation
              bytes solution = 3;
            }
        + add VampSolutionForValidationProto into `vamp_fun.proto`
        + add VampSolutionValidatedDetailsProto into `vamp_fun.proto`
        + add SolutionValidatedDetailsProto into SubmitSolutionForValidationResponseProto   
   + validator key signing in IndividualBalanceEntry
   + validator key managment
   + ipfs map submission
   + change the request lifecycle stage to Validated with according solver_pubkey on success
   + configuration
   - dockerize
   - add actual balance validation by Merkle proof

### Orchestrator

   - orchestrator should temporarily accept both solutions in new state from the solver (deprecate this flow) and accept the solutions in the 
validated state.
   - orchestrator - work with new Redis storage pattern
    
### Solver

   - Read the intent_id from the UserEventProto;
   - Sign the balance with the solver private key, the balance is
   - keccak256(erc20_address, amount, intent_id)
   - Send the additional data to the validator:
       - The solver public key;
       - The intent_id;

### Solana SPL factory

   - Save the solver and validator public key and the request ID into the SPL; 

### Solana SPL claimer code

  - Add the solver and validator signature verification;
    Claimer Frontend
  - Implement the changes for the claimer
