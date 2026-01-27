#!/usr/bin/env bash

################################################################################
# Test Helper Functions
# 
# This file contains functions from deploy_node.sh that can be sourced
# by test scripts without executing the main deployment logic.
################################################################################

# Color codes for terminal output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Script options (can be overridden by tests)
VERBOSE=${VERBOSE:-false}
QUIET=${QUIET:-false}
TEST_LOG_FILE=${TEST_LOG_FILE:-"/tmp/test-deployment.log"}

################################################################################
# Color Output Functions
################################################################################

print_info() {
    local msg="$1"
    if [ "$QUIET" != "true" ]; then
        echo -e "${GREEN}[INFO]${NC} $msg"
    fi
}

print_warn() {
    local msg="$1"
    if [ "$QUIET" != "true" ]; then
        echo -e "${YELLOW}[WARN]${NC} $msg"
    fi
}

print_error() {
    local msg="$1"
    echo -e "${RED}[ERROR]${NC} $msg" >&2
}

print_debug() {
    local msg="$1"
    if [ "$VERBOSE" = "true" ]; then
        echo -e "${BLUE}[DEBUG]${NC} $msg"
    fi
}

################################################################################
# Logging Functions
################################################################################

log_info() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Write to log file
    echo "[$timestamp] [INFO] $msg" >> "$TEST_LOG_FILE" 2>/dev/null || true
    
    # Print to console
    print_info "$msg"
}

log_warn() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Write to log file
    echo "[$timestamp] [WARN] $msg" >> "$TEST_LOG_FILE" 2>/dev/null || true
    
    # Print to console
    print_warn "$msg"
}

log_error() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Write to log file
    echo "[$timestamp] [ERROR] $msg" >> "$TEST_LOG_FILE" 2>/dev/null || true
    
    # Print to console
    print_error "$msg"
}

log_debug() {
    local msg="$1"
    
    if [ "$VERBOSE" = "true" ]; then
        local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        
        # Write to log file
        echo "[$timestamp] [DEBUG] $msg" >> "$TEST_LOG_FILE" 2>/dev/null || true
        
        # Print to console
        print_debug "$msg"
    fi
}

################################################################################
# Sensitive Information Masking
################################################################################

mask_sensitive() {
    local value="$1"
    local visible_chars=8
    
    if [ -z "$value" ]; then
        echo "***"
        return
    fi
    
    if [ ${#value} -gt $visible_chars ]; then
        echo "${value:0:$visible_chars}..."
    else
        echo "***"
    fi
}

################################################################################
# Deployment Configuration (for testing)
################################################################################

declare -A DEPLOY_CONFIG=(
    [api_url]=""
    [admin_token]=""
    [node_name]=""
    [node_host]=""
    [node_port]="443"
    [node_protocol]="vless"
    [node_config]="{}"
    [node_id]=""
    [node_secret]=""
)

FORCE=false
BATCH_CONFIG_FILE=""

################################################################################
# Parameter Parsing Functions
################################################################################

parse_parameters() {
    log_debug "Parsing command-line parameters..."
    
    # Parse command-line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --api-url)
                DEPLOY_CONFIG[api_url]="$2"
                shift 2
                ;;
            --admin-token)
                DEPLOY_CONFIG[admin_token]="$2"
                shift 2
                ;;
            --node-name)
                DEPLOY_CONFIG[node_name]="$2"
                shift 2
                ;;
            --node-host)
                DEPLOY_CONFIG[node_host]="$2"
                shift 2
                ;;
            --node-port)
                DEPLOY_CONFIG[node_port]="$2"
                shift 2
                ;;
            --node-protocol)
                DEPLOY_CONFIG[node_protocol]="$2"
                shift 2
                ;;
            --node-config)
                DEPLOY_CONFIG[node_config]="$2"
                shift 2
                ;;
            --batch-config)
                BATCH_CONFIG_FILE="$2"
                shift 2
                ;;
            --force)
                FORCE=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --quiet)
                QUIET=true
                shift
                ;;
            *)
                shift
                ;;
        esac
    done
    
    # Fall back to environment variables if command-line parameters not provided
    if [ -z "${DEPLOY_CONFIG[api_url]}" ] && [ -n "$API_URL" ]; then
        DEPLOY_CONFIG[api_url]="$API_URL"
        log_debug "Using API_URL from environment variable"
    fi
    
    if [ -z "${DEPLOY_CONFIG[admin_token]}" ] && [ -n "$ADMIN_TOKEN" ]; then
        DEPLOY_CONFIG[admin_token]="$ADMIN_TOKEN"
        log_debug "Using ADMIN_TOKEN from environment variable"
    fi
    
    if [ -z "${DEPLOY_CONFIG[node_name]}" ] && [ -n "$NODE_NAME" ]; then
        DEPLOY_CONFIG[node_name]="$NODE_NAME"
        log_debug "Using NODE_NAME from environment variable"
    fi
    
    if [ -z "${DEPLOY_CONFIG[node_host]}" ] && [ -n "$NODE_HOST" ]; then
        DEPLOY_CONFIG[node_host]="$NODE_HOST"
        log_debug "Using NODE_HOST from environment variable"
    fi
    
    if [ "${DEPLOY_CONFIG[node_port]}" = "443" ] && [ -n "$NODE_PORT" ]; then
        DEPLOY_CONFIG[node_port]="$NODE_PORT"
        log_debug "Using NODE_PORT from environment variable"
    fi
    
    if [ "${DEPLOY_CONFIG[node_protocol]}" = "vless" ] && [ -n "$NODE_PROTOCOL" ]; then
        DEPLOY_CONFIG[node_protocol]="$NODE_PROTOCOL"
        log_debug "Using NODE_PROTOCOL from environment variable"
    fi
    
    if [ "${DEPLOY_CONFIG[node_config]}" = "{}" ] && [ -n "$NODE_CONFIG" ]; then
        DEPLOY_CONFIG[node_config]="$NODE_CONFIG"
        log_debug "Using NODE_CONFIG from environment variable"
    fi
    
    log_debug "Parameter parsing completed"
}

