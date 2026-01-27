#!/bin/bash

# Clash Configuration Feature Update Script
# This script updates the server with the new Clash configuration management feature

set -e

echo "=== Clash Configuration Feature Update ==="
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

# Get database info from .env or use defaults
if [ -f .env ]; then
    source .env
fi

DB_NAME="${DATABASE_NAME:-vpn_platform}"
DB_USER="${DATABASE_USER:-postgres}"

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

# Step 3: Rebuild and restart services
echo -e "${BLUE}Step 3: Rebuilding and restarting services...${NC}"
docker-compose down
docker-compose build api
docker-compose up -d

echo -e "${GREEN}✓ Services restarted${NC}"
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
echo -e "${BLUE}Step 5: Testing new endpoints...${NC}"
API_URL="${API_BASE_URL:-http://localhost:8080}"

# Test health check
if curl -s -f "$API_URL/health" > /dev/null; then
    echo -e "${GREEN}✓ API health check passed${NC}"
else
    echo -e "${RED}✗ API health check failed${NC}"
    exit 1
fi

# Verify database tables
echo "Verifying database tables..."
TABLE_COUNT=$(docker-compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name IN ('clash_proxies', 'clash_proxy_groups', 'clash_rules');")

if [ "$TABLE_COUNT" -eq 3 ]; then
    echo -e "${GREEN}✓ All 3 tables created successfully${NC}"
else
    echo -e "${RED}✗ Tables not created properly (found $TABLE_COUNT/3)${NC}"
    exit 1
fi
echo ""

# Summary
echo -e "${GREEN}=== Update Completed Successfully ===${NC}"
echo ""
echo "New features available:"
echo "  • Clash proxy management"
echo "  • Clash proxy group management"
echo "  • Clash rule management"
echo "  • Dynamic YAML configuration generation"
echo ""
echo "API Endpoints:"
echo "  • GET/POST    /api/admin/clash/proxies"
echo "  • GET/POST    /api/admin/clash/proxy-groups"
echo "  • GET/POST    /api/admin/clash/rules"
echo "  • GET         /api/admin/clash/generate"
echo ""
echo "Documentation:"
echo "  • Quick Start: docs/CLASH_CONFIG_QUICKSTART.md"
echo "  • API Reference: docs/CLASH_CONFIG_MANAGEMENT.md"
echo "  • Features: docs/FEATURES_CLASH_CONFIG.md"
echo ""
echo "Next steps:"
echo "  1. Login as admin to get JWT token"
echo "  2. Create proxies using the API"
echo "  3. Configure proxy groups"
echo "  4. Set up routing rules"
echo "  5. Generate Clash configuration"
echo ""
echo -e "${BLUE}Example:${NC}"
echo "  export ADMIN_TOKEN='your_token'"
echo "  ./examples/clash_config_example.sh"
echo ""
