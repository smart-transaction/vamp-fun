const crypto = require('crypto');

// Your data from the IPFS entry
const address = '0x3b819cca456a577f75378787cdafd46f1d540101';
const balance = 100000000000; // "100000000000" from IPFS
const intentId = '0xede80c404c310092755d6a47a27e8a2edc1a5cc0170d603bc07957d3ea928e6a';

console.log('🔍 Testing Endianness Issue');
console.log('============================');
console.log(`Address: ${address}`);
console.log(`Balance: ${balance}`);
console.log(`Intent ID: ${intentId}`);
console.log('');

// Remove 0x prefix and convert to bytes
const addressBytes = Buffer.from(address.slice(2), 'hex');
const intentIdBytes = Buffer.from(intentId.slice(2), 'hex');

console.log('Address bytes:', addressBytes.toString('hex'));
console.log('Intent ID bytes:', intentIdBytes.toString('hex'));
console.log('');

// Function to create hash like the Solana program (balance_util)
function createSolanaHash(address, balance, intentId) {
    const hasher = crypto.createHash('sha3-256');
    hasher.update(address);
    hasher.update(Buffer.from(balance.toString(16).padStart(16, '0'), 'hex')); // to_le_bytes equivalent
    hasher.update(intentId);
    return hasher.digest();
}

// Function to create hash like the validator (big-endian)
function createValidatorHash(address, balance, intentId) {
    const hasher = crypto.createHash('sha3-256');
    hasher.update(address);
    hasher.update(Buffer.from(balance.toString(16).padStart(16, '0'), 'hex')); // to_be_bytes equivalent
    hasher.update(intentId);
    return hasher.digest();
}

// Function to create hash like the solver (little-endian)
function createSolverHash(address, balance, intentId) {
    const hasher = crypto.createHash('sha3-256');
    hasher.update(address);
    hasher.update(Buffer.from(balance.toString(16).padStart(16, '0'), 'hex')); // to_le_bytes equivalent
    hasher.update(intentId);
    return hasher.digest();
}

// Test with actual balance bytes
const balanceLE = Buffer.alloc(8);
balanceLE.writeBigUInt64LE(BigInt(balance), 0);

const balanceBE = Buffer.alloc(8);
balanceBE.writeBigUInt64BE(BigInt(balance), 0);

console.log('Balance as little-endian bytes:', balanceLE.toString('hex'));
console.log('Balance as big-endian bytes:', balanceBE.toString('hex'));
console.log('');

// Create hashes with correct byte representations
function createSolanaHashCorrect(address, balance, intentId) {
    const hasher = crypto.createHash('sha3-256');
    hasher.update(address);
    hasher.update(balanceLE); // Little-endian
    hasher.update(intentId);
    return hasher.digest();
}

function createValidatorHashCorrect(address, balance, intentId) {
    const hasher = crypto.createHash('sha3-256');
    hasher.update(address);
    hasher.update(balanceBE); // Big-endian
    hasher.update(intentId);
    return hasher.digest();
}

function createSolverHashCorrect(address, balance, intentId) {
    const hasher = crypto.createHash('sha3-256');
    hasher.update(address);
    hasher.update(balanceLE); // Little-endian
    hasher.update(intentId);
    return hasher.digest();
}

// Create the hashes
const solanaHash = createSolanaHashCorrect(addressBytes, balance, intentIdBytes);
const validatorHash = createValidatorHashCorrect(addressBytes, balance, intentIdBytes);
const solverHash = createSolverHashCorrect(addressBytes, balance, intentIdBytes);

console.log('🔍 Hash Comparison:');
console.log('==================');
console.log('Solana Program (expects):', solanaHash.toString('hex'));
console.log('Validator (creates):    ', validatorHash.toString('hex'));
console.log('Solver (creates):       ', solverHash.toString('hex'));
console.log('');

console.log('🔍 Endianness Analysis:');
console.log('=======================');
console.log('✅ Solana Program: Uses LITTLE-ENDIAN (to_le_bytes())');
console.log('❌ Validator:      Uses BIG-ENDIAN (to_be_bytes())');
console.log('✅ Solver:         Uses LITTLE-ENDIAN (to_le_bytes())');
console.log('');

if (solanaHash.equals(validatorHash)) {
    console.log('✅ Solana and Validator hashes MATCH');
} else {
    console.log('❌ Solana and Validator hashes DO NOT MATCH');
}

if (solanaHash.equals(solverHash)) {
    console.log('✅ Solana and Solver hashes MATCH');
} else {
    console.log('❌ Solana and Solver hashes DO NOT MATCH');
}

if (validatorHash.equals(solverHash)) {
    console.log('✅ Validator and Solver hashes MATCH');
} else {
    console.log('❌ Validator and Solver hashes DO NOT MATCH');
}

console.log('');
console.log('🔧 SOLUTION:');
console.log('============');
console.log('Change in validator_vamp/src/validator_vamp/validator_grpc_service.rs:');
console.log('Line ~60: Change from:');
console.log('  hasher.update(&entry.balance.to_be_bytes());');
console.log('To:');
console.log('  hasher.update(&entry.balance.to_le_bytes());'); 