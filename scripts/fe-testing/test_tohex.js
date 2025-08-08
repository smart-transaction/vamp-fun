// Simple test to understand what toHex() produces
// We'll simulate the values manually

// Test values similar to what the frontend is using
const paidClaimingEnabled = true;
const useBondingCurve = true;
const curveSlope = 1n;
const basePrice = 100n;
const maxPrice = 100000n;
const flatPricePerToken = 1n;

console.log("Testing toHex() simulation:");
console.log("paidClaimingEnabled:", paidClaimingEnabled, "->", paidClaimingEnabled ? "0x01" : "0x00");
console.log("useBondingCurve:", useBondingCurve, "->", useBondingCurve ? "0x01" : "0x00");
console.log("curveSlope:", curveSlope.toString(), "->", `0x${curveSlope.toString(16)}`);
console.log("basePrice:", basePrice.toString(), "->", `0x${basePrice.toString(16)}`);
console.log("maxPrice:", maxPrice.toString(), "->", `0x${maxPrice.toString(16)}`);
console.log("flatPricePerToken:", flatPricePerToken.toString(), "->", `0x${flatPricePerToken.toString(16)}`);

// Now let's simulate what toHex() might produce (based on viem documentation)
console.log("\nWhat toHex() likely produces (raw bytes as hex):");
console.log("paidClaimingEnabled:", paidClaimingEnabled, "->", "0x01");
console.log("useBondingCurve:", useBondingCurve, "->", "0x01");
console.log("curveSlope:", curveSlope.toString(), "->", "0x01");
console.log("basePrice:", basePrice.toString(), "->", "0x64");
console.log("maxPrice:", maxPrice.toString(), "->", "0x0186a0");
console.log("flatPricePerToken:", flatPricePerToken.toString(), "->", "0x01");

// Test what happens when we convert these to strings
console.log("\nAs strings (what the solver receives):");
console.log("'0x01' as string:", "0x01");
console.log("'0x64' as string:", "0x64");
console.log("'0x0186a0' as string:", "0x0186a0");

// Test what happens when we parse these as hex
console.log("\nParsing as hex (what the solver does):");
console.log("parseInt('01', 16):", parseInt('01', 16));
console.log("parseInt('64', 16):", parseInt('64', 16));
console.log("parseInt('0186a0', 16):", parseInt('0186a0', 16));

// Compare with what we saw in the logs
console.log("\nWhat we saw in the solver logs:");
console.log("paid_claiming_enabled: false (hex: , value: 0)");
console.log("use_bonding_curve: false (hex: , value: 0)");
console.log("base_price: 13 (hex: d)");
console.log("max_price: Some(1000) (hex: j)");

console.log("\nAnalysis:");
console.log("- The empty hex strings suggest the frontend is sending empty values");
console.log("- The 'd' and 'j' suggest the frontend is sending raw bytes instead of hex strings");
console.log("- The solver is trying to parse these as hex strings but they're not valid hex"); 