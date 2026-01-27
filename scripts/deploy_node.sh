#!/usr/bin/env bash

################################################################################
# VPN Node One-Click Deployment Script
# 
# This script automates the deployment of VPN nodes including:
# - Node creation via API
# - Xray-core installation
# - Node Agent installation and configuration
# - Service startup and verification
#
# Usage: ./deploy_node.sh --api-url <URL> --admin-token <TOKEN> --node-name <NAME>
#
# Requirements: 10.1, 10.2, 12.3
################################################################################

# Check bash version (requires 4.0+ for associative arrays)
if [ "${BASH_VERSINFO[0]}" -lt 4 ]; then
    echo "Error: This script requires Bash 4.0 or higher" >&2
    echo "Current version: $BASH_VERSION" >&2
    echo "Please upgrade bash or run on a Linux system with bash 4.0+" >&2
    exit 1
fi

set -e  # Exit immediately if a command exits with a non-zero status
set -o pipefail  # Pipe failures cause script to fail

################################################################################
# Global Configuration Variables
################################################################################

# Script version
readonly SCRIPT_VERSION="1.0.0"

# Log file location
readonly LOG_FILE="/var/log/node-deployment.log"

# Deployment state
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

declare -A DEPLOY_STATE=(
    [phase]="init"
    [start_time]=""
    [end_time]=""
    [errors]=0
    [warnings]=0
)

# Batch deployment results
declare -a BATCH_RESULTS=()

# Batch configuration file
BATCH_CONFIG_FILE=""

# Batch node count
BATCH_NODE_COUNT=0

# Batch deployment counters
BATCH_SUCCESS_COUNT=0
BATCH_FAIL_COUNT=0

# Script options
VERBOSE=false
QUIET=false
FORCE=false
ROLLBACK=false
CLEANUP=false
TEST_MODE=false

# Color codes for terminal output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

################################################################################
# Color Output Functions
################################################################################

# Print info message in green
# Usage: print_info "message"
print_info() {
    local msg="$1"
    if [ "$QUIET" != "true" ]; then
        echo -e "${GREEN}[INFO]${NC} $msg"
    fi
}

# Print warning message in yellow
# Usage: print_warn "message"
print_warn() {
    local msg="$1"
    if [ "$QUIET" != "true" ]; then
        echo -e "${YELLOW}[WARN]${NC} $msg"
    fi
}

# Print error message in red to stderr
# Usage: print_error "message"
print_error() {
    local msg="$1"
    echo -e "${RED}[ERROR]${NC} $msg" >&2
}

# Print debug message in blue (only in verbose mode)
# Usage: print_debug "message"
print_debug() {
    local msg="$1"
    if [ "$VERBOSE" = "true" ]; then
        echo -e "${BLUE}[DEBUG]${NC} $msg"
    fi
}

################################################################################
# Logging Functions
################################################################################

# Initialize log file
# Creates log directory and writes initial log header
# All operations are logged to /var/log/node-deployment.log with timestamps
# Usage: init_log
# Requirements: 10.1
init_log() {
    # Create log directory if it doesn't exist
    local log_dir=$(dirname "$LOG_FILE")
    if [ ! -d "$log_dir" ]; then
        mkdir -p "$log_dir" 2>/dev/null || true
    fi
    
    # Create or append to log file
    if [ -w "$log_dir" ] || [ -w "$LOG_FILE" ]; then
        echo "========================================" >> "$LOG_FILE"
        echo "Deployment started at $(date '+%Y-%m-%d %H:%M:%S')" >> "$LOG_FILE"
        echo "Script version: $SCRIPT_VERSION" >> "$LOG_FILE"
        echo "User: $(whoami)" >> "$LOG_FILE"
        echo "Hostname: $(hostname)" >> "$LOG_FILE"
        echo "Working directory: $(pwd)" >> "$LOG_FILE"
        echo "Command line: $0 $*" >> "$LOG_FILE"
        echo "========================================" >> "$LOG_FILE"
    else
        # If we can't write to log file, warn user but continue
        echo "Warning: Cannot write to log file: $LOG_FILE" >&2
        echo "Logging will be limited to console output only" >&2
    fi
}

# Log info message
# Usage: log_info "message"
log_info() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Write to log file
    echo "[$timestamp] [INFO] $msg" >> "$LOG_FILE" 2>/dev/null || true
    
    # Print to console
    print_info "$msg"
}

# Log warning message
# Usage: log_warn "message"
log_warn() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Write to log file
    echo "[$timestamp] [WARN] $msg" >> "$LOG_FILE" 2>/dev/null || true
    
    # Print to console
    print_warn "$msg"
    
    # Increment warning counter
    ((DEPLOY_STATE[warnings]++)) || true
}

# Log error message
# Usage: log_error "message"
log_error() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Write to log file
    echo "[$timestamp] [ERROR] $msg" >> "$LOG_FILE" 2>/dev/null || true
    
    # Print to console
    print_error "$msg"
    
    # Increment error counter
    ((DEPLOY_STATE[errors]++)) || true
}

# Log debug message (only in verbose mode)
# Usage: log_debug "message"
log_debug() {
    local msg="$1"
    
    if [ "$VERBOSE" = "true" ]; then
        local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        
        # Write to log file
        echo "[$timestamp] [DEBUG] $msg" >> "$LOG_FILE" 2>/dev/null || true
        
        # Print to console
        print_debug "$msg"
    fi
}

################################################################################
# Sensitive Information Masking
################################################################################

# Mask sensitive information for logging
# Shows only the first 8 characters followed by "..."
# Usage: mask_sensitive "secret_value"
# Returns: "secret_v..." or "***" for short values
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
# Cleanup and Error Handling
################################################################################

# Cleanup function called on script exit
cleanup() {
    local exit_code=$?
    
    # Record end time
    DEPLOY_STATE[end_time]=$(date '+%Y-%m-%d %H:%M:%S')
    
    if [ $exit_code -ne 0 ]; then
        log_error "Deployment failed with exit code $exit_code"
        log_error "Phase: ${DEPLOY_STATE[phase]}"
        log_error "Errors: ${DEPLOY_STATE[errors]}, Warnings: ${DEPLOY_STATE[warnings]}"
        
        # TODO: Implement phase-specific cleanup in future tasks
        # This will be expanded in task 13 (rollback mechanism)
    else
        log_info "Deployment completed successfully"
        log_info "Errors: ${DEPLOY_STATE[errors]}, Warnings: ${DEPLOY_STATE[warnings]}"
    fi
    
    # Write final log entry
    echo "========================================" >> "$LOG_FILE" 2>/dev/null || true
    echo "Deployment ended at ${DEPLOY_STATE[end_time]}" >> "$LOG_FILE" 2>/dev/null || true
    echo "Exit code: $exit_code" >> "$LOG_FILE" 2>/dev/null || true
    echo "========================================" >> "$LOG_FILE" 2>/dev/null || true
}

# Set trap to call cleanup with rollback on exit
trap cleanup_with_rollback EXIT

################################################################################
# Help and Usage
################################################################################

# Display usage information
show_usage() {
    cat << EOF
VPN Node One-Click Deployment Script v$SCRIPT_VERSION

Usage: $0 [OPTIONS]

Required Parameters:
  --api-url <URL>           API service URL (e.g., https://api.example.com)
  --admin-token <TOKEN>     Admin JWT token for authentication
  --node-name <NAME>        Node name/identifier

Optional Parameters:
  --node-host <HOST>        Node host address (default: auto-detect public IP)
  --node-port <PORT>        Node port (default: 443)
  --node-protocol <PROTO>   Protocol type: vless|vmess|trojan|shadowsocks|hysteria2 (default: vless)
  --node-config <JSON>      Protocol-specific configuration JSON (default: {})
  --batch-config <FILE>     Batch deployment configuration file

Options:
  --force                   Force redeployment without confirmation (skips update prompt)
  --rollback                Rollback to previous stable version
  --cleanup                 Clean up all node deployment files and services
  --verbose                 Enable verbose/debug output
  --quiet                   Suppress non-error output
  --help                    Display this help message

Environment Variables (used if command-line parameters not provided):
  API_URL                   API service URL
  ADMIN_TOKEN               Admin JWT token
  NODE_NAME                 Node name
  NODE_HOST                 Node host address
  NODE_PORT                 Node port
  NODE_PROTOCOL             Protocol type
  NODE_CONFIG               Protocol configuration JSON

Examples:
  # Basic deployment
  $0 --api-url https://api.example.com \\
     --admin-token eyJhbGc... \\
     --node-name node-hk-01

  # Deployment with custom port and protocol
  $0 --api-url https://api.example.com \\
     --admin-token eyJhbGc... \\
     --node-name node-us-01 \\
     --node-port 8443 \\
     --node-protocol vmess

  # Using environment variables
  export API_URL=https://api.example.com
  export ADMIN_TOKEN=eyJhbGc...
  export NODE_NAME=node-jp-01
  $0

  # Batch deployment
  $0 --batch-config /path/to/batch_config.yaml

Requirements: 1.1, 1.2, 1.3, 1.7
EOF
}

################################################################################
# Parameter Parsing
################################################################################

# Parse command-line arguments
# Supports both command-line parameters and environment variables
# Command-line parameters take precedence over environment variables
# Usage: parse_parameters "$@"
# Requirements: 1.1, 1.2, 1.3, 1.7
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
            --rollback)
                ROLLBACK=true
                shift
                ;;
            --cleanup)
                CLEANUP=true
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
            --help|-h)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown parameter: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Fall back to environment variables if command-line parameters not provided
    # Command-line parameters take precedence
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
    log_debug "API URL: ${DEPLOY_CONFIG[api_url]}"
    log_debug "Admin Token: $(mask_sensitive "${DEPLOY_CONFIG[admin_token]}")"
    log_debug "Node Name: ${DEPLOY_CONFIG[node_name]}"
    log_debug "Node Host: ${DEPLOY_CONFIG[node_host]}"
    log_debug "Node Port: ${DEPLOY_CONFIG[node_port]}"
    log_debug "Node Protocol: ${DEPLOY_CONFIG[node_protocol]}"
}

################################################################################
# Parameter Validation
################################################################################

# Validate URL format
# Usage: validate_url "https://example.com"
# Returns: 0 if valid, 1 if invalid
validate_url() {
    local url="$1"
    
    # Check if URL is empty
    if [ -z "$url" ]; then
        return 1
    fi
    
    # Basic URL format validation (http:// or https://)
    if [[ "$url" =~ ^https?://[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*(/.*)?$ ]]; then
        return 0
    else
        return 1
    fi
}

# Validate port number (1-65535)
# Usage: validate_port "443"
# Returns: 0 if valid, 1 if invalid
validate_port() {
    local port="$1"
    
    # Check if port is a number
    if ! [[ "$port" =~ ^[0-9]+$ ]]; then
        return 1
    fi
    
    # Check if port is in valid range (1-65535)
    if [ "$port" -ge 1 ] && [ "$port" -le 65535 ]; then
        return 0
    else
        return 1
    fi
}

# Validate protocol type
# Usage: validate_protocol "vless"
# Returns: 0 if valid, 1 if invalid
validate_protocol() {
    local protocol="$1"
    
    # List of supported protocols
    local valid_protocols=("shadowsocks" "vmess" "trojan" "hysteria2" "vless")
    
    # Check if protocol is in the list
    for valid_proto in "${valid_protocols[@]}"; do
        if [ "$protocol" = "$valid_proto" ]; then
            return 0
        fi
    done
    
    return 1
}

# Validate JWT token format
# JWT format: three base64-encoded segments separated by dots
# Usage: validate_jwt_format "eyJhbGc..."
# Returns: 0 if valid, 1 if invalid
# Requirements: 12.1
validate_jwt_format() {
    local token="$1"
    
    # Check if token is empty
    if [ -z "$token" ]; then
        return 1
    fi
    
    # JWT should have exactly 3 parts separated by dots
    local parts_count=$(echo "$token" | tr '.' '\n' | wc -l)
    if [ "$parts_count" -ne 3 ]; then
        return 1
    fi
    
    # Each part should be base64-like (alphanumeric, -, _)
    # We don't validate the actual JWT signature, just the format
    if [[ "$token" =~ ^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$ ]]; then
        return 0
    else
        return 1
    fi
}

# Validate all required parameters
# Usage: validate_parameters
# Returns: 0 if all valid, exits with error code 1 if invalid
# Requirements: 1.1, 1.2, 1.3, 12.1, 12.4
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
        else
            # Validate HTTPS usage (Task 14.2 - Requirement 12.4)
            if ! validate_https_url "${DEPLOY_CONFIG[api_url]}"; then
                log_error "API URL must use HTTPS for secure communication"
                ((validation_errors++))
            fi
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
        exit 1
    fi
    
    log_debug "Parameter validation completed successfully"
    return 0
}

################################################################################
# Default Value Handling
################################################################################

# Apply default values for optional parameters
# Usage: apply_defaults
# Requirements: 1.5, 1.6
apply_defaults() {
    log_debug "Applying default values for optional parameters..."
    
    # NODE_PORT defaults to 443 if not provided
    if [ -z "${DEPLOY_CONFIG[node_port]}" ] || [ "${DEPLOY_CONFIG[node_port]}" = "" ]; then
        DEPLOY_CONFIG[node_port]="443"
        log_debug "Applied default NODE_PORT: 443"
    fi
    
    # NODE_PROTOCOL defaults to vless if not provided
    if [ -z "${DEPLOY_CONFIG[node_protocol]}" ] || [ "${DEPLOY_CONFIG[node_protocol]}" = "" ]; then
        DEPLOY_CONFIG[node_protocol]="vless"
        log_debug "Applied default NODE_PROTOCOL: vless"
    fi
    
    # NODE_CONFIG defaults to empty object {} if not provided
    if [ -z "${DEPLOY_CONFIG[node_config]}" ] || [ "${DEPLOY_CONFIG[node_config]}" = "" ]; then
        DEPLOY_CONFIG[node_config]="{}"
        log_debug "Applied default NODE_CONFIG: {}"
    fi
    
    log_debug "Default values applied successfully"
}

################################################################################
# Environment Detection
################################################################################

# Global variables for OS detection
OS_TYPE=""
OS_VERSION=""
OS_ID=""

# Detect operating system type and version
# Reads /etc/os-release file to identify Ubuntu, CentOS, Debian
# Sets global variables: OS_TYPE, OS_VERSION, OS_ID
# Usage: detect_os
# Returns: 0 if supported OS detected, 1 if unsupported
# Requirements: 2.1
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

# Check if script is running with root privileges
# Usage: check_root
# Returns: 0 if root, exits with error code 2 if not root
# Requirements: 2.2
check_root() {
    log_debug "Checking root privileges..."
    
    # Check if EUID (Effective User ID) is 0 (root)
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run as root"
        log_error "Please run with sudo or as root user:"
        log_error "  sudo $0 $*"
        exit 2
    fi
    
    log_debug "Root privileges confirmed"
    return 0
}

################################################################################
# Dependency Check
################################################################################

# List of required system commands
REQUIRED_COMMANDS=("curl" "jq" "systemctl" "openssl")

# Check if required system commands are available
# Usage: check_dependencies
# Returns: 0 if all dependencies available, 1 if missing dependencies
# Requirements: 2.3, 2.4
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
    
    # Try to install missing dependencies
    log_info "Attempting to install missing dependencies..."
    if install_dependencies "${missing_deps[@]}"; then
        log_info "Successfully installed missing dependencies"
        return 0
    else
        log_error "Failed to install some dependencies"
        log_error "Please install manually: ${missing_deps[*]}"
        return 1
    fi
}

