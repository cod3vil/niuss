#!/bin/bash

# Clash Configuration Feature Update Script (Simple Version)
# This script updates without rebuilding Docker images

set -e

echo "=== Clash Configuration Feature Update (Simple) ==="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Please run with sudo${NC}"
    exit 1
fi

# Step 1: Pull latest code
echo -e "${BLUE}Step 1: Pulling latest code from Git...${NC}"
git pull origin main
echo -e "${GREEN}✓ Code updated${NC}"
echo ""

# Step 2: Apply database migration using Docker
echo -e "${BLUE}Step 2: Applying database migration...${NC}"
echo -e "${YELLOW}This will create 3 new tables: clash_proxies, clash_proxy_groups, clash_rules${NC}"
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Migration cancelled"
    exit 1
fi

DB_NAME="vpn_platform"
DB_USER="vpn_user"

echo "Running migration via Docker..."

# Copy migration file to postgres container
POSTGRES_CONTAINER=$(docker-compose ps -q postgres)
if [ -z "$POSTGRES_CONTAINER" ]; then
    echo -e "${RED}✗ Postgres container not found. Is docker-compose running?${NC}"
    exit 1
fi

docker cp migrations/003_clash_config_management.sql ${POSTGRES_CONTAINER}:/tmp/migration.sql

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Failed to copy migration file to container${NC}"
    exit 1
fi

# Execute migration
docker-compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -f /tmp/migration.sql

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Database migration completed${NC}"
    # Clean up
    docker-compose exec -T postgres rm /tmp/migration.sql 2>/dev/null || true
else
    echo -e "${RED}✗ Database migration failed${NC}"
    echo "Try running manually:"
    echo "  docker cp migrations/003_clash_config_management.sql \$(docker-compose ps -q postgres):/tmp/migration.sql"
    echo "  docker-compose exec postgres psql -U $DB_USER -d $DB_NAME -f /tmp/migration.sql"
    exit 1
fi
echo ""

# Step 3: Restart API service only (no rebuild)
echo -e "${BLUE}Step 3: Restarting API service...${NC}"
echo -e "${YELLOW}Note: Skipping rebuild due to network issues. Using existing image.${NC}"
docker-compose restart api

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ API service restarted${NC}"
else
    echo -e "${RED}✗ Failed to restart API service${NC}"
    exit 1
fi
echo ""

# Step 4: Verify services are running
echo -e "${BLUE}Step 4: Verifying services...${NC}"
sleep 5

if docker-compose ps | grep -q "api.*Up"; then
    echo -e "${GREEN}✓ API service is running${NC}"
else
    echo -e "${RED}✗ API service is not running${NC}"
    echo "Check logs with: docker-compose logs api"
    exit 1
fi
echo ""

# Step 5: Test the new endpoints
echo -e "${BLUE}Step 5: Testing endpoints...${NC}"
API_URL="${API_BASE_URL:-http://localhost:8080}"

# Test health check
if curl -s -f "$API_URL/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ API health check passed${NC}"
else
    echo -e "${RED}✗ API health check failed${NC}"
    echo "The API might need to be rebuilt. Try:"
    echo "  docker-compose build api"
    echo "  docker-compose up -d"
fi

# Verify database tables
echo "Verifying database tables..."
TABLE_COUNT=$(docker-compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name IN ('clash_proxies', 'clash_proxy_groups', 'clash_rules');")

if [ "$TABLE_COUNT" -eq 3 ]; then
    echo -e "${GREEN}✓ All 3 tables created successfully${NC}"
else
    echo -e "${RED}✗ Tables not created properly (found $TABLE_COUNT/3)${NC}"
fi
echo ""

# Summary
echo -e "${GREEN}=== Database Migration Completed ===${NC}"
echo ""
echo -e "${YELLOW}⚠️  Important: The API service was restarted but NOT rebuilt.${NC}"
echo -e "${YELLOW}   The new Clash endpoints will only work after rebuilding the API.${NC}"
echo ""
echo "To rebuild the API when network is stable:"
echo "  docker-compose build api"
echo "  docker-compose up -d"
echo ""
echo "Or use a Docker mirror/proxy to speed up image pulling."
echo ""
echo "Documentation:"
echo "  • Quick Start: docs/CLASH_CONFIG_QUICKSTART.md"
echo "  • API Reference: docs/CLASH_CONFIG_MANAGEMENT.md"
echo ""
