#[path = "../generated/stxn.io.rs"]
mod stxn;
#[path = "../generated/stxn.io.serde.rs"]
mod stxn_json;
pub use stxn::*;

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn test_chain_id_encoding() {
        let mut event = UserEventProto::default();
        event.chain_id = 42;

        let mut buf = Vec::new();
        event.encode(&mut buf).unwrap();
        println!("Encoded bytes (registrator): {:?}", buf);

        let decoded = UserEventProto::decode(&*buf).unwrap();
        assert_eq!(decoded.chain_id, 42);
    }
}
