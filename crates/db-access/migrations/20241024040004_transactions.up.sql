-- Create transactions table if it doesn't exist
CREATE TABLE IF NOT EXISTS public.transactions (
    block_number BIGINT,
    transaction_hash CHAR(66) NOT NULL,
    transaction_index INTEGER NOT NULL,
    from_addr CHAR(42),
    to_addr CHAR(42),
    value VARCHAR(78) NOT NULL,
    gas_price VARCHAR(78) NOT NULL,
    max_priority_fee_per_gas VARCHAR(78),
    max_fee_per_gas VARCHAR(78),
    gas VARCHAR(78) NOT NULL,
    chain_id VARCHAR(78),
    CONSTRAINT transactions2_pkey PRIMARY KEY (transaction_hash),
    CONSTRAINT transactions_block_number_fkey FOREIGN KEY (block_number)
        REFERENCES public.blockheaders ("number") MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE RESTRICT
);

-- Set the owner of the table
ALTER TABLE IF EXISTS public.transactions
    OWNER TO postgres;

-- Create an index on block_number for the transactions table
CREATE INDEX IF NOT EXISTS transactions_blocknumber_idx
    ON public.transactions USING btree (block_number ASC NULLS LAST);
