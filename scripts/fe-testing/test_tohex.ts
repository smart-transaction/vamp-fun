import { toHex } from "viem";

// Test values similar to what the frontend is using
const paidClaimingEnabled = true;
const useBondingCurve = true;
const curveSlope = BigInt(1);
const basePrice = BigInt(100);
const maxPrice = BigInt(100000);
const flatPricePerToken = BigInt(1);

console.log("Testing toHex() output:");
console.log("paidClaimingEnabled:", paidClaimingEnabled, "->", toHex(paidClaimingEnabled));
console.log("useBondingCurve:", useBondingCurve, "->", toHex(useBondingCurve));
console.log("curveSlope:", curveSlope.toString(), "->", toHex(curveSlope));
console.log("basePrice:", basePrice.toString(), "->", toHex(basePrice));
console.log("maxPrice:", maxPrice.toString(), "->", toHex(maxPrice));
console.log("flatPricePerToken:", flatPricePerToken.toString(), "->", toHex(flatPricePerToken));

// Also test what these look like as strings
console.log("\nAs strings:");
console.log("paidClaimingEnabled:", paidClaimingEnabled, "->", String(toHex(paidClaimingEnabled)));
console.log("useBondingCurve:", useBondingCurve, "->", String(toHex(useBondingCurve)));
console.log("curveSlope:", curveSlope.toString(), "->", String(toHex(curveSlope)));
console.log("basePrice:", basePrice.toString(), "->", String(toHex(basePrice)));
console.log("maxPrice:", maxPrice.toString(), "->", String(toHex(maxPrice)));
console.log("flatPricePerToken:", flatPricePerToken.toString(), "->", String(toHex(flatPricePerToken)));

// Test what the solver expects (hex strings)
console.log("\nWhat solver expects (hex strings):");
console.log("paidClaimingEnabled:", paidClaimingEnabled, "->", paidClaimingEnabled ? "0x01" : "0x00");
console.log("useBondingCurve:", useBondingCurve, "->", useBondingCurve ? "0x01" : "0x00");
console.log("curveSlope:", curveSlope.toString(), "->", `0x${curveSlope.toString(16)}`);
console.log("basePrice:", basePrice.toString(), "->", `0x${basePrice.toString(16)}`);
console.log("maxPrice:", maxPrice.toString(), "->", `0x${maxPrice.toString(16)}`);
console.log("flatPricePerToken:", flatPricePerToken.toString(), "->", `0x${flatPricePerToken.toString(16)}`);

// Test parsing the toHex output as hex strings
console.log("\nParsing toHex output as hex strings:");
const toHexPaidClaiming = toHex(paidClaimingEnabled);
const toHexUseBonding = toHex(useBondingCurve);
const toHexCurveSlope = toHex(curveSlope);
const toHexBasePrice = toHex(basePrice);
const toHexMaxPrice = toHex(maxPrice);
const toHexFlatPrice = toHex(flatPricePerToken);

console.log("toHex(paidClaimingEnabled) as string:", String(toHexPaidClaiming));
console.log("toHex(useBondingCurve) as string:", String(toHexUseBonding));
console.log("toHex(curveSlope) as string:", String(toHexCurveSlope));
console.log("toHex(basePrice) as string:", String(toHexBasePrice));
console.log("toHex(maxPrice) as string:", String(toHexMaxPrice));
console.log("toHex(flatPricePerToken) as string:", String(toHexFlatPrice)); 