#!/bin/bash
# Analytics Service Setup Script
# Sets up PostgreSQL database and runs migrations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}X3 Chain Analytics Service Setup${NC}"
echo -e "${GREEN}========================================${NC}"

# Default configuration
DB_USER="${DB_USER:-postgres}"
DB_PASSWORD="${DB_PASSWORD:-password}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-analytics}"

# Check if PostgreSQL is running
echo -e "\n${YELLOW}Checking PostgreSQL connection...${NC}"
if ! pg_isready -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" > /dev/null 2>&1; then
    echo -e "${RED}PostgreSQL is not running or not accessible at $DB_HOST:$DB_PORT${NC}"
    echo -e "Please start PostgreSQL or update connection settings."
    echo -e "\nTo start PostgreSQL with Docker:"
    echo -e "  docker run -d --name x3-postgres -p 5432:5432 -e POSTGRES_PASSWORD=password postgres:15"
    exit 1
fi
echo -e "${GREEN}PostgreSQL is running.${NC}"

# Create database if it doesn't exist
echo -e "\n${YELLOW}Creating database '$DB_NAME' if not exists...${NC}"
PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -tc \
    "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" | grep -q 1 || \
    PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -c \
    "CREATE DATABASE $DB_NAME"
echo -e "${GREEN}Database '$DB_NAME' is ready.${NC}"

# Run migrations
echo -e "\n${YELLOW}Running migrations...${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATIONS_DIR="$SCRIPT_DIR/migrations"

if [ -d "$MIGRATIONS_DIR" ]; then
    for migration in "$MIGRATIONS_DIR"/*.sql; do
        if [ -f "$migration" ]; then
            echo -e "  Running: $(basename "$migration")"
            PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$migration"
        fi
    done
    echo -e "${GREEN}Migrations completed.${NC}"
else
    echo -e "${RED}Migrations directory not found at $MIGRATIONS_DIR${NC}"
    exit 1
fi

# Create .env file if it doesn't exist
ENV_FILE="$SCRIPT_DIR/.env"
if [ ! -f "$ENV_FILE" ]; then
    echo -e "\n${YELLOW}Creating .env file...${NC}"
    cat > "$ENV_FILE" << EOF
HOST=127.0.0.1
PORT=8080
DATABASE_URL=postgres://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME
DATABASE_POOL_SIZE=16
RUST_LOG=info,analytics_service=debug
EOF
    echo -e "${GREEN}.env file created.${NC}"
fi

# Verify setup
echo -e "\n${YELLOW}Verifying setup...${NC}"
TABLE_COUNT=$(PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -tAc \
    "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'")
echo -e "${GREEN}Found $TABLE_COUNT tables in database.${NC}"

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "\nTo start the analytics service:"
echo -e "  cd $SCRIPT_DIR"
echo -e "  cargo run --release"
echo -e "\nAPI will be available at: http://127.0.0.1:8080"
echo -e "Health check: http://127.0.0.1:8080/health"
echo -e "\nDocumentation:"
echo -e "  POST /api/v1/events        - Record an event"
echo -e "  GET  /api/v1/events        - List events"
echo -e "  GET  /api/v1/metrics/summary - Get metrics summary"
echo -e "  GET  /api/v1/comits/stats  - Get comit statistics"
