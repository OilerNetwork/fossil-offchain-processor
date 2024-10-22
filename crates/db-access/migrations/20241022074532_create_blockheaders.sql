-- Up: Create blockheaders table if it doesn’t exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_tables WHERE tablename = 'blockheaders'
    ) THEN
        CREATE TABLE public.blockheaders (
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
            CONSTRAINT blockheaders_pkey PRIMARY KEY ("number"),
            CONSTRAINT blockheaders_number_key UNIQUE ("number")
        );
    END IF;
END $$;

-- Create Indexes if they don’t exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes WHERE indexname = 'idx_blockheaders_base_fee'
    ) THEN
        CREATE INDEX idx_blockheaders_base_fee ON public.blockheaders (base_fee_per_gas ASC NULLS LAST);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes WHERE indexname = 'idx_blockheaders_number'
    ) THEN
        CREATE INDEX idx_blockheaders_number ON public.blockheaders (number ASC NULLS LAST);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes WHERE indexname = 'idx_blockheaders_timestamp'
    ) THEN
        CREATE INDEX idx_blockheaders_timestamp ON public.blockheaders ("timestamp" ASC NULLS LAST);
    END IF;
END $$;
