# srfc02 - vamp.fun - vamp.library

## Description of the problem

In our stxn2.0/vamp.fun AppChain API process we need to move our informative/helper client REST API from solver to AppChain side.
Currently, both vamping and claiming frontend do not have a way to deterministically define the intent_id of exact cloning process, e.g. in case we talk 
about the cloning of the same token multiple times into the same or different destination networks.
We may just guess it, like the last cloning or so on by non-deterministic criteria like source address. 
Technically the vamping frontend may listen to the source contract events, but it is not a good solution even in current scope, since it solves the problem 
for vamping stage (if we somehow don't lose the vamper session), but not for the claimng stage (if only that session is inherited from the vamping sessions 
via cookies or url params).
We need to implement the vamp.library - the new API + several client side integrations to browse through the cloning history by combinations of the  number of 
criteria like source token address, destination token address, source chain_id, destination chain_id, source transaction, source target_block, one of solution 
tx_id's, vamper source private key, status, probably approximate date of cloning, maybe even token holder address (mostly for claiming UI), etc.
It should return the list of cloning intents (successful or not), all the parameters mentioned earlier from the filtering.
In future maybe the list of vamping parameters, like the predefined configuration of the bounding curve and so on.

## Urgency

Actually vamp.fun needs it quite desperately and urgently, since we already allow multiple cloning of the same token and currently relay on workarounds as
returning the last cloning and so on.

##  Required changes per component

### AppChain

   - Move the REST (or maybe something more generic in the way of Ethereum JSON-RPC) from solver to AppChain side in principle.
   - Think about merging the AppChain components into single multipurpose node, part of new orchestrator, or separate API node.
   - Create all the necessary API with filtering requests, pagination, etc.
   - Think about moving the AppChain state from Redis to SQL or something more suitable for the API. Maybe something blockchain-like with blocks and transactions.

### Vamp frontend

   - TBD

### Claim frontend

   - TBD
   - claimers can receive the "aggressive marketing" suggesting to claim the tokens they are eligible for, based on the cloning history from same or other 
     vampers.

### Library frontend (new component)

   - TBD

