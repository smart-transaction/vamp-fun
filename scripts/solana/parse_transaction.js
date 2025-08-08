const { Connection, PublicKey } = require('@solana/web3.js');

async function parseTransaction() {
    // Connect to Solana devnet
    const connection = new Connection('https://api.devnet.solana.com/', {
        commitment: 'confirmed'
    });
    
    // The transaction signature
    const txSignature = '3EL6WcQDj7PC1ZLbcU1T1aotYiougafv3LSQnh4TNzGdQ8ah9RJJCwWVLtggq2z3nQwrLSMGA3Cq4ZwRDBUoS7P8';
    
    console.log('🔍 Parsing transaction:', txSignature);
    console.log('=====================================');
    
    try {
        const tx = await connection.getTransaction(txSignature, {
            encoding: 'json',
            maxSupportedTransactionVersion: 0
        });
        
        if (!tx) {
            console.log('❌ Transaction not found');
            return;
        }
        
        console.log('📊 Transaction Details:');
        console.log(`   Status: ${tx.meta?.err ? 'Failed' : 'Success'}`);
        console.log(`   Fee: ${tx.meta?.fee} lamports (${(tx.meta?.fee || 0) / 1e9} SOL)`);
        console.log(`   Compute Units: ${tx.meta?.computeUnitsConsumed}`);
        
        // Parse token transfers
        console.log('\n💰 Token Transfers:');
        
        if (tx.meta?.preTokenBalances && tx.meta?.postTokenBalances) {
            const preBalances = new Map();
            const postBalances = new Map();
            
            // Build pre-balance map
            tx.meta.preTokenBalances.forEach(balance => {
                const key = `${balance.accountIndex}_${balance.mint}`;
                preBalances.set(key, {
                    amount: balance.uiTokenAmount.amount,
                    decimals: balance.uiTokenAmount.decimals,
                    owner: balance.owner
                });
            });
            
            // Build post-balance map and calculate differences
            tx.meta.postTokenBalances.forEach(balance => {
                const key = `${balance.accountIndex}_${balance.mint}`;
                const preBalance = preBalances.get(key);
                
                if (preBalance) {
                    const preAmount = BigInt(preBalance.amount);
                    const postAmount = BigInt(balance.uiTokenAmount.amount);
                    const difference = postAmount - preAmount;
                    
                    if (difference !== 0n) {
                        console.log(`   Account ${balance.accountIndex} (${balance.owner}):`);
                        console.log(`     Mint: ${balance.mint}`);
                        console.log(`     Pre:  ${preBalance.amount} (${preBalance.amount / Math.pow(10, balance.uiTokenAmount.decimals)})`);
                        console.log(`     Post: ${balance.uiTokenAmount.amount} (${balance.uiTokenAmount.amount / Math.pow(10, balance.uiTokenAmount.decimals)})`);
                        console.log(`     Change: ${difference > 0n ? '+' : ''}${difference} (${difference > 0n ? '+' : ''}${Number(difference) / Math.pow(10, balance.uiTokenAmount.decimals)})`);
                        console.log('');
                    }
                } else {
                    // New account
                    console.log(`   Account ${balance.accountIndex} (${balance.owner}):`);
                    console.log(`     Mint: ${balance.mint}`);
                    console.log(`     New Balance: ${balance.uiTokenAmount.amount} (${balance.uiTokenAmount.amount / Math.pow(10, balance.uiTokenAmount.decimals)})`);
                    console.log('');
                }
                
                postBalances.set(key, {
                    amount: balance.uiTokenAmount.amount,
                    decimals: balance.uiTokenAmount.decimals,
                    owner: balance.owner
                });
            });
            
            // Check for accounts that were closed
            preBalances.forEach((preBalance, key) => {
                if (!postBalances.has(key)) {
                    const [accountIndex, mint] = key.split('_');
                    console.log(`   Account ${accountIndex} (${preBalance.owner}):`);
                    console.log(`     Mint: ${mint}`);
                    console.log(`     Closed: -${preBalance.amount} (-${preBalance.amount / Math.pow(10, preBalance.decimals)})`);
                    console.log('');
                }
            });
        }
        
        // Parse SOL transfers
        console.log('💎 SOL Transfers:');
        if (tx.meta?.preBalances && tx.meta?.postBalances) {
            for (let i = 0; i < tx.meta.preBalances.length; i++) {
                const preBalance = tx.meta.preBalances[i];
                const postBalance = tx.meta.postBalances[i];
                const difference = postBalance - preBalance;
                
                if (difference !== 0) {
                    const account = tx.transaction.message.accountKeys[i];
                    console.log(`   Account ${i} (${account}):`);
                    console.log(`     Pre:  ${preBalance} lamports (${preBalance / 1e9} SOL)`);
                    console.log(`     Post: ${postBalance} lamports (${postBalance / 1e9} SOL)`);
                    console.log(`     Change: ${difference > 0 ? '+' : ''}${difference} lamports (${difference > 0 ? '+' : ''}${difference / 1e9} SOL)`);
                    console.log('');
                }
            }
        }
        
        // Show program logs
        console.log('📝 Program Logs:');
        if (tx.meta?.logMessages) {
            tx.meta.logMessages.forEach((log, index) => {
                console.log(`   ${index}: ${log}`);
            });
        }
        
    } catch (error) {
        console.error('❌ Error parsing transaction:', error);
    }
}

// Run the parsing
parseTransaction().catch(console.error); 