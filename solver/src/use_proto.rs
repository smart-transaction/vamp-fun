pub mod proto {
    tonic::include_proto!("vamp.fun");
}

#[cfg(test)]
mod tests {
    use super::proto::*;
    use prost::Message;

    #[test]
    fn test_chain_id_encoding() {
        let mut event = UserEventProto::default();
        event.chain_id = 42;

        let mut buf = Vec::new();
        event.encode(&mut buf).unwrap();
        println!("Encoded bytes (solver): {:?}", buf);

        let decoded = UserEventProto::decode(&*buf).unwrap();
        assert_eq!(decoded.chain_id, 42);
    }
}
