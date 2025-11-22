# Build stage
FROM rust:1.91.1-slim-trixie as builder
WORKDIR /app

COPY . .

RUN cargo build --release

# Runtime stage
FROM bitnami/minideb:trixie

RUN apt-get update && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*

RUN useradd -m parabellum
USER parabellum

WORKDIR /app
COPY --from=builder /app/target/release/parabellum /app/parabellum
COPY --from=builder /app/frontend/assets /app/frontend/assets

ENV PORT=8080
EXPOSE 8080

CMD ["./parabellum"]
