#!/usr/bin/env bash

################################################################################
# VPN 节点管理脚本
# 
# 功能：
# - 部署节点（创建节点记录 + 安装服务）
# - 卸载节点
# - 更新节点配置
# - 查看节点状态
#
# 使用方法：
#   sudo ./node.sh deploy --api-url <URL> --admin-token <TOKEN> --node-name <NAME>
#   sudo ./node.sh uninstall
#   sudo ./node.sh status
################################################################################

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 脚本版本
readonly VERSION="2.0.0"

# 日志文件
LOG_FILE="/var/log/node-deployment.log"

# 配置变量
API_URL=""
ADMIN_TOKEN=""
NODE_NAME=""
NODE_HOST=""
NODE_PORT="443"
NODE_PROTOCOL="vless"
NODE_CONFIG="{}"
NODE_ID=""
NODE_SECRET=""

# 选项
VERBOSE=false
FORCE=false

################################################################################
# 工具函数
################################################################################

log_info() {
    local msg="$1"
    echo -e "${GREEN}[INFO]${NC} $msg"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INFO] $msg" >> "$LOG_FILE" 2>/dev/null || true
}

log_warn() {
    local msg="$1"
    echo -e "${YELLOW}[WARN]${NC} $msg"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [WARN] $msg" >> "$LOG_FILE" 2>/dev/null || true
}

log_error() {
    local msg="$1"
    echo -e "${RED}[ERROR]${NC} $msg" >&2
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR] $msg" >> "$LOG_FILE" 2>/dev/null || true
}

print_banner() {
    echo ""
    echo "=========================================="
    echo "  VPN 节点管理脚本 v${VERSION}"
    echo "=========================================="
    echo ""
}

mask_sensitive() {
    local value="$1"
    local visible_chars=8
    if [ ${#value} -gt $visible_chars ]; then
        echo "${value:0:$visible_chars}..."
    else
        echo "***"
    fi
}

print_usage() {
    cat <<EOF
使用方法: $0 <命令> [选项]

命令:
  deploy              部署节点
  uninstall           卸载节点
  update              更新节点配置
  status              查看节点状态
  version             显示版本信息
  help                显示此帮助信息

部署选项:
  --api-url URL       API 服务地址（必需）
  --admin-token TOKEN 管理员 JWT 令牌（必需）
  --node-name NAME    节点名称（必需）
  --node-host HOST    节点主机地址（可选，默认自动检测）
  --node-port PORT    节点端口（可选，默认 443）
  --node-protocol P   协议类型（可选，默认 vless）
  --node-config JSON  协议配置（可选，默认 {}）
  --force             强制重新部署
  --verbose           详细输出

支持的协议:
  vless, vmess, trojan, shadowsocks, hysteria2

示例:
  # 部署节点
  sudo $0 deploy \\
    --api-url https://api.yourdomain.com \\
    --admin-token eyJhbGci... \\
    --node-name node-hk-01

  # 卸载节点
  sudo $0 uninstall

  # 查看节点状态
  sudo $0 status
EOF
}

################################################################################
# 环境检测
################################################################################

check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "此脚本需要 root 权限运行"
        log_error "请使用: sudo $0 $*"
        exit 1
    fi
}

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
        log_info "检测到操作系统: $OS $OS_VERSION"
    else
        log_error "无法检测操作系统"
        exit 1
    fi
}

check_dependencies() {
    log_info "检查依赖..."
    
    local missing_deps=()
    for cmd in curl jq systemctl openssl; do
        if ! command -v $cmd &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_warn "缺少依赖: ${missing_deps[*]}"
        log_info "尝试自动安装..."
        
        case $OS in
            ubuntu|debian)
                apt-get update
                apt-get install -y curl jq systemd openssl
                ;;
            centos|rhel|rocky|almalinux)
                yum install -y curl jq systemd openssl
                ;;
            *)
                log_error "不支持的操作系统: $OS"
                exit 1
                ;;
        esac
    fi
    
    log_info "依赖检查完成"
}

detect_public_ip() {
    log_info "检测公网 IP..."
    
    local services=(
        "https://api.ipify.org"
        "https://icanhazip.com"
        "https://ifconfig.me"
    )
    
    for service in "${services[@]}"; do
        local ip=$(curl -s --max-time 5 "$service" 2>/dev/null || true)
        if [ -n "$ip" ]; then
            log_info "检测到公网 IP: $ip"
            NODE_HOST="$ip"
            return 0
        fi
    done
    
    log_error "无法检测公网 IP"
    exit 1
}

################################################################################
# 命令实现
################################################################################

