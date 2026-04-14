# rust_queue

A distributed task queue built with Rust, Axum, and PostgreSQL.

Uses `SELECT FOR UPDATE SKIP LOCKED` for concurrent job claiming — no Redis or RabbitMQ needed. Postgres handles the queue, the locking, and the persistence.

## What it does

- Submit jobs via HTTP API with priority and scheduling
- Background workers poll and execute jobs concurrently
- Exponential backoff retries (2s, 4s, 8s, ...) with configurable max attempts
- Jobs that exhaust retries land in a `dead` state for review
- Stale job reaper recovers work from crashed workers
- Graceful shutdown — workers finish in-flight jobs before exiting
- Auth (JWT + cookies) and dashboard endpoints

## Stack

- **Axum** — HTTP server
- **sqlx** — Postgres queries (compile-time checked)
- **Tokio** — async runtime, background task spawning
- **PostgreSQL** — queue backend, job persistence
- **tracing** — structured logging

## Running locally

Start Postgres:

```bash
docker compose up -d
```

Copy `.env.example` to `.env` (or use the defaults), then:

```bash
cd apps/backend
cargo run
```

You should see workers start polling and the server listening on `:8000`.

Swagger docs at [http://localhost:8000/docs](http://localhost:8000/docs).

## Submitting jobs

```bash
# register + grab token
curl -s localhost:8000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@test.com","name":"Test","password":"password123"}' | jq .data.access_token

# submit a job
curl -s localhost:8000/api/jobs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"job_type":"flaky_task","priority":3,"max_retries":5}' | jq
```

Job types included for demo: `fast_task`, `slow_task`, `flaky_task`, `critical_report`. These simulate varying durations and failure rates.

## Tests

Requires Postgres running on localhost:5432.

```bash
cargo test
```

Each test gets its own database, so they don't interfere with each other or your dev data.

## How the queue works

1. Producer inserts a row into `jobs` with status `pending`
2. Worker runs `SELECT ... FOR UPDATE SKIP LOCKED` to atomically claim a job
3. Job transitions to `running`, worker executes the handler
4. On success → `completed`. On failure → back to `pending` with backoff delay, or `dead` if retries exhausted
5. Stale job reaper periodically finds orphaned `running` jobs and resets them

The partial index `WHERE status = 'pending'` on the jobs table keeps polling fast regardless of how many completed jobs accumulate.

## Project structure

```
apps/backend/
├── src/
│   ├── api/          # HTTP handlers (auth, users, jobs)
│   ├── middleware/    # JWT auth middleware
│   ├── models/       # Request/response types, DB entities
│   ├── repository/   # Database queries
│   ├── services/     # JWT, password hashing
│   ├── worker/       # Job handler trait, registry, polling loop, reaper
│   ├── config.rs
│   ├── error.rs
│   ├── state.rs
│   └── main.rs
├── migrations/
└── tests/
```