# Install missing dependencies based on OS type
# Usage: install_dependencies "curl" "jq" ...
# Returns: 0 if successful, 1 if failed
# Requirements: 2.4
install_dependencies() {
    local deps=("$@")
    
    if [ ${#deps[@]} -eq 0 ]; then
        return 0
    fi
    
    log_info "Installing dependencies: ${deps[*]}"
    
    case "$OS_TYPE" in
        ubuntu|debian)
            log_debug "Using apt package manager"
            
            # Update package list
            if ! apt-get update -qq; then
                log_error "Failed to update package list"
                return 1
            fi
            
            # Install each dependency
            for dep in "${deps[@]}"; do
                # Map command names to package names if needed
                local package="$dep"
                case "$dep" in
                    systemctl)
                        package="systemd"
                        ;;
                esac
                
                log_debug "Installing package: $package"
                if apt-get install -y -qq "$package"; then
                    log_info "Installed: $package"
                else
                    log_error "Failed to install: $package"
                    return 1
                fi
            done
            ;;
            
        centos)
            log_debug "Using yum package manager"
            
            # Install each dependency
            for dep in "${deps[@]}"; do
                # Map command names to package names if needed
                local package="$dep"
                case "$dep" in
                    systemctl)
                        package="systemd"
                        ;;
                esac
                
                log_debug "Installing package: $package"
                if yum install -y -q "$package"; then
                    log_info "Installed: $package"
                else
                    log_error "Failed to install: $package"
                    return 1
                fi
            done
            ;;
            
        *)
            log_error "Cannot install dependencies: unsupported OS type"
            return 1
            ;;
    esac
    
    return 0
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

# Detect public IP address using multiple services
# Tries multiple IP detection services with retry and fallback
# Usage: detect_public_ip
# Returns: IP address on stdout, 0 on success, 1 on failure
# Requirements: 1.4
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

# Generate a cryptographically secure random node secret
# Uses openssl to generate 32 bytes of random data, converts to base64,
# and cleans special characters to ensure only alphanumeric characters
# Usage: generate_node_secret
# Returns: 32+ character alphanumeric secret on stdout
# Requirements: 3.1, 3.3
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
# API Client Functions
################################################################################

# Create a node via API
# Sends POST request to /api/admin/nodes with node configuration
# Usage: create_node
# Returns: 0 on success, non-zero on failure
# Sets DEPLOY_CONFIG[node_id] and DEPLOY_CONFIG[node_secret] on success
# Requirements: 4.1, 4.2, 4.3, 4.4
create_node() {
    log_info "Creating node via API..."
    log_debug "API URL: ${DEPLOY_CONFIG[api_url]}"
    log_debug "Node Name: ${DEPLOY_CONFIG[node_name]}"
    log_debug "Node Host: ${DEPLOY_CONFIG[node_host]}"
    log_debug "Node Port: ${DEPLOY_CONFIG[node_port]}"
    log_debug "Node Protocol: ${DEPLOY_CONFIG[node_protocol]}"
    
    # Generate node secret
    local node_secret
    if ! node_secret=$(generate_node_secret); then
        log_error "Failed to generate node secret"
        return 1
    fi
    
    log_debug "Generated secret: $(mask_sensitive "$node_secret")"
    
    # Build JSON request body
    local request_body
    request_body=$(cat <<EOF
{
    "name": "${DEPLOY_CONFIG[node_name]}",
    "host": "${DEPLOY_CONFIG[node_host]}",
    "port": ${DEPLOY_CONFIG[node_port]},
    "protocol": "${DEPLOY_CONFIG[node_protocol]}",
    "secret": "$node_secret",
    "config": ${DEPLOY_CONFIG[node_config]}
}
EOF
)
    
    log_debug "Request body prepared (secret masked)"
    
    # Prepare API endpoint
    local api_endpoint="${DEPLOY_CONFIG[api_url]}/api/admin/nodes"
    log_debug "API endpoint: $api_endpoint"
    
    # Send POST request with retry mechanism
    local response
    local http_code
    local attempt=1
    local max_attempts=3
    local retry_delay=5
    
    while [ $attempt -le $max_attempts ]; do
        log_debug "API call attempt $attempt/$max_attempts"
        
        # Make API call and capture both response body and HTTP status code
        local temp_response
        temp_response=$(curl -s -w "\n%{http_code}" \
            -X POST \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer ${DEPLOY_CONFIG[admin_token]}" \
            -d "$request_body" \
            "$api_endpoint" 2>&1)
        
        local curl_exit_code=$?
        
        # Check if curl command succeeded
        if [ $curl_exit_code -ne 0 ]; then
            log_warn "Network error: curl failed with exit code $curl_exit_code"
            
            if [ $attempt -lt $max_attempts ]; then
                log_info "Retrying in ${retry_delay}s... (attempt $((attempt + 1))/$max_attempts)"
                sleep $retry_delay
                ((attempt++))
                continue
            else
                log_error "Network error: Failed to connect to API after $max_attempts attempts"
                log_error "Please check:"
                log_error "  - Network connectivity"
                log_error "  - API URL is correct: ${DEPLOY_CONFIG[api_url]}"
                log_error "  - Firewall settings"
                return 3
            fi
        fi
        
        # Extract HTTP status code (last line) and response body (everything else)
        http_code=$(echo "$temp_response" | tail -n1)
        response=$(echo "$temp_response" | sed '$d')
        
        log_debug "HTTP Status Code: $http_code"
        log_debug "Response body: $response"
        
        # Handle response based on HTTP status code
        case "$http_code" in
            200|201)
                # Success - parse response
                log_info "Node created successfully (HTTP $http_code)"
                
                # Extract node_id and secret from response
                local node_id
                local returned_secret
                
                node_id=$(echo "$response" | jq -r '.id // .node_id // empty' 2>/dev/null)
                returned_secret=$(echo "$response" | jq -r '.secret // empty' 2>/dev/null)
                
                # Validate extracted values
                if [ -z "$node_id" ]; then
                    log_error "Failed to extract node_id from API response"
                    log_error "Response: $response"
                    return 4
                fi
                
                # Store node_id and secret in config
                DEPLOY_CONFIG[node_id]="$node_id"
                
                # Use returned secret if provided, otherwise use the one we generated
                if [ -n "$returned_secret" ]; then
                    DEPLOY_CONFIG[node_secret]="$returned_secret"
                    log_debug "Using secret from API response"
                else
                    DEPLOY_CONFIG[node_secret]="$node_secret"
                    log_debug "Using generated secret"
                fi
                
                log_info "Node ID: $node_id"
                log_info "Node Secret: $(mask_sensitive "${DEPLOY_CONFIG[node_secret]}")"
                
                return 0
                ;;
                
            400)
                # Bad Request - parameter error
                log_error "API Error (HTTP 400): Bad Request"
                log_error "The request parameters are invalid"
                
                # Try to extract error message from response
                local error_msg
                error_msg=$(echo "$response" | jq -r '.error // .message // empty' 2>/dev/null)
                if [ -n "$error_msg" ]; then
                    log_error "Error details: $error_msg"
                fi
                
                log_error "Please check:"
                log_error "  - Node name is valid"
                log_error "  - Host address is valid"
                log_error "  - Port number is valid (1-65535)"
                log_error "  - Protocol is supported"
                
                return 4
                ;;
                
            401|403)
                # Unauthorized/Forbidden - authentication error
                log_error "API Error (HTTP $http_code): Authentication Failed"
                log_error "The admin token is invalid or you don't have permission"
                
                # Try to extract error message
                local error_msg
                error_msg=$(echo "$response" | jq -r '.error // .message // empty' 2>/dev/null)
                if [ -n "$error_msg" ]; then
                    log_error "Error details: $error_msg"
                fi
                
                log_error "Please check:"
                log_error "  - Admin token is valid and not expired"
                log_error "  - Token has admin privileges"
                log_error "  - Token format: $(mask_sensitive "${DEPLOY_CONFIG[admin_token]}")"
                
                return 4
                ;;
                
            409)
                # Conflict - node already exists
                log_error "API Error (HTTP 409): Conflict"
                log_error "A node with this name already exists"
                
                # Try to extract error message
                local error_msg
                error_msg=$(echo "$response" | jq -r '.error // .message // empty' 2>/dev/null)
                if [ -n "$error_msg" ]; then
                    log_error "Error details: $error_msg"
                fi
                
                log_error "Please:"
                log_error "  - Use a different node name"
                log_error "  - Or delete the existing node first"
                log_error "  - Or use --force to update the existing node"
                
                return 4
                ;;
                
            500|502|503|504)
                # Server Error - retry
                log_warn "API Error (HTTP $http_code): Server Error"
                
                # Try to extract error message
                local error_msg
                error_msg=$(echo "$response" | jq -r '.error // .message // empty' 2>/dev/null)
                if [ -n "$error_msg" ]; then
                    log_warn "Error details: $error_msg"
                fi
                
                if [ $attempt -lt $max_attempts ]; then
                    log_info "Retrying in ${retry_delay}s... (attempt $((attempt + 1))/$max_attempts)"
                    sleep $retry_delay
                    ((attempt++))
                    continue
                else
                    log_error "Server error persists after $max_attempts attempts"
                    log_error "Please contact the system administrator"
                    return 4
                fi
                ;;
                
            *)
                # Unknown error
                log_error "API Error (HTTP $http_code): Unexpected response"
                log_error "Response: $response"
                
                if [ $attempt -lt $max_attempts ]; then
                    log_info "Retrying in ${retry_delay}s... (attempt $((attempt + 1))/$max_attempts)"
                    sleep $retry_delay
                    ((attempt++))
                    continue
                else
                    log_error "Failed after $max_attempts attempts"
                    return 4
                fi
                ;;
        esac
    done
    
    # Should not reach here
    log_error "Unexpected error in create_node function"
    return 4
}

################################################################################
# Xray-core Installation
################################################################################

# Install Xray-core using the official installation script
# Downloads and installs the latest stable version of Xray-core
# Enables systemd service for automatic startup
# Usage: install_xray
# Returns: 0 on success, 5 on failure
# Requirements: 5.1, 5.2, 5.3, 5.4
install_xray() {
    log_info "Installing Xray-core..."
    
    # Check if Xray is already installed
    if command -v xray &> /dev/null; then
        local xray_version
        xray_version=$(xray version 2>/dev/null | head -n1 || echo "unknown")
        log_info "Xray-core is already installed: $xray_version"
        log_info "Skipping installation (use --force to reinstall)"
        return 0
    fi
    
    # Download and run official installation script
    log_info "Downloading Xray-core installation script..."
    
    local install_script_url="https://github.com/XTLS/Xray-install/raw/main/install-release.sh"
    local temp_install_script="/tmp/xray-install.sh"
    
    # Register temporary file for cleanup (Task 14.4)
    register_temp_file "$temp_install_script"
    
    # Download installation script
    if ! curl -L -o "$temp_install_script" "$install_script_url" 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Failed to download Xray-core installation script"
        log_error "URL: $install_script_url"
        log_error "Please check network connectivity"
        rm -f "$temp_install_script"
        return 5
    fi
    
    # Make script executable
    chmod +x "$temp_install_script"
    
    # Run installation script
    log_info "Running Xray-core installation script..."
    if bash "$temp_install_script" install 2>&1 | tee -a "$LOG_FILE"; then
        log_info "Xray-core installation script completed"
    else
        log_error "Xray-core installation script failed"
        rm -f "$temp_install_script"
        return 5
    fi
    
    # Clean up installation script
    rm -f "$temp_install_script"
    
    # Verify installation
    if ! command -v xray &> /dev/null; then
        log_error "Xray-core installation verification failed"
        log_error "xray command not found in PATH"
        return 5
    fi
    
    # Get installed version
    local xray_version
    xray_version=$(xray version 2>/dev/null | head -n1 || echo "unknown")
    log_info "Xray-core installed successfully: $xray_version"
    
    # Enable systemd service (but don't start it yet - Node Agent will manage it)
    log_info "Enabling Xray-core systemd service..."
    if systemctl enable xray 2>&1 | tee -a "$LOG_FILE"; then
        log_info "Xray-core service enabled"
    else
        log_warn "Failed to enable Xray-core service"
        log_warn "Service may need to be enabled manually"
    fi
    
    log_info "Xray-core installation completed successfully"
    return 0
}

# Generate Xray-core configuration based on protocol type
# Supports: vless, vmess, trojan, shadowsocks, hysteria2
# Writes configuration to /etc/xray/config.json
# Usage: generate_xray_config
# Returns: 0 on success, 5 on failure
# Requirements: 5.5
generate_xray_config() {
    log_info "Generating Xray-core configuration..."
    log_debug "Protocol: ${DEPLOY_CONFIG[node_protocol]}"
    log_debug "Port: ${DEPLOY_CONFIG[node_port]}"
    
    local config_dir="/etc/xray"
    local config_file="$config_dir/config.json"
    
    # Create config directory if it doesn't exist
    if [ ! -d "$config_dir" ]; then
        log_debug "Creating config directory: $config_dir"
        mkdir -p "$config_dir"
    fi
    
    # Backup existing config if it exists
    if [ -f "$config_file" ]; then
        local backup_file="${config_file}.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backing up existing config to: $backup_file"
        cp "$config_file" "$backup_file"
    fi
    
    # Generate configuration based on protocol
    local config_content
    
    case "${DEPLOY_CONFIG[node_protocol]}" in
        vless)
            config_content=$(generate_vless_config)
            ;;
        vmess)
            config_content=$(generate_vmess_config)
            ;;
        trojan)
            config_content=$(generate_trojan_config)
            ;;
        shadowsocks)
            config_content=$(generate_shadowsocks_config)
            ;;
        hysteria2)
            config_content=$(generate_hysteria2_config)
            ;;
        *)
            log_error "Unsupported protocol: ${DEPLOY_CONFIG[node_protocol]}"
            return 5
            ;;
    esac
    
    # Write configuration to file
    if echo "$config_content" > "$config_file"; then
        log_info "Configuration written to: $config_file"
    else
        log_error "Failed to write configuration file"
        return 5
    fi
    
    # Set appropriate permissions
    chmod 644 "$config_file"
    
    # Validate JSON syntax
    if ! jq empty "$config_file" 2>/dev/null; then
        log_error "Generated configuration has invalid JSON syntax"
        log_error "Please check the configuration file: $config_file"
        return 5
    fi
    
    log_info "Xray-core configuration generated successfully"
    return 0
}

