---
name: welfare-store
description: Full-stack welfare store: Vue frontend + Rust/Axum backend, MySQL, JWT auth
type: project
---

# Welfare Store

## Project Type
Full-stack welfare store application

## Project Structure
```
welfare-store/
├── backend/           # Rust/Axum REST API
│   └── src/
│       ├── routes/    # auth.rs, product.rs
│       ├── models/    # admin_user.rs, product.rs
│       └── services/
├── frontend/          # Vue 2 + Element UI Admin Template
└── CLAUDE.md         # Harness knowledge base
```

## Tech Stack
- **Backend**: Axum 0.8 + Tower HTTP + Tokio
- **Frontend**: Vue 2.6 + Element UI + Vuex + Vue Router
- **Database**: MySQL via SQLx with migrations
- **Auth**: JWT + bcrypt

## API Endpoints
- `POST /auth/login` — Admin login
- `POST /auth/refresh` — Refresh JWT
- `POST /auth/logout` — Logout
- `GET /auth/codes` — Get permission codes by role
- `GET/POST /api/products` — List/Create products
- `GET/PUT/DELETE /api/products/{id}` — Product operations

## Roles
- `admin` — Full access
- `operator` — Manage products, orders, inventory
- `viewer` — Read-only

## Important Notes
- JWT secret from environment (JWT_SECRET)
- Admin password from environment (ADMIN_PASSWORD)
- Migrations run automatically on startup
- CORS allows all origins (configure for production)
