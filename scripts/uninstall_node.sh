#!/bin/bash

# VPN Node Agent Uninstallation Script
# This script removes Node Agent and optionally Xray-core

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

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

# Function to stop and disable services
stop_services() {
    print_info "Stopping services..."
    
    if systemctl is-active --quiet node-agent; then
        systemctl stop node-agent
        print_info "Node Agent stopped"
    fi
    
    if systemctl is-enabled --quiet node-agent 2>/dev/null; then
        systemctl disable node-agent
        print_info "Node Agent disabled"
    fi
}

# Function to remove Node Agent
remove_node_agent() {
    print_info "Removing Node Agent..."
    
    # Remove binary
    if [ -f /usr/local/bin/node-agent ]; then
        rm -f /usr/local/bin/node-agent
        print_info "Node Agent binary removed"
    fi
    
    # Remove systemd service
    if [ -f /etc/systemd/system/node-agent.service ]; then
        rm -f /etc/systemd/system/node-agent.service
        systemctl daemon-reload
        print_info "Node Agent service removed"
    fi
    
    # Remove configuration (ask user)
    if [ -d /etc/node-agent ]; then
        read -p "Remove configuration directory /etc/node-agent? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf /etc/node-agent
            print_info "Configuration removed"
        else
            print_info "Configuration kept at /etc/node-agent"
        fi
    fi
}

# Function to remove Xray
remove_xray() {
    read -p "Remove Xray-core as well? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_info "Removing Xray-core..."
        
        # Stop and disable Xray
        if systemctl is-active --quiet xray; then
            systemctl stop xray
        fi
        
        if systemctl is-enabled --quiet xray 2>/dev/null; then
            systemctl disable xray
        fi
        
        # Run Xray uninstall script
        bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ remove
        
        print_info "Xray-core removed"
    else
        print_info "Xray-core kept"
    fi
}

# Function to display completion message
display_completion() {
    echo ""
    echo "=========================================="
    print_info "Uninstallation completed!"
    echo "=========================================="
    echo ""
}

# Main uninstallation flow
main() {
    print_info "Starting VPN Node Agent uninstallation..."
    echo ""
    
    check_root
    stop_services
    remove_node_agent
    remove_xray
    display_completion
}

# Run main function
main
