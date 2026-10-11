# --- 1. Frontend Build Stage ---
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
# Enable pnpm (固定 10.x：pnpm 11+ 不再读取 package.json 的 pnpm.overrides/supportedArchitectures，
# 会导致 node_modules 与 lockfile 不一致并报 ERR_PNPM_VERIFY_DEPS_BEFORE_RUN)
RUN corepack enable && corepack prepare pnpm@10.30.2 --activate
COPY frontend/package.json frontend/pnpm-lock.yaml ./
# Install all dependencies including devDependencies to support build
RUN pnpm install --production=false
COPY frontend .
# Build the frontend
RUN pnpm build

# --- 2. Backend Build Stage (with cargo-chef for better caching) ---
FROM rust:alpine AS chef
# Install build tools and libraries for musl compatibility
RUN apk add --no-cache musl-dev gcc make openssl-dev openssl-libs-static pkgconfig curl
RUN cargo install cargo-chef
WORKDIR /app/backend

# Planner stage: create the recipe for dependencies
FROM chef AS planner
COPY backend/ .
RUN cargo chef prepare --recipe-path recipe.json

# Cacher stage: build only the dependencies
FROM chef AS backend-builder
COPY --from=planner /app/backend/recipe.json recipe.json
# Build and cache dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Final backend build: compile the actual source
COPY backend/ .
# Copy built frontend assets to where backend expects them (for rust-embed)
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
# Build backend with embed-frontend feature
RUN cargo build --release --features embed-frontend

# --- 3. Final Stage ---
FROM alpine:3.20
RUN apk add --no-cache ca-certificates openssl tzdata curl
WORKDIR /app
RUN mkdir -p /app/data && chmod 777 /app/data
# Copy binary and migrations
COPY --from=backend-builder /app/backend/target/release/tranrss-backend /app/tranrss
COPY --from=backend-builder /app/backend/migrations /app/migrations
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

ENV DATABASE_URL=sqlite:/app/data/data.database
ENV TZ=Asia/Shanghai
EXPOSE 8000

ENTRYPOINT ["/app/entrypoint.sh"]
