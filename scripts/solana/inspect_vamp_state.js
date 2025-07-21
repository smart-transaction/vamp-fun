const { Connection, PublicKey } = require('@solana/web3.js');
const { Program, AnchorProvider, web3, BN } = require('@project-serum/anchor');
const fs = require('fs');

// VampState account discriminator
const VAMP_STATE_DISCRIMINATOR = [222, 91, 2, 48, 244, 96, 192, 196];

async function inspectVampState() {
    // Connect to Solana devnet
    const connection = new Connection('https://api.devnet.solana.com', {
        commitment: 'confirmed',
        wsEndpoint: 'wss://api.devnet.solana.com/'
    });
    
    // The mint account address from your vamping
    const mintAccountAddress = 'FNYH2GXJztxyVxtTXzJ4co9qL6VeT6Kry8KgWHQwYfHB';
    const mintPubkey = new PublicKey(mintAccountAddress);
    
    console.log('🔍 Inspecting Vamp State for mint:', mintAccountAddress);
    console.log('=====================================');
    
    try {
        // Get all accounts owned by the program
        const programId = new PublicKey('CABA3ibLCuTDcTF4DQXuHK54LscXM5vBg7nWx1rzPaJH');
        
        console.log('📡 Fetching all accounts owned by the program...');
        
        // First, let's try to get all accounts without filters
        let accounts = [];
        try {
            accounts = await connection.getProgramAccounts(programId, {
            filters: [
                {
                    memcmp: {
                        offset: 0,
                        bytes: Buffer.from(VAMP_STATE_DISCRIMINATOR).toString('base64')
                    }
                }
            ]
        });
        } catch (error) {
            console.log('⚠️  Filtered query failed, trying without discriminator filter...');
            try {
                accounts = await connection.getProgramAccounts(programId);
            } catch (error2) {
                console.error('❌ Failed to get program accounts:', error2.message);
                return;
            }
        }
        
        console.log(`Found ${accounts.length} VampState accounts`);
        
        for (let i = 0; i < accounts.length; i++) {
            const account = accounts[i];
            const data = account.account.data;
            
            // Check if this is a VampState account by looking at the discriminator
            if (data.length < 8) {
                console.log(`\n⚠️  Skipping account ${i + 1} (too small): ${account.pubkey.toString()}`);
                continue;
            }
            
            const discriminator = data.slice(0, 8);
            const expectedDiscriminator = Buffer.from(VAMP_STATE_DISCRIMINATOR);
            
            if (!discriminator.equals(expectedDiscriminator)) {
                console.log(`\n⚠️  Skipping account ${i + 1} (not VampState): ${account.pubkey.toString()}`);
                continue;
            }
            
            console.log(`\n📋 VampState Account ${i + 1}:`);
            console.log(`   Address: ${account.pubkey.toString()}`);
            
            // Parse the account data
            let offset = 8;
            
            // Read bump (1 byte)
            const bump = data[offset];
            offset += 1;
            
            // Read mint (32 bytes)
            const mint = new PublicKey(data.slice(offset, offset + 32));
            offset += 32;
            
            // Declare variables outside try block
            let solverPublicKey, validatorPublicKey, vampIdentifier, intentId, totalClaimed, reserveBalance, tokenSupply, curveExponent, initialPrice, solVault;
            
            try {
            // Read solver_public_key length (4 bytes for Vec<u8>)
            const solverKeyLength = data.readUInt32LE(offset);
            offset += 4;
            
            // Read solver_public_key
                solverPublicKey = data.slice(offset, offset + solverKeyLength);
            offset += solverKeyLength;
            
            // Read validator_public_key length (4 bytes for Vec<u8>)
            const validatorKeyLength = data.readUInt32LE(offset);
            offset += 4;
            
            // Read validator_public_key
                validatorPublicKey = data.slice(offset, offset + validatorKeyLength);
            offset += validatorKeyLength;
            
            // Read vamp_identifier (8 bytes)
                vampIdentifier = data.readBigUInt64LE(offset);
            offset += 8;
            
            // Read intent_id length (4 bytes for Vec<u8>)
            const intentIdLength = data.readUInt32LE(offset);
            offset += 4;
            
            // Read intent_id
                intentId = data.slice(offset, offset + intentIdLength);
            offset += intentIdLength;
            
            // Read bonding curve parameters
                totalClaimed = data.readBigUInt64LE(offset);
            offset += 8;
                reserveBalance = data.readBigUInt64LE(offset);
            offset += 8;
                tokenSupply = data.readBigUInt64LE(offset);
            offset += 8;
                curveExponent = data.readBigUInt64LE(offset);
            offset += 8;
                initialPrice = data.readBigUInt64LE(offset);
            offset += 8;
                solVault = new PublicKey(data.slice(offset, offset + 32));
            
            console.log(`   Bump: ${bump}`);
            console.log(`   Mint: ${mint.toString()}`);
            console.log(`   Solver Public Key: 0x${solverPublicKey.toString('hex')}`);
            console.log(`   Validator Public Key: 0x${validatorPublicKey.toString('hex')}`);
            console.log(`   Vamp Identifier: ${vampIdentifier.toString()}`);
            console.log(`   Intent ID: 0x${intentId.toString('hex')}`);
            console.log(`   Total Claimed: ${totalClaimed.toString()}`);
            console.log(`   Reserve Balance: ${reserveBalance.toString()}`);
            console.log(`   Token Supply: ${tokenSupply.toString()}`);
            console.log(`   Curve Exponent: ${curveExponent.toString()}`);
            console.log(`   Initial Price: ${initialPrice.toString()}`);
            console.log(`   SOL Vault: ${solVault.toString()}`);
            
            } catch (parseError) {
                console.log(`   ❌ Error parsing account data: ${parseError.message}`);
                continue;
            }
            
            // Check if this is the account we're looking for
            if (mint.toString() === mintAccountAddress) {
                console.log('\n🎯 FOUND TARGET VAMP STATE!');
                console.log('=====================================');
                console.log(`Validator Public Key: 0x${validatorPublicKey.toString('hex')}`);
                console.log(`Solver Public Key: 0x${solverPublicKey.toString('hex')}`);
                console.log(`Intent ID: 0x${intentId.toString('hex')}`);
                console.log('=====================================');
                
                // Save to file for easy reference
                const output = {
                    mintAccountAddress: mintAccountAddress,
                    vampStateAddress: account.pubkey.toString(),
                    validatorPublicKey: `0x${validatorPublicKey.toString('hex')}`,
                    solverPublicKey: `0x${solverPublicKey.toString('hex')}`,
                    intentId: `0x${intentId.toString('hex')}`,
                    totalClaimed: totalClaimed.toString(),
                    reserveBalance: reserveBalance.toString(),
                    tokenSupply: tokenSupply.toString()
                };
                
                fs.writeFileSync('vamp_state_debug.json', JSON.stringify(output, null, 2));
                console.log('💾 Debug info saved to vamp_state_debug.json');
            }
        }
        
        if (accounts.length === 0) {
            console.log('❌ No VampState accounts found');
        }
        
    } catch (error) {
        console.error('❌ Error:', error);
    }
}

// Run the inspection
inspectVampState().catch(console.error); 