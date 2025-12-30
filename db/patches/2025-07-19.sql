-- Add intent_id column to clonings table for better tracking and debugging
-- This allows us to correlate clonings with specific intent requests

-- Add intent_id column to store the intent identifier
ALTER TABLE clonings ADD COLUMN intent_id VARCHAR(128) NOT NULL DEFAULT '';

-- Add index for efficient querying by intent_id
CREATE INDEX idx_clonings_intent_id ON clonings(intent_id);

-- Add composite index for efficient querying by chain_id, erc20_address, and intent_id
CREATE INDEX idx_clonings_chain_token_intent ON clonings(chain_id, erc20_address, intent_id); 