#!/bin/bash

# Deployment Testing Script
# Tests the complete deployment flow of the VPN platform

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

print_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

print_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to wait for service
wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=30
    local attempt=0
    
    print_info "Waiting for $name to be ready..."
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -f -s "$url" >/dev/null 2>&1; then
            print_pass "$name is ready"
            return 0
        fi
        
        attempt=$((attempt + 1))
        sleep 2
    done
    
    print_fail "$name failed to start within timeout"
    return 1
}

# Test 1: Check prerequisites
test_prerequisites() {
    print_header "Test 1: Prerequisites"
    
    print_test "Checking Docker..."
    if command_exists docker; then
        print_pass "Docker is installed"
    else
        print_fail "Docker is not installed"
        return 1
    fi
    
    print_test "Checking Docker Compose..."
    if command_exists docker-compose; then
        print_pass "Docker Compose is installed"
    else
        print_fail "Docker Compose is not installed"
        return 1
    fi
    
    print_test "Checking Docker daemon..."
    if docker info >/dev/null 2>&1; then
        print_pass "Docker daemon is running"
    else
        print_fail "Docker daemon is not running"
        return 1
    fi
}

# Test 2: Check configuration files
test_configuration() {
    print_header "Test 2: Configuration Files"
    
    print_test "Checking docker-compose.yml..."
    if [ -f "docker-compose.yml" ]; then
        print_pass "docker-compose.yml exists"
    else
        print_fail "docker-compose.yml not found"
        return 1
    fi
    
    print_test "Checking .env file..."
    if [ -f ".env" ]; then
        print_pass ".env file exists"
    else
        print_warn ".env file not found, using defaults"
    fi
    
    print_test "Checking Dockerfiles..."
    local dockerfiles=("api/Dockerfile" "frontend/Dockerfile" "admin/Dockerfile" "node-agent/Dockerfile")
    for dockerfile in "${dockerfiles[@]}"; do
        if [ -f "$dockerfile" ]; then
            print_pass "$dockerfile exists"
        else
            print_fail "$dockerfile not found"
        fi
    done
    
    print_test "Checking migration files..."
    if [ -f "migrations/001_init.sql" ]; then
        print_pass "Database migration exists"
    else
        print_fail "Database migration not found"
        return 1
    fi
}

# Test 3: Build images
test_build() {
    print_header "Test 3: Build Docker Images"
    
    print_test "Building Docker images..."
    if docker-compose build 2>&1 | tee /tmp/build.log; then
        print_pass "Docker images built successfully"
    else
        print_fail "Failed to build Docker images"
        cat /tmp/build.log
        return 1
    fi
}

# Test 4: Start services
test_start_services() {
    print_header "Test 4: Start Services"
    
    print_test "Starting services..."
    if docker-compose up -d; then
        print_pass "Services started"
    else
        print_fail "Failed to start services"
        return 1
    fi
    
    sleep 5
    
    print_test "Checking service status..."
    docker-compose ps
}

