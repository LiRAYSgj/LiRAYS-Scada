# =================== Stage 1: Frontend build with Node.js 24 ===================
FROM node:24-alpine AS frontend-builder

WORKDIR /app

COPY frontend ./frontend
COPY proto ./proto

RUN \
  apk add --no-cache protobuf \
  && cd frontend \
  && npm install \
  && npm run generate:proto \
  && npm run build

# =================== Stage 2: Rust backend build ===================
FROM rust:1.94-alpine AS backend-builder

WORKDIR /app

RUN apk add --no-cache musl-dev openssl-dev protobuf && mkdir /app/frontend

COPY --from=frontend-builder /app/frontend/build ./frontend/build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY proto ./proto
COPY build.rs .

RUN cargo build --release

# =================== Stage 3: Final runtime image ===================
FROM alpine:latest AS production

ENV DATA_DIR="/data"

RUN \
  adduser -D -u 1000 appuser \
  && mkdir -p /data \
  && chown -R appuser:appuser /data

WORKDIR /app

COPY --from=backend-builder /app/target/release/lirays-scada ./lirays-scada

EXPOSE 8245
EXPOSE 8246

CMD ["./lirays-scada"]
