use std::convert::TryInto;

fn main() {
    // Test the raw bytes parsing logic we implemented
    
    // Test cases based on what the frontend sends
    let test_cases = vec![
        ("paidClaimingEnabled true", vec![1]),
        ("useBondingCurve true", vec![1]),
        ("curveSlope 1", vec![1]),
        ("basePrice 100", vec![100]),
        ("maxPrice 100000", vec![1, 134, 160]), // Actual bytes from frontend
        ("flatPricePerToken 1", vec![1]),
    ];
    
    for (name, bytes) in test_cases {
        let value = if bytes.is_empty() {
            0
        } else {
            // Convert variable-length bytes to u64 (little endian)
            let mut result = 0u64;
            for (i, &byte) in bytes.iter().enumerate() {
                result += (byte as u64) << (i * 8);
            }
            println!("  Debug: bytes={:?}, result={}", bytes, result);
            result
        };
        
        println!("{}: {:?} -> {} (0x{:x})", name, bytes, value, value);
    }
    
    // Test boolean parsing
    println!("\nBoolean parsing:");
    println!("[1] -> {}", 1 != 0);
    println!("[0] -> {}", 0 != 0);
    println!("[] -> {}", 0 != 0);
} 