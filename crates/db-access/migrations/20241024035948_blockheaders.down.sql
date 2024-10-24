-- Drop the indexes if they exist
DROP INDEX IF EXISTS public.idx_blockheaders_base_fee;
DROP INDEX IF EXISTS public.idx_blockheaders_number;
DROP INDEX IF EXISTS public.idx_blockheaders_number_base_fee;
DROP INDEX IF EXISTS public.idx_blockheaders_timestamp;

-- Drop the blockheaders table if it exists
DROP TABLE IF EXISTS public.blockheaders;
