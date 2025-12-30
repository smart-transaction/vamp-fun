-- Add timestamp support for deterministic cloning retrieval
-- This allows multiple clonings for the same token and enables ordering by creation time

-- Add timestamp column for tracking when clonings were created
ALTER TABLE clonings ADD COLUMN created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

-- Add index for efficient ordering by creation time
CREATE INDEX idx_clonings_created_at ON clonings(created_at);

-- Add composite index for efficient querying by chain_id and erc20_address (non-unique)
CREATE INDEX idx_clonings_chain_token ON clonings(chain_id, erc20_address); 