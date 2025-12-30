CREATE TABLE IF NOT EXISTS clonings(
  chain_id BIGINT NOT NULL,
  erc20_address CHAR(42) NOT NULL,
  target_txid VARCHAR(128) NOT NULL,
  UNIQUE INDEX key_idx(chain_id, erc20_address)
);
