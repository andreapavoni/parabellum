# Build stage
FROM rust:1.85 as builder
WORKDIR /app

# FIXME: ignore files from .gitignore
COPY . .

RUN cargo build --release -p parabellum_server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*

RUN useradd -m parabellum
USER parabellum

WORKDIR /app
COPY --from=builder /app/target/release/parabellum_server /app/parabellum_server

COPY --from=builder /app/migrations ./migrations
# FIXME: adjust frontend assets paths
COPY --from=builder /app/parabellum_web ./parabellum_web

ENV PORT=8080
EXPOSE 8080

CMD ["./parabellum_server"]
