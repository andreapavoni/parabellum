#!/bin/sh

# docker-compose up -d
docker exec -it parabellum_db dropdb -U parabellum parabellum_test --if-exists
docker exec -it parabellum_db createdb -U parabellum parabellum_test

DATABASE_URL=$(grep TEST_DATABASE_URL .env | cut -d '=' -f2) sqlx database reset
