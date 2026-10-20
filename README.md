# Tenants ERP

Multi-tenant ERP platform. Monorepo with two workspaces:

```
backend/    Rust (Axum + SQLx) API, serves the built frontend too
frontend/   React + TypeScript + Vite + Tailwind SPA
```

Backend code follows DDD. Each business module under `backend/src/modules/`
has the same three layers:

```
domain.rs           types and business rules
application.rs      use cases, called by the HTTP layer
infrastructure/     Postgres queries, HTTP handlers, outside clients
```

A layer starts as one file and becomes a folder once it outgrows it.

`platform/` holds shared technical pieces (database pool, error responses,
tokens, passwords, and `permissions`, which every module checks access
through)
and `shared_kernel/` holds types every module uses, like `TenantId`.

Modules so far:

- `tenancy`: which tenant this deployment is, asked from the admin service
- `employees`: employees and sign-in

`debt_core` and `dialer_integration` are laid out but still empty.

## Employees and permissions

The first employee is the **executive manager**, created on startup from
`EXECUTIVE_MANAGER_EMAIL` / `EXECUTIVE_MANAGER_PASSWORD` while the tenant
has no employees (see `.env.example`). They can do everything, now and in
any module added later, and they are the only one who sets other
employees' permissions. They can't be deleted or deactivated.

Every other employee starts with no access. Permissions are create / read /
update / delete flags per resource (only `employees` exists so far):

```
PUT /api/v1/employees/{id}/permissions
[{ "resource": "employees", "create": false, "read": true, "update": false, "delete": false }]
```

Sign-in works like the admin service: `POST /api/v1/auth/login` returns an
opaque token, sent back as `Authorization: Bearer <token>`.
`GET /api/v1/auth/me` returns the employee with their effective permissions.

| route | needs |
| --- | --- |
| `GET /api/v1/employees`, `GET /api/v1/employees/{id}` | employees: read |
| `POST /api/v1/employees` | employees: create |
| `PATCH /api/v1/employees/{id}` (`full_name`, `is_active`) | employees: update |
| `DELETE /api/v1/employees/{id}` | employees: delete |
| `PUT /api/v1/employees/{id}/permissions` | executive manager |

## Run everything (recommended)

Requires Podman and [podman-compose](https://github.com/containers/podman-compose)
(`sudo pacman -S podman-compose` on Arch). All images come from `quay.io`
and Red Hat's registry (`registry.access.redhat.com`).

### Start

```bash
cd backend
cp .env.example .env   # adjust credentials if you want; defaults work locally
podman-compose up -d --build
```

The first build takes several minutes (image pulls + a full Rust release
compile); later builds reuse the cargo cache mounts.

Check it's running:

```bash
podman-compose ps            # postgres/redis should be healthy before app starts
podman-compose logs -f app
```

This starts three containers on one Podman network:

| service    | what                                              | port  |
| ---------- | ------------------------------------------------- | ----- |
| `postgres` | `debt_collection` database                         | 5432  |
| `redis`    | token / session store, AOF persistence             | 6379  |
| `app`      | backend + built frontend, one image, one process   | 8080  |

The `Containerfile` lives at the repo root because the image needs both
`backend/` and `frontend/`; `backend/compose.yaml` builds it with the
root as context.

Once it's up: **http://localhost:8080** serves the SPA, and the API lives
under `http://localhost:8080/api/v1`.

Tear down with `podman-compose down` (add `-v` to also drop the Postgres
and Redis volumes).

## Run for local development (hot reload)

Keep Postgres/Redis in Podman, run backend and frontend natively so you
get incremental compiles and Vite's dev server instead of a full image
rebuild per change.

```bash
# 1. Data services only
cd backend && podman-compose up -d postgres redis

# 2. Backend (new terminal)
cd backend && cargo run

# 3. Frontend (new terminal) — proxies /api/v1 to localhost:8080, see
#    frontend/vite.config.ts
cd frontend && npm install && npm run dev
```

Frontend dev server: **http://localhost:5173**. Backend directly:
**http://localhost:8080**.

## Tests

```bash
cd backend && cargo test
```

Integration tests use `#[sqlx::test]`, which creates a throwaway database
per test, so `DATABASE_URL` (from `.env`) must point at a reachable Postgres
whose role has `CREATEDB`:

```sql
ALTER ROLE debt_admin CREATEDB;
```

## Individual container builds

```bash
# from the repo root
podman build -t tenants:local .
podman run --rm -p 8080:8080 \
  -e DATABASE_URL=postgres://debt_admin:debt_dev_password@host.containers.internal:5432/debt_collection \
  -e REDIS_URL=redis://:debt_dev_password@host.containers.internal:6379 \
  tenants:local
```

(Podman resolves `host.containers.internal` to the host automatically —
needed here since Postgres/Redis are running as separate containers, not
inside this one.)

Runs as non-root (`uid=10001`) on a UBI 10 minimal base.
