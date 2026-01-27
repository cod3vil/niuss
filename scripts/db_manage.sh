#!/bin/bash

# Database Management Script
# Provides utilities for managing the VPN platform database

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Function to check if Docker Compose is running
check_docker() {
    if ! docker-compose ps postgres | grep -q "Up"; then
        print_error "PostgreSQL container is not running"
        print_info "Start it with: docker-compose up -d postgres"
        exit 1
    fi
}

# Function to backup database
backup_db() {
    print_header "Database Backup"
    
    check_docker
    
    BACKUP_DIR="./backups"
    mkdir -p "$BACKUP_DIR"
    
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    BACKUP_FILE="$BACKUP_DIR/vpn_platform_${TIMESTAMP}.sql"
    
    print_info "Creating backup..."
    docker-compose exec -T postgres pg_dump -U vpn_user vpn_platform > "$BACKUP_FILE"
    
    if [ $? -eq 0 ]; then
        print_info "Backup created successfully: $BACKUP_FILE"
        
        # Compress backup
        gzip "$BACKUP_FILE"
        print_info "Backup compressed: ${BACKUP_FILE}.gz"
        
        # Show backup size
        SIZE=$(du -h "${BACKUP_FILE}.gz" | cut -f1)
        print_info "Backup size: $SIZE"
    else
        print_error "Backup failed"
        exit 1
    fi
}

# Function to restore database
restore_db() {
    print_header "Database Restore"
    
    if [ -z "$1" ]; then
        print_error "Please specify backup file"
        echo "Usage: $0 restore <backup_file>"
        exit 1
    fi
    
    BACKUP_FILE="$1"
    
    if [ ! -f "$BACKUP_FILE" ]; then
        print_error "Backup file not found: $BACKUP_FILE"
        exit 1
    fi
    
    check_docker
    
    print_warn "This will overwrite the current database!"
    read -p "Are you sure? (yes/no): " -r
    if [[ ! $REPLY =~ ^yes$ ]]; then
        print_info "Restore cancelled"
        exit 0
    fi
    
    print_info "Restoring from: $BACKUP_FILE"
    
    # Check if file is gzipped
    if [[ "$BACKUP_FILE" == *.gz ]]; then
        gunzip -c "$BACKUP_FILE" | docker-compose exec -T postgres psql -U vpn_user vpn_platform
    else
        docker-compose exec -T postgres psql -U vpn_user vpn_platform < "$BACKUP_FILE"
    fi
    
    if [ $? -eq 0 ]; then
        print_info "Database restored successfully"
    else
        print_error "Restore failed"
        exit 1
    fi
}

# Function to reset database
reset_db() {
    print_header "Database Reset"
    
    print_warn "This will DELETE ALL DATA and recreate the database!"
    read -p "Are you sure? Type 'yes' to confirm: " -r
    if [[ ! $REPLY =~ ^yes$ ]]; then
        print_info "Reset cancelled"
        exit 0
    fi
    
    print_info "Stopping services..."
    docker-compose down
    
    print_info "Removing database volume..."
    docker volume rm vpn-subscription-platform_postgres_data 2>/dev/null || true
    
    print_info "Starting services..."
    docker-compose up -d postgres redis
    
    print_info "Waiting for database to be ready..."
    sleep 10
    
    print_info "Database reset complete. Migrations will run automatically."
}

# Function to run migrations
run_migrations() {
    print_header "Run Migrations"
    
    check_docker
    
    if [ -z "$1" ]; then
        print_error "Please specify migration file"
        echo "Usage: $0 migrate <migration_file>"
        exit 1
    fi
    
    MIGRATION_FILE="$1"
    
    if [ ! -f "$MIGRATION_FILE" ]; then
        print_error "Migration file not found: $MIGRATION_FILE"
        exit 1
    fi
    
    print_info "Running migration: $MIGRATION_FILE"
    docker-compose exec -T postgres psql -U vpn_user vpn_platform < "$MIGRATION_FILE"
    
    if [ $? -eq 0 ]; then
        print_info "Migration completed successfully"
    else
        print_error "Migration failed"
        exit 1
    fi
}

# Function to connect to database shell
db_shell() {
    print_header "Database Shell"
    
    check_docker
    
    print_info "Connecting to database..."
    docker-compose exec postgres psql -U vpn_user vpn_platform
}

# Function to show database stats
db_stats() {
    print_header "Database Statistics"
    
    check_docker
    
    print_info "Fetching statistics..."
    
    docker-compose exec -T postgres psql -U vpn_user vpn_platform <<EOF
-- Table sizes
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size,
    pg_total_relation_size(schemaname||'.'||tablename) AS size_bytes
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY size_bytes DESC;

-- Row counts
SELECT 'users' AS table_name, COUNT(*) AS row_count FROM users
UNION ALL
SELECT 'packages', COUNT(*) FROM packages
UNION ALL
SELECT 'orders', COUNT(*) FROM orders
UNION ALL
SELECT 'user_packages', COUNT(*) FROM user_packages
UNION ALL
SELECT 'nodes', COUNT(*) FROM nodes
UNION ALL
SELECT 'traffic_logs', COUNT(*) FROM traffic_logs
UNION ALL
SELECT 'subscriptions', COUNT(*) FROM subscriptions
UNION ALL
SELECT 'coin_transactions', COUNT(*) FROM coin_transactions
UNION ALL
SELECT 'admin_logs', COUNT(*) FROM admin_logs;

-- Database size
SELECT pg_size_pretty(pg_database_size('vpn_platform')) AS database_size;

-- Active connections
SELECT COUNT(*) AS active_connections FROM pg_stat_activity WHERE datname = 'vpn_platform';
EOF
}

# Function to clean old data
clean_old_data() {
    print_header "Clean Old Data"
    
    check_docker
    
    DAYS=${1:-90}
    
    print_warn "This will delete traffic logs older than $DAYS days"
    read -p "Continue? (yes/no): " -r
    if [[ ! $REPLY =~ ^yes$ ]]; then
        print_info "Cleanup cancelled"
        exit 0
    fi
    
    print_info "Cleaning traffic logs older than $DAYS days..."
    
    docker-compose exec -T postgres psql -U vpn_user vpn_platform <<EOF
BEGIN;

-- Delete old traffic logs
DELETE FROM traffic_logs WHERE recorded_at < NOW() - INTERVAL '$DAYS days';

-- Vacuum to reclaim space
VACUUM ANALYZE traffic_logs;

COMMIT;

SELECT 'Cleanup completed' AS status;
EOF
    
    print_info "Cleanup completed"
}

# Function to show help
show_help() {
    cat <<EOF
Database Management Script

Usage: $0 <command> [options]

Commands:
    backup              Create a database backup
    restore <file>      Restore database from backup file
    reset               Reset database (WARNING: deletes all data)
    migrate <file>      Run a specific migration file
    shell               Open database shell (psql)
    stats               Show database statistics
    clean [days]        Clean old data (default: 90 days)
    help                Show this help message

Examples:
    $0 backup
    $0 restore backups/vpn_platform_20240101_120000.sql.gz
    $0 migrate migrations/003_new_feature.sql
    $0 clean 30
    $0 stats

EOF
}

# Main script
case "$1" in
    backup)
        backup_db
        ;;
    restore)
        restore_db "$2"
        ;;
    reset)
        reset_db
        ;;
    migrate)
        run_migrations "$2"
        ;;
    shell)
        db_shell
        ;;
    stats)
        db_stats
        ;;
    clean)
        clean_old_data "$2"
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        show_help
        exit 1
        ;;
esac
