-- Add migration script here
CREATE TYPE job_status AS ENUM (
    'pending',
    'running',
    'completed',
    'dead',
    'cancelled'
);

CREATE TABLE jobs (
    -- identity
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type      VARCHAR(255) NOT NULL,        -- e.g. "send_email", "generate_report"
    payload       JSONB NOT NULL DEFAULT '{}',   -- arguments for the handler

    -- lifecycle
    status        job_status NOT NULL DEFAULT 'pending',
    priority      SMALLINT NOT NULL DEFAULT 5,   -- 0 = highest, 10 = lowest
    
    -- scheduling & retries
    attempt       INT NOT NULL DEFAULT 0,
    max_retries   INT NOT NULL DEFAULT 3,
    scheduled_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- don't pick up before this time
    
    -- execution tracking
    started_at    TIMESTAMPTZ,
    completed_at  TIMESTAMPTZ,
    locked_by     VARCHAR(255),                  -- worker identifier
    last_error    TEXT,                           -- last failure message

    -- bookkeeping
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- THE critical index: this is what the worker's polling query hits
CREATE INDEX idx_jobs_poll ON jobs (priority ASC, scheduled_at ASC)
    WHERE status = 'pending';

-- For the dashboard: look up jobs by status quickly
CREATE INDEX idx_jobs_status ON jobs (status);

-- For "check my job" API calls
CREATE INDEX idx_jobs_type ON jobs (job_type);