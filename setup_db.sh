#!/bin/sh

# docker-compose up -d

DATABASE_URL=$(grep -e '^DATABASE_URL' .env | cut -d '=' -f2)
DATABASE_NAME=$(echo $DATABASE_URL | cut -d '/' -f4)

docker exec -it parabellum_db dropdb -U parabellum $DATABASE_NAME --if-exists
docker exec -it parabellum_db createdb -U parabellum $DATABASE_NAME

diesel migration run
