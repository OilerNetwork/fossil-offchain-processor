CREATE TABLE job_requests (
    job_id VARCHAR(255) PRIMARY KEY,  -- Store job ID as a string
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- Store timestamp
    status VARCHAR(20) NOT NULL  -- Store job status with 3 possible states
        CHECK (status IN ('Completed', 'Pending', 'Failed'))
);
