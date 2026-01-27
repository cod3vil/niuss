# Database Migrations

This directory contains SQL migration scripts for the VPN Subscription Platform database.

## Migration Files

- `001_init.sql` - Initial database schema and default data

## Migration Order

Migrations are executed in alphabetical order by filename. The naming convention is:
```
<number>_<description>.sql
```

For example:
- `001_init.sql`
- `002_add_feature.sql`
- `003_update_schema.sql`

## Automatic Execution

When using Docker Compose, all SQL files in this directory are automatically executed when the PostgreSQL container is first initialized. The files are mounted to `/docker-entrypoint-initdb.d/` in the container.

## Manual Execution

To manually run a migration:

```bash
# Using Docker Compose
docker-compose exec postgres psql -U vpn_user -d vpn_platform -f /docker-entrypoint-initdb.d/002_migration.sql

# Using psql directly
psql -U vpn_user -d vpn_platform -f migrations/002_migration.sql
```

## Default Data

The `001_init.sql` migration includes:

### Default Packages
- 体验套餐 (Trial): 10GB, 100 coins, 30 days
- 标准套餐 (Standard): 50GB, 500 coins, 30 days
- 高级套餐 (Premium): 100GB, 900 coins, 30 days
- 旗舰套餐 (Ultimate): 500GB, 4000 coins, 90 days

### Default Admin User
- Email: `admin@example.com`
- Password: `admin123` (must be changed in production)
- Role: Admin

**⚠️ IMPORTANT**: Change the default admin password immediately after deployment!

## Creating New Migrations

When creating a new migration:

1. Create a new file with the next sequential number
2. Use descriptive names
3. Include both UP and DOWN migrations if possible
4. Test the migration on a development database first
5. Document any breaking changes

Example migration template:

```sql
-- Migration: <description>
-- Created: <date>
-- Author: <name>

-- UP Migration
BEGIN;

-- Your schema changes here

COMMIT;

-- DOWN Migration (optional, for rollback)
-- BEGIN;
-- 
-- Rollback changes here
-- 
-- COMMIT;
```

## Rollback

To rollback a migration, you'll need to manually execute the DOWN migration or restore from a backup.

## Best Practices

1. **Always backup** before running migrations in production
2. **Test migrations** on a staging environment first
3. **Use transactions** (BEGIN/COMMIT) for atomic changes
4. **Document changes** in comments
5. **Keep migrations small** and focused on one change
6. **Never modify** existing migration files after they've been deployed
7. **Create new migrations** for schema changes instead

## Troubleshooting

### Migration fails to execute

Check the PostgreSQL logs:
```bash
docker-compose logs postgres
```

### Need to reset database

**⚠️ WARNING: This will delete all data!**

```bash
# Stop services
docker-compose down

# Remove volumes
docker volume rm vpn-subscription-platform_postgres_data

# Start services (migrations will run again)
docker-compose up -d
```

### Check migration status

```bash
# Connect to database
docker-compose exec postgres psql -U vpn_user vpn_platform

# List all tables
\dt

# Check table structure
\d users
\d packages
```
