#[path = "../generated/vamp.fun.rs"]
mod vamp_fun;
#[path = "../generated/vamp.fun.serde.rs"]
mod vamp_fun_json;
pub use vamp_fun::*;

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
