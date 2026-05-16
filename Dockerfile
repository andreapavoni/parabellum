FROM oven/bun:1.2.20 AS bun

# Build stage
FROM rust:1.91.1-slim-trixie AS builder
WORKDIR /app

COPY --from=bun /usr/local/bin/bun /usr/local/bin/bun
COPY --from=bun /usr/local/bin/bunx /usr/local/bin/bunx

ENV SQLX_OFFLINE=true
ENV SKIP_FRONTEND=

COPY . .
RUN bun install --frozen-lockfile
RUN cargo build --release --locked \
    --bin parabellum \
    --bin parabellum-seed \
    --bin parabellum-replay

# Runtime stage
FROM bitnami/minideb:trixie

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 parabellum
WORKDIR /app
RUN mkdir -p /app/logs /app/frontend/assets /app/frontend/static && \
    chown -R parabellum:parabellum /app
USER parabellum

COPY --from=builder /app/target/release/parabellum /app/parabellum
COPY --from=builder /app/target/release/parabellum-seed /app/parabellum-seed
COPY --from=builder /app/target/release/parabellum-replay /app/parabellum-replay
COPY --from=builder /app/frontend/assets /app/frontend/assets
COPY --from=builder /app/frontend/static /app/frontend/static

ENV PORT=8080
EXPOSE 8080

CMD ["./parabellum"]
