-- Up: Create api_keys table
CREATE TABLE IF NOT EXISTS public.api_keys (
    id integer NOT NULL DEFAULT nextval('api_keys_id_seq'::regclass),
    key text NOT NULL,
    name character varying(255) NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT now(),
    CONSTRAINT api_keys_pkey PRIMARY KEY (id),
    CONSTRAINT api_keys_key_key UNIQUE (key)
);
ALTER TABLE IF EXISTS public.api_keys OWNER TO postgres;


