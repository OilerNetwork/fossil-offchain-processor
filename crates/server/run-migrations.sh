#!/bin/bash
set -e

echo "Waiting for database to be ready..."
until sqlx database create && sqlx migrate run; do
  >&2 echo "Database is unavailable - sleeping"
  sleep 1
done

echo "Migrations completed successfully"