# Generate VLESS protocol configuration
# Usage: generate_vless_config
# Returns: JSON configuration on stdout
generate_vless_config() {
    local uuid="${DEPLOY_CONFIG[node_secret]}"
    local port="${DEPLOY_CONFIG[node_port]}"
    
    cat <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": ${port},
      "protocol": "vless",
      "settings": {
        "clients": [],
        "decryption": "none"
      },
      "streamSettings": {
        "network": "tcp",
        "security": "none"
      },
      "sniffing": {
        "enabled": true,
        "destOverride": ["http", "tls"]
      }
    },
    {
      "listen": "127.0.0.1",
      "port": 10085,
      "protocol": "dokodemo-door",
      "settings": {
        "address": "127.0.0.1"
      },
      "tag": "api"
    }
  ],
  "outbounds": [
    {
      "protocol": "freedom",
      "settings": {}
    },
    {
      "protocol": "blackhole",
      "settings": {},
      "tag": "blocked"
    }
  ],
  "api": {
    "tag": "api",
    "services": [
      "HandlerService",
      "StatsService"
    ]
  },
  "stats": {},
  "policy": {
    "levels": {
      "0": {
        "statsUserUplink": true,
        "statsUserDownlink": true
      }
    },
    "system": {
      "statsInboundUplink": true,
      "statsInboundDownlink": true
    }
  },
  "routing": {
    "rules": [
      {
        "inboundTag": ["api"],
        "outboundTag": "api",
        "type": "field"
      }
    ]
  }
}
EOF
}

# Generate VMess protocol configuration
# Usage: generate_vmess_config
# Returns: JSON configuration on stdout
generate_vmess_config() {
    local uuid="${DEPLOY_CONFIG[node_secret]}"
    local port="${DEPLOY_CONFIG[node_port]}"
    
    cat <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": ${port},
      "protocol": "vmess",
      "settings": {
        "clients": []
      },
      "streamSettings": {
        "network": "tcp",
        "security": "none"
      },
      "sniffing": {
        "enabled": true,
        "destOverride": ["http", "tls"]
      }
    },
    {
      "listen": "127.0.0.1",
      "port": 10085,
      "protocol": "dokodemo-door",
      "settings": {
        "address": "127.0.0.1"
      },
      "tag": "api"
    }
  ],
  "outbounds": [
    {
      "protocol": "freedom",
      "settings": {}
    },
    {
      "protocol": "blackhole",
      "settings": {},
      "tag": "blocked"
    }
  ],
  "api": {
    "tag": "api",
    "services": [
      "HandlerService",
      "StatsService"
    ]
  },
  "stats": {},
  "policy": {
    "levels": {
      "0": {
        "statsUserUplink": true,
        "statsUserDownlink": true
      }
    },
    "system": {
      "statsInboundUplink": true,
      "statsInboundDownlink": true
    }
  },
  "routing": {
    "rules": [
      {
        "inboundTag": ["api"],
        "outboundTag": "api",
        "type": "field"
      }
    ]
  }
}
EOF
}

# Generate Trojan protocol configuration
# Usage: generate_trojan_config
# Returns: JSON configuration on stdout
generate_trojan_config() {
    local password="${DEPLOY_CONFIG[node_secret]}"
    local port="${DEPLOY_CONFIG[node_port]}"
    
    cat <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": ${port},
      "protocol": "trojan",
      "settings": {
        "clients": [],
        "fallbacks": [
          {
            "dest": 80
          }
        ]
      },
      "streamSettings": {
        "network": "tcp",
        "security": "none"
      },
      "sniffing": {
        "enabled": true,
        "destOverride": ["http", "tls"]
      }
    },
    {
      "listen": "127.0.0.1",
      "port": 10085,
      "protocol": "dokodemo-door",
      "settings": {
        "address": "127.0.0.1"
      },
      "tag": "api"
    }
  ],
  "outbounds": [
    {
      "protocol": "freedom",
      "settings": {}
    },
    {
      "protocol": "blackhole",
      "settings": {},
      "tag": "blocked"
    }
  ],
  "api": {
    "tag": "api",
    "services": [
      "HandlerService",
      "StatsService"
    ]
  },
  "stats": {},
  "policy": {
    "levels": {
      "0": {
        "statsUserUplink": true,
        "statsUserDownlink": true
      }
    },
    "system": {
      "statsInboundUplink": true,
      "statsInboundDownlink": true
    }
  },
  "routing": {
    "rules": [
      {
        "inboundTag": ["api"],
        "outboundTag": "api",
        "type": "field"
      }
    ]
  }
}
EOF
}

# Generate Shadowsocks protocol configuration
# Usage: generate_shadowsocks_config
# Returns: JSON configuration on stdout
generate_shadowsocks_config() {
    local password="${DEPLOY_CONFIG[node_secret]}"
    local port="${DEPLOY_CONFIG[node_port]}"
    
    cat <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": ${port},
      "protocol": "shadowsocks",
      "settings": {
        "method": "aes-256-gcm",
        "password": "${password}",
        "network": "tcp,udp"
      },
      "sniffing": {
        "enabled": true,
        "destOverride": ["http", "tls"]
      }
    },
    {
      "listen": "127.0.0.1",
      "port": 10085,
      "protocol": "dokodemo-door",
      "settings": {
        "address": "127.0.0.1"
      },
      "tag": "api"
    }
  ],
  "outbounds": [
    {
      "protocol": "freedom",
      "settings": {}
    },
    {
      "protocol": "blackhole",
      "settings": {},
      "tag": "blocked"
    }
  ],
  "api": {
    "tag": "api",
    "services": [
      "HandlerService",
      "StatsService"
    ]
  },
  "stats": {},
  "policy": {
    "levels": {
      "0": {
        "statsUserUplink": true,
        "statsUserDownlink": true
      }
    },
    "system": {
      "statsInboundUplink": true,
      "statsInboundDownlink": true
    }
  },
  "routing": {
    "rules": [
      {
        "inboundTag": ["api"],
        "outboundTag": "api",
        "type": "field"
      }
    ]
  }
}
EOF
}

# Generate Hysteria2 protocol configuration
# Usage: generate_hysteria2_config
# Returns: JSON configuration on stdout
generate_hysteria2_config() {
    local password="${DEPLOY_CONFIG[node_secret]}"
    local port="${DEPLOY_CONFIG[node_port]}"
    
    cat <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": ${port},
      "protocol": "hysteria2",
      "settings": {
        "password": "${password}",
        "auth": {
          "type": "password"
        }
      },
      "streamSettings": {
        "network": "udp"
      },
      "sniffing": {
        "enabled": true,
        "destOverride": ["http", "tls"]
      }
    },
    {
      "listen": "127.0.0.1",
      "port": 10085,
      "protocol": "dokodemo-door",
      "settings": {
        "address": "127.0.0.1"
      },
      "tag": "api"
    }
  ],
  "outbounds": [
    {
      "protocol": "freedom",
      "settings": {}
    },
    {
      "protocol": "blackhole",
      "settings": {},
      "tag": "blocked"
    }
  ],
  "api": {
    "tag": "api",
    "services": [
      "HandlerService",
      "StatsService"
    ]
  },
  "stats": {},
  "policy": {
    "levels": {
      "0": {
        "statsUserUplink": true,
        "statsUserDownlink": true
      }
    },
    "system": {
      "statsInboundUplink": true,
      "statsInboundDownlink": true
    }
  },
  "routing": {
    "rules": [
      {
        "inboundTag": ["api"],
        "outboundTag": "api",
        "type": "field"
      }
    ]
  }
}
EOF
}

################################################################################
# Node Agent Installation
################################################################################

# Install Node Agent binary
# Detects system architecture and downloads the appropriate binary
# Usage: install_node_agent
# Returns: 0 on success, 5 on failure
# Requirements: 6.1, 6.2
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
    # Note: This URL should be updated to point to your actual release location
    local download_url="https://github.com/your-org/vpn-platform/releases/latest/download/node-agent-${binary_arch}"
    log_debug "Download URL: $download_url"
    
    # Download binary to temporary location first
    local temp_binary="/tmp/node-agent-${binary_arch}-$$"
    log_info "Downloading Node Agent binary..."
    log_debug "Temporary file: $temp_binary"
    
    # Register temporary file for cleanup (Task 14.4)
    register_temp_file "$temp_binary"
    
    # Download with retry mechanism
    local max_attempts=3
    local attempt=1
    local download_success=false
    
    while [ $attempt -le $max_attempts ]; do
        log_debug "Download attempt $attempt/$max_attempts"
        
        if wget -q --show-progress -O "$temp_binary" "$download_url" 2>&1 | tee -a "$LOG_FILE"; then
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
        log_error "Please check:"
        log_error "  - Network connectivity"
        log_error "  - Release URL is correct"
        log_error "  - Binary exists for your architecture"
        rm -f "$temp_binary"
        return 5
    fi
    
    # Verify download (check file size)
    local file_size
    file_size=$(stat -f%z "$temp_binary" 2>/dev/null || stat -c%s "$temp_binary" 2>/dev/null || echo "0")
    log_debug "Downloaded file size: $file_size bytes"
    
    if [ "$file_size" -lt 1000 ]; then
        log_error "Downloaded file is too small ($file_size bytes)"
        log_error "Download may have failed or file is corrupted"
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
        log_error "Check permissions for /usr/local/bin"
        rm -f "$temp_binary"
        return 5
    fi
    
    # Verify installation
    if ! command -v node-agent &> /dev/null; then
        log_error "Node Agent installation verification failed"
        log_error "node-agent command not found in PATH"
        return 5
    fi
    
    # Get installed version
    local agent_version
    agent_version=$(node-agent --version 2>/dev/null || echo "installed")
    log_info "Node Agent installed successfully: $agent_version"
    
    return 0
}

# Create Node Agent configuration
# Creates /etc/node-agent directory and config.env file
# Usage: create_node_agent_config
# Returns: 0 on success, 5 on failure
# Requirements: 6.3, 12.2
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
        log_warn "Configuration file may be readable by other users"
        return 5
    fi
    
    # Verify permissions
    local file_perms
    file_perms=$(stat -f%Lp "$config_file" 2>/dev/null || stat -c%a "$config_file" 2>/dev/null)
    log_debug "File permissions: $file_perms"
    
    if [ "$file_perms" != "600" ]; then
        log_warn "File permissions are not 600 (actual: $file_perms)"
        log_warn "Configuration file may not be properly secured"
    else
        log_info "File permissions verified: 600 (secure)"
    fi
    
    log_info "Node Agent configuration completed successfully"
    return 0
}

# Create systemd service for Node Agent
# Creates /etc/systemd/system/node-agent.service
# Configures service dependencies, restart policy, and security settings
# Usage: create_node_agent_service
# Returns: 0 on success, 5 on failure
# Requirements: 6.4, 6.5
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
    if ! systemctl daemon-reload 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Failed to reload systemd daemon"
        return 5
    fi
    
    log_info "Systemd daemon reloaded successfully"
    
    # Enable service for automatic startup on boot
    log_info "Enabling Node Agent service for automatic startup..."
    if ! systemctl enable node-agent 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Failed to enable Node Agent service"
        log_warn "Service will not start automatically on boot"
        return 5
    fi
    
    log_info "Node Agent service enabled successfully"
    log_info "Service will start automatically on system boot"
    
    return 0
}

################################################################################
# Service Startup and Verification
################################################################################

# Start Node Agent service
# Reloads systemd configuration, starts the service, and waits for startup
# Usage: start_services
# Returns: 0 on success, 6 on failure
# Requirements: 7.1, 7.2
start_services() {
    log_info "Starting Node Agent service..."
    
    # Reload systemd daemon to ensure latest configuration is loaded
    log_debug "Reloading systemd daemon..."
    if ! systemctl daemon-reload 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Failed to reload systemd daemon"
        return 6
    fi
    log_debug "Systemd daemon reloaded"
    
    # Start Node Agent service
    log_info "Starting node-agent service..."
    if ! systemctl start node-agent 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Failed to start node-agent service"
        log_error "Check service status: systemctl status node-agent"
        log_error "Check service logs: journalctl -u node-agent -n 50"
        return 6
    fi
    
    log_info "Node Agent service start command issued"
    
    # Wait for service to start (give it a few seconds)
    log_info "Waiting for service to start..."
    local wait_time=5
    local elapsed=0
    local check_interval=1
    
    while [ $elapsed -lt $wait_time ]; do
        sleep $check_interval
        ((elapsed += check_interval))
        
        # Check if service is active
        if systemctl is-active --quiet node-agent; then
            log_info "Node Agent service is active"
            return 0
        fi
        
        log_debug "Waiting for service... ($elapsed/${wait_time}s)"
    done
    
    # Check final status after wait period
    if systemctl is-active --quiet node-agent; then
        log_info "Node Agent service started successfully"
        return 0
    else
        log_error "Node Agent service failed to start within ${wait_time}s"
        log_error "Service may still be starting, or there may be an error"
        
        # Show service status for debugging
        log_error "Service status:"
        systemctl status node-agent --no-pager 2>&1 | tee -a "$LOG_FILE"
        
        return 6
    fi
}