# Test 5: Database initialization
test_database() {
    print_header "Test 5: Database Initialization"
    
    print_test "Waiting for PostgreSQL..."
    local max_attempts=30
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if docker-compose exec -T postgres pg_isready -U vpn_user >/dev/null 2>&1; then
            print_pass "PostgreSQL is ready"
            break
        fi
        attempt=$((attempt + 1))
        sleep 2
    done
    
    if [ $attempt -eq $max_attempts ]; then
        print_fail "PostgreSQL failed to start"
        return 1
    fi
    
    print_test "Checking database tables..."
    local tables=$(docker-compose exec -T postgres psql -U vpn_user -d vpn_platform -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';" 2>/dev/null | tr -d ' ')
    
    if [ "$tables" -gt 0 ]; then
        print_pass "Database tables created ($tables tables)"
    else
        print_fail "No database tables found"
        return 1
    fi
    
    print_test "Checking default data..."
    local packages=$(docker-compose exec -T postgres psql -U vpn_user -d vpn_platform -t -c "SELECT COUNT(*) FROM packages;" 2>/dev/null | tr -d ' ')
    
    if [ "$packages" -gt 0 ]; then
        print_pass "Default packages created ($packages packages)"
    else
        print_fail "No default packages found"
    fi
}

# Test 6: Redis connectivity
test_redis() {
    print_header "Test 6: Redis Connectivity"
    
    print_test "Checking Redis..."
    if docker-compose exec -T redis redis-cli ping 2>/dev/null | grep -q "PONG"; then
        print_pass "Redis is responding"
    else
        print_fail "Redis is not responding"
        return 1
    fi
    
    print_test "Testing Redis operations..."
    docker-compose exec -T redis redis-cli SET test_key "test_value" >/dev/null 2>&1
    local value=$(docker-compose exec -T redis redis-cli GET test_key 2>/dev/null | tr -d '\r')
    
    if [ "$value" = "test_value" ]; then
        print_pass "Redis read/write operations work"
        docker-compose exec -T redis redis-cli DEL test_key >/dev/null 2>&1
    else
        print_fail "Redis read/write operations failed"
    fi
}

# Test 7: API service
test_api() {
    print_header "Test 7: API Service"
    
    wait_for_service "http://localhost:8080/health" "API service" || return 1
    
    print_test "Testing API health endpoint..."
    local response=$(curl -s http://localhost:8080/health)
    
    if echo "$response" | grep -q "ok\|healthy"; then
        print_pass "API health check passed"
    else
        print_fail "API health check failed"
        echo "Response: $response"
    fi
}

# Test 8: Frontend service
test_frontend() {
    print_header "Test 8: Frontend Service"
    
    print_test "Checking frontend..."
    if curl -f -s http://localhost/ >/dev/null 2>&1; then
        print_pass "Frontend is accessible"
    else
        print_fail "Frontend is not accessible"
        return 1
    fi
    
    print_test "Checking frontend content..."
    local content=$(curl -s http://localhost/)
    
    if echo "$content" | grep -q "<!DOCTYPE html>"; then
        print_pass "Frontend returns HTML content"
    else
        print_fail "Frontend does not return valid HTML"
    fi
}

# Test 9: Admin service
test_admin() {
    print_header "Test 9: Admin Service"
    
    print_test "Checking admin panel..."
    if curl -f -s http://localhost:8081/ >/dev/null 2>&1; then
        print_pass "Admin panel is accessible"
    else
        print_fail "Admin panel is not accessible"
        return 1
    fi
    
    print_test "Checking admin content..."
    local content=$(curl -s http://localhost:8081/)
    
    if echo "$content" | grep -q "<!DOCTYPE html>"; then
        print_pass "Admin panel returns HTML content"
    else
        print_fail "Admin panel does not return valid HTML"
    fi
}

# Test 10: Network connectivity
test_network() {
    print_header "Test 10: Network Connectivity"
    
    print_test "Checking inter-service connectivity..."
    
    # Test API to PostgreSQL
    if docker-compose exec -T api sh -c "nc -z postgres 5432" 2>/dev/null; then
        print_pass "API can reach PostgreSQL"
    else
        print_warn "Cannot verify API to PostgreSQL connectivity"
    fi
    
    # Test API to Redis
    if docker-compose exec -T api sh -c "nc -z redis 6379" 2>/dev/null; then
        print_pass "API can reach Redis"
    else
        print_warn "Cannot verify API to Redis connectivity"
    fi
}

# Test 11: Resource usage
test_resources() {
    print_header "Test 11: Resource Usage"
    
    print_test "Checking container resource usage..."
    docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" | grep vpn-
    
    print_pass "Resource usage displayed"
}

# Test 12: Logs
test_logs() {
    print_header "Test 12: Service Logs"
    
    print_test "Checking for errors in logs..."
    
    local services=("api" "postgres" "redis")
    local has_errors=false
    
    for service in "${services[@]}"; do
        local errors=$(docker-compose logs "$service" 2>&1 | grep -i "error\|fatal\|panic" | wc -l)
        
        if [ "$errors" -gt 0 ]; then
            print_warn "$service has $errors error messages in logs"
            has_errors=true
        else
            print_pass "$service logs look clean"
        fi
    done
    
    if [ "$has_errors" = true ]; then
        print_warn "Some services have errors in logs (this may be normal during startup)"
    fi
}

# Cleanup function
cleanup() {
    print_header "Cleanup"
    
    read -p "Do you want to stop the services? (y/N): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_info "Stopping services..."
        docker-compose down
        print_info "Services stopped"
    else
        print_info "Services left running"
    fi
}

# Print summary
print_summary() {
    print_header "Test Summary"
    
    local total=$((TESTS_PASSED + TESTS_FAILED))
    
    echo "Total tests: $total"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo ""
        print_pass "All tests passed! ✓"
        echo ""
        echo "Your VPN platform is ready to use:"
        echo "  - Frontend: http://localhost"
        echo "  - Admin: http://localhost:8081"
        echo "  - API: http://localhost:8080"
        echo ""
        return 0
    else
        echo ""
        print_fail "Some tests failed. Please check the output above."
        echo ""
        return 1
    fi
}

# Main test flow
main() {
    print_header "VPN Platform Deployment Test"
    
    echo "This script will test the complete deployment of the VPN platform."
    echo "It will build images, start services, and verify functionality."
    echo ""
    
    read -p "Continue? (Y/n): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        print_info "Test cancelled"
        exit 0
    fi
    
    # Run tests
    test_prerequisites || exit 1
    test_configuration || exit 1
    test_build || exit 1
    test_start_services || exit 1
    test_database || exit 1
    test_redis || exit 1
    test_api || exit 1
    test_frontend || exit 1
    test_admin || exit 1
    test_network
    test_resources
    test_logs
    
    # Print summary
    print_summary
    local result=$?
    
    # Cleanup
    cleanup
    
    exit $result
}

# Run main function
main
