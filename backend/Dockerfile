# syntax=docker/dockerfile:1.7
#
# Build context is the MONOREPO ROOT, not this backend/ directory, since
# this now bundles the sibling frontend/ workspace. Build from the repo
# root with:
#   docker build -f backend/Dockerfile -t tenants:local .
# (docker-compose.yml only orchestrates Postgres/Redis and is unaffected.)

########################################################################
# Stage 1/4 — Frontend: compile the React/Vite/Tailwind SPA into dist/
########################################################################
FROM node:20-bookworm-slim AS frontend
WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

########################################################################
# Stage 2/4 — Cargo Chef planner: compute the dependency recipe used to
# cache the `cargo build` dependency layer independently of app source
########################################################################
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo chef prepare --recipe-path recipe.json

########################################################################
# Stage 3/4 — Builder: cook the cached dependency layer (rebuilt only when
# Cargo.toml/Cargo.lock change), then compile the Axum backend and bundle
# the compiled SPA assets next to the release binary
########################################################################
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN cargo build --release --bin tenants

COPY --from=frontend /app/frontend/dist ./web/dist

########################################################################
# Stage 4/4 — Runtime: minimal, non-root Debian Bookworm slim image
########################################################################
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 tenants \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin tenants

WORKDIR /app

COPY --from=builder /app/target/release/tenants ./tenants
COPY --from=builder /app/web/dist ./web/dist
COPY --from=builder /app/migrations ./migrations

RUN chown -R tenants:tenants /app

USER 10001:10001

ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080

ENTRYPOINT ["/app/tenants"]