# Verify deployment by checking service status, ports, API connection, and logs
# Usage: verify_deployment
# Returns: 0 on success, 6 on failure
# Requirements: 7.3, 7.4, 7.5
verify_deployment() {
    log_info "Verifying deployment..."
    
    local verification_errors=0
    local verification_warnings=0
    
    # 1. Check service status (active/running)
    log_info "Checking service status..."
    if systemctl is-active --quiet node-agent; then
        log_info "✓ Node Agent service is active"
        
        # Get detailed status
        local service_state
        service_state=$(systemctl show node-agent --property=ActiveState --value)
        log_debug "Service state: $service_state"
        
        local service_substate
        service_substate=$(systemctl show node-agent --property=SubState --value)
        log_debug "Service substate: $service_substate"
        
        if [ "$service_substate" = "running" ]; then
            log_info "✓ Node Agent is running"
        else
            log_warn "⚠ Node Agent is active but not in 'running' state: $service_substate"
            ((verification_warnings++))
        fi
    else
        log_error "✗ Node Agent service is not active"
        ((verification_errors++))
        
        # Show why service is not active
        local service_state
        service_state=$(systemctl show node-agent --property=ActiveState --value)
        log_error "Service state: $service_state"
    fi
    
    # 2. Check port listening
    log_info "Checking port listening..."
    local node_port="${DEPLOY_CONFIG[node_port]}"
    
    # Give the service a moment to bind to the port
    sleep 2
    
    # Check if port is listening (using ss or netstat)
    local port_listening=false
    
    if command -v ss &> /dev/null; then
        log_debug "Using 'ss' to check port"
        if ss -tuln | grep -q ":${node_port}"; then
            port_listening=true
        fi
    elif command -v netstat &> /dev/null; then
        log_debug "Using 'netstat' to check port"
        if netstat -tuln | grep -q ":${node_port}"; then
            port_listening=true
        fi
    else
        log_warn "Neither 'ss' nor 'netstat' available, skipping port check"
        ((verification_warnings++))
    fi
    
    if [ "$port_listening" = "true" ]; then
        log_info "✓ Port ${node_port} is listening"
    else
        log_warn "⚠ Port ${node_port} is not listening yet"
        log_warn "This may be normal if the service is still initializing"
        log_warn "Check again in a few moments: ss -tuln | grep ${node_port}"
        ((verification_warnings++))
    fi
    
    # 3. Check API connection
    log_info "Checking API connection..."
    
    # Try to verify node can reach API (basic connectivity check)
    local api_host
    api_host=$(echo "${DEPLOY_CONFIG[api_url]}" | sed -E 's|^https?://||' | cut -d'/' -f1)
    log_debug "API host: $api_host"
    
    if curl -s --max-time 5 --head "${DEPLOY_CONFIG[api_url]}" > /dev/null 2>&1; then
        log_info "✓ API service is reachable"
    else
        log_warn "⚠ Cannot reach API service at ${DEPLOY_CONFIG[api_url]}"
        log_warn "This may affect node functionality"
        log_warn "Check network connectivity and firewall rules"
        ((verification_warnings++))
    fi
    
    # 4. Check logs for errors
    log_info "Checking service logs for errors..."
    
    # Get recent logs (last 20 lines)
    local recent_logs
    recent_logs=$(journalctl -u node-agent -n 20 --no-pager 2>/dev/null)
    
    if [ -n "$recent_logs" ]; then
        # Check for error keywords in logs
        local error_count
        error_count=$(echo "$recent_logs" | grep -i "error" | wc -l)
        
        local fatal_count
        fatal_count=$(echo "$recent_logs" | grep -i "fatal" | wc -l)
        
        local panic_count
        panic_count=$(echo "$recent_logs" | grep -i "panic" | wc -l)
        
        if [ "$error_count" -gt 0 ] || [ "$fatal_count" -gt 0 ] || [ "$panic_count" -gt 0 ]; then
            log_warn "⚠ Found errors in service logs:"
            log_warn "  Errors: $error_count, Fatal: $fatal_count, Panic: $panic_count"
            log_warn ""
            log_warn "Recent error messages:"
            echo "$recent_logs" | grep -i -E "error|fatal|panic" | tail -n 5 | while read -r line; do
                log_warn "  $line"
            done
            ((verification_warnings++))
        else
            log_info "✓ No errors found in recent logs"
        fi
        
        # Log recent messages for debugging
        log_debug "Recent service logs:"
        echo "$recent_logs" | tail -n 5 | while read -r line; do
            log_debug "  $line"
        done
    else
        log_warn "⚠ Could not retrieve service logs"
        log_warn "Check logs manually: journalctl -u node-agent -n 50"
        ((verification_warnings++))
    fi
    
    # Summary
    log_info ""
    log_info "=========================================="
    log_info "Verification Summary"
    log_info "=========================================="
    log_info "Errors: $verification_errors"
    log_info "Warnings: $verification_warnings"
    
    if [ $verification_errors -eq 0 ]; then
        if [ $verification_warnings -eq 0 ]; then
            log_info "✓ All verification checks passed"
            log_info "Deployment completed successfully!"
        else
            log_info "✓ Deployment completed with warnings"
            log_warn "Please review warnings above"
        fi
        return 0
    else
        log_error "✗ Deployment verification failed"
        log_error "Please review errors above and check service logs"
        return 6
    fi
}

# Show troubleshooting suggestions based on common issues
# Usage: show_troubleshooting_tips [exit_code]
# Requirements: 7.6, 10.3, 10.4
show_troubleshooting_tips() {
    local exit_code="${1:-0}"
    
    log_info ""
    log_info "=========================================="
    log_info "Troubleshooting Tips"
    log_info "=========================================="
    log_info ""
    
    # Provide context-specific suggestions based on exit code
    case "$exit_code" in
        1)
            log_info "Parameter Error:"
            log_info "  - Check if all required parameters are provided"
            log_info "  - Verify parameter formats (URL, port, protocol)"
            log_info "  - Run with --help to see usage examples"
            log_info ""
            log_info "Required parameters:"
            log_info "  --api-url <URL>        API service URL"
            log_info "  --admin-token <TOKEN>  Admin JWT token"
            log_info "  --node-name <NAME>     Node name"
            ;;
        2)
            log_info "Environment Error:"
            log_info "  - Ensure you are running as root (sudo)"
            log_info "  - Check if your OS is supported (Ubuntu/CentOS/Debian)"
            log_info "  - Install missing dependencies manually"
            log_info ""
            log_info "To run as root:"
            log_info "  sudo $0 $*"
            log_info ""
            log_info "To check OS:"
            log_info "  cat /etc/os-release"
            ;;
        3)
            log_info "Network Error:"
            log_info "  - Check your internet connection"
            log_info "  - Verify API_URL is correct and accessible"
            log_info "  - Check firewall settings"
            log_info "  - Try using a different DNS server"
            log_info ""
            log_info "Test network connectivity:"
            log_info "  ping -c 3 8.8.8.8"
            log_info "  curl -I ${DEPLOY_CONFIG[api_url]}"
            log_info ""
            log_info "Check firewall:"
            log_info "  sudo iptables -L"
            log_info "  sudo ufw status"
            ;;
        4)
            log_info "API Error:"
            log_info "  - Verify ADMIN_TOKEN is valid and not expired"
            log_info "  - Check if you have admin permissions"
            log_info "  - Review API error message above"
            log_info "  - Contact system administrator if needed"
            log_info ""
            log_info "Token format: $(mask_sensitive "${DEPLOY_CONFIG[admin_token]}")"
            log_info ""
            log_info "Test API access:"
            log_info "  curl -H 'Authorization: Bearer YOUR_TOKEN' ${DEPLOY_CONFIG[api_url]}/api/admin/nodes"
            ;;
        5)
            log_info "Installation Error:"
            log_info "  - Check disk space (need at least 500MB)"
            log_info "  - Verify write permissions to /usr/local/bin"
            log_info "  - Check installation logs for details"
            log_info "  - Try manual installation if automated fails"
            log_info ""
            log_info "Check disk space:"
            log_info "  df -h /"
            log_info ""
            log_info "Check permissions:"
            log_info "  ls -la /usr/local/bin"
            ;;
        6)
            log_info "Service Error:"
            log_info "  - Check service logs: journalctl -u node-agent -n 50"
            log_info "  - Verify configuration file: /etc/node-agent/config.env"
            log_info "  - Check if port is already in use: netstat -tuln | grep ${DEPLOY_CONFIG[node_port]}"
            log_info "  - Ensure Xray-core is properly installed"
            log_info ""
            log_info "Service status:"
            log_info "  systemctl status node-agent"
            log_info "  systemctl status xray"
            ;;
        *)
            # Generic troubleshooting tips
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
            ;;
    esac
    
    log_info ""
    log_info "8. Review deployment log:"
    log_info "   cat $LOG_FILE"
    log_info ""
    log_info "Common issues:"
    log_info "  - Port already in use: Change NODE_PORT or stop conflicting service"
    log_info "  - API unreachable: Check firewall rules and network connectivity"
    log_info "  - Permission denied: Ensure script was run as root"
    log_info "  - Binary not found: Check if download completed successfully"
    log_info ""
    log_info "For more help:"
    log_info "  - View full logs: cat $LOG_FILE"
    log_info "  - Check service status: systemctl status node-agent"
    log_info "  - Run with --verbose for detailed output"
    log_info ""
}

################################################################################
# Security Enhancement Functions (Task 14)
################################################################################

# Set and verify configuration file permissions
# Ensures all sensitive configuration files have secure permissions (600)
# Usage: set_config_file_permissions
# Returns: 0 on success, 1 on failure
# Requirements: 12.2
set_config_file_permissions() {
    log_info "Setting secure file permissions for configuration files..."
    
    local errors=0
    local files_secured=0
    
    # List of sensitive configuration files that should have 600 permissions
    local -a sensitive_files=(
        "/etc/node-agent/config.env"
        "/etc/xray/config.json"
    )
    
    # Set permissions for each file
    for file in "${sensitive_files[@]}"; do
        if [ -f "$file" ]; then
            log_debug "Setting permissions for: $file"
            
            # Set permissions to 600 (owner read/write only)
            if chmod 600 "$file" 2>/dev/null; then
                log_debug "✓ Permissions set to 600 for $file"
                ((files_secured++))
            else
                log_error "✗ Failed to set permissions for $file"
                ((errors++))
            fi
        else
            log_debug "File not found (skipping): $file"
        fi
    done
    
    log_info "Secured $files_secured configuration file(s)"
    
    if [ $errors -gt 0 ]; then
        log_error "Failed to secure $errors file(s)"
        return 1
    fi
    
    return 0
}

# Verify configuration file permissions
# Checks that all sensitive files have correct permissions (600)
# Usage: verify_config_file_permissions
# Returns: 0 if all permissions correct, 1 if any issues found
# Requirements: 12.2
verify_config_file_permissions() {
    log_info "Verifying configuration file permissions..."
    
    local errors=0
    local warnings=0
    
    # List of sensitive configuration files that should have 600 permissions
    local -a sensitive_files=(
        "/etc/node-agent/config.env"
        "/etc/xray/config.json"
    )
    
    # Check permissions for each file
    for file in "${sensitive_files[@]}"; do
        if [ -f "$file" ]; then
            # Get file permissions (works on both Linux and macOS)
            local file_perms
            file_perms=$(stat -f%Lp "$file" 2>/dev/null || stat -c%a "$file" 2>/dev/null)
            
            log_debug "Checking permissions for: $file (current: $file_perms)"
            
            if [ "$file_perms" = "600" ]; then
                log_info "✓ $file has secure permissions (600)"
            elif [ "$file_perms" = "400" ]; then
                log_info "✓ $file has read-only permissions (400) - acceptable"
            else
                log_warn "⚠ $file has insecure permissions ($file_perms)"
                log_warn "  Expected: 600 (owner read/write only)"
                log_warn "  Current: $file_perms"
                ((warnings++))
                
                # Check if file is world-readable or group-readable
                if [ "${file_perms:1:1}" != "0" ] || [ "${file_perms:2:1}" != "0" ]; then
                    log_error "✗ $file is readable by non-owner users!"
                    log_error "  This is a security risk - sensitive data may be exposed"
                    ((errors++))
                fi
            fi
        else
            log_debug "File not found (skipping): $file"
        fi
    done
    
    if [ $errors -gt 0 ]; then
        log_error "Found $errors file(s) with insecure permissions"
        log_error "Run with --force to automatically fix permissions"
        return 1
    fi
    
    if [ $warnings -gt 0 ]; then
        log_warn "Found $warnings file(s) with non-standard permissions"
        log_warn "Consider running: chmod 600 <file>"
    fi
    
    log_info "Configuration file permissions verified"
    return 0
}

# Validate HTTPS usage for API URL
# Ensures API_URL uses HTTPS protocol for secure communication
# Usage: validate_https_url
# Returns: 0 if HTTPS, 1 if not HTTPS
# Requirements: 12.4
validate_https_url() {
    local url="$1"
    
    log_debug "Validating HTTPS for URL: $url"
    
    # Check if URL starts with https://
    if [[ "$url" =~ ^https:// ]]; then
        log_debug "✓ URL uses HTTPS protocol"
        return 0
    elif [[ "$url" =~ ^http:// ]]; then
        log_error "✗ URL uses insecure HTTP protocol: $url"
        log_error "  HTTPS is required for secure communication"
        log_error "  Please use https:// instead of http://"
        return 1
    else
        log_error "✗ URL has invalid protocol: $url"
        log_error "  URL must start with https://"
        return 1
    fi
}

# Verify SSL certificate for API URL
# Performs a test connection to verify SSL certificate is valid
# Usage: verify_ssl_certificate <url>
# Returns: 0 if certificate valid, 1 if invalid or connection fails
# Requirements: 12.4
verify_ssl_certificate() {
    local url="$1"
    
    log_info "Verifying SSL certificate for API URL..."
    log_debug "URL: $url"
    
    # Extract hostname from URL for certificate verification
    local hostname
    hostname=$(echo "$url" | sed -e 's|^https://||' -e 's|/.*$||' -e 's|:.*$||')
    log_debug "Hostname: $hostname"
    
    # Test SSL connection using curl
    log_debug "Testing SSL connection..."
    
    # Try to connect with SSL verification enabled
    local curl_output
    local curl_exit_code
    
    curl_output=$(curl -s -I --max-time 10 "$url" 2>&1)
    curl_exit_code=$?
    
    if [ $curl_exit_code -eq 0 ]; then
        log_info "✓ SSL certificate is valid"
        log_debug "Connection successful"
        return 0
    else
        # Check specific curl error codes
        case $curl_exit_code in
            51)
                log_error "✗ SSL certificate verification failed"
                log_error "  The server's SSL certificate is invalid or untrusted"
                log_error "  Error code: $curl_exit_code"
                ;;
            60)
                log_error "✗ SSL certificate problem: unable to get local issuer certificate"
                log_error "  The certificate chain is incomplete or CA certificates are missing"
                log_error "  Error code: $curl_exit_code"
                ;;
            *)
                log_warn "⚠ Could not verify SSL certificate"
                log_warn "  Connection failed with error code: $curl_exit_code"
                log_warn "  This may be a network issue rather than a certificate problem"
                log_debug "Curl output: $curl_output"
                ;;
        esac
        
        log_warn "Proceeding with deployment, but SSL verification failed"
        log_warn "Ensure the API server has a valid SSL certificate"
        
        # Return warning but don't fail deployment
        # In production, you might want to make this a hard failure
        return 0
    fi
}

# Check batch configuration file permissions
# Ensures batch config file is not world-readable or group-readable
# Usage: check_batch_config_permissions <config_file>
# Returns: 0 if permissions acceptable, 1 if too permissive
# Requirements: 12.5
check_batch_config_permissions() {
    local config_file="$1"
    
    log_info "Checking batch configuration file permissions..."
    log_debug "Config file: $config_file"
    
    # Check if file exists
    if [ ! -f "$config_file" ]; then
        log_error "Configuration file not found: $config_file"
        return 1
    fi
    
    # Get file permissions
    local file_perms
    file_perms=$(stat -f%Lp "$config_file" 2>/dev/null || stat -c%a "$config_file" 2>/dev/null)
    
    log_debug "File permissions: $file_perms"
    
    # Check if file is world-readable (last digit > 0)
    local world_perms="${file_perms:2:1}"
    if [ "$world_perms" != "0" ]; then
        log_error "✗ Configuration file is world-readable!"
        log_error "  File: $config_file"
        log_error "  Permissions: $file_perms"
        log_error "  This is a security risk - sensitive data may be exposed"
        log_error ""
        log_error "Fix with: chmod 600 $config_file"
        return 1
    fi
    
    # Check if file is group-readable (middle digit > 4)
    local group_perms="${file_perms:1:1}"
    if [ "$group_perms" -ge 4 ]; then
        log_warn "⚠ Configuration file is group-readable"
        log_warn "  File: $config_file"
        log_warn "  Permissions: $file_perms"
        log_warn "  Consider restricting to owner-only: chmod 600 $config_file"
    fi
    
    # Acceptable permissions: 600, 400, 640, 440
    case "$file_perms" in
        600|400)
            log_info "✓ Configuration file has secure permissions ($file_perms)"
            return 0
            ;;
        640|440)
            log_info "✓ Configuration file has acceptable permissions ($file_perms)"
            log_warn "  Consider using 600 for maximum security"
            return 0
            ;;
        *)
            log_warn "⚠ Configuration file has non-standard permissions ($file_perms)"
            log_warn "  Recommended: 600 (owner read/write only)"
            return 0
            ;;
    esac
}

