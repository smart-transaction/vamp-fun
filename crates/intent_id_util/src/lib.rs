use anyhow::Result;

pub fn fold_intent_id(intent_id: &[u8]) -> Result<u64> {
    let mut hash64 = 0u64;
    for chunk in intent_id.chunks(8) {
        let chunk_value = u64::from_le_bytes(chunk.try_into()?);
        hash64 ^= chunk_value; // XOR the chunks to reduce to 64 bits
    }
    Ok(hash64)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_intent_id_empty() {
        let intent_id = vec![];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_fold_intent_id_single_chunk() {
        let intent_id = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_fold_intent_id_multiple_chunks() {
        let intent_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, 0, // Second chunk
        ];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3); // 1 XOR 2 = 3
    }

    #[test]
    fn test_fold_intent_id_partial_chunk() {
        let intent_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, // Partial second chunk
        ];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_fold_intent_id_large_input() {
        let intent_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, 0, // Second chunk
            3, 0, 0, 0, 0, 0, 0, 0, // Third chunk
        ];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // 1 XOR 2 XOR 3 = 0
    }
}
