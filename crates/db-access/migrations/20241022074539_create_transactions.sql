-- Up: Create transactions table and index
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
        REFERENCES public.blockheaders ("number") ON UPDATE NO ACTION ON DELETE RESTRICT
);
ALTER TABLE IF EXISTS public.transactions OWNER TO postgres;

CREATE INDEX IF NOT EXISTS transactions_blocknumber_idx ON public.transactions (block_number ASC NULLS LAST);

