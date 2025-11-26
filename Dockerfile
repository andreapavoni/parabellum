# Build stage
FROM rust:1.91.1-slim-trixie as builder

# Caching Rust strategy from https://depot.dev/docs/container-builds/optimal-dockerfiles/rust-dockerfile
# Need that for sccache
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef sccache --locked
ENV RUSTC_WRAPPER=sccache \
    SCCACHE_DIR=/sccache

WORKDIR /app

# We need just Cargo files for chef recipe.json
# Don't COPY all code as .
COPY Cargo.toml Cargo.lock ./

# COPY */Cargo.toml ./*/ doesn't work
COPY parabellum_app/Cargo.toml ./parabellum_app/
COPY parabellum_db/Cargo.toml ./parabellum_db/
COPY parabellum_game/Cargo.toml ./parabellum_game/
COPY parabellum_server/Cargo.toml ./parabellum_server/
COPY parabellum_types/Cargo.toml ./parabellum_types/
COPY parabellum_web/Cargo.toml ./parabellum_web/

# Make cache magic
RUN cargo chef prepare --recipe-path recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

# Now we can copy everything
COPY . .

ENV SQLX_OFFLINE=true

# Build with cached deps
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release

# Runtime stage
FROM bitnami/minideb:trixie

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m parabellum
RUN mkdir -p /app/logs && chown -R parabellum:parabellum /app
USER parabellum

WORKDIR /app
COPY --from=builder /app/target/release/parabellum /app/parabellum
COPY --from=builder /app/frontend/assets /app/frontend/assets

ENV PORT=8080
EXPOSE 8080

CMD ["./parabellum"]
