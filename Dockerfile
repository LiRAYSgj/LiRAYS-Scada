# =================== Stage 1: Frontend build with Node.js 24 ===================
FROM node:24-alpine AS frontend-builder

WORKDIR /app

COPY frontend ./frontend

ARG FRONTEND_BUILD_NODE_OPTIONS="--max-old-space-size=4096"

RUN \
  cd frontend \
  && npm ci \
  && NODE_OPTIONS="${FRONTEND_BUILD_NODE_OPTIONS}" npm run build

# =================== Stage 2: Rust backend build ====================
FROM rust:1.94-alpine AS backend-builder

WORKDIR /app

RUN apk add --no-cache musl-dev openssl-dev && mkdir /app/frontend

COPY --from=frontend-builder /app/frontend/build ./frontend/build

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# =================== Stage 3: Final runtime image ===================
FROM alpine:latest AS production

ENV DATA_DIR="/data"

RUN \
  adduser -D -u 1000 lirays \
  && mkdir -p /data \
  && chown -R lirays:lirays /data

WORKDIR /app

COPY --from=backend-builder /app/target/release/lirays-scada ./lirays-scada
COPY --from=backend-builder /app/target/release/lirays /usr/local/bin/lirays

USER lirays

ENTRYPOINT ["./lirays-scada"]
CMD []
