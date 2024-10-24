-- Create blockheaders table if it doesn't exist
CREATE TABLE IF NOT EXISTS public.blockheaders (
    block_hash CHAR(66) NOT NULL,
    "number" BIGINT NOT NULL,
    gas_limit BIGINT NOT NULL,
    gas_used BIGINT NOT NULL,
    nonce VARCHAR(78) NOT NULL,
    transaction_root CHAR(66),
    receipts_root CHAR(66),
    state_root CHAR(66),
    base_fee_per_gas VARCHAR(78),
    parent_hash VARCHAR(66),
    miner VARCHAR(42),
    logs_bloom VARCHAR(1024),
    difficulty VARCHAR(78),
    totaldifficulty VARCHAR(78),
    sha3_uncles VARCHAR(66),
    "timestamp" BIGINT,
    extra_data VARCHAR(1024),
    mix_hash VARCHAR(66),
    withdrawals_root VARCHAR(66),
    blob_gas_used VARCHAR(78),
    excess_blob_gas VARCHAR(78),
    parent_beacon_block_root VARCHAR(66),
    CONSTRAINT blockheaders_pkey PRIMARY KEY ("number"),
    CONSTRAINT blockheaders_number_key UNIQUE ("number")
);

-- Set the owner of the blockheaders table
ALTER TABLE IF EXISTS public.blockheaders
    OWNER TO postgres;

-- Revoke all privileges from readonly user
REVOKE ALL ON TABLE public.blockheaders FROM matt_readonly_user;

-- Grant SELECT privilege to readonly user
GRANT SELECT ON TABLE public.blockheaders TO matt_readonly_user;

-- Grant all privileges to postgres user
GRANT ALL ON TABLE public.blockheaders TO postgres;

-- Create an index on base_fee_per_gas
CREATE INDEX IF NOT EXISTS idx_blockheaders_base_fee
    ON public.blockheaders USING btree (base_fee_per_gas ASC NULLS LAST);

-- Create an index on block number
CREATE INDEX IF NOT EXISTS idx_blockheaders_number
    ON public.blockheaders USING btree (number ASC NULLS LAST);

-- Create a composite index on block number and base_fee_per_gas
CREATE INDEX IF NOT EXISTS idx_blockheaders_number_base_fee
    ON public.blockheaders USING btree (number ASC NULLS LAST, base_fee_per_gas ASC NULLS LAST);

-- Create an index on timestamp
CREATE INDEX IF NOT EXISTS idx_blockheaders_timestamp
    ON public.blockheaders USING btree ("timestamp" ASC NULLS LAST);
