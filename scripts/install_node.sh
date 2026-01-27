#!/bin/bash

# VPN Node Agent Installation Script
# This script installs Xray-core and Node Agent on a VPN node server

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration variables
API_URL="${API_URL}"
NODE_ID="${NODE_ID}"
NODE_SECRET="${NODE_SECRET}"
XRAY_API_PORT="${XRAY_API_PORT:-10085}"
TRAFFIC_REPORT_INTERVAL="${TRAFFIC_REPORT_INTERVAL:-30}"
HEARTBEAT_INTERVAL="${HEARTBEAT_INTERVAL:-60}"

# Function to print colored messages
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_error "This script must be run as root"
        exit 1
    fi
}

# Function to validate required variables
validate_config() {
    if [ -z "$API_URL" ]; then
        print_error "API_URL is not set. Please set it before running this script."
        echo "Example: export API_URL=https://api.yourdomain.com"
        exit 1
    fi

    if [ -z "$NODE_ID" ]; then
        print_error "NODE_ID is not set. Please set it before running this script."
        echo "Example: export NODE_ID=node-001"
        exit 1
    fi

    if [ -z "$NODE_SECRET" ]; then
        print_error "NODE_SECRET is not set. Please set it before running this script."
        echo "Example: export NODE_SECRET=your-node-secret"
        exit 1
    fi

    print_info "Configuration validated successfully"
    print_info "API URL: $API_URL"
    print_info "Node ID: $NODE_ID"
}

# Function to detect OS
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        VERSION=$VERSION_ID
    else
        print_error "Cannot detect OS"
        exit 1
    fi
    print_info "Detected OS: $OS $VERSION"
}

# Function to install dependencies
install_dependencies() {
    print_info "Installing dependencies..."
    
    case $OS in
        ubuntu|debian)
            apt-get update
            apt-get install -y curl wget unzip systemctl
            ;;
        centos|rhel|fedora)
            yum install -y curl wget unzip systemd
            ;;
        *)
            print_warn "Unknown OS, attempting to continue..."
            ;;
    esac
}

# Function to install Xray-core
install_xray() {
    print_info "Installing Xray-core..."
    
    # Download and run Xray installation script
    bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
    
    if [ $? -eq 0 ]; then
        print_info "Xray-core installed successfully"
    else
        print_error "Failed to install Xray-core"
        exit 1
    fi
    
    # Enable Xray service but don't start it yet (Node Agent will manage it)
    systemctl enable xray
    print_info "Xray service enabled"
}

# Function to download Node Agent
download_node_agent() {
    print_info "Downloading Node Agent..."
    
    # Determine architecture
    ARCH=$(uname -m)
    case $ARCH in
        x86_64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    # Download the latest release
    # Note: Update this URL to your actual release URL
    DOWNLOAD_URL="https://github.com/your-org/vpn-platform/releases/latest/download/node-agent-${ARCH}"
    
    print_info "Downloading from: $DOWNLOAD_URL"
    
    if wget -O /usr/local/bin/node-agent "$DOWNLOAD_URL"; then
        chmod +x /usr/local/bin/node-agent
        print_info "Node Agent downloaded successfully"
    else
        print_error "Failed to download Node Agent"
        print_warn "You may need to build and copy the binary manually"
        exit 1
    fi
}

# Function to create configuration
create_config() {
    print_info "Creating configuration..."
    
    # Create configuration directory
    mkdir -p /etc/node-agent
    
    # Create configuration file
    cat > /etc/node-agent/config.env <<EOF
API_URL=${API_URL}
NODE_ID=${NODE_ID}
NODE_SECRET=${NODE_SECRET}
XRAY_API_PORT=${XRAY_API_PORT}
TRAFFIC_REPORT_INTERVAL=${TRAFFIC_REPORT_INTERVAL}
HEARTBEAT_INTERVAL=${HEARTBEAT_INTERVAL}
RUST_LOG=info
EOF
    
    chmod 600 /etc/node-agent/config.env
    print_info "Configuration file created at /etc/node-agent/config.env"
}

# Function to create systemd service
create_systemd_service() {
    print_info "Creating systemd service..."
    
    cat > /etc/systemd/system/node-agent.service <<EOF
[Unit]
Description=VPN Node Agent
Documentation=https://github.com/your-org/vpn-platform
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
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

[Install]
WantedBy=multi-user.target
EOF
    
    print_info "Systemd service created"
}

# Function to start services
start_services() {
    print_info "Starting services..."
    
    # Reload systemd
    systemctl daemon-reload
    
    # Enable and start Node Agent
    systemctl enable node-agent
    systemctl start node-agent
    
    if [ $? -eq 0 ]; then
        print_info "Node Agent started successfully"
    else
        print_error "Failed to start Node Agent"
        print_info "Check logs with: journalctl -u node-agent -f"
        exit 1
    fi
}

# Function to check service status
check_status() {
    print_info "Checking service status..."
    
    sleep 3
    
    if systemctl is-active --quiet node-agent; then
        print_info "✓ Node Agent is running"
    else
        print_error "✗ Node Agent is not running"
        print_info "Check logs with: journalctl -u node-agent -f"
        return 1
    fi
    
    if systemctl is-active --quiet xray; then
        print_info "✓ Xray is running"
    else
        print_warn "✗ Xray is not running (this is normal if not yet configured)"
    fi
}

# Function to display post-installation info
display_info() {
    echo ""
    echo "=========================================="
    print_info "Installation completed successfully!"
    echo "=========================================="
    echo ""
    echo "Useful commands:"
    echo "  - Check Node Agent status: systemctl status node-agent"
    echo "  - View Node Agent logs:    journalctl -u node-agent -f"
    echo "  - Restart Node Agent:      systemctl restart node-agent"
    echo "  - Stop Node Agent:         systemctl stop node-agent"
    echo ""
    echo "  - Check Xray status:       systemctl status xray"
    echo "  - View Xray logs:          journalctl -u xray -f"
    echo ""
    echo "Configuration file: /etc/node-agent/config.env"
    echo ""
}

# Main installation flow
main() {
    print_info "Starting VPN Node Agent installation..."
    echo ""
    
    check_root
    validate_config
    detect_os
    install_dependencies
    install_xray
    download_node_agent
    create_config
    create_systemd_service
    start_services
    check_status
    display_info
}

# Run main function
main
