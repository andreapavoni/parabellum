# Check manuat at https://just.systems/man/en/
default: run

run:
    cargo run --release

debug:
    cargo run

dev:
    #!/usr/bin/env bash
    trap 'kill $(jobs -p) 2>/dev/null' EXIT
    bun run build:dev &
    SKIP_FRONTEND=1 cargo run

# db
# app
start target:
    docker-compose up -d {{target}}

start-all:
    docker-compose up -d

stop-all:
    docker-compose stop

setup-db:
    ./setup-db

test:
    cargo test --release -- --test-threads=1
