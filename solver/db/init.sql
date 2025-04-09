-- Initial database setup. Run as root@

-- Create the database.
CREATE DATABASE IF NOT EXISTS vampfun;
USE vampfun;

-- Create the tables.
CREATE TABLE IF NOT EXISTS epochs(
  chain_id BIGINT NOT NULL,
  block_number BIGINT NOT NULL,
  erc20_address CHAR(42) NOT NULL,
  ts TIMESTAMP DEFAULT current_timestamp,
);

CREATE TABLE IF NOT EXISTS tokens(
  chain_id BIGINT NOT NULL,
  erc20_address CHAR(42) NOT NULL,
  holder_address CHAR(42) NOT NULL,
  holder_amount VARCHAR(78) NOT NULL,
  INDEX chain_id_idx(chain_id),
  INDEX erc20_address_idx(erc20_address),
  INDEX holder_address_idx(holder_address)
);

CREATE TABLE IF NOT EXISTS request_logs(
  user_event_id BINARY(32) NOT NULL,
  sequence_id BIGINT NOT NULL,
  ts TIMESTAMP DEFAULT current_timestamp,
  INDEX user_event_id_idx(user_event_id),
  INDEX sequence_id_idx(sequence_id)
);

-- Create the user.
-- 1. Remove '%' user
--    if the server and mysql run on the same instance.
--    (still needed if run from two images)
CREATE USER IF NOT EXISTS 'server'@'localhost' IDENTIFIED BY 'secret_app';
CREATE USER IF NOT EXISTS 'server'@'%' IDENTIFIED BY 'secret_app';
CREATE USER IF NOT EXISTS 'importer'@'%' IDENTIFIED BY 'secret_importer';
SELECT User, Host FROM mysql.user;

-- Grant rights to the user.
GRANT ALL ON vampfun.* TO 'server'@'localhost';
GRANT ALL ON vampfun.* TO 'server'@'%';
GRANT SELECT ON vampfun.* TO 'importer'@'%';  -- We don't make secret out of reports, so that's safe.