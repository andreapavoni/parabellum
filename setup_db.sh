#!/bin/sh

# Exit immediately if a command exits with a non-zero status.
set -e

# Load environment variables from .env file
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
else
  echo ".env file not found"
  exit 1
fi

DATABASE_NAME=$(echo $DATABASE_URL | cut -d '/' -f4)

echo "--- Dropping database '$DATABASE_NAME' (if exists) ---"
docker exec -it parabellum_db dropdb -U parabellum $DATABASE_NAME --if-exists

echo "--- Creating database '$DATABASE_NAME' ---"
docker exec -it parabellum_db createdb -U parabellum $DATABASE_NAME

echo "--- Running migrations on '$DATABASE_NAME' ---"
# Explicitly pass the database URL to the command
sqlx migrate run --database-url "$DATABASE_URL"

echo "--- Database setup complete! ---"
