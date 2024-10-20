-- Add up migration script here
CREATE TABLE api_keys (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
-- Add down migration script here
DROP TABLE api_keys;