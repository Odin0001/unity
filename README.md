# Tenants ERP

Multi-tenant ERP platform. Monorepo with two workspaces:

```
backend/    Rust (Axum + SQLx + Redis) API, serves the built frontend too
frontend/   React + TypeScript + Vite + Tailwind SPA
```

`backend/src/modules/` and `frontend/src/modules/` are currently empty —
they're where feature modules (inventory, manufacturing, ...) land as
they're built, one directory per module, each following the
`domain/application/infrastructure` (backend) or co-located
components/hooks (frontend) pattern already established in `platform/`.

## Run everything (recommended)

Requires Docker.

```bash
cd backend
cp .env.example .env   # adjust credentials if you want; defaults work locally
docker compose up -d --build
```

This starts three containers on one Docker network:

| service    | what                                              | port  |
| ---------- | ------------------------------------------------- | ----- |
| `postgres` | `erp_master_db` + `erp_tenant_template_db`        | 5432  |
| `redis`    | opaque token / session store, AOF persistence      | 6379  |
| `app`      | backend + built frontend, one image, one process   | 8080  |

The `app` image is built from the monorepo root (it needs both
`backend/` and `frontend/`) even though the compose file lives in
`backend/` — see the comment on the `app` service in
`backend/docker-compose.yml` if you're touching that build config.

Once it's up: **http://localhost:8080** serves the SPA, and the API lives
under `http://localhost:8080/api/v1`.

Tear down with `docker compose down` (add `-v` to also drop the Postgres
and Redis volumes, e.g. after a migration reset).

## Run for local development (hot reload)

Keep Postgres/Redis in Docker, run backend and frontend natively so you
get incremental compiles and Vite's dev server instead of a full image
rebuild per change.

```bash
# 1. Data services only
cd backend && docker compose up -d postgres redis

# 2. Backend (new terminal)
cd backend && cargo run

# 3. Frontend (new terminal) — proxies /api/v1 to localhost:8080, see
#    frontend/vite.config.ts
cd frontend && npm install && npm run dev
```

Frontend dev server: **http://localhost:5173**. Backend directly:
**http://localhost:8080**.

## Known gap

The frontend's `AuthContext` (`frontend/src/platform/auth/AuthContext.tsx`)
calls `POST /api/v1/auth/login` and `GET /api/v1/session`, but the backend
doesn't implement either route yet — it only has the `OpaqueAuth` /
`RequireModule` extractors that *validate* an already-issued token. Signing
in won't work end-to-end until a login handler (that writes a session into
Redis) and a session-introspection endpoint are added on the backend.

## Individual container builds

```bash
# from the monorepo root
docker build -f backend/Dockerfile -t tenants:local .
docker run --rm -p 8080:8080 \
  --add-host=host.docker.internal:host-gateway \
  -e DATABASE_URL=postgres://erp_admin:erp_dev_password@host.docker.internal:5432/erp_master_db \
  -e REDIS_URL=redis://:erp_dev_password@host.docker.internal:6379 \
  tenants:local
```

(`--add-host` is what makes `host.docker.internal` resolve on native Linux
Docker, not just Docker Desktop — needed here since Postgres/Redis are
running as separate containers, not inside this one.)

Runs as non-root (`uid=10001`), Debian bookworm-slim base.
