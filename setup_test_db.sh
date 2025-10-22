#!/bin/sh

# docker-compose up -d

# Exit immediately if a command exits with a non-zero status.
set -e

# Load environment variables from .env file
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
else
  echo ".env file not found"
  exit 1
fi

DATABASE_NAME=$(echo $TEST_DATABASE_URL | cut -d '/' -f4)

docker exec -it parabellum_db dropdb -U parabellum $DATABASE_NAME --if-exists
docker exec -it parabellum_db createdb -U parabellum $DATABASE_NAME
sqlx migrate run --database-url "$TEST_DATABASE_URL"
reset
exit 0