# Temporary files tracking for cleanup
declare -a TEMP_FILES=()

# Register a temporary file for cleanup
# Usage: register_temp_file <file_path>
register_temp_file() {
    local file="$1"
    TEMP_FILES+=("$file")
    log_debug "Registered temporary file for cleanup: $file"
}

# Clean up temporary files containing sensitive information
# Removes all registered temporary files securely
# Usage: cleanup_temp_files
# Returns: 0 on success
# Requirements: 12.6
cleanup_temp_files() {
    log_debug "Cleaning up temporary files..."
    
    local cleaned_count=0
    local failed_count=0
    
    # Clean up registered temporary files
    for temp_file in "${TEMP_FILES[@]}"; do
        if [ -f "$temp_file" ]; then
            log_debug "Removing temporary file: $temp_file"
            
            # Securely remove file (overwrite with zeros first if possible)
            if command -v shred &> /dev/null; then
                # Use shred for secure deletion (overwrites file data)
                if shred -u -z "$temp_file" 2>/dev/null; then
                    log_debug "✓ Securely removed: $temp_file"
                    ((cleaned_count++))
                else
                    log_debug "Failed to shred, using rm: $temp_file"
                    if rm -f "$temp_file" 2>/dev/null; then
                        ((cleaned_count++))
                    else
                        ((failed_count++))
                    fi
                fi
            else
                # Fallback to regular rm
                if rm -f "$temp_file" 2>/dev/null; then
                    log_debug "✓ Removed: $temp_file"
                    ((cleaned_count++))
                else
                    log_debug "✗ Failed to remove: $temp_file"
                    ((failed_count++))
                fi
            fi
        else
            log_debug "Temporary file already removed: $temp_file"
        fi
    done
    
    # Clean up common temporary file patterns
    local -a common_temp_patterns=(
        "/tmp/xray-install*.sh"
        "/tmp/node-agent-*-$$"
        "/tmp/deploy-*.tmp"
    )
    
    for pattern in "${common_temp_patterns[@]}"; do
        # Use find to safely handle patterns
        local matching_files
        matching_files=$(find /tmp -maxdepth 1 -name "$(basename "$pattern")" 2>/dev/null || true)
        
        if [ -n "$matching_files" ]; then
            while IFS= read -r file; do
                if [ -f "$file" ]; then
                    log_debug "Removing temporary file: $file"
                    if rm -f "$file" 2>/dev/null; then
                        ((cleaned_count++))
                    else
                        ((failed_count++))
                    fi
                fi
            done <<< "$matching_files"
        fi
    done
    
    if [ $cleaned_count -gt 0 ]; then
        log_debug "Cleaned up $cleaned_count temporary file(s)"
    fi
    
    if [ $failed_count -gt 0 ]; then
        log_debug "Failed to clean up $failed_count temporary file(s)"
    fi
    
    # Clear the temporary files array
    TEMP_FILES=()
    
    return 0
}

################################################################################
# Batch Deployment Functions
################################################################################

# Parse batch configuration file (YAML format)
# Extracts API configuration and node list from YAML file
# Usage: parse_batch_config "config_file.yaml"
# Returns: 0 on success, 1 on failure
# Requirements: 8.1
parse_batch_config() {
    local config_file="$1"
    
    log_info "Parsing batch configuration file: $config_file"
    
    # Check if config file exists
    if [ ! -f "$config_file" ]; then
        log_error "Configuration file not found: $config_file"
        return 1
    fi
    
    # Check file permissions (should not be world-readable for security)
    local file_perms
    file_perms=$(stat -f%Lp "$config_file" 2>/dev/null || stat -c%a "$config_file" 2>/dev/null)
    log_debug "Config file permissions: $file_perms"
    
    # Warn if file is world-readable (last digit > 0)
    if [[ "$file_perms" =~ [0-9][0-9]([4-7])$ ]]; then
        log_warn "Configuration file is world-readable (permissions: $file_perms)"
        log_warn "This may expose sensitive information (tokens, secrets)"
        log_warn "Recommended: chmod 600 $config_file"
    fi
    
    # Check if yq is available (preferred for YAML parsing)
    if command -v yq &> /dev/null; then
        log_debug "Using 'yq' for YAML parsing"
        parse_batch_config_yq "$config_file"
        return $?
    else
        log_warn "'yq' not found, attempting to parse as JSON with jq"
        log_warn "For proper YAML support, install yq: https://github.com/mikefarah/yq"
        
        # Try to parse as JSON using jq
        if command -v jq &> /dev/null; then
            parse_batch_config_jq "$config_file"
            return $?
        else
            log_error "Neither 'yq' nor 'jq' is available"
            log_error "Please install yq for YAML parsing or jq for JSON parsing"
            log_error "  Ubuntu/Debian: apt-get install yq"
            log_error "  Or download from: https://github.com/mikefarah/yq/releases"
            return 1
        fi
    fi
}

# Parse batch configuration using yq (YAML parser)
# Usage: parse_batch_config_yq "config_file.yaml"
# Returns: 0 on success, 1 on failure
parse_batch_config_yq() {
    local config_file="$1"
    
    log_debug "Parsing YAML configuration with yq..."
    
    # Extract API configuration
    local api_url
    api_url=$(yq eval '.api_url' "$config_file" 2>/dev/null)
    if [ -z "$api_url" ] || [ "$api_url" = "null" ]; then
        log_error "Missing 'api_url' in configuration file"
        return 1
    fi
    DEPLOY_CONFIG[api_url]="$api_url"
    log_debug "API URL: $api_url"
    
    local admin_token
    admin_token=$(yq eval '.admin_token' "$config_file" 2>/dev/null)
    if [ -z "$admin_token" ] || [ "$admin_token" = "null" ]; then
        log_error "Missing 'admin_token' in configuration file"
        return 1
    fi
    DEPLOY_CONFIG[admin_token]="$admin_token"
    log_debug "Admin Token: $(mask_sensitive "$admin_token")"
    
    # Count nodes in configuration
    local node_count
    node_count=$(yq eval '.nodes | length' "$config_file" 2>/dev/null)
    
    if [ -z "$node_count" ] || [ "$node_count" = "null" ] || [ "$node_count" -eq 0 ]; then
        log_error "No nodes defined in configuration file"
        log_error "Configuration must contain a 'nodes' array with at least one node"
        return 1
    fi
    
    log_info "Found $node_count node(s) in configuration"
    
    # Store node count for later use
    BATCH_NODE_COUNT=$node_count
    
    return 0
}

