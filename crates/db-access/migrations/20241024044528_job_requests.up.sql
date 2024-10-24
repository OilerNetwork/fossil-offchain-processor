-- Create job_requests table if it doesn't exist
CREATE TABLE IF NOT EXISTS public.job_requests (
    job_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(20) NOT NULL,
    result JSONB,
    CONSTRAINT job_requests_pkey PRIMARY KEY (job_id),
    CONSTRAINT job_requests_status_check CHECK (
        status::TEXT = ANY (ARRAY['Completed'::TEXT, 'Pending'::TEXT, 'Failed'::TEXT])
    )
);

-- Set the owner of the job_requests table
ALTER TABLE IF EXISTS public.job_requests
    OWNER TO postgres;
