# Check manuat at https://just.systems/man/en/
default: bacon

bacon: start_db
    bacon

run: start_db
    cargo run --release

debug: start_db
    cargo run

dev: start_db
    #!/usr/bin/env bash
    trap 'kill $(jobs -p) 2>/dev/null' EXIT
    bun run build:dev &
    SKIP_FRONTEND=1 cargo run

start_db:
    docker-compose up -d db

start_app:
    docker-compose up app

start_all: start_db start_app

stop_all:
    docker-compose stop

setup_db:
    ./setup-db

test:
    cargo test --release -- --test-threads=1
