# vamp-fun requests registrator stub
A temporary centralized stub for the requests registrator service.
It is considered to be replaced with a proper decentralized service in the future based on the atelerix appchain infrastructure.
## Usage 
### Build the stub
```
$ cargo build
```
### Run the stub

You must have the Redis server running locally or elsewhere (`config/config.toml:storage.redis_url`).

Optionally set the desired block to index events from it - in redis with:
`vamp:intents:global:last_processed_block` key (default will be set to `1_216_830`).

Start the requests registrator stub:
```
$ cargo run
```
Give it some time to re-index until you see the current blocks in the logs.
After that you might try connecting to it via RPC (e.g. with `grpcurl`):
```
$ grpcurl \
  -plaintext \
  -protoset src/generated/user_descriptor.pb \
  -d '{"lastSequenceId": 0}' \
  127.0.0.1:50051 \
  vamp.fun.RequestRegistratorService/Poll
```
