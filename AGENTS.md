# welfare-store / mcx-system

## Project Overview

This repository is a welfare store platform. The main backend service lives in
`mcx-system/` and is a Rust API for the mini app and admin system.

The backend uses:

- `axum` for HTTP routing
- `sqlx` with MySQL for persistence
- `redis` for cache/session-style state
- `jsonwebtoken` and `bcrypt` for auth
- OSS integration for file upload
- JK Pay integration for payment flows

The repo also contains a `frontend/` app and deployment helpers under
`mcx-system/`.

## Important Paths

- `mcx-system/src/main.rs` - service entrypoint
- `mcx-system/src/config.rs` - environment configuration
- `mcx-system/src/state.rs` - shared app state
- `mcx-system/src/routes/` - API routes
- `mcx-system/src/models/` - database models
- `mcx-system/migrations/` - SQL migrations
- `deploy/` - all release scripts live here
- `deploy.ps1` - single entrypoint for backend/frontend publishing

## Deployment Notes

- Prefer `deploy.ps1` for release work.
- `deploy.ps1 -Target backend` publishes the backend service.
- `deploy.ps1 -Target frontend` publishes the frontend dist bundle.
- `deploy.ps1 -Target all` runs backend then frontend.
- Default remote deployment values used by the script:
  - host: `47.103.220.84`
  - user: `root`
  - deploy dir: `/root/workspace/mcx`
  - port: `8081`
- SSH passwords must not be hardcoded in scripts; deployment helpers read them
  from the environment variable `DEPLOY_SSH_PASSWORD` only.

## Housekeeping

- Generated artifacts such as `target/`, `logs/`, and `cargo-run.*` files are
  disposable.
- The old `.Codex/` helper tree has been retired in favor of this file.
- If a task touches deployment behavior, update `deploy.ps1` first so the
  workflow stays in one place.
