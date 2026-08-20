# Imagen única: la API sirve además el SPA ya compilado.
# Se construye desde la raíz del repositorio:
#   docker build -f docker/Dockerfile -t tarascapay .

# --- Etapa 1: frontend ---
FROM node:24-slim AS web-builder

WORKDIR /build

# Primero las dependencias: si no cambian, esta capa se reutiliza.
COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

# --- Etapa 2: API ---
FROM rust:1.90-slim AS api-builder

WORKDIR /usr/src/tarascapay

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY api/ ./api/
COPY shared/ ./shared/

RUN cargo build --release -p api

# --- Etapa 3: runtime ---
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home tarasca

COPY --from=api-builder /usr/src/tarascapay/target/release/api /app/tarascapay-api
COPY --from=web-builder /build/dist /app/web
COPY api/migrations /app/migrations

# Los comprobantes viven acá; montá un volumen para que sobrevivan al contenedor.
RUN mkdir -p /app/uploads && chown -R tarasca:tarasca /app

USER tarasca

ENV HOST=0.0.0.0 \
    PORT=3000 \
    UPLOAD_DIR=/app/uploads \
    WEB_DIST=/app/web

EXPOSE 3000

CMD ["/app/tarascapay-api"]
