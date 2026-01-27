#!/usr/bin/env bash

################################################################################
# VPN 节点快速部署脚本
# 
# 功能：
# - 自动创建节点（通过 API）
# - 安装 Xray-core
# - 安装和配置 Node Agent
# - 启动服务并验证
#
# 使用方法：
#   # 交互式部署
#   sudo bash quick_deploy_node.sh
#
#   # 命令行参数部署
#   sudo bash quick_deploy_node.sh \
#     --api-url https://api.yourdomain.com \
#     --admin-token your-jwt-token \
#     --node-name node-hk-01
#
#   # 批量部署
#   sudo bash quick_deploy_node.sh --batch-config nodes.yaml
################################################################################

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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
BATCH_CONFIG=""

# 选项
VERBOSE=false
FORCE=false

################################################################################
# 工具函数
################################################################################

log_info() {
    local msg="$1"
    echo -e "${GREEN}[INFO]${NC} $msg"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INFO] $msg" >> "$LOG_FILE"
}

log_warn() {
    local msg="$1"
    echo -e "${YELLOW}[WARN]${NC} $msg"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [WARN] $msg" >> "$LOG_FILE"
}

log_error() {
    local msg="$1"
    echo -e "${RED}[ERROR]${NC} $msg" >&2
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR] $msg" >> "$LOG_FILE"
}

print_banner() {
    echo ""
    echo "=========================================="
    echo "  VPN 节点快速部署脚本"
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

################################################################################
# 参数解析
################################################################################

parse_arguments() {
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
            --batch-config)
                BATCH_CONFIG="$2"
                shift 2
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --force)
                FORCE=true
                shift
                ;;
            -h|--help)
                print_usage
                exit 0
                ;;
            *)
                log_error "未知参数: $1"
                exit 1
                ;;
        esac
    done
}

print_usage() {
    cat <<EOF
使用方法: $0 [选项]

选项:
  --api-url URL              API 服务地址（必需）
  --admin-token TOKEN        管理员 JWT 令牌（必需）
  --node-name NAME           节点名称（必需）
  --node-host HOST           节点主机地址（可选，默认自动检测）
  --node-port PORT           节点端口（可选，默认 443）
  --node-protocol PROTOCOL   协议类型（可选，默认 vless）
  --node-config JSON         协议配置（可选，默认 {}）
  --batch-config FILE        批量部署配置文件
  --verbose                  详细输出
  --force                    强制重新部署
  -h, --help                 显示此帮助信息

支持的协议:
  vless, vmess, trojan, shadowsocks, hysteria2

示例:
  # 单节点部署
  sudo $0 \\
    --api-url https://api.yourdomain.com \\
    --admin-token eyJhbGci... \\
    --node-name node-hk-01

  # 指定协议和端口
  sudo $0 \\
    --api-url https://api.yourdomain.com \\
    --admin-token eyJhbGci... \\
    --node-name node-us-01 \\
    --node-protocol vmess \\
    --node-port 8443

  # 批量部署
  sudo $0 --batch-config nodes.yaml
EOF
}

################################################################################
# 交互式输入
################################################################################

interactive_input() {
    log_info "进入交互式配置模式..."
    echo ""

    # API URL
    if [ -z "$API_URL" ]; then
        read -p "请输入 API 服务地址 (例如: https://api.yourdomain.com): " API_URL
    fi

    # Admin Token
    if [ -z "$ADMIN_TOKEN" ]; then
        read -p "请输入管理员 JWT 令牌: " ADMIN_TOKEN
    fi

    # Node Name
    if [ -z "$NODE_NAME" ]; then
        read -p "请输入节点名称 (例如: node-hk-01): " NODE_NAME
    fi

    # Node Host (optional)
    if [ -z "$NODE_HOST" ]; then
        read -p "请输入节点主机地址 (留空自动检测): " NODE_HOST
    fi

    # Node Port
    read -p "请输入节点端口 [443]: " input_port
    if [ -n "$input_port" ]; then
        NODE_PORT="$input_port"
    fi

    # Node Protocol
    echo ""
    echo "支持的协议:"
    echo "  1) vless (默认)"
    echo "  2) vmess"
    echo "  3) trojan"
    echo "  4) shadowsocks"
    echo "  5) hysteria2"
    read -p "请选择协议 [1]: " protocol_choice

    case $protocol_choice in
        2) NODE_PROTOCOL="vmess" ;;
        3) NODE_PROTOCOL="trojan" ;;
        4) NODE_PROTOCOL="shadowsocks" ;;
        5) NODE_PROTOCOL="hysteria2" ;;
        *) NODE_PROTOCOL="vless" ;;
    esac

    echo ""
    log_info "配置完成"
}

################################################################################
# 环境检测
################################################################################

