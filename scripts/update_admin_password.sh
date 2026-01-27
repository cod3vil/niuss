#!/bin/bash

# Update Admin Password Script
# This script updates the admin password hash in an existing database

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}=== Update Admin Password Hash ===${NC}"
echo ""

# Check if PostgreSQL is running
if ! docker compose ps postgres 2>/dev/null | grep -q "Up"; then
    echo -e "${RED}Error: PostgreSQL container is not running${NC}"
    echo "Please start the database: docker compose up -d postgres"
    exit 1
fi

echo -e "${YELLOW}This will update the admin password hash to a valid Argon2 hash.${NC}"
echo -e "${YELLOW}Password will be: admin123${NC}"
echo ""
read -p "Continue? (y/n): " confirm

if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "Aborted."
    exit 0
fi

# Valid Argon2 hash for "admin123"
HASH='$argon2id$v=19$m=19456,t=2,p=1$6HqspZKtuGhEhGzqaKfWvA$vh5qa/0HFo6HIhbywr0nkr/voSPNNsdbqM6vA6o2XKU'

echo ""
echo -e "${YELLOW}Updating database...${NC}"

# Update database
docker compose exec -T postgres psql -U vpn_user -d vpn_platform << EOF
UPDATE users SET password_hash = '$HASH' WHERE email = 'admin@example.com';
SELECT email, is_admin, status, substring(password_hash, 1, 30) as hash_preview FROM users WHERE email = 'admin@example.com';
EOF

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ Admin password hash updated successfully!${NC}"
    echo ""
    echo -e "${GREEN}Login credentials:${NC}"
    echo "  Email:    admin@example.com"
    echo "  Password: admin123"
    echo ""
    echo -e "${YELLOW}⚠️  Please change this password after logging in!${NC}"
else
    echo -e "${RED}Error: Failed to update database${NC}"
    exit 1
fi