cmd_deploy() {
    log_info "开始部署节点..."
    
    check_root
    detect_os
    check_dependencies
    
    # 验证必需参数
    if [ -z "$API_URL" ] || [ -z "$ADMIN_TOKEN" ] || [ -z "$NODE_NAME" ]; then
        log_error "缺少必需参数"
        print_usage
        exit 1
    fi
    
    # 检测公网 IP（如果未指定）
    if [ -z "$NODE_HOST" ]; then
        detect_public_ip
    fi
    
    # 创建节点记录
    create_node_via_api
    
    # 安装 Xray
    install_xray
    
    # 生成 Xray 配置
    generate_xray_config
    
    # 安装 Node Agent
    install_node_agent
    
    # 启动服务
    start_services
    
    # 验证部署
    if verify_deployment; then
        show_deployment_info
        log_info "节点部署完成"
    else
        log_error "部署验证失败"
        exit 1
    fi
}

cmd_uninstall() {
    log_info "开始卸载节点..."
    
    check_root
    
    # 停止服务
    log_info "停止服务..."
    systemctl stop node-agent 2>/dev/null || true
    systemctl stop xray 2>/dev/null || true
    systemctl disable node-agent 2>/dev/null || true
    
    # 删除文件
    log_info "删除文件..."
    rm -f /usr/local/bin/node-agent
    rm -f /etc/systemd/system/node-agent.service
    
    # 询问是否删除配置
    read -p "删除配置目录 /etc/node-agent? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf /etc/node-agent
        log_info "配置已删除"
    else
        log_info "配置保留在 /etc/node-agent"
    fi
    
    # 询问是否删除 Xray
    read -p "卸载 Xray-core? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ remove
        log_info "Xray-core 已卸载"
    else
        log_info "Xray-core 保留"
    fi
    
    systemctl daemon-reload
    log_info "节点卸载完成"
}

cmd_status() {
    log_info "节点状态:"
    echo ""
    
    # 检查 Node Agent
    if systemctl is-active --quiet node-agent; then
        echo -e "${GREEN}✓${NC} Node Agent: 运行中"
    else
        echo -e "${RED}✗${NC} Node Agent: 未运行"
    fi
    
    # 检查 Xray
    if systemctl is-active --quiet xray; then
        echo -e "${GREEN}✓${NC} Xray: 运行中"
    else
        echo -e "${RED}✗${NC} Xray: 未运行"
    fi
    
    echo ""
    echo "详细状态:"
    systemctl status node-agent --no-pager || true
    echo ""
    systemctl status xray --no-pager || true
}

cmd_update() {
    log_info "更新节点配置..."
    log_warn "更新功能待实现"
    # 可以添加更新节点配置的逻辑
}

cmd_version() {
    echo "VPN 节点管理脚本 v${VERSION}"
}

################################################################################
# 辅助函数
################################################################################

generate_node_secret() {
    local secret=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)
    echo "$secret"
}

create_node_via_api() {
    log_info "通过 API 创建节点..."
    
    NODE_SECRET=$(generate_node_secret)
    
    local request_body=$(cat <<EOF
{
    "name": "$NODE_NAME",
    "host": "$NODE_HOST",
    "port": $NODE_PORT,
    "protocol": "$NODE_PROTOCOL",
    "config": $NODE_CONFIG,
    "secret": "$NODE_SECRET"
}
EOF
)
    
    log_info "节点名称: $NODE_NAME"
    log_info "节点地址: $NODE_HOST:$NODE_PORT"
    log_info "协议类型: $NODE_PROTOCOL"
    
    local response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "$request_body" \
        "$API_URL/api/admin/nodes" 2>&1)
    
    local http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        NODE_ID=$(echo "$body" | jq -r '.id // .node_id // empty')
        
        if [ -z "$NODE_ID" ]; then
            log_error "无法从响应中提取节点 ID"
            exit 1
        fi
        
        log_info "节点创建成功"
        log_info "节点 ID: $NODE_ID"
        return 0
    else
        log_error "节点创建失败 (HTTP $http_code)"
        log_error "响应: $body"
        exit 1
    fi
}

install_xray() {
    log_info "安装 Xray-core..."
    
    if command -v xray &> /dev/null && [ "$FORCE" != true ]; then
        log_info "Xray-core 已安装，跳过"
        return 0
    fi
    
    bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
    
    if ! command -v xray &> /dev/null; then
        log_error "Xray-core 安装失败"
        exit 1
    fi
    
    log_info "Xray-core 安装完成"
}