# Parse batch configuration using jq (JSON parser)
# Usage: parse_batch_config_jq "config_file.json"
# Returns: 0 on success, 1 on failure
parse_batch_config_jq() {
    local config_file="$1"
    
    log_debug "Parsing JSON configuration with jq..."
    
    # Validate JSON syntax
    if ! jq empty "$config_file" 2>/dev/null; then
        log_error "Invalid JSON syntax in configuration file"
        log_error "If using YAML format, please install yq"
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
# Usage: extract_node_config "config_file" index
# Sets DEPLOY_CONFIG values for the specified node
# Returns: 0 on success, 1 on failure
extract_node_config() {
    local config_file="$1"
    local index="$2"
    
    log_debug "Extracting node configuration at index $index"
    
    # Determine which parser to use
    local parser=""
    if command -v yq &> /dev/null; then
        parser="yq"
    elif command -v jq &> /dev/null; then
        parser="jq"
    else
        log_error "No YAML/JSON parser available"
        return 1
    fi
    
    # Extract node fields based on parser
    if [ "$parser" = "yq" ]; then
        DEPLOY_CONFIG[node_name]=$(yq eval ".nodes[$index].name" "$config_file" 2>/dev/null)
        DEPLOY_CONFIG[node_host]=$(yq eval ".nodes[$index].host // \"\"" "$config_file" 2>/dev/null)
        DEPLOY_CONFIG[node_port]=$(yq eval ".nodes[$index].port // 443" "$config_file" 2>/dev/null)
        DEPLOY_CONFIG[node_protocol]=$(yq eval ".nodes[$index].protocol // \"vless\"" "$config_file" 2>/dev/null)
        
        # Extract config object (if present)
        local node_config
        node_config=$(yq eval ".nodes[$index].config // {}" "$config_file" 2>/dev/null)
        if [ "$node_config" != "null" ] && [ -n "$node_config" ]; then
            DEPLOY_CONFIG[node_config]="$node_config"
        else
            DEPLOY_CONFIG[node_config]="{}"
        fi
    else
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
# Usage: record_batch_result "node_name" "success|failed" "message"
# Stores result in BATCH_RESULTS array
record_batch_result() {
    local node_name="$1"
    local status="$2"
    local message="$3"
    
    BATCH_RESULTS+=("$node_name|$status|$message")
    log_debug "Recorded batch result: $node_name - $status"
}

# Deploy a single node from batch configuration
# Usage: deploy_single_node "config_file" index
# Returns: 0 on success, non-zero on failure
# Requirements: 8.2, 8.4, 8.5
deploy_single_node() {
    local config_file="$1"
    local index="$2"
    
    log_info "----------------------------------------"
    log_info "Deploying node at index $index"
    log_info "----------------------------------------"
    
    # Extract node configuration
    if ! extract_node_config "$config_file" "$index"; then
        log_error "Failed to extract node configuration at index $index"
        return 1
    fi
    
    local node_name="${DEPLOY_CONFIG[node_name]}"
    log_info "Node name: $node_name"
    
    # Apply default values
    apply_defaults
    
    # Validate parameters
    if ! validate_parameters; then
        log_error "Parameter validation failed for node: $node_name"
        return 1
    fi
    
    # Auto-detect public IP if NODE_HOST not provided
    if [ -z "${DEPLOY_CONFIG[node_host]}" ]; then
        log_info "NODE_HOST not specified, auto-detecting public IP..."
        local detected_ip
        if detected_ip=$(detect_public_ip); then
            DEPLOY_CONFIG[node_host]="$detected_ip"
            log_info "Using detected IP: ${DEPLOY_CONFIG[node_host]}"
        else
            log_error "Failed to auto-detect public IP for node: $node_name"
            return 1
        fi
    fi
    
    # Create node via API
    DEPLOY_STATE[phase]="api_call"
    if ! create_node; then
        log_error "Node creation failed for: $node_name"
        return 4
    fi
    
    # Install Xray-core
    DEPLOY_STATE[phase]="install"
    if ! install_xray; then
        log_error "Xray-core installation failed for: $node_name"
        return 5
    fi
    
    # Generate Xray-core configuration
    if ! generate_xray_config; then
        log_error "Xray-core configuration generation failed for: $node_name"
        return 5
    fi
    
    # Install Node Agent
    if ! install_node_agent; then
        log_error "Node Agent installation failed for: $node_name"
        return 5
    fi
    
    # Create Node Agent configuration
    DEPLOY_STATE[phase]="config"
    if ! create_node_agent_config; then
        log_error "Node Agent configuration failed for: $node_name"
        return 5
    fi
    
    # Create Node Agent systemd service
    if ! create_node_agent_service; then
        log_error "Node Agent service creation failed for: $node_name"
        return 5
    fi
    
    # Start services
    DEPLOY_STATE[phase]="start"
    if ! start_services; then
        log_error "Service startup failed for: $node_name"
        return 6
    fi
    
    # Verify deployment
    DEPLOY_STATE[phase]="verify"
    if ! verify_deployment; then
        log_warn "Deployment verification failed for: $node_name"
        log_warn "Node may still be functional, check logs"
        return 6
    fi
    
    log_info "Node deployed successfully: $node_name"
    return 0
}

# Batch deploy multiple nodes from configuration file
# Iterates through all nodes in the configuration file
# Continues deployment even if individual nodes fail
# Usage: batch_deploy "config_file"
# Returns: 0 if at least one node succeeded, 1 if all failed
# Requirements: 8.2, 8.4, 8.5
batch_deploy() {
    local config_file="$1"
    
    log_info "=========================================="
    log_info "Starting Batch Deployment"
    log_info "=========================================="
    log_info ""
    
    # Parse batch configuration
    if ! parse_batch_config "$config_file"; then
        log_error "Failed to parse batch configuration file"
        return 1
    fi
    
    local node_count=$BATCH_NODE_COUNT
    log_info "Total nodes to deploy: $node_count"
    log_info ""
    
    # Initialize counters
    local success_count=0
    local fail_count=0
    
    # Deploy each node
    for i in $(seq 0 $((node_count - 1))); do
        local node_index=$i
        local node_number=$((i + 1))
        
        log_info ""
        log_info "=========================================="
        log_info "Deploying Node $node_number/$node_count"
        log_info "=========================================="
        
        # Extract node name for logging (before deployment)
        local node_name
        if command -v yq &> /dev/null; then
            node_name=$(yq eval ".nodes[$i].name" "$config_file" 2>/dev/null)
        elif command -v jq &> /dev/null; then
            node_name=$(jq -r ".nodes[$i].name // \"node-$node_number\"" "$config_file" 2>/dev/null)
        else
            node_name="node-$node_number"
        fi
        
        log_info "Node: $node_name"
        log_info ""
        
        # Deploy single node
        if deploy_single_node "$config_file" "$node_index"; then
            ((success_count++))
            log_info "✓ Node $node_number/$node_count deployed successfully: $node_name"
            record_batch_result "$node_name" "success" "Deployed successfully"
        else
            ((fail_count++))
            log_error "✗ Node $node_number/$node_count deployment failed: $node_name"
            record_batch_result "$node_name" "failed" "Deployment failed"
            
            # Continue with next node even if this one failed
            log_warn "Continuing with remaining nodes..."
        fi
        
        log_info ""
    done
    
    # Store final counts for summary report
    BATCH_SUCCESS_COUNT=$success_count
    BATCH_FAIL_COUNT=$fail_count
    
    log_info "=========================================="
    log_info "Batch Deployment Completed"
    log_info "=========================================="
    log_info "Total: $node_count nodes"
    log_info "Successful: $success_count"
    log_info "Failed: $fail_count"
    log_info ""
    
    # Return success if at least one node was deployed
    if [ $success_count -gt 0 ]; then
        return 0
    else
        return 1
    fi
}

# Generate and display batch deployment summary report
# Shows detailed status for each node and overall statistics
# Usage: generate_batch_report
# Requirements: 8.6
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
        log_info "  1. Check deployment log: $LOG_FILE"
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
# Idempotency and Rollback Functions (Task 13)
################################################################################

# Check if Xray-core is already installed
# Usage: check_xray_installed
# Returns: 0 if installed, 1 if not
# Requirements: 11.1
check_xray_installed() {
    if command -v xray &> /dev/null; then
        local xray_version
        xray_version=$(xray version 2>/dev/null | head -n1 || echo "unknown")
        log_debug "Xray-core is already installed: $xray_version"
        return 0
    else
        log_debug "Xray-core is not installed"
        return 1
    fi
}

# Check if Node Agent is already installed
# Usage: check_node_agent_installed
# Returns: 0 if installed, 1 if not
# Requirements: 11.1
check_node_agent_installed() {
    if command -v node-agent &> /dev/null; then
        local agent_version
        agent_version=$(node-agent --version 2>/dev/null || echo "unknown")
        log_debug "Node Agent is already installed: $agent_version"
        return 0
    else
        log_debug "Node Agent is not installed"
        return 1
    fi
}

# Check if Xray-core configuration exists
# Usage: check_xray_config_exists
# Returns: 0 if exists, 1 if not
check_xray_config_exists() {
    if [ -f "/etc/xray/config.json" ]; then
        log_debug "Xray-core configuration exists"
        return 0
    else
        log_debug "Xray-core configuration does not exist"
        return 1
    fi
}

# Check if Node Agent configuration exists
# Usage: check_node_agent_config_exists
# Returns: 0 if exists, 1 if not
check_node_agent_config_exists() {
    if [ -f "/etc/node-agent/config.env" ]; then
        log_debug "Node Agent configuration exists"
        return 0
    else
        log_debug "Node Agent configuration does not exist"
        return 1
    fi
}

# Check if Node Agent service is installed
# Usage: check_node_agent_service_exists
# Returns: 0 if exists, 1 if not
check_node_agent_service_exists() {
    if [ -f "/etc/systemd/system/node-agent.service" ]; then
        log_debug "Node Agent systemd service exists"
        return 0
    else
        log_debug "Node Agent systemd service does not exist"
        return 1
    fi
}

# Install Xray-core with idempotency check
# Skips installation if already installed, only updates configuration
# Usage: install_xray_idempotent
# Returns: 0 on success, 5 on failure
# Requirements: 11.1, 11.2
install_xray_idempotent() {
    log_info "Checking Xray-core installation..."
    
    if check_xray_installed; then
        local xray_version
        xray_version=$(xray version 2>/dev/null | head -n1 || echo "unknown")
        log_info "✓ Xray-core is already installed: $xray_version"
        log_info "Skipping Xray-core installation (idempotent)"
        
        # Check if configuration needs update
        if check_xray_config_exists; then
            log_info "Xray-core configuration exists, will be updated if needed"
        fi
        
        return 0
    else
        log_info "Xray-core not found, proceeding with installation..."
        return install_xray
    fi
}

# Install Node Agent with idempotency check
# Skips installation if already installed, only updates configuration
# Usage: install_node_agent_idempotent
# Returns: 0 on success, 5 on failure
# Requirements: 11.1, 11.2
install_node_agent_idempotent() {
    log_info "Checking Node Agent installation..."
    
    if check_node_agent_installed; then
        local agent_version
        agent_version=$(node-agent --version 2>/dev/null || echo "unknown")
        log_info "✓ Node Agent is already installed: $agent_version"
        log_info "Skipping Node Agent installation (idempotent)"
        
        # Check if configuration needs update
        if check_node_agent_config_exists; then
            log_info "Node Agent configuration exists, will be updated if needed"
        fi
        
        return 0
    else
        log_info "Node Agent not found, proceeding with installation..."
        return install_node_agent
    fi
}

# Create or update Xray-core configuration (idempotent)
# Backs up existing configuration before updating
# Usage: generate_xray_config_idempotent
# Returns: 0 on success, 5 on failure
# Requirements: 11.2
generate_xray_config_idempotent() {
    log_info "Checking Xray-core configuration..."
    
    if check_xray_config_exists; then
        log_info "Xray-core configuration exists, updating..."
        # generate_xray_config already handles backup
        return generate_xray_config
    else
        log_info "Creating new Xray-core configuration..."
        return generate_xray_config
    fi
}

# Create or update Node Agent configuration (idempotent)
# Backs up existing configuration before updating
# Usage: create_node_agent_config_idempotent
# Returns: 0 on success, 5 on failure
# Requirements: 11.2
create_node_agent_config_idempotent() {
    log_info "Checking Node Agent configuration..."
    
    if check_node_agent_config_exists; then
        log_info "Node Agent configuration exists, updating..."
        # create_node_agent_config already handles backup
        return create_node_agent_config
    else
        log_info "Creating new Node Agent configuration..."
        return create_node_agent_config
    fi
}

# Create or update Node Agent service (idempotent)
# Backs up existing service file before updating
# Usage: create_node_agent_service_idempotent
# Returns: 0 on success, 5 on failure
# Requirements: 11.2
create_node_agent_service_idempotent() {
    log_info "Checking Node Agent systemd service..."
    
    if check_node_agent_service_exists; then
        log_info "Node Agent service exists, updating..."
        # create_node_agent_service already handles backup
        return create_node_agent_service
    else
        log_info "Creating new Node Agent service..."
        return create_node_agent_service
    fi
}

################################################################################
# Error Rollback Functions (Task 13.2)
################################################################################

# Global variable to track deployment state for rollback
declare -A ROLLBACK_STATE=(
    [services_started]=false
    [config_created]=false
    [agent_installed]=false
    [xray_installed]=false
    [node_created]=false
)

# Stop all services on error
# Usage: stop_services_on_error
# Returns: 0 on success
# Requirements: 11.3
stop_services_on_error() {
    log_info "Stopping services due to error..."
    
    # Stop Node Agent service if it was started
    if [ "${ROLLBACK_STATE[services_started]}" = "true" ]; then
        if systemctl is-active --quiet node-agent; then
            log_info "Stopping Node Agent service..."
            if systemctl stop node-agent 2>&1 | tee -a "$LOG_FILE"; then
                log_info "✓ Node Agent service stopped"
            else
                log_warn "Failed to stop Node Agent service"
            fi
        fi
    fi
    
    # Stop Xray-core service if it's running
    if systemctl is-active --quiet xray; then
        log_info "Stopping Xray-core service..."
        if systemctl stop xray 2>&1 | tee -a "$LOG_FILE"; then
            log_info "✓ Xray-core service stopped"
        else
            log_warn "Failed to stop Xray-core service"
        fi
    fi
    
    log_info "Services stopped"
    return 0
}

# Restore backup configuration files
# Usage: restore_backup_config
# Returns: 0 on success
# Requirements: 11.4
restore_backup_config() {
    log_info "Restoring backup configuration..."
    
    local restored=false
    
    # Find most recent backup
    local backup_dir="/var/backups/node-deployment"
    
    if [ ! -d "$backup_dir" ]; then
        log_debug "No backup directory found"
        return 0
    fi
    
    # Find most recent backup directory
    local latest_backup
    latest_backup=$(find "$backup_dir" -maxdepth 1 -type d -name "backup_*" | sort -r | head -n1)
    
    if [ -z "$latest_backup" ]; then
        log_debug "No backup found to restore"
        return 0
    fi
    
    log_info "Found backup: $latest_backup"
    
    # Restore Node Agent configuration
    if [ -f "$latest_backup/config.env" ]; then
        log_info "Restoring Node Agent configuration..."
        if cp "$latest_backup/config.env" "/etc/node-agent/config.env"; then
            log_info "✓ Node Agent configuration restored"
            restored=true
        else
            log_warn "Failed to restore Node Agent configuration"
        fi
    fi
    
    # Restore Xray-core configuration
    if [ -f "$latest_backup/xray_config.json" ]; then
        log_info "Restoring Xray-core configuration..."
        if cp "$latest_backup/xray_config.json" "/etc/xray/config.json"; then
            log_info "✓ Xray-core configuration restored"
            restored=true
        else
            log_warn "Failed to restore Xray-core configuration"
        fi
    fi
    
    # Restore systemd service file
    if [ -f "$latest_backup/node-agent.service" ]; then
        log_info "Restoring systemd service file..."
        if cp "$latest_backup/node-agent.service" "/etc/systemd/system/node-agent.service"; then
            systemctl daemon-reload
            log_info "✓ Systemd service file restored"
            restored=true
        else
            log_warn "Failed to restore systemd service file"
        fi
    fi
    
    if [ "$restored" = "true" ]; then
        log_info "Backup configuration restored successfully"
    else
        log_debug "No configuration files were restored"
    fi
    
    return 0
}

# Clean up partially installed files
# Usage: cleanup_partial_installation
# Returns: 0 on success
# Requirements: 11.4
cleanup_partial_installation() {
    log_info "Cleaning up partial installation..."
    
    # Only clean up if this was a new installation (not an update)
    if [ "${ROLLBACK_STATE[node_created]}" = "true" ]; then
        log_info "Node was created via API, keeping configuration for retry"
        log_info "To completely remove, use the cleanup command"
        return 0
    fi
    
    # Remove temporary files
    log_debug "Removing temporary files..."
    rm -f /tmp/xray-install.sh 2>/dev/null || true
    rm -f /tmp/node-agent-* 2>/dev/null || true
    
    log_info "Partial installation cleaned up"
    return 0
}

# Perform complete rollback on error
# Stops services, restores backups, and cleans up
# Usage: perform_rollback
# Returns: 0 on success
# Requirements: 11.3, 11.4
perform_rollback() {
    log_error "=========================================="
    log_error "Performing Rollback Due to Error"
    log_error "=========================================="
    log_error ""
    
    # Stop services
    stop_services_on_error
    
    # Restore backup configuration
    restore_backup_config
    
    # Clean up partial installation
    cleanup_partial_installation
    
    log_error ""
    log_error "Rollback completed"
    log_error "System has been restored to previous state"
    log_error ""
    
    return 0
}

# Enhanced cleanup function with rollback support
# Called on script exit via trap
# Usage: cleanup_with_rollback
# Requirements: 10.3, 10.4
cleanup_with_rollback() {
    local exit_code=$?
    
    # Clean up temporary files first (Task 14.4)
    cleanup_temp_files
    
    # Record end time
    DEPLOY_STATE[end_time]=$(date '+%Y-%m-%d %H:%M:%S')
    
    if [ $exit_code -ne 0 ]; then
        log_error "Deployment failed with exit code $exit_code"
        log_error "Phase: ${DEPLOY_STATE[phase]}"
        log_error "Errors: ${DEPLOY_STATE[errors]}, Warnings: ${DEPLOY_STATE[warnings]}"
        
        # Perform rollback based on deployment phase
        case "${DEPLOY_STATE[phase]}" in
            install|config|start)
                log_error "Error occurred during ${DEPLOY_STATE[phase]} phase"
                log_error "Initiating rollback..."
                perform_rollback
                ;;
            api_call)
                log_error "Error occurred during API call"
                log_error "No rollback needed (no local changes made)"
                ;;
            verify)
                log_error "Error occurred during verification"
                log_error "Deployment may be partially functional"
                log_error "Check logs and service status"
                ;;
            *)
                log_debug "No rollback needed for phase: ${DEPLOY_STATE[phase]}"
                ;;
        esac
        
        # Show context-specific troubleshooting tips
        log_info ""
        show_troubleshooting_tips "$exit_code"
    else
        log_info "Deployment completed successfully"
        log_info "Errors: ${DEPLOY_STATE[errors]}, Warnings: ${DEPLOY_STATE[warnings]}"
    fi
    
    # Write final log entry
    echo "========================================" >> "$LOG_FILE" 2>/dev/null || true
    echo "Deployment ended at ${DEPLOY_STATE[end_time]}" >> "$LOG_FILE" 2>/dev/null || true
    echo "Exit code: $exit_code" >> "$LOG_FILE" 2>/dev/null || true
    echo "Phase: ${DEPLOY_STATE[phase]}" >> "$LOG_FILE" 2>/dev/null || true
    echo "Errors: ${DEPLOY_STATE[errors]}, Warnings: ${DEPLOY_STATE[warnings]}" >> "$LOG_FILE" 2>/dev/null || true
    echo "========================================" >> "$LOG_FILE" 2>/dev/null || true
}

################################################################################
# Cleanup and Rollback Commands (Task 13.3)
################################################################################

