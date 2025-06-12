# vamp-fun validator
## Description
This is a simple validator for the [vamp-fun] solver solutions.
## Usage 
### Build
```
$ cargo build
```
### Run

Start:
```
$ cargo run
```
### Test
[ipfs_publisher.rs](src/bin/ipfs_publisher.rs)
This utility can be used to test around mocking balance map generation and IPFS flows and scenarios.
### GRPC server
```
$ VALIDATOR_PRIVATE_KEY={validator_private_key} 
$ validator_vamp config/validator_vamp_config.toml
```
### GRPC testing
```
$ grpcurl -plaintext 127.0.0.1:50053 describe vamp.fun.ValidatorService
$ grpcurl   -plaintext   -emit-defaults   -format json   -protoset src/generated/user_descriptor.pb   -d '{"intentId": 
"4e47921898fb2cc0d91662cd8ff76036bd4c155c145343e2624d0fb8216ab066", ...}'   127.0.0.1:50053   vamp.fun.ValidatorService/SubmitSolution
```
