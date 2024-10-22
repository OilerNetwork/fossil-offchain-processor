-- Up: Create job_requests table
CREATE TABLE IF NOT EXISTS public.job_requests (
    job_id character varying(255) NOT NULL,
    created_at timestamp without time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status character varying(20) NOT NULL,
    CONSTRAINT job_requests_pkey PRIMARY KEY (job_id),
    CONSTRAINT job_requests_status_check CHECK (status = ANY (ARRAY['Completed', 'Pending', 'Failed']))
);
ALTER TABLE IF EXISTS public.job_requests OWNER TO postgres;

-- Down: Drop job_requests table
DROP TABLE IF EXISTS public.job_requests;
