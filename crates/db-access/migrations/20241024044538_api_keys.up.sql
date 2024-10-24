-- Create api_keys table if it doesn't exist
CREATE TABLE IF NOT EXISTS public.api_keys (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT api_keys_key_key UNIQUE (key)
);

-- Set the owner of the api_keys table
ALTER TABLE IF EXISTS public.api_keys
    OWNER TO postgres;
