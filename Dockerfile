FROM oven/bun:1.2.20 AS bun

# Build stage
FROM rust:1.95-slim-trixie AS builder
WORKDIR /app

COPY --from=bun /usr/local/bin/bun /usr/local/bin/bun
COPY --from=bun /usr/local/bin/bunx /usr/local/bin/bunx

ENV SQLX_OFFLINE=true

COPY . .
RUN bun install --frozen-lockfile
RUN bun run build:release
RUN cargo build --release \
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
RUN mkdir -p /app/logs /app/mnt
COPY --from=builder /app/target/release/parabellum /app/bin/parabellum
COPY --from=builder /app/target/release/parabellum-seed /app/bin/parabellum-seed
COPY --from=builder /app/target/release/parabellum-replay /app/bin/parabellum-replay
COPY --from=builder /app/seed /app/seed/
COPY --from=builder /app/frontend /app/frontend/

RUN chown -R parabellum:parabellum /app

USER parabellum
ENV PORT=8080
EXPOSE 8080

CMD ["./bin/parabellum"]
