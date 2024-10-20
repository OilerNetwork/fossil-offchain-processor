-- Up migration: Create blockheaders and transactions tables

CREATE TABLE IF NOT EXISTS public.blockheaders (
    block_hash character(66) NOT NULL,
    "number" bigint NOT NULL,
    gas_limit bigint NOT NULL,
    gas_used bigint NOT NULL,
    nonce character varying(78) NOT NULL,
    transaction_root character(66),
    receipts_root character(66),
    state_root character(66),
    base_fee_per_gas character varying(78),
    parent_hash character varying(66),
    miner character varying(42),
    logs_bloom character varying(1024),
    difficulty character varying(78),
    totaldifficulty character varying(78),
    sha3_uncles character varying(66),
    "timestamp" bigint,
    extra_data character varying(1024),
    mix_hash character varying(66),
    withdrawals_root character varying(66),
    blob_gas_used character varying(78),
    excess_blob_gas character varying(78),
    parent_beacon_block_root character varying(66),
    CONSTRAINT blockheaders_pkey PRIMARY KEY ("number")
);

CREATE TABLE IF NOT EXISTS public.transactions (
    block_number bigint,
    transaction_hash character(66) NOT NULL,
    transaction_index integer NOT NULL,
    from_addr character(42),
    to_addr character(42),
    value character varying(78) NOT NULL,
    gas_price character varying(78) NOT NULL,
    max_priority_fee_per_gas character varying(78),
    max_fee_per_gas character varying(78),
    gas character varying(78) NOT NULL,
    chain_id character varying(78),
    CONSTRAINT transactions2_pkey PRIMARY KEY (transaction_hash),
    CONSTRAINT transactions_block_number_fkey FOREIGN KEY (block_number)
        REFERENCES public.blockheaders ("number") MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE RESTRICT
);

-- Indexes for blockheaders
CREATE INDEX IF NOT EXISTS idx_blockheaders_base_fee
    ON public.blockheaders (base_fee_per_gas);
CREATE INDEX IF NOT EXISTS idx_blockheaders_number
    ON public.blockheaders (number);
CREATE INDEX IF NOT EXISTS idx_blockheaders_number_base_fee
    ON public.blockheaders (number, base_fee_per_gas);
CREATE INDEX IF NOT EXISTS idx_blockheaders_timestamp
    ON public.blockheaders ("timestamp");

-- Index for transactions
CREATE INDEX IF NOT EXISTS transactions_blocknumber_idx
    ON public.transactions (block_number);

-- Permissions
REVOKE ALL ON public.blockheaders FROM matt_readonly_user;
GRANT SELECT ON public.blockheaders TO matt_readonly_user;

REVOKE ALL ON public.transactions FROM matt_readonly_user;
GRANT SELECT ON public.transactions TO matt_readonly_user;

-- Down migration: No action needed
