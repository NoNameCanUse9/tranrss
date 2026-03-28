# --- 1. Frontend Build Stage ---
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
# Enable pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate
COPY frontend/package.json frontend/pnpm-lock.yaml ./
# Install all dependencies including devDependencies to support build
RUN pnpm install --frozen-lockfile --production=false
COPY frontend .
# Build the frontend
RUN pnpm build

# --- 2. Backend Build Stage ---
FROM rust:alpine AS backend-builder
# Install build tools and libraries for musl compatibility
RUN apk add --no-cache musl-dev gcc make openssl-dev openssl-libs-static pkgconfig
WORKDIR /app/backend
# Copy backend source
COPY backend .
# Copy built frontend assets to where backend expects them (for rust-embed)
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
# Build backend with embed-frontend feature
RUN cargo build --release --features embed-frontend

# --- 3. Final Stage ---
FROM alpine:3.20
RUN apk add --no-cache ca-certificates openssl tzdata
WORKDIR /app
RUN mkdir -p /app/data && chmod 777 /app/data
# Copy binary and migrations
COPY --from=backend-builder /app/backend/target/release/tranrss-backend /app/tranrss
COPY --from=backend-builder /app/backend/migrations /app/migrations

ENV DATABASE_URL=sqlite:/app/data/data.database
ENV TZ=Asia/Shanghai
EXPOSE 8000

CMD ["./tranrss"]