# Complete cleanup of node deployment
# Removes all configuration files, services, and optionally binaries
# Usage: cleanup_deployment
# Returns: 0 on success
# Requirements: 11.5
cleanup_deployment() {
    log_info "=========================================="
    log_info "Node Deployment Cleanup"
    log_info "=========================================="
    log_info ""
    
    log_warn "This will remove all node deployment files and services"
    log_warn "The node will be completely uninstalled from this server"
    log_info ""
    
    # Confirm cleanup unless --force is set
    if [ "$FORCE" != "true" ]; then
        read -p "Are you sure you want to proceed? (yes/no): " confirm
        if [ "$confirm" != "yes" ]; then
            log_info "Cleanup cancelled by user"
            return 0
        fi
    fi
    
    log_info "Starting cleanup process..."
    log_info ""
    
    # Stop services
    log_info "Stopping services..."
    
    if systemctl is-active --quiet node-agent; then
        log_info "Stopping Node Agent service..."
        systemctl stop node-agent 2>&1 | tee -a "$LOG_FILE" || true
    fi
    
    if systemctl is-active --quiet xray; then
        log_info "Stopping Xray-core service..."
        systemctl stop xray 2>&1 | tee -a "$LOG_FILE" || true
    fi
    
    # Disable services
    log_info "Disabling services..."
    
    if systemctl is-enabled --quiet node-agent 2>/dev/null; then
        log_info "Disabling Node Agent service..."
        systemctl disable node-agent 2>&1 | tee -a "$LOG_FILE" || true
    fi
    
    # Remove service files
    log_info "Removing service files..."
    
    if [ -f "/etc/systemd/system/node-agent.service" ]; then
        log_info "Removing Node Agent service file..."
        rm -f "/etc/systemd/system/node-agent.service"
        log_info "✓ Service file removed"
    fi
    
    # Reload systemd
    systemctl daemon-reload 2>&1 | tee -a "$LOG_FILE" || true
    
    # Remove configuration files
    log_info "Removing configuration files..."
    
    if [ -d "/etc/node-agent" ]; then
        log_info "Removing Node Agent configuration directory..."
        rm -rf "/etc/node-agent"
        log_info "✓ Configuration directory removed"
    fi
    
    # Ask about removing binaries
    log_info ""
    log_info "Do you want to remove installed binaries?"
    log_info "  - Node Agent binary: /usr/local/bin/node-agent"
    log_info "  - Xray-core (if installed)"
    log_info ""
    
    local remove_binaries=false
    if [ "$FORCE" = "true" ]; then
        remove_binaries=true
    else
        read -p "Remove binaries? (yes/no): " confirm_binaries
        if [ "$confirm_binaries" = "yes" ]; then
            remove_binaries=true
        fi
    fi
    
    if [ "$remove_binaries" = "true" ]; then
        log_info "Removing binaries..."
        
        if [ -f "/usr/local/bin/node-agent" ]; then
            log_info "Removing Node Agent binary..."
            rm -f "/usr/local/bin/node-agent"
            log_info "✓ Node Agent binary removed"
        fi
        
        # Note: We don't remove Xray-core as it may be used by other services
        log_info "Note: Xray-core is not removed (may be used by other services)"
        log_info "To remove Xray-core, run: bash -c \"\$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)\" @ remove"
    fi
    
    # Remove backups (optional)
    log_info ""
    if [ -d "/var/backups/node-deployment" ]; then
        log_info "Backup directory found: /var/backups/node-deployment"
        
        local remove_backups=false
        if [ "$FORCE" = "true" ]; then
            remove_backups=false  # Keep backups by default even with --force
        else
            read -p "Remove backup files? (yes/no): " confirm_backups
            if [ "$confirm_backups" = "yes" ]; then
                remove_backups=true
            fi
        fi
        
        if [ "$remove_backups" = "true" ]; then
            log_info "Removing backup directory..."
            rm -rf "/var/backups/node-deployment"
            log_info "✓ Backup directory removed"
        else
            log_info "Keeping backup directory (can be removed manually later)"
        fi
    fi
    
    log_info ""
    log_info "=========================================="
    log_info "Cleanup Completed"
    log_info "=========================================="
    log_info ""
    log_info "Node deployment has been removed from this server"
    log_info ""
    log_info "Remaining items (if any):"
    log_info "  - Xray-core installation (not removed)"
    log_info "  - Backup files in /var/backups/node-deployment (if kept)"
    log_info "  - Deployment logs in $LOG_FILE"
    log_info ""
    
    return 0
}

# Rollback to previous stable version
# Restores the most recent backup configuration and restarts services
# Usage: rollback_to_previous
# Returns: 0 on success, 1 on failure
# Requirements: 11.6
rollback_to_previous() {
    log_info "=========================================="
    log_info "Rollback to Previous Version"
    log_info "=========================================="
    log_info ""
    
    # Check if backups exist
    local backup_dir="/var/backups/node-deployment"
    
    if [ ! -d "$backup_dir" ]; then
        log_error "No backup directory found: $backup_dir"
        log_error "Cannot rollback without backups"
        return 1
    fi
    
    # Find most recent backup
    local latest_backup
    latest_backup=$(find "$backup_dir" -maxdepth 1 -type d -name "backup_*" | sort -r | head -n1)
    
    if [ -z "$latest_backup" ]; then
        log_error "No backup found in $backup_dir"
        log_error "Cannot rollback without backups"
        return 1
    fi
    
    log_info "Found backup: $latest_backup"
    
    # Show backup contents
    log_info ""
    log_info "Backup contains:"
    ls -lh "$latest_backup" | tail -n +2 | while read -r line; do
        log_info "  $line"
    done
    log_info ""
    
    # Confirm rollback unless --force is set
    if [ "$FORCE" != "true" ]; then
        log_warn "This will restore the backup and restart services"
        read -p "Proceed with rollback? (yes/no): " confirm
        if [ "$confirm" != "yes" ]; then
            log_info "Rollback cancelled by user"
            return 0
        fi
    fi
    
    log_info "Starting rollback process..."
    log_info ""
    
    # Stop services
    log_info "Stopping services..."
    
    if systemctl is-active --quiet node-agent; then
        log_info "Stopping Node Agent service..."
        if systemctl stop node-agent 2>&1 | tee -a "$LOG_FILE"; then
            log_info "✓ Node Agent stopped"
        else
            log_warn "Failed to stop Node Agent service"
        fi
    fi
    
    if systemctl is-active --quiet xray; then
        log_info "Stopping Xray-core service..."
        if systemctl stop xray 2>&1 | tee -a "$LOG_FILE"; then
            log_info "✓ Xray-core stopped"
        else
            log_warn "Failed to stop Xray-core service"
        fi
    fi
    
    # Wait for services to stop
    sleep 2
    
    # Restore configuration files
    log_info "Restoring configuration files..."
    
    local restored_count=0
    
    # Restore Node Agent configuration
    if [ -f "$latest_backup/config.env" ]; then
        log_info "Restoring Node Agent configuration..."
        if cp "$latest_backup/config.env" "/etc/node-agent/config.env"; then
            chmod 600 "/etc/node-agent/config.env"
            log_info "✓ Node Agent configuration restored"
            ((restored_count++))
        else
            log_error "Failed to restore Node Agent configuration"
            return 1
        fi
    fi
    
    # Restore Xray-core configuration
    if [ -f "$latest_backup/xray_config.json" ]; then
        log_info "Restoring Xray-core configuration..."
        if cp "$latest_backup/xray_config.json" "/etc/xray/config.json"; then
            chmod 644 "/etc/xray/config.json"
            log_info "✓ Xray-core configuration restored"
            ((restored_count++))
        else
            log_error "Failed to restore Xray-core configuration"
            return 1
        fi
    fi
    
    # Restore systemd service file
    if [ -f "$latest_backup/node-agent.service" ]; then
        log_info "Restoring systemd service file..."
        if cp "$latest_backup/node-agent.service" "/etc/systemd/system/node-agent.service"; then
            systemctl daemon-reload
            log_info "✓ Systemd service file restored"
            ((restored_count++))
        else
            log_error "Failed to restore systemd service file"
            return 1
        fi
    fi
    
    if [ $restored_count -eq 0 ]; then
        log_error "No configuration files were restored"
        log_error "Backup may be incomplete or corrupted"
        return 1
    fi
    
    log_info "Restored $restored_count configuration file(s)"
    log_info ""
    
    # Restart services
    log_info "Restarting services..."
    
    log_info "Starting Node Agent service..."
    if systemctl start node-agent 2>&1 | tee -a "$LOG_FILE"; then
        log_info "✓ Node Agent started"
    else
        log_error "Failed to start Node Agent service"
        log_error "Check logs: journalctl -u node-agent -n 50"
        return 1
    fi
    
    # Wait for service to start
    sleep 3
    
    # Verify service is running
    if systemctl is-active --quiet node-agent; then
        log_info "✓ Node Agent is running"
    else
        log_error "Node Agent failed to start"
        log_error "Check logs: journalctl -u node-agent -n 50"
        return 1
    fi
    
    log_info ""
    log_info "=========================================="
    log_info "Rollback Completed Successfully"
    log_info "=========================================="
    log_info ""
    log_info "Configuration has been restored from backup"
    log_info "Services have been restarted"
    log_info ""
    log_info "Backup used: $latest_backup"
    log_info ""
    log_info "Next steps:"
    log_info "  1. Verify service status: systemctl status node-agent"
    log_info "  2. Check service logs: journalctl -u node-agent -n 50"
    log_info "  3. Test node connectivity"
    log_info ""
    
    return 0
}

################################################################################
# Node Update and Redeployment Functions
################################################################################

# Check if node is already deployed on this server
# Checks for configuration files and installed services
# Usage: check_node_exists
# Returns: 0 if node exists, 1 if not
# Requirements: 9.1
check_node_exists() {
    log_debug "Checking if node is already deployed..."
    
    local node_exists=false
    local config_exists=false
    local service_exists=false
    local agent_installed=false
    
    # Check if Node Agent configuration exists
    if [ -f "/etc/node-agent/config.env" ]; then
        log_debug "Found Node Agent configuration file"
        config_exists=true
        node_exists=true
    fi
    
    # Check if Node Agent service is installed
    if systemctl list-unit-files | grep -q "node-agent.service"; then
        log_debug "Found Node Agent systemd service"
        service_exists=true
        node_exists=true
    fi
    
    # Check if Node Agent binary is installed
    if command -v node-agent &> /dev/null; then
        log_debug "Found Node Agent binary"
        agent_installed=true
        node_exists=true
    fi
    
    # Check if Xray-core is installed
    local xray_installed=false
    if command -v xray &> /dev/null; then
        log_debug "Found Xray-core installation"
        xray_installed=true
    fi
    
    # Log findings
    if [ "$node_exists" = "true" ]; then
        log_info "Existing node deployment detected:"
        
        if [ "$config_exists" = "true" ]; then
            log_info "  ✓ Configuration file exists: /etc/node-agent/config.env"
            
            # Try to extract existing NODE_ID and NODE_SECRET
            if [ -f "/etc/node-agent/config.env" ]; then
                local existing_node_id
                local existing_node_secret
                
                existing_node_id=$(grep "^NODE_ID=" /etc/node-agent/config.env 2>/dev/null | cut -d'=' -f2)
                existing_node_secret=$(grep "^NODE_SECRET=" /etc/node-agent/config.env 2>/dev/null | cut -d'=' -f2)
                
                if [ -n "$existing_node_id" ]; then
                    log_info "  ✓ Existing Node ID: $existing_node_id"
                fi
                
                if [ -n "$existing_node_secret" ]; then
                    log_info "  ✓ Existing Node Secret: $(mask_sensitive "$existing_node_secret")"
                fi
            fi
        fi
        
        if [ "$service_exists" = "true" ]; then
            log_info "  ✓ Systemd service exists"
            
            # Check service status
            if systemctl is-active --quiet node-agent; then
                log_info "  ✓ Service is currently running"
            else
                log_info "  ⚠ Service is installed but not running"
            fi
        fi
        
        if [ "$agent_installed" = "true" ]; then
            local agent_version
            agent_version=$(node-agent --version 2>/dev/null || echo "unknown")
            log_info "  ✓ Node Agent binary installed: $agent_version"
        fi
        
        if [ "$xray_installed" = "true" ]; then
            local xray_version
            xray_version=$(xray version 2>/dev/null | head -n1 || echo "unknown")
            log_info "  ✓ Xray-core installed: $xray_version"
        fi
        
        return 0
    else
        log_debug "No existing node deployment found"
        return 1
    fi
}

# Load existing node configuration from config file
# Extracts NODE_ID and NODE_SECRET from existing configuration
# Usage: load_existing_config
# Sets DEPLOY_CONFIG[node_id] and DEPLOY_CONFIG[node_secret]
# Returns: 0 on success, 1 on failure
load_existing_config() {
    log_debug "Loading existing node configuration..."
    
    local config_file="/etc/node-agent/config.env"
    
    if [ ! -f "$config_file" ]; then
        log_error "Configuration file not found: $config_file"
        return 1
    fi
    
    # Extract NODE_ID
    local existing_node_id
    existing_node_id=$(grep "^NODE_ID=" "$config_file" 2>/dev/null | cut -d'=' -f2)
    
    if [ -z "$existing_node_id" ]; then
        log_error "Could not extract NODE_ID from configuration file"
        return 1
    fi
    
    # Extract NODE_SECRET
    local existing_node_secret
    existing_node_secret=$(grep "^NODE_SECRET=" "$config_file" 2>/dev/null | cut -d'=' -f2)
    
    if [ -z "$existing_node_secret" ]; then
        log_error "Could not extract NODE_SECRET from configuration file"
        return 1
    fi
    
    # Store in DEPLOY_CONFIG
    DEPLOY_CONFIG[node_id]="$existing_node_id"
    DEPLOY_CONFIG[node_secret]="$existing_node_secret"
    
    log_info "Loaded existing configuration:"
    log_info "  Node ID: $existing_node_id"
    log_info "  Node Secret: $(mask_sensitive "$existing_node_secret")"
    
    return 0
}