check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "此脚本需要 root 权限运行"
        log_error "请使用: sudo $0"
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

    local ip=""
    local services=(
        "https://api.ipify.org"
        "https://icanhazip.com"
        "https://ifconfig.me"
    )

    for service in "${services[@]}"; do
        ip=$(curl -s --max-time 5 "$service" 2>/dev/null || true)
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
# 密钥生成
################################################################################

generate_node_secret() {
    log_info "生成节点密钥..."
    local secret=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)
    log_info "节点密钥: $(mask_sensitive "$secret")"
    echo "$secret"
}

################################################################################
# API 调用
################################################################################

create_node_via_api() {
    log_info "通过 API 创建节点..."

    # 如果未指定主机地址，自动检测
    if [ -z "$NODE_HOST" ]; then
        detect_public_ip
    fi

    # 生成节点密钥
    NODE_SECRET=$(generate_node_secret)

    # 构建请求体
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

    log_info "发送创建节点请求..."
    log_info "节点名称: $NODE_NAME"
    log_info "节点地址: $NODE_HOST:$NODE_PORT"
    log_info "协议类型: $NODE_PROTOCOL"

    # 发送 API 请求
    local response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "$request_body" \
        "$API_URL/api/admin/nodes" 2>&1)

    local http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | sed '$d')

    log_info "HTTP 状态码: $http_code"

    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        NODE_ID=$(echo "$body" | jq -r '.id // .node_id // empty')
        
        if [ -z "$NODE_ID" ]; then
            log_error "无法从响应中提取节点 ID"
            log_error "响应: $body"
            exit 1
        fi

        log_info "节点创建成功"
        log_info "节点 ID: $NODE_ID"
        log_info "节点密钥: $(mask_sensitive "$NODE_SECRET")"
        return 0
    else
        log_error "节点创建失败"
        log_error "HTTP 状态码: $http_code"
        log_error "响应: $body"

        case $http_code in
            401|403)
                log_error "认证失败，请检查 admin-token 是否正确"
                ;;
            400)
                log_error "请求参数错误，请检查输入"
                ;;
            409)
                log_error "节点已存在，使用 --force 强制重新部署"
                ;;
            500|502|503|504)
                log_error "服务器错误，请稍后重试"
                ;;
        esac

        exit 1
    fi
}

################################################################################
# Xray-core 安装
################################################################################

install_xray() {
    log_info "安装 Xray-core..."

    if command -v xray &> /dev/null && [ "$FORCE" != true ]; then
        log_info "Xray-core 已安装，跳过"
        return 0
    fi

    # 使用官方安装脚本
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

    # 根据协议生成配置
    case $NODE_PROTOCOL in
        vless)
            cat > "$config_file" <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": $NODE_PORT,
      "protocol": "vless",
      "settings": {
        "clients": [],
        "decryption": "none"
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
            ;;
        vmess)
            cat > "$config_file" <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": $NODE_PORT,
      "protocol": "vmess",
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
            ;;
        *)
            log_warn "协议 $NODE_PROTOCOL 使用默认配置"
            ;;
    esac

    log_info "Xray 配置生成完成: $config_file"
}

################################################################################
# Node Agent 安装
################################################################################

install_node_agent() {
    log_info "安装 Node Agent..."

    # 检测系统架构
    local arch=$(uname -m)
    case $arch in
        x86_64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)
            log_error "不支持的架构: $arch"
            exit 1
            ;;
    esac

    log_info "系统架构: $arch"

    # 下载 Node Agent
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
            log_error "请检查网络连接或手动下载"
            exit 1
        fi
    fi

    # 创建配置目录
    mkdir -p /etc/node-agent

    # 创建配置文件
    log_info "创建配置文件..."
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
    log_info "配置文件创建完成: /etc/node-agent/config.env"

    # 创建 systemd 服务
    log_info "创建 systemd 服务..."
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

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/etc/xray /var/log

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    log_info "systemd 服务创建完成"
}

################################################################################
# 服务启动
################################################################################

start_services() {
    log_info "启动服务..."

    # 启动 Xray
    systemctl enable xray
    systemctl restart xray

    # 启动 Node Agent
    systemctl enable node-agent
    systemctl restart node-agent

    # 等待服务启动
    sleep 3

    log_info "服务启动完成"
}

