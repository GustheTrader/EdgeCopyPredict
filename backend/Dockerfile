# ---------- Build stage ----------
FROM rust:1.78-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --bin arbitek-api --bin arbitek-worker

# ---------- Runtime stage ----------
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/arbitek-api /usr/local/bin/arbitek-api
COPY --from=builder /app/target/release/arbitek-worker /usr/local/bin/arbitek-worker
EXPOSE 8080
CMD ["arbitek-api"]