################################################################################
# Default Value Handling
################################################################################

apply_defaults() {
    log_debug "Applying default values for optional parameters..."
    
    if [ -z "${DEPLOY_CONFIG[node_port]}" ] || [ "${DEPLOY_CONFIG[node_port]}" = "" ]; then
        DEPLOY_CONFIG[node_port]="443"
        log_debug "Applied default NODE_PORT: 443"
    fi
    
    if [ -z "${DEPLOY_CONFIG[node_protocol]}" ] || [ "${DEPLOY_CONFIG[node_protocol]}" = "" ]; then
        DEPLOY_CONFIG[node_protocol]="vless"
        log_debug "Applied default NODE_PROTOCOL: vless"
    fi
    
    if [ -z "${DEPLOY_CONFIG[node_config]}" ] || [ "${DEPLOY_CONFIG[node_config]}" = "" ]; then
        DEPLOY_CONFIG[node_config]="{}"
        log_debug "Applied default NODE_CONFIG: {}"
    fi
    
    log_debug "Default values applied successfully"
}

################################################################################
# Parameter Validation Functions
################################################################################

validate_url() {
    local url="$1"
    
    if [ -z "$url" ]; then
        return 1
    fi
    
    if [[ "$url" =~ ^https?://[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*(/.*)?$ ]]; then
        return 0
    else
        return 1
    fi
}

validate_port() {
    local port="$1"
    
    if ! [[ "$port" =~ ^[0-9]+$ ]]; then
        return 1
    fi
    
    if [ "$port" -ge 1 ] && [ "$port" -le 65535 ]; then
        return 0
    else
        return 1
    fi
}

validate_protocol() {
    local protocol="$1"
    local valid_protocols=("shadowsocks" "vmess" "trojan" "hysteria2" "vless")
    
    for valid_proto in "${valid_protocols[@]}"; do
        if [ "$protocol" = "$valid_proto" ]; then
            return 0
        fi
    done
    
    return 1
}

validate_jwt_format() {
    local token="$1"
    
    if [ -z "$token" ]; then
        return 1
    fi
    
    local parts_count=$(echo "$token" | tr '.' '\n' | wc -l)
    if [ "$parts_count" -ne 3 ]; then
        return 1
    fi
    
    if [[ "$token" =~ ^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$ ]]; then
        return 0
    else
        return 1
    fi
}


validate_parameters() {
    log_debug "Validating parameters..."
    
    local validation_errors=0
    
    # Check required parameters
    if [ -z "${DEPLOY_CONFIG[api_url]}" ]; then
        log_error "Missing required parameter: --api-url or API_URL"
        ((validation_errors++))
    else
        # Validate URL format
        if ! validate_url "${DEPLOY_CONFIG[api_url]}"; then
            log_error "Invalid API URL format: ${DEPLOY_CONFIG[api_url]}"
            log_error "URL must start with http:// or https://"
            ((validation_errors++))
        fi
    fi
    
    if [ -z "${DEPLOY_CONFIG[admin_token]}" ]; then
        log_error "Missing required parameter: --admin-token or ADMIN_TOKEN"
        ((validation_errors++))
    else
        # Validate JWT format
        if ! validate_jwt_format "${DEPLOY_CONFIG[admin_token]}"; then
            log_error "Invalid JWT token format: $(mask_sensitive "${DEPLOY_CONFIG[admin_token]}")"
            log_error "Token must be a valid JWT (three base64-encoded segments separated by dots)"
            ((validation_errors++))
        fi
    fi
    
    if [ -z "${DEPLOY_CONFIG[node_name]}" ]; then
        log_error "Missing required parameter: --node-name or NODE_NAME"
        ((validation_errors++))
    fi
    
    # Validate optional parameters if provided
    if [ -n "${DEPLOY_CONFIG[node_port]}" ]; then
        if ! validate_port "${DEPLOY_CONFIG[node_port]}"; then
            log_error "Invalid port number: ${DEPLOY_CONFIG[node_port]}"
            log_error "Port must be between 1 and 65535"
            ((validation_errors++))
        fi
    fi
    
    if [ -n "${DEPLOY_CONFIG[node_protocol]}" ]; then
        if ! validate_protocol "${DEPLOY_CONFIG[node_protocol]}"; then
            log_error "Invalid protocol: ${DEPLOY_CONFIG[node_protocol]}"
            log_error "Supported protocols: shadowsocks, vmess, trojan, hysteria2, vless"
            ((validation_errors++))
        fi
    fi
    
    # Check for validation errors
    if [ $validation_errors -gt 0 ]; then
        log_error "Parameter validation failed with $validation_errors error(s)"
        log_error "Run with --help for usage information"
        return 1
    fi
    
    log_debug "Parameter validation completed successfully"
    return 0
}

################################################################################
# Environment Detection
################################################################################

# Global variables for OS detection
OS_TYPE=""
OS_VERSION=""
OS_ID=""

# Detect operating system type and version
detect_os() {
    log_debug "Detecting operating system..."
    
    # Check if /etc/os-release exists
    if [ ! -f /etc/os-release ]; then
        log_error "Cannot detect OS: /etc/os-release not found"
        log_error "This script requires a modern Linux distribution"
        return 1
    fi
    
    # Source the os-release file to get OS information
    # shellcheck disable=SC1091
    source /etc/os-release
    
    # Extract OS ID (ubuntu, centos, debian, etc.)
    OS_ID="${ID:-unknown}"
    OS_VERSION="${VERSION_ID:-unknown}"
    
    # Normalize OS type
    case "$OS_ID" in
        ubuntu)
            OS_TYPE="ubuntu"
            log_info "Detected OS: Ubuntu $OS_VERSION"
            ;;
        centos|rhel)
            OS_TYPE="centos"
            log_info "Detected OS: CentOS/RHEL $OS_VERSION"
            ;;
        debian)
            OS_TYPE="debian"
            log_info "Detected OS: Debian $OS_VERSION"
            ;;
        *)
            log_error "Unsupported operating system: $OS_ID"
            log_error "Supported OS: Ubuntu, CentOS/RHEL, Debian"
            return 1
            ;;
    esac
    
    log_debug "OS Type: $OS_TYPE"
    log_debug "OS Version: $OS_VERSION"
    log_debug "OS ID: $OS_ID"
    
    return 0
}

