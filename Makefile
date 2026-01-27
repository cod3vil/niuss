.PHONY: help build up down logs clean test

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build all Docker images
	docker-compose build

build-no-cache: ## Build all Docker images without cache
	docker-compose build --no-cache

up: ## Start all services
	docker-compose up -d

down: ## Stop all services
	docker-compose down

restart: ## Restart all services
	docker-compose restart

logs: ## Show logs from all services
	docker-compose logs -f

logs-api: ## Show API logs
	docker-compose logs -f api

logs-postgres: ## Show PostgreSQL logs
	docker-compose logs -f postgres

logs-redis: ## Show Redis logs
	docker-compose logs -f redis

status: ## Show status of all services
	docker-compose ps

clean: ## Remove all containers, volumes, and images
	docker-compose down -v
	docker system prune -f

test: ## Run all tests
	cargo test

test-api: ## Run API tests only
	cargo test --package api

test-node-agent: ## Run node-agent tests only
	cargo test --package node-agent

dev-api: ## Run API service locally
	cd api && cargo run

dev-frontend: ## Run frontend locally
	cd frontend && npm run dev

dev-admin: ## Run admin locally
	cd admin && npm run dev

install-frontend: ## Install frontend dependencies
	cd frontend && npm install

install-admin: ## Install admin dependencies
	cd admin && npm install

install-deps: install-frontend install-admin ## Install all frontend dependencies

db-backup: ## Backup database
	./scripts/db_manage.sh backup

db-restore: ## Restore database (usage: make db-restore FILE=backup.sql.gz)
	./scripts/db_manage.sh restore $(FILE)

db-reset: ## Reset database (WARNING: deletes all data)
	./scripts/db_manage.sh reset

db-migrate: ## Run database migrations
	@echo "Running database migrations..."
	@docker-compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/001_init.sql
	@echo "✓ Database migrations completed"

db-init: ## Initialize database (run migrations)
	@echo "Initializing database..."
	@echo "Waiting for PostgreSQL to be ready..."
	@until docker-compose exec -T postgres pg_isready -U vpn_user > /dev/null 2>&1; do \
		echo "Waiting for database..."; \
		sleep 2; \
	done
	@echo "Checking if database is already initialized..."
	@TABLES=$$(docker-compose exec -T postgres psql -U vpn_user -d vpn_platform -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name='users';" 2>/dev/null || echo "0"); \
	if [ "$$TABLES" = "0" ]; then \
		echo "Running initialization script..."; \
		docker-compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/001_init.sql; \
		echo "✓ Database initialized successfully"; \
	else \
		echo "Database already initialized, skipping"; \
	fi

db-seed: ## Load test data (development only)
	@echo "Loading test data..."
	@docker-compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/002_seed_test_data.sql
	@echo "✓ Test data loaded"

db-shell: ## Open PostgreSQL shell
	docker-compose exec postgres psql -U vpn_user -d vpn_platform

db-stats: ## Show database statistics
	./scripts/db_manage.sh stats

redis-shell: ## Open Redis shell
	docker-compose exec redis redis-cli

redis-cli: ## Run Redis CLI command (usage: make redis-cli CMD="KEYS *")
	docker-compose exec redis redis-cli $(CMD)

health-check: ## Check health of all services
	@echo "Checking API health..."
	@curl -f http://localhost:8080/health 2>/dev/null && echo "✓ API is healthy" || echo "✗ API is not responding"
	@echo "Checking Frontend..."
	@curl -f http://localhost/ 2>/dev/null && echo "✓ Frontend is healthy" || echo "✗ Frontend is not responding"
	@echo "Checking Admin..."
	@curl -f http://localhost:8081/ 2>/dev/null && echo "✓ Admin is healthy" || echo "✗ Admin is not responding"
	@echo "Checking PostgreSQL..."
	@docker-compose exec -T postgres pg_isready -U vpn_user 2>/dev/null && echo "✓ PostgreSQL is ready" || echo "✗ PostgreSQL is not ready"
	@echo "Checking Redis..."
	@docker-compose exec -T redis redis-cli ping 2>/dev/null && echo "✓ Redis is responding" || echo "✗ Redis is not responding"

deploy: ## Deploy all services (build and start)
	@echo "Building images..."
	docker-compose build
	@echo "Starting services..."
	docker-compose up -d
	@echo "Waiting for services to be ready..."
	@sleep 10
	@echo "Initializing database..."
	@make db-init
	@echo "Checking health..."
	@make health-check
	@echo ""
	@echo "✓ Deployment complete!"
	@echo ""
	@echo "Access the platform:"
	@echo "  User Frontend: http://localhost"
	@echo "  Admin Panel:   http://localhost:8081"
	@echo "  API:           http://localhost:8080"
	@echo ""
	@echo "Default admin credentials:"
	@echo "  Email:    admin@example.com"
	@echo "  Password: admin123"
	@echo ""
	@echo "⚠️  Please change the admin password immediately!"

deploy-prod: ## Deploy for production (with optimizations)
	@echo "Building production images..."
	docker-compose build --no-cache
	@echo "Starting services..."
	docker-compose up -d
	@echo "Services deployed successfully"

init-env: ## Initialize environment file
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "Created .env file from .env.example"; \
		echo "Please edit .env and set your configuration"; \
	else \
		echo ".env file already exists"; \
	fi

setup: init-env install-deps ## Initial setup (create .env and install dependencies)
	@echo "Setup complete!"
	@echo "Next steps:"
	@echo "  1. Edit .env file with your configuration"
	@echo "  2. Run 'make deploy' to start all services"

node-agent-build: ## Build node-agent binary
	cargo build --release --package node-agent

node-agent-install: node-agent-build ## Install node-agent (requires root)
	@echo "This will install node-agent to /usr/local/bin"
	@sudo cp target/release/node-agent /usr/local/bin/
	@sudo chmod +x /usr/local/bin/node-agent
	@echo "Node agent installed successfully"