verify_deployment() {
    log_info "验证部署..."

    local errors=0

    # 检查 Xray 服务
    if systemctl is-active --quiet xray; then
        log_info "✓ Xray 服务运行中"
    else
        log_error "✗ Xray 服务未运行"
        ((errors++))
    fi

    # 检查 Node Agent 服务
    if systemctl is-active --quiet node-agent; then
        log_info "✓ Node Agent 服务运行中"
    else
        log_error "✗ Node Agent 服务未运行"
        ((errors++))
    fi

    # 检查端口监听
    if netstat -tuln 2>/dev/null | grep -q ":$NODE_PORT " || ss -tuln 2>/dev/null | grep -q ":$NODE_PORT "; then
        log_info "✓ 端口 $NODE_PORT 正在监听"
    else
        log_warn "✗ 端口 $NODE_PORT 未监听（可能需要等待）"
    fi

    if [ $errors -eq 0 ]; then
        log_info "部署验证通过"
        return 0
    else
        log_error "部署验证失败，发现 $errors 个错误"
        log_error "请检查日志:"
        log_error "  Xray: journalctl -u xray -n 50"
        log_error "  Node Agent: journalctl -u node-agent -n 50"
        return 1
    fi
}

################################################################################
# 批量部署
################################################################################

batch_deploy() {
    log_info "开始批量部署..."

    if [ ! -f "$BATCH_CONFIG" ]; then
        log_error "批量配置文件不存在: $BATCH_CONFIG"
        exit 1
    fi

    # 检查配置文件格式
    if ! command -v yq &> /dev/null; then
        log_error "批量部署需要 yq 工具"
        log_error "安装: sudo apt-get install yq 或 sudo yum install yq"
        exit 1
    fi

    # 解析配置
    local api_url=$(yq eval '.api_url' "$BATCH_CONFIG")
    local admin_token=$(yq eval '.admin_token' "$BATCH_CONFIG")
    local node_count=$(yq eval '.nodes | length' "$BATCH_CONFIG")

    log_info "API URL: $api_url"
    log_info "节点数量: $node_count"

    # 部署每个节点
    for i in $(seq 0 $((node_count - 1))); do
        local node_name=$(yq eval ".nodes[$i].name" "$BATCH_CONFIG")
        local node_host=$(yq eval ".nodes[$i].host" "$BATCH_CONFIG")
        local node_port=$(yq eval ".nodes[$i].port" "$BATCH_CONFIG")
        local node_protocol=$(yq eval ".nodes[$i].protocol" "$BATCH_CONFIG")

        log_info "部署节点 $((i + 1))/$node_count: $node_name"

        # 设置变量
        API_URL="$api_url"
        ADMIN_TOKEN="$admin_token"
        NODE_NAME="$node_name"
        NODE_HOST="$node_host"
        NODE_PORT="$node_port"
        NODE_PROTOCOL="$node_protocol"

        # 执行部署
        if deploy_single_node; then
            log_info "✓ $node_name 部署成功"
        else
            log_error "✗ $node_name 部署失败"
        fi

        echo ""
    done

    log_info "批量部署完成"
}

deploy_single_node() {
    create_node_via_api
    install_xray
    generate_xray_config
    install_node_agent
    start_services
    verify_deployment
}

################################################################################
# 显示部署信息
################################################################################

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
    echo "  节点密钥:  $(mask_sensitive "$NODE_SECRET")"
    echo ""
    echo "服务状态:"
    echo "  Xray:       $(systemctl is-active xray)"
    echo "  Node Agent: $(systemctl is-active node-agent)"
    echo ""
    echo "常用命令:"
    echo "  查看 Xray 状态:       systemctl status xray"
    echo "  查看 Node Agent 状态: systemctl status node-agent"
    echo "  查看 Xray 日志:       journalctl -u xray -f"
    echo "  查看 Node Agent 日志: journalctl -u node-agent -f"
    echo "  重启服务:             systemctl restart node-agent"
    echo ""
    echo "配置文件:"
    echo "  Xray 配置:       /usr/local/etc/xray/config.json"
    echo "  Node Agent 配置: /etc/node-agent/config.env"
    echo ""
    echo "=========================================="
}

################################################################################
# 主函数
################################################################################

main() {
    print_banner

    # 解析参数
    parse_arguments "$@"

    # 环境检查
    check_root
    detect_os
    check_dependencies

    # 批量部署模式
    if [ -n "$BATCH_CONFIG" ]; then
        batch_deploy
        exit 0
    fi

    # 交互式输入（如果参数不完整）
    if [ -z "$API_URL" ] || [ -z "$ADMIN_TOKEN" ] || [ -z "$NODE_NAME" ]; then
        interactive_input
    fi

    # 验证必需参数
    if [ -z "$API_URL" ] || [ -z "$ADMIN_TOKEN" ] || [ -z "$NODE_NAME" ]; then
        log_error "缺少必需参数"
        print_usage
        exit 1
    fi

    # 执行部署
    if deploy_single_node; then
        show_deployment_info
        exit 0
    else
        log_error "部署失败"
        exit 1
    fi
}

# 执行主函数
main "$@"