################################################################################
# Root Permission Check
################################################################################

check_root() {
    log_debug "Checking root privileges..."
    
    # Check if EUID (Effective User ID) is 0 (root)
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run as root"
        log_error "Please run with sudo or as root user"
        return 2
    fi
    
    log_debug "Root privileges confirmed"
    return 0
}

################################################################################
# Dependency Check
################################################################################

# List of required system commands
REQUIRED_COMMANDS=("curl" "jq" "systemctl" "openssl")

check_dependencies() {
    log_debug "Checking system dependencies..."
    
    local missing_deps=()
    
    # Check each required command
    for cmd in "${REQUIRED_COMMANDS[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            log_warn "Missing dependency: $cmd"
            missing_deps+=("$cmd")
        else
            log_debug "Found dependency: $cmd"
        fi
    done
    
    # If no missing dependencies, return success
    if [ ${#missing_deps[@]} -eq 0 ]; then
        log_info "All required dependencies are installed"
        return 0
    fi
    
    # Report missing dependencies
    log_warn "Missing ${#missing_deps[@]} required dependencies: ${missing_deps[*]}"
    log_error "Please install manually: ${missing_deps[*]}"
    return 1
}

################################################################################
# Public IP Detection
################################################################################

# List of IP detection services (in order of preference)
IP_DETECTION_SERVICES=(
    "https://api.ipify.org"
    "https://icanhazip.com"
    "https://ifconfig.me"
)

detect_public_ip() {
    log_debug "Detecting public IP address..."
    
    local max_retries=2
    local timeout=5
    
    # Try each IP detection service
    for service in "${IP_DETECTION_SERVICES[@]}"; do
        log_debug "Trying IP detection service: $service"
        
        # Try with retries
        for attempt in $(seq 1 $max_retries); do
            log_debug "Attempt $attempt/$max_retries for $service"
            
            # Try to get IP address
            local ip
            ip=$(curl -s --max-time "$timeout" "$service" 2>/dev/null | tr -d '[:space:]')
            
            # Validate IP address format (basic IPv4 validation)
            if [[ "$ip" =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]]; then
                log_info "Detected public IP: $ip"
                echo "$ip"
                return 0
            fi
            
            # If not last attempt, wait before retry
            if [ $attempt -lt $max_retries ]; then
                log_debug "Invalid response, retrying..."
                sleep 1
            fi
        done
        
        log_warn "Failed to get IP from $service"
    done
    
    # All services failed
    log_error "Failed to detect public IP address"
    log_error "Please specify NODE_HOST manually using --node-host parameter"
    return 1
}

################################################################################
# Node Secret Generation
################################################################################

generate_node_secret() {
    log_debug "Generating node secret..."
    
    # Generate 32 bytes of random data using openssl
    # Convert to base64 and remove special characters (+, /, =)
    # Take first 32 characters to ensure consistent length
    local secret
    secret=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)
    
    # Verify the secret meets requirements
    if [ ${#secret} -lt 32 ]; then
        log_error "Generated secret is too short: ${#secret} characters"
        return 1
    fi
    
    # Verify the secret contains only alphanumeric characters
    if ! [[ "$secret" =~ ^[a-zA-Z0-9]+$ ]]; then
        log_error "Generated secret contains non-alphanumeric characters"
        return 1
    fi
    
    log_debug "Generated secret: $(mask_sensitive "$secret")"
    echo "$secret"
    return 0
}


################################################################################
# Node Agent Installation Functions
################################################################################

# Install Node Agent binary
install_node_agent() {
    log_info "Installing Node Agent..."
    
    # Check if Node Agent is already installed
    if command -v node-agent &> /dev/null; then
        local agent_version
        agent_version=$(node-agent --version 2>/dev/null || echo "unknown")
        log_info "Node Agent is already installed: $agent_version"
        log_info "Skipping installation (use --force to reinstall)"
        return 0
    fi
    
    # Detect system architecture
    local arch
    arch=$(uname -m)
    log_debug "Detected architecture: $arch"
    
    # Map architecture to binary naming convention
    local binary_arch
    case "$arch" in
        x86_64)
            binary_arch="x86_64"
            log_info "Architecture: x86_64 (64-bit Intel/AMD)"
            ;;
        aarch64|arm64)
            binary_arch="aarch64"
            log_info "Architecture: aarch64 (64-bit ARM)"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            log_error "Supported architectures: x86_64, aarch64"
            return 5
            ;;
    esac
    
    # Construct download URL
    local download_url="https://github.com/your-org/vpn-platform/releases/latest/download/node-agent-${binary_arch}"
    log_debug "Download URL: $download_url"
    
    # Download binary to temporary location first
    local temp_binary="/tmp/node-agent-${binary_arch}-$$"
    log_info "Downloading Node Agent binary..."
    log_debug "Temporary file: $temp_binary"
    
    # Download with retry mechanism
    local max_attempts=3
    local attempt=1
    local download_success=false
    
    while [ $attempt -le $max_attempts ]; do
        log_debug "Download attempt $attempt/$max_attempts"
        
        if wget -q --show-progress -O "$temp_binary" "$download_url" 2>&1 | tee -a "$TEST_LOG_FILE"; then
            download_success=true
            break
        else
            log_warn "Download attempt $attempt failed"
            
            if [ $attempt -lt $max_attempts ]; then
                log_info "Retrying in 3 seconds..."
                sleep 3
            fi
        fi
        
        ((attempt++))
    done
    
    if [ "$download_success" != "true" ]; then
        log_error "Failed to download Node Agent after $max_attempts attempts"
        log_error "URL: $download_url"
        rm -f "$temp_binary"
        return 5
    fi
    
    # Verify download (check file size)
    local file_size
    file_size=$(stat -f%z "$temp_binary" 2>/dev/null || stat -c%s "$temp_binary" 2>/dev/null || echo "0")
    log_debug "Downloaded file size: $file_size bytes"
    
    if [ "$file_size" -lt 1000 ]; then
        log_error "Downloaded file is too small ($file_size bytes)"
        rm -f "$temp_binary"
        return 5
    fi
    
    log_info "Node Agent binary downloaded successfully ($file_size bytes)"
    
    # Set execute permissions
    log_debug "Setting execute permissions..."
    if ! chmod +x "$temp_binary"; then
        log_error "Failed to set execute permissions"
        rm -f "$temp_binary"
        return 5
    fi
    
    # Move to installation directory
    local install_path="/usr/local/bin/node-agent"
    log_info "Installing to: $install_path"
    
    if ! mv "$temp_binary" "$install_path"; then
        log_error "Failed to move binary to $install_path"
        rm -f "$temp_binary"
        return 5
    fi
    
    # Verify installation
    if ! command -v node-agent &> /dev/null; then
        log_error "Node Agent installation verification failed"
        return 5
    fi
    
    # Get installed version
    local agent_version
    agent_version=$(node-agent --version 2>/dev/null || echo "installed")
    log_info "Node Agent installed successfully: $agent_version"
    
    return 0
}

# Create Node Agent configuration
create_node_agent_config() {
    log_info "Creating Node Agent configuration..."
    
    # Validate required configuration values
    if [ -z "${DEPLOY_CONFIG[api_url]}" ]; then
        log_error "API_URL is not set"
        return 5
    fi
    
    if [ -z "${DEPLOY_CONFIG[node_id]}" ]; then
        log_error "NODE_ID is not set"
        return 5
    fi
    
    if [ -z "${DEPLOY_CONFIG[node_secret]}" ]; then
        log_error "NODE_SECRET is not set"
        return 5
    fi
    
    # Create configuration directory
    local config_dir="/etc/node-agent"
    log_debug "Configuration directory: $config_dir"
    
    if [ ! -d "$config_dir" ]; then
        log_info "Creating configuration directory: $config_dir"
        if ! mkdir -p "$config_dir"; then
            log_error "Failed to create configuration directory"
            return 5
        fi
    else
        log_debug "Configuration directory already exists"
    fi
    
    # Backup existing configuration if it exists
    local config_file="$config_dir/config.env"
    if [ -f "$config_file" ]; then
        local backup_file="${config_file}.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backing up existing configuration to: $backup_file"
        cp "$config_file" "$backup_file"
    fi
    
    # Generate configuration file
    log_info "Writing configuration file: $config_file"
    log_debug "API_URL: ${DEPLOY_CONFIG[api_url]}"
    log_debug "NODE_ID: ${DEPLOY_CONFIG[node_id]}"
    log_debug "NODE_SECRET: $(mask_sensitive "${DEPLOY_CONFIG[node_secret]}")"
    
    cat > "$config_file" <<EOF
# Node Agent Configuration
# Generated by deployment script on $(date '+%Y-%m-%d %H:%M:%S')

# API Service URL
API_URL=${DEPLOY_CONFIG[api_url]}

# Node Identification
NODE_ID=${DEPLOY_CONFIG[node_id]}
NODE_SECRET=${DEPLOY_CONFIG[node_secret]}

# Xray API Configuration
XRAY_API_PORT=10085

# Reporting Intervals (in seconds)
TRAFFIC_REPORT_INTERVAL=30
HEARTBEAT_INTERVAL=60

# Logging
RUST_LOG=info
EOF
    
    if [ $? -ne 0 ]; then
        log_error "Failed to write configuration file"
        return 5
    fi
    
    log_info "Configuration file created successfully"
    
    # Set secure file permissions (600 = owner read/write only)
    log_info "Setting secure file permissions (600)..."
    if ! chmod 600 "$config_file"; then
        log_error "Failed to set file permissions"
        return 5
    fi
    
    # Verify permissions
    local file_perms
    file_perms=$(stat -f%Lp "$config_file" 2>/dev/null || stat -c%a "$config_file" 2>/dev/null)
    log_debug "File permissions: $file_perms"
    
    if [ "$file_perms" != "600" ]; then
        log_warn "File permissions are not 600 (actual: $file_perms)"
    else
        log_info "File permissions verified: 600 (secure)"
    fi
    
    log_info "Node Agent configuration completed successfully"
    return 0
}

# Create systemd service for Node Agent
create_node_agent_service() {
    log_info "Creating Node Agent systemd service..."
    
    local service_file="/etc/systemd/system/node-agent.service"
    log_debug "Service file: $service_file"
    
    # Backup existing service file if it exists
    if [ -f "$service_file" ]; then
        local backup_file="${service_file}.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backing up existing service file to: $backup_file"
        cp "$service_file" "$backup_file"
    fi
    
    # Create systemd service unit file
    log_info "Writing systemd service file..."
    
    cat > "$service_file" <<'EOF'
[Unit]
Description=VPN Node Agent
Documentation=https://github.com/your-org/vpn-platform
After=network.target network-online.target
Wants=network-online.target
Requires=xray.service

[Service]
Type=simple
User=root
WorkingDirectory=/etc/node-agent
EnvironmentFile=/etc/node-agent/config.env
ExecStart=/usr/local/bin/node-agent
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/etc/xray /var/log

# Resource limits
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
    
    if [ $? -ne 0 ]; then
        log_error "Failed to write systemd service file"
        return 5
    fi
    
    log_info "Systemd service file created successfully"
    
    # Reload systemd daemon to recognize new service
    log_info "Reloading systemd daemon..."
    if ! systemctl daemon-reload 2>&1 | tee -a "$TEST_LOG_FILE"; then
        log_error "Failed to reload systemd daemon"
        return 5
    fi
    
    log_info "Systemd daemon reloaded successfully"
    
    # Enable service for automatic startup on boot
    log_info "Enabling Node Agent service for automatic startup..."
    if ! systemctl enable node-agent 2>&1 | tee -a "$TEST_LOG_FILE"; then
        log_error "Failed to enable Node Agent service"
        return 5
    fi
    
    log_info "Node Agent service enabled successfully"
    log_info "Service will start automatically on system boot"
    
    return 0
}

################################################################################
# Batch Deployment Functions
################################################################################

# Batch deployment results
declare -a BATCH_RESULTS=()

# Batch configuration file
BATCH_CONFIG_FILE=""

# Batch node count
BATCH_NODE_COUNT=0

# Batch deployment counters
BATCH_SUCCESS_COUNT=0
BATCH_FAIL_COUNT=0

# Parse batch configuration file (JSON format with jq)
parse_batch_config_jq() {
    local config_file="$1"
    
    log_debug "Parsing JSON configuration with jq..."
    
    # Validate JSON syntax
    if ! jq empty "$config_file" 2>/dev/null; then
        log_error "Invalid JSON syntax in configuration file"
        return 1
    fi
    
    # Extract API configuration
    local api_url
    api_url=$(jq -r '.api_url // empty' "$config_file" 2>/dev/null)
    if [ -z "$api_url" ]; then
        log_error "Missing 'api_url' in configuration file"
        return 1
    fi
    DEPLOY_CONFIG[api_url]="$api_url"
    log_debug "API URL: $api_url"
    
    local admin_token
    admin_token=$(jq -r '.admin_token // empty' "$config_file" 2>/dev/null)
    if [ -z "$admin_token" ]; then
        log_error "Missing 'admin_token' in configuration file"
        return 1
    fi
    DEPLOY_CONFIG[admin_token]="$admin_token"
    log_debug "Admin Token: $(mask_sensitive "$admin_token")"
    
    # Count nodes in configuration
    local node_count
    node_count=$(jq '.nodes | length' "$config_file" 2>/dev/null)
    
    if [ -z "$node_count" ] || [ "$node_count" -eq 0 ]; then
        log_error "No nodes defined in configuration file"
        log_error "Configuration must contain a 'nodes' array with at least one node"
        return 1
    fi
    
    log_info "Found $node_count node(s) in configuration"
    
    # Store node count for later use
    BATCH_NODE_COUNT=$node_count
    
    return 0
}

# Extract node configuration at specific index
extract_node_config() {
    local config_file="$1"
    local index="$2"
    
    log_debug "Extracting node configuration at index $index"
    
    # Use jq for JSON parsing
    DEPLOY_CONFIG[node_name]=$(jq -r ".nodes[$index].name // empty" "$config_file" 2>/dev/null)
    DEPLOY_CONFIG[node_host]=$(jq -r ".nodes[$index].host // \"\"" "$config_file" 2>/dev/null)
    DEPLOY_CONFIG[node_port]=$(jq -r ".nodes[$index].port // 443" "$config_file" 2>/dev/null)
    DEPLOY_CONFIG[node_protocol]=$(jq -r ".nodes[$index].protocol // \"vless\"" "$config_file" 2>/dev/null)
    
    # Extract config object (if present)
    local node_config
    node_config=$(jq -c ".nodes[$index].config // {}" "$config_file" 2>/dev/null)
    if [ "$node_config" != "null" ] && [ -n "$node_config" ]; then
        DEPLOY_CONFIG[node_config]="$node_config"
    else
        DEPLOY_CONFIG[node_config]="{}"
    fi
    
    # Validate required fields
    if [ -z "${DEPLOY_CONFIG[node_name]}" ] || [ "${DEPLOY_CONFIG[node_name]}" = "null" ]; then
        log_error "Node at index $index is missing 'name' field"
        return 1
    fi
    
    # Handle "auto" for node_host (auto-detect IP)
    if [ "${DEPLOY_CONFIG[node_host]}" = "auto" ] || [ -z "${DEPLOY_CONFIG[node_host]}" ]; then
        log_debug "Node host set to 'auto', will detect public IP"
        DEPLOY_CONFIG[node_host]=""
    fi
    
    # Ensure null values are converted to defaults
    if [ "${DEPLOY_CONFIG[node_port]}" = "null" ]; then
        DEPLOY_CONFIG[node_port]="443"
    fi
    
    if [ "${DEPLOY_CONFIG[node_protocol]}" = "null" ]; then
        DEPLOY_CONFIG[node_protocol]="vless"
    fi
    
    log_debug "Extracted node config:"
    log_debug "  Name: ${DEPLOY_CONFIG[node_name]}"
    log_debug "  Host: ${DEPLOY_CONFIG[node_host]:-auto}"
    log_debug "  Port: ${DEPLOY_CONFIG[node_port]}"
    log_debug "  Protocol: ${DEPLOY_CONFIG[node_protocol]}"
    
    return 0
}

# Record batch deployment result
record_batch_result() {
    local node_name="$1"
    local status="$2"
    local message="$3"
    
    BATCH_RESULTS+=("$node_name|$status|$message")
    log_debug "Recorded batch result: $node_name - $status"
}

# Generate and display batch deployment summary report
generate_batch_report() {
    log_info ""
    log_info "=========================================="
    log_info "Batch Deployment Summary Report"
    log_info "=========================================="
    log_info ""
    log_info "Deployment Statistics:"
    log_info "  Total Nodes: $((BATCH_SUCCESS_COUNT + BATCH_FAIL_COUNT))"
    log_info "  Successful: $BATCH_SUCCESS_COUNT"
    log_info "  Failed: $BATCH_FAIL_COUNT"
    
    if [ $BATCH_SUCCESS_COUNT -gt 0 ] && [ $BATCH_FAIL_COUNT -eq 0 ]; then
        log_info "  Status: ✓ All nodes deployed successfully"
    elif [ $BATCH_SUCCESS_COUNT -gt 0 ] && [ $BATCH_FAIL_COUNT -gt 0 ]; then
        log_info "  Status: ⚠ Partial success"
    elif [ $BATCH_SUCCESS_COUNT -eq 0 ]; then
        log_info "  Status: ✗ All nodes failed"
    fi
    
    log_info ""
    log_info "Node Details:"
    log_info "----------------------------------------"
    
    # Display each node's result
    if [ ${#BATCH_RESULTS[@]} -eq 0 ]; then
        log_info "  No deployment results recorded"
    else
        for result in "${BATCH_RESULTS[@]}"; do
            # Parse result string (format: "name|status|message")
            IFS='|' read -r name status message <<< "$result"
            
            if [ "$status" = "success" ]; then
                log_info "  ✓ $name"
                log_info "    Status: Success"
                log_info "    Message: $message"
            else
                log_error "  ✗ $name"
                log_error "    Status: Failed"
                log_error "    Message: $message"
            fi
            log_info ""
        done
    fi
    
    log_info "=========================================="
    log_info ""
    
    # Provide recommendations based on results
    if [ $BATCH_FAIL_COUNT -gt 0 ]; then
        log_info "Troubleshooting Failed Nodes:"
        log_info "  1. Check deployment log: $TEST_LOG_FILE"
        log_info "  2. Review error messages above"
        log_info "  3. Verify network connectivity and API access"
        log_info "  4. Check node-specific configuration"
        log_info "  5. Retry failed nodes individually for detailed errors"
        log_info ""
    fi
    
    if [ $BATCH_SUCCESS_COUNT -gt 0 ]; then
        log_info "Next Steps for Successful Nodes:"
        log_info "  1. Verify nodes in admin panel"
        log_info "  2. Test connectivity from client devices"
        log_info "  3. Monitor service logs: journalctl -u node-agent -f"
        log_info "  4. Check node status: systemctl status node-agent"
        log_info ""
    fi
    
    log_info "Deployment completed at: $(date '+%Y-%m-%d %H:%M:%S')"
    log_info ""
}

################################################################################
# Service Management Functions (Stubs for Testing)
################################################################################

show_troubleshooting_tips() {
    log_info ""
    log_info "=========================================="
    log_info "Troubleshooting Tips"
    log_info "=========================================="
    log_info ""
    log_info "If the service failed to start, try these steps:"
    log_info ""
    log_info "1. Check service status:"
    log_info "   systemctl status node-agent"
    log_info ""
    log_info "2. View detailed service logs:"
    log_info "   journalctl -u node-agent -n 50 --no-pager"
    log_info ""
    log_info "3. Check configuration file:"
    log_info "   cat /etc/node-agent/config.env"
    log_info ""
    log_info "4. Verify Node Agent binary:"
    log_info "   ls -la /usr/local/bin/node-agent"
    log_info "   /usr/local/bin/node-agent --version"
    log_info ""
    log_info "5. Check Xray-core status:"
    log_info "   systemctl status xray"
    log_info ""
    log_info "6. Verify network connectivity:"
    log_info "   curl -I ${DEPLOY_CONFIG[api_url]}"
    log_info ""
    log_info "7. Check port availability:"
    log_info "   ss -tuln | grep ${DEPLOY_CONFIG[node_port]}"
    log_info ""
    log_info "8. Review deployment log:"
    log_info "   cat $TEST_LOG_FILE"
    log_info ""
    log_info "Common issues:"
    log_info "  - Port already in use: Change NODE_PORT or stop conflicting service"
    log_info "  - API unreachable: Check firewall rules and network connectivity"
    log_info "  - Permission denied: Ensure script was run as root"
    log_info "  - Binary not found: Check if download completed successfully"
    log_info ""
    log_info "For more help, check the documentation or contact support"
    log_info ""
}