generate_xray_config() {
    log_info "生成 Xray 配置..."
    
    local config_file="/usr/local/etc/xray/config.json"
    
    cat > "$config_file" <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": $NODE_PORT,
      "protocol": "$NODE_PROTOCOL",
      "settings": {
        "clients": []
      },
      "streamSettings": {
        "network": "tcp"
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
      "protocol": "freedom"
    }
  ],
  "api": {
    "tag": "api",
    "services": ["HandlerService", "StatsService"]
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
  }
}
EOF
    
    log_info "Xray 配置生成完成"
}

install_node_agent() {
    log_info "安装 Node Agent..."
    
    local arch=$(uname -m)
    case $arch in
        x86_64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)
            log_error "不支持的架构: $arch"
            exit 1
            ;;
    esac
    
    local download_url="https://github.com/your-org/vpn-platform/releases/latest/download/node-agent-${arch}"
    local binary_path="/usr/local/bin/node-agent"
    
    if [ -f "$binary_path" ] && [ "$FORCE" != true ]; then
        log_info "Node Agent 已安装，跳过"
    else
        log_info "下载 Node Agent..."
        if curl -L -o "$binary_path" "$download_url"; then
            chmod +x "$binary_path"
            log_info "Node Agent 下载完成"
        else
            log_error "Node Agent 下载失败"
            exit 1
        fi
    fi
    
    mkdir -p /etc/node-agent
    
    cat > /etc/node-agent/config.env <<EOF
API_URL=${API_URL}
NODE_ID=${NODE_ID}
NODE_SECRET=${NODE_SECRET}
XRAY_API_PORT=10085
TRAFFIC_REPORT_INTERVAL=30
HEARTBEAT_INTERVAL=60
RUST_LOG=info
EOF
    
    chmod 600 /etc/node-agent/config.env
    
    cat > /etc/systemd/system/node-agent.service <<EOF
[Unit]
Description=VPN Node Agent
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

[Install]
WantedBy=multi-user.target
EOF
    
    systemctl daemon-reload
    log_info "Node Agent 安装完成"
}

start_services() {
    log_info "启动服务..."
    
    systemctl enable xray
    systemctl restart xray
    
    systemctl enable node-agent
    systemctl restart node-agent
    
    sleep 3
    log_info "服务启动完成"
}

verify_deployment() {
    log_info "验证部署..."
    
    local errors=0
    
    if systemctl is-active --quiet xray; then
        log_info "✓ Xray 服务运行中"
    else
        log_error "✗ Xray 服务未运行"
        ((errors++))
    fi
    
    if systemctl is-active --quiet node-agent; then
        log_info "✓ Node Agent 服务运行中"
    else
        log_error "✗ Node Agent 服务未运行"
        ((errors++))
    fi
    
    return $errors
}

show_deployment_info() {
    echo ""
    echo "=========================================="
    echo "  节点部署完成！"
    echo "=========================================="
    echo ""
    echo "节点信息:"
    echo "  节点 ID:   $NODE_ID"
    echo "  节点名称:  $NODE_NAME"
    echo "  节点地址:  $NODE_HOST:$NODE_PORT"
    echo "  协议类型:  $NODE_PROTOCOL"
    echo ""
    echo "常用命令:"
    echo "  查看状态: $0 status"
    echo "  查看日志: journalctl -u node-agent -f"
    echo "  重启服务: systemctl restart node-agent"
    echo ""
}

################################################################################
# 主函数
################################################################################

main() {
    local command="${1:-help}"
    shift || true
    
    # 解析选项
    while [[ $# -gt 0 ]]; do
        case $1 in
            --api-url)
                API_URL="$2"
                shift 2
                ;;
            --admin-token)
                ADMIN_TOKEN="$2"
                shift 2
                ;;
            --node-name)
                NODE_NAME="$2"
                shift 2
                ;;
            --node-host)
                NODE_HOST="$2"
                shift 2
                ;;
            --node-port)
                NODE_PORT="$2"
                shift 2
                ;;
            --node-protocol)
                NODE_PROTOCOL="$2"
                shift 2
                ;;
            --node-config)
                NODE_CONFIG="$2"
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
            *)
                break
                ;;
        esac
    done
    
    # 执行命令
    case $command in
        deploy)
            print_banner
            cmd_deploy
            ;;
        uninstall)
            print_banner
            cmd_uninstall
            ;;
        update)
            cmd_update
            ;;
        status)
            cmd_status
            ;;
        version)
            cmd_version
            ;;
        help|--help|-h)
            print_usage
            ;;
        *)
            log_error "未知命令: $command"
            print_usage
            exit 1
            ;;
    esac
}

# 执行主函数
main "$@"
