# --- Etapa 1: Compilación ---
FROM rust:1.78-slim AS builder

WORKDIR /usr/src/tarascapay

# copiamos los archivos de configuración y fuentes
COPY Cargo.toml Cargo.lock ./
COPY api/ ./api/
COPY shared/ ./shared/
COPY web/ ./web/

# binario de la API en modo release
RUN cargo build --release -p api


FROM debian:bookworm-slim

WORKDIR /app

# Instalamos paquetes mínimos esenciales para seguridad y certificados TLS
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copiamos el ejecutable compilado en la etapa anterior
COPY --from=builder /usr/src/tarascapay/target/release/api /app/tarascapay_api

# Exponemos el puerto nativo de Axum
EXPOSE 3000

# Iniciamos el proceso aislado del backend
CMD ["./tarascapay_api"]