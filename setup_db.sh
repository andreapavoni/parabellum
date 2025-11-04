#!/bin/sh

set -e

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
else
  echo ".env file not found"
  exit 1
fi

DATABASE_NAME=$(echo $DATABASE_URL | cut -d '/' -f4)
TEST_DATABASE_NAME=$(echo $TEST_DATABASE_URL | cut -d '/' -f4)

docker exec -it parabellum_db dropdb -U parabellum $DATABASE_NAME --if-exists
docker exec -it parabellum_db createdb -U parabellum $DATABASE_NAME
sqlx migrate run --database-url "$DATABASE_URL"

docker exec -it parabellum_db dropdb -U parabellum $TEST_DATABASE_NAME --if-exists
docker exec -it parabellum_db createdb -U parabellum $TEST_DATABASE_NAME
sqlx migrate run --database-url "$TEST_DATABASE_URL"
