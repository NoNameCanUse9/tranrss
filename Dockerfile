# --- 1. Frontend Build Stage ---
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
# 启用 pnpm 并强制拉取最新依赖
RUN corepack enable && corepack prepare pnpm@latest --activate
COPY frontend/package.json frontend/pnpm-lock.yaml ./
# 关键：强制安装包括 devDependencies 在内的所有依赖，以支持编译
RUN pnpm install --frozen-lockfile --production=false
COPY frontend .
# 执行编译
RUN pnpm build

# --- 2. Backend Build Stage ---
FROM rust:alpine AS backend-builder
# 安装最新编译工具链和 musl 兼容库
RUN apk add --no-cache musl-dev gcc make openssl-dev openssl-libs-static pkgconfig
WORKDIR /app/backend
# 全量拷贝并开启内嵌前端特性构建
COPY backend .
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
RUN cargo build --release --features embed-frontend

# --- 3. Final Stage ---
FROM alpine:3.20
RUN apk add --no-cache ca-certificates openssl tzdata
WORKDIR /app
RUN mkdir -p /app/data && chmod 777 /app/data
COPY --from=backend-builder /app/backend/target/release/tranrss-backend /app/tranrss
COPY --from=backend-builder /app/backend/migrations /app/migrations

ENV DATABASE_URL=sqlite:/app/data/data.database
EXPOSE 8000

CMD ["./tranrss"]