# Update existing node deployment
# Preserves NODE_ID and NODE_SECRET, updates configuration and restarts services
# Usage: update_node
# Returns: 0 on success, non-zero on failure
# Requirements: 9.2, 9.5
update_node() {
    log_info "=========================================="
    log_info "Updating Existing Node Deployment"
    log_info "=========================================="
    log_info ""
    
    # Load existing configuration
    if ! load_existing_config; then
        log_error "Failed to load existing configuration"
        log_error "Cannot proceed with update"
        return 1
    fi
    
    log_info "Update will preserve existing Node ID and Secret"
    log_info "Configuration and services will be updated"
    log_info ""
    
    # Update Xray-core configuration if protocol or port changed
    log_info "Updating Xray-core configuration..."
    if ! generate_xray_config; then
        log_error "Failed to update Xray-core configuration"
        return 5
    fi
    log_info "✓ Xray-core configuration updated"
    
    # Update Node Agent configuration
    log_info "Updating Node Agent configuration..."
    if ! create_node_agent_config; then
        log_error "Failed to update Node Agent configuration"
        return 5
    fi
    log_info "✓ Node Agent configuration updated"
    
    # Update systemd service file (in case it changed)
    log_info "Updating systemd service..."
    if ! create_node_agent_service; then
        log_error "Failed to update systemd service"
        return 5
    fi
    log_info "✓ Systemd service updated"
    
    # Restart services to apply changes
    log_info "Restarting services to apply changes..."
    
    # Stop service first
    log_info "Stopping Node Agent service..."
    if systemctl stop node-agent 2>&1 | tee -a "$LOG_FILE"; then
        log_info "✓ Service stopped"
    else
        log_warn "Failed to stop service gracefully"
    fi
    
    # Wait a moment for clean shutdown
    sleep 2
    
    # Start service
    log_info "Starting Node Agent service..."
    if ! start_services; then
        log_error "Failed to restart services"
        log_error "Node may be in inconsistent state"
        show_troubleshooting_tips 6
        return 6
    fi
    
    log_info "✓ Services restarted successfully"
    log_info ""
    
    # Verify updated deployment
    log_info "Verifying updated deployment..."
    if ! verify_deployment; then
        log_warn "Deployment verification failed"
        log_warn "Node may still be functional, check logs"
        return 6
    fi
    
    log_info ""
    log_info "=========================================="
    log_info "Node Update Completed Successfully!"
    log_info "=========================================="
    log_info ""
    log_info "Updated Node Information:"
    log_info "  Node ID: ${DEPLOY_CONFIG[node_id]} (preserved)"
    log_info "  Node Secret: $(mask_sensitive "${DEPLOY_CONFIG[node_secret]}") (preserved)"
    log_info "  Node Name: ${DEPLOY_CONFIG[node_name]}"
    log_info "  Node Host: ${DEPLOY_CONFIG[node_host]}"
    log_info "  Node Port: ${DEPLOY_CONFIG[node_port]}"
    log_info "  Protocol: ${DEPLOY_CONFIG[node_protocol]}"
    log_info ""
    
    return 0
}

# Prompt user for update confirmation
# Asks user whether to update existing node or cancel
# Usage: prompt_update_confirmation
# Returns: 0 if user confirms, 1 if user cancels
prompt_update_confirmation() {
    log_info ""
    log_info "=========================================="
    log_info "Existing Node Deployment Detected"
    log_info "=========================================="
    log_info ""
    log_info "An existing node deployment was found on this server."
    log_info ""
    log_info "Options:"
    log_info "  1. Update existing node (preserves Node ID and Secret)"
    log_info "  2. Redeploy from scratch (creates new Node ID and Secret)"
    log_info "  3. Cancel deployment"
    log_info ""
    
    # If --force flag is set, skip confirmation
    if [ "$FORCE" = "true" ]; then
        log_info "Force mode enabled (--force), proceeding with redeployment..."
        return 2  # Return 2 to indicate redeploy
    fi
    
    # Prompt user for choice
    while true; do
        read -p "Enter your choice (1/2/3): " choice
        
        case "$choice" in
            1)
                log_info "User selected: Update existing node"
                return 0  # Update mode
                ;;
            2)
                log_info "User selected: Redeploy from scratch"
                return 2  # Redeploy mode
                ;;
            3)
                log_info "User selected: Cancel deployment"
                return 1  # Cancel
                ;;
            *)
                echo "Invalid choice. Please enter 1, 2, or 3."
                ;;
        esac
    done
}

# Redeploy node from scratch
# Stops existing services, backs up configuration, and performs full deployment
# Usage: redeploy_node
# Returns: 0 on success, non-zero on failure
# Requirements: 9.3, 9.4
redeploy_node() {
    log_info "=========================================="
    log_info "Redeploying Node from Scratch"
    log_info "=========================================="
    log_info ""
    
    log_warn "This will stop existing services and create a new node"
    log_warn "The old Node ID and Secret will be replaced"
    log_info ""
    
    # Stop existing services
    log_info "Stopping existing services..."
    
    if systemctl is-active --quiet node-agent; then
        log_info "Stopping Node Agent service..."
        if systemctl stop node-agent 2>&1 | tee -a "$LOG_FILE"; then
            log_info "✓ Node Agent stopped"
        else
            log_warn "Failed to stop Node Agent service"
        fi
    else
        log_debug "Node Agent service is not running"
    fi
    
    if systemctl is-active --quiet xray; then
        log_info "Stopping Xray-core service..."
        if systemctl stop xray 2>&1 | tee -a "$LOG_FILE"; then
            log_info "✓ Xray-core stopped"
        else
            log_warn "Failed to stop Xray-core service"
        fi
    else
        log_debug "Xray-core service is not running"
    fi
    
    # Wait for services to stop completely
    sleep 2
    
    # Backup existing configuration files
    log_info "Backing up existing configuration..."
    
    local backup_dir="/var/backups/node-deployment"
    local backup_timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_path="$backup_dir/backup_$backup_timestamp"
    
    # Create backup directory
    if ! mkdir -p "$backup_path"; then
        log_error "Failed to create backup directory: $backup_path"
        return 1
    fi
    
    log_info "Backup directory: $backup_path"
    
    # Backup Node Agent configuration
    if [ -f "/etc/node-agent/config.env" ]; then
        if cp "/etc/node-agent/config.env" "$backup_path/config.env"; then
            log_info "✓ Backed up Node Agent configuration"
        else
            log_warn "Failed to backup Node Agent configuration"
        fi
    fi
    
    # Backup Xray-core configuration
    if [ -f "/etc/xray/config.json" ]; then
        if cp "/etc/xray/config.json" "$backup_path/xray_config.json"; then
            log_info "✓ Backed up Xray-core configuration"
        else
            log_warn "Failed to backup Xray-core configuration"
        fi
    fi
    
    # Backup systemd service file
    if [ -f "/etc/systemd/system/node-agent.service" ]; then
        if cp "/etc/systemd/system/node-agent.service" "$backup_path/node-agent.service"; then
            log_info "✓ Backed up systemd service file"
        else
            log_warn "Failed to backup systemd service file"
        fi
    fi
    
    log_info "Configuration backed up to: $backup_path"
    log_info ""
    
    # Now proceed with full deployment
    log_info "Starting full deployment process..."
    log_info ""
    
    # The deployment will continue in the main function
    # We just need to ensure we don't skip any steps
    
    return 0
}

################################################################################
# Main Entry Point (Placeholder)
################################################################################

main() {
    # Initialize logging
    init_log
    
    log_info "VPN Node Deployment Script v$SCRIPT_VERSION"
    log_info "Starting deployment process..."
    
    # Parse parameters
    parse_parameters "$@"
    
    # Handle cleanup command
    if [ "$CLEANUP" = "true" ]; then
        log_info "Cleanup mode enabled"
        log_info ""
        
        # Check root privileges
        check_root
        
        # Run cleanup
        if cleanup_deployment; then
            log_info "Cleanup completed successfully"
            exit 0
        else
            log_error "Cleanup failed"
            exit 1
        fi
    fi
    
    # Handle rollback command
    if [ "$ROLLBACK" = "true" ]; then
        log_info "Rollback mode enabled"
        log_info ""
        
        # Check root privileges
        check_root
        
        # Run rollback
        if rollback_to_previous; then
            log_info "Rollback completed successfully"
            exit 0
        else
            log_error "Rollback failed"
            exit 1
        fi
    fi
    
    # Check if batch deployment mode
    if [ -n "$BATCH_CONFIG_FILE" ]; then
        log_info "Batch deployment mode enabled"
        log_info "Configuration file: $BATCH_CONFIG_FILE"
        log_info ""
        
        # Check batch configuration file permissions (Task 14.3 - Requirement 12.5)
        if ! check_batch_config_permissions "$BATCH_CONFIG_FILE"; then
            log_error "Batch configuration file has insecure permissions"
            log_error "Please fix file permissions before proceeding"
            exit 1
        fi
        
        # Check root privileges
        check_root
        
        # Detect operating system
        DEPLOY_STATE[phase]="env_check"
        if ! detect_os; then
            log_error "Operating system detection failed"
            exit 2
        fi
        
        # Check dependencies
        if ! check_dependencies; then
            log_error "Dependency check failed"
            exit 2
        fi
        
        # Run batch deployment
        if batch_deploy "$BATCH_CONFIG_FILE"; then
            # Generate summary report
            generate_batch_report
            
            # Exit with success if at least one node succeeded
            if [ $BATCH_FAIL_COUNT -eq 0 ]; then
                log_info "All nodes deployed successfully!"
                exit 0
            else
                log_warn "Some nodes failed to deploy"
                exit 1
            fi
        else
            # Generate summary report even on failure
            generate_batch_report
            
            log_error "Batch deployment failed"
            exit 1
        fi
    fi
    
    # Single node deployment mode (original logic)
    log_info "Single node deployment mode"
    log_info ""
    
    # Apply default values
    apply_defaults
    
    # Validate parameters
    validate_parameters
    
    # Check root privileges
    check_root
    
    # Detect operating system
    DEPLOY_STATE[phase]="env_check"
    if ! detect_os; then
        log_error "Operating system detection failed"
        exit 2
    fi
    
    # Check dependencies
    if ! check_dependencies; then
        log_error "Dependency check failed"
        exit 2
    fi
    
    # Verify SSL certificate for API URL (Task 14.2 - Requirement 12.4)
    log_info ""
    verify_ssl_certificate "${DEPLOY_CONFIG[api_url]}"
    log_info ""
    
    # Check if node already exists
    local deployment_mode="new"  # new, update, or redeploy
    
    if check_node_exists; then
        log_info ""
        
        # If --force flag is set, automatically redeploy
        if [ "$FORCE" = "true" ]; then
            log_info "Force mode enabled (--force), proceeding with redeployment..."
            deployment_mode="redeploy"
        else
            # Prompt user for action
            prompt_update_confirmation
            local user_choice=$?
            
            case $user_choice in
                0)
                    # User chose to update
                    deployment_mode="update"
                    ;;
                1)
                    # User chose to cancel
                    log_info "Deployment cancelled by user"
                    exit 0
                    ;;
                2)
                    # User chose to redeploy
                    deployment_mode="redeploy"
                    ;;
                *)
                    log_error "Unexpected return value from prompt"
                    exit 1
                    ;;
            esac
        fi
        
        log_info ""
    fi
    
    # Handle update mode
    if [ "$deployment_mode" = "update" ]; then
        log_info "Proceeding with update mode..."
        log_info ""
        
        # Update existing node
        if update_node; then
            log_info "Node update completed successfully!"
            exit 0
        else
            log_error "Node update failed"
            exit 1
        fi
    fi
    
    # Handle redeploy mode
    if [ "$deployment_mode" = "redeploy" ]; then
        log_info "Proceeding with redeployment mode..."
        log_info ""
        
        # Redeploy node (stop services and backup)
        if ! redeploy_node; then
            log_error "Redeployment preparation failed"
            exit 1
        fi
        
        # Continue with full deployment below
        log_info "Continuing with full deployment..."
        log_info ""
    fi
    
    # Auto-detect public IP if NODE_HOST not provided
    if [ -z "${DEPLOY_CONFIG[node_host]}" ]; then
        log_info "NODE_HOST not specified, auto-detecting public IP..."
        local detected_ip
        if detected_ip=$(detect_public_ip); then
            DEPLOY_CONFIG[node_host]="$detected_ip"
            log_info "Using detected IP: ${DEPLOY_CONFIG[node_host]}"
        else
            log_error "Failed to auto-detect public IP"
            log_error "Please specify NODE_HOST using --node-host parameter"
            exit 1
        fi
    fi
    
    # Create node via API
    DEPLOY_STATE[phase]="api_call"
    if ! create_node; then
        log_error "Node creation failed"
        exit 4
    fi
    
    # Install Xray-core
    DEPLOY_STATE[phase]="install"
    if ! install_xray; then
        log_error "Xray-core installation failed"
        exit 5
    fi
    
    # Generate Xray-core configuration
    if ! generate_xray_config; then
        log_error "Xray-core configuration generation failed"
        exit 5
    fi
    
    # Install Node Agent
    if ! install_node_agent; then
        log_error "Node Agent installation failed"
        exit 5
    fi
    
    # Create Node Agent configuration
    DEPLOY_STATE[phase]="config"
    if ! create_node_agent_config; then
        log_error "Node Agent configuration failed"
        exit 5
    fi
    
    # Set and verify configuration file permissions (Task 14.1 - Requirement 12.2)
    log_info ""
    if ! set_config_file_permissions; then
        log_error "Failed to set secure file permissions"
        exit 5
    fi
    
    if ! verify_config_file_permissions; then
        log_warn "Configuration file permissions verification failed"
        log_warn "Continuing with deployment, but files may not be properly secured"
    fi
    
    # Create Node Agent systemd service
    if ! create_node_agent_service; then
        log_error "Node Agent service creation failed"
        exit 5
    fi
    
    # Start services
    DEPLOY_STATE[phase]="start"
    if ! start_services; then
        log_error "Service startup failed"
        show_troubleshooting_tips 6
        exit 6
    fi
    
    # Verify deployment
    DEPLOY_STATE[phase]="verify"
    if ! verify_deployment; then
        log_error "Deployment verification failed"
        show_troubleshooting_tips 6
        exit 6
    fi
    
    # Mark deployment as complete
    DEPLOY_STATE[phase]="complete"
    log_info ""
    log_info "=========================================="
    
    if [ "$deployment_mode" = "redeploy" ]; then
        log_info "Redeployment Completed Successfully!"
    else
        log_info "Deployment Completed Successfully!"
    fi
    
    log_info "=========================================="
    log_info ""
    log_info "Node Information:"
    log_info "  Node ID: ${DEPLOY_CONFIG[node_id]}"
    log_info "  Node Name: ${DEPLOY_CONFIG[node_name]}"
    log_info "  Node Host: ${DEPLOY_CONFIG[node_host]}"
    log_info "  Node Port: ${DEPLOY_CONFIG[node_port]}"
    log_info "  Protocol: ${DEPLOY_CONFIG[node_protocol]}"
    log_info ""
    log_info "Service Status:"
    log_info "  Node Agent: $(systemctl is-active node-agent)"
    log_info "  Xray-core: $(systemctl is-active xray 2>/dev/null || echo 'not started')"
    log_info ""
    log_info "Next Steps:"
    log_info "  1. Monitor service logs: journalctl -u node-agent -f"
    log_info "  2. Check node status in admin panel"
    log_info "  3. Verify connectivity from client devices"
    log_info ""
}

# Run main function if script is executed directly (not sourced)
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi
