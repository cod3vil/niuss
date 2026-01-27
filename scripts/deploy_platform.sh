#!/usr/bin/env bash

################################################################################
# VPN 订阅平台一键部署脚本
# 
# 功能：
# - 自动检测系统环境
# - 安装 Docker 和 Docker Compose
# - 配置环境变量
# - 部署完整的管理平台（API、前端、管理后台、数据库、Redis）
# - 配置 SSL 证书（可选）
# - 设置防火墙规则
#
# 使用方法：
#   sudo bash deploy_platform.sh
#   sudo bash deploy_platform.sh --domain yourdomain.com --email your@email.com
################################################################################

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志文件
LOG_FILE="/var/log/vpn-platform-deployment.log"

# 配置变量
DOMAIN=""
EMAIL=""
ENABLE_SSL=false
SKIP_DOCKER_INSTALL=false
SKIP_FIREWALL=false

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
    echo "  VPN 订阅平台一键部署脚本"
    echo "=========================================="
    echo ""
}

print_usage() {
    cat <<EOF
使用方法: $0 [选项]

选项:
  --domain DOMAIN          设置域名（用于 SSL 证书）
  --email EMAIL            设置邮箱（用于 Let's Encrypt）
  --enable-ssl             启用 SSL 证书自动配置
  --skip-docker            跳过 Docker 安装（如果已安装）
  --skip-firewall          跳过防火墙配置
  -h, --help               显示此帮助信息

示例:
  # 基础部署（不配置 SSL）
  sudo $0

  # 完整部署（包含 SSL）
  sudo $0 --domain yourdomain.com --email your@email.com --enable-ssl

  # 跳过 Docker 安装
  sudo $0 --skip-docker
EOF
}

################################################################################
# 参数解析
################################################################################

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --domain)
                DOMAIN="$2"
                shift 2
                ;;
            --email)
                EMAIL="$2"
                shift 2
                ;;
            --enable-ssl)
                ENABLE_SSL=true
                shift
                ;;
            --skip-docker)
                SKIP_DOCKER_INSTALL=true
                shift
                ;;
            --skip-firewall)
                SKIP_FIREWALL=true
                shift
                ;;
            -h|--help)
                print_usage
                exit 0
                ;;
            *)
                log_error "未知参数: $1"
                print_usage
                exit 1
                ;;
        esac
    done

    # 验证 SSL 配置
    if [ "$ENABLE_SSL" = true ]; then
        if [ -z "$DOMAIN" ] || [ -z "$EMAIL" ]; then
            log_error "启用 SSL 需要提供 --domain 和 --email 参数"
            exit 1
        fi
    fi
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
    log_info "Root 权限检查通过"
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

    # 检查是否为支持的系统
    case $OS in
        ubuntu|debian|centos|rhel|rocky|almalinux)
            log_info "操作系统支持: $OS"
            ;;
        *)
            log_warn "未测试的操作系统: $OS，可能会遇到问题"
            ;;
    esac
}

check_system_resources() {
    log_info "检查系统资源..."

    # 检查内存
    local total_mem=$(free -m | awk '/^Mem:/{print $2}')
    if [ "$total_mem" -lt 2048 ]; then
        log_warn "系统内存不足 2GB (当前: ${total_mem}MB)，建议至少 4GB"
    else
        log_info "内存检查通过: ${total_mem}MB"
    fi

    # 检查磁盘空间
    local free_space=$(df -BG / | awk 'NR==2 {print $4}' | sed 's/G//')
    if [ "$free_space" -lt 10 ]; then
        log_warn "磁盘空间不足 10GB (当前: ${free_space}GB)，建议至少 20GB"
    else
        log_info "磁盘空间检查通过: ${free_space}GB"
    fi

    # 检查 CPU 核心数
    local cpu_cores=$(nproc)
    if [ "$cpu_cores" -lt 2 ]; then
        log_warn "CPU 核心数不足 2 (当前: ${cpu_cores})，建议至少 2 核"
    else
        log_info "CPU 核心数检查通过: ${cpu_cores} 核"
    fi
}

################################################################################
# Docker 安装
################################################################################

install_docker() {
    if [ "$SKIP_DOCKER_INSTALL" = true ]; then
        log_info "跳过 Docker 安装"
        return
    fi

    log_info "检查 Docker 安装状态..."

    if command -v docker &> /dev/null; then
        local docker_version=$(docker --version | awk '{print $3}' | sed 's/,//')
        log_info "Docker 已安装: $docker_version"
    else
        log_info "开始安装 Docker..."

        case $OS in
            ubuntu|debian)
                # 更新包索引
                apt-get update
                apt-get install -y ca-certificates curl gnupg lsb-release

                # 添加 Docker 官方 GPG 密钥
                mkdir -p /etc/apt/keyrings
                curl -fsSL https://download.docker.com/linux/$OS/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg

                # 设置仓库
                echo \
                  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$OS \
                  $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

                # 安装 Docker
                apt-get update
                apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
                ;;

            centos|rhel|rocky|almalinux)
                # 安装依赖
                yum install -y yum-utils
                yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo

                # 安装 Docker
                yum install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
                ;;

            *)
                log_error "不支持的操作系统: $OS"
                exit 1
                ;;
        esac

        # 启动 Docker
        systemctl start docker
        systemctl enable docker

        log_info "Docker 安装完成"
    fi

    # 检查 Docker Compose
    if docker compose version &> /dev/null; then
        local compose_version=$(docker compose version | awk '{print $4}')
        log_info "Docker Compose 已安装: $compose_version"
    else
        log_error "Docker Compose 未安装"
        exit 1
    fi
}

################################################################################
# 项目配置
################################################################################

setup_project() {
    log_info "配置项目..."

    # 检查是否在项目目录中
    if [ ! -f "docker-compose.yml" ]; then
        log_error "未找到 docker-compose.yml 文件"
        log_error "请在项目根目录运行此脚本"
        exit 1
    fi

    # 生成环境变量文件
    if [ ! -f ".env" ]; then
        log_info "创建 .env 文件..."

        # 生成随机密钥
        local db_password=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)
        local jwt_secret=$(openssl rand -base64 48 | tr -d '/+=' | head -c 64)

        cat > .env <<EOF
# 数据库配置
DB_PASSWORD=${db_password}

# JWT 配置
JWT_SECRET=${jwt_secret}
JWT_EXPIRATION=86400

# API 配置
API_HOST=0.0.0.0
API_PORT=8080

# CORS 配置
CORS_ORIGINS=http://localhost,http://localhost:80,http://localhost:8081

# 日志级别
RUST_LOG=info

# 前端 API URL
VITE_API_URL=http://localhost:8080
EOF

        log_info ".env 文件创建完成"
        log_info "数据库密码: ${db_password:0:8}..."
        log_info "JWT 密钥: ${jwt_secret:0:8}..."
    else
        log_info ".env 文件已存在，跳过创建"
    fi

    # 如果提供了域名，更新 CORS 配置
    if [ -n "$DOMAIN" ]; then
        log_info "更新域名配置: $DOMAIN"
        sed -i "s|CORS_ORIGINS=.*|CORS_ORIGINS=https://${DOMAIN},https://admin.${DOMAIN}|" .env
        sed -i "s|VITE_API_URL=.*|VITE_API_URL=https://${DOMAIN}|" .env
    fi
}

################################################################################
# 部署服务
################################################################################

deploy_services() {
    log_info "开始部署服务..."

    # 跳过拉取镜像（使用本地已有镜像）
    log_info "使用本地镜像，跳过拉取步骤"

    # 构建镜像
    log_info "构建应用镜像..."
    docker compose build

    # 启动服务
    log_info "启动服务..."
    docker compose up -d

    # 等待服务启动
    log_info "等待服务启动..."
    sleep 10

    # 等待数据库就绪
    log_info "等待数据库就绪..."
    local db_ready=false
    for i in {1..30}; do
        if docker compose exec -T postgres pg_isready -U vpn_user > /dev/null 2>&1; then
            log_info "数据库已就绪"
            db_ready=true
            break
        fi
        log_info "等待数据库启动... ($i/30)"
        sleep 2
    done

    if [ "$db_ready" = false ]; then
        log_error "数据库启动超时"
        exit 1
    fi

    # 运行数据库迁移
    log_info "运行数据库迁移..."
    
    # 检查数据库是否已初始化
    local tables_exist=$(docker compose exec -T postgres psql -U vpn_user -d vpn_platform -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name='users';" 2>/dev/null || echo "0")
    
    if [ "$tables_exist" = "0" ]; then
        log_info "初始化数据库..."
        
        # 运行初始化脚本
        if [ -f "migrations/001_init.sql" ]; then
            docker compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/001_init.sql
            if [ $? -eq 0 ]; then
                log_info "✓ 数据库初始化完成"
            else
                log_error "✗ 数据库初始化失败"
                exit 1
            fi
        else
            log_error "找不到数据库初始化脚本: migrations/001_init.sql"
            exit 1
        fi
        
        # 可选：运行测试数据脚本（仅开发环境）
        if [ -f "migrations/002_seed_test_data.sql" ] && [ "${ENVIRONMENT:-production}" = "development" ]; then
            log_info "加载测试数据..."
            docker compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/002_seed_test_data.sql
        fi
    else
        log_info "数据库已存在，跳过初始化"
    fi

    # 检查服务状态
    log_info "检查服务状态..."
    docker compose ps

    # 验证服务健康
    local max_attempts=30
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
            log_info "API 服务健康检查通过"
            break
        fi
        attempt=$((attempt + 1))
        log_info "等待 API 服务启动... ($attempt/$max_attempts)"
        sleep 2
    done

    if [ $attempt -eq $max_attempts ]; then
        log_error "API 服务启动超时"
        log_error "请检查日志: docker compose logs api"
        exit 1
    fi

    log_info "所有服务部署完成"
}

################################################################################
# 防火墙配置
################################################################################

configure_firewall() {
    if [ "$SKIP_FIREWALL" = true ]; then
        log_info "跳过防火墙配置"
        return
    fi

    log_info "配置防火墙..."

    if command -v ufw &> /dev/null; then
        # Ubuntu/Debian 使用 ufw
        log_info "使用 ufw 配置防火墙..."
        ufw allow 22/tcp comment 'SSH'
        ufw allow 80/tcp comment 'HTTP'
        ufw allow 443/tcp comment 'HTTPS'
        ufw --force enable
        log_info "ufw 防火墙配置完成"
    elif command -v firewall-cmd &> /dev/null; then
        # CentOS/RHEL 使用 firewalld
        log_info "使用 firewalld 配置防火墙..."
        firewall-cmd --permanent --add-service=ssh
        firewall-cmd --permanent --add-service=http
        firewall-cmd --permanent --add-service=https
        firewall-cmd --reload
        log_info "firewalld 防火墙配置完成"
    else
        log_warn "未检测到防火墙工具，跳过防火墙配置"
    fi
}

################################################################################
# SSL 证书配置
################################################################################

configure_ssl() {
    if [ "$ENABLE_SSL" != true ]; then
        log_info "跳过 SSL 配置"
        return
    fi

    log_info "配置 SSL 证书..."

    # 安装 Nginx
    case $OS in
        ubuntu|debian)
            apt-get update
            apt-get install -y nginx certbot python3-certbot-nginx
            ;;
        centos|rhel|rocky|almalinux)
            yum install -y nginx certbot python3-certbot-nginx
            ;;
    esac

    # 创建 Nginx 配置
    log_info "创建 Nginx 配置..."

    # 用户前端配置
    cat > /etc/nginx/sites-available/vpn-frontend <<EOF
server {
    listen 80;
    server_name ${DOMAIN};

    location / {
        proxy_pass http://localhost:80;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF

    # 管理后台配置
    cat > /etc/nginx/sites-available/vpn-admin <<EOF
server {
    listen 80;
    server_name admin.${DOMAIN};

    location / {
        proxy_pass http://localhost:8081;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF

    # 启用配置
    if [ -d /etc/nginx/sites-enabled ]; then
        ln -sf /etc/nginx/sites-available/vpn-frontend /etc/nginx/sites-enabled/
        ln -sf /etc/nginx/sites-available/vpn-admin /etc/nginx/sites-enabled/
    fi

    # 测试 Nginx 配置
    nginx -t

    # 重启 Nginx
    systemctl restart nginx
    systemctl enable nginx

    log_info "Nginx 配置完成"

    # 获取 SSL 证书
    log_info "获取 SSL 证书..."
    certbot --nginx -d ${DOMAIN} -d admin.${DOMAIN} --non-interactive --agree-tos --email ${EMAIL}

    # 设置自动续期
    log_info "配置证书自动续期..."
    (crontab -l 2>/dev/null; echo "0 0 * * * certbot renew --quiet") | crontab -

    log_info "SSL 证书配置完成"
}

################################################################################
# 部署验证
################################################################################

verify_deployment() {
    log_info "验证部署..."

    local errors=0

    # 检查 Docker 容器状态
    log_info "检查容器状态..."
    local containers=("vpn-postgres" "vpn-redis" "vpn-api" "vpn-frontend" "vpn-admin")
    for container in "${containers[@]}"; do
        if docker ps | grep -q "$container"; then
            log_info "✓ $container 运行中"
        else
            log_error "✗ $container 未运行"
            ((errors++))
        fi
    done

    # 检查端口监听
    log_info "检查端口监听..."
    local ports=("50080" "50081" "50082" "55432" "56379")
    for port in "${ports[@]}"; do
        if netstat -tuln 2>/dev/null | grep -q ":$port " || ss -tuln 2>/dev/null | grep -q ":$port "; then
            log_info "✓ 端口 $port 正在监听"
        else
            log_warn "✗ 端口 $port 未监听"
        fi
    done

    # 检查 API 健康
    log_info "检查 API 健康..."
    if curl -sf http://localhost:50082/health > /dev/null 2>&1; then
        log_info "✓ API 健康检查通过"
    else
        log_error "✗ API 健康检查失败"
        ((errors++))
    fi

    # 检查前端
    log_info "检查前端..."
    if curl -sf http://localhost:50080/ > /dev/null 2>&1; then
        log_info "✓ 前端访问正常"
    else
        log_error "✗ 前端访问失败"
        ((errors++))
    fi

    # 检查管理后台
    log_info "检查管理后台..."
    if curl -sf http://localhost:50081/ > /dev/null 2>&1; then
        log_info "✓ 管理后台访问正常"
    else
        log_error "✗ 管理后台访问失败"
        ((errors++))
    fi

    if [ $errors -eq 0 ]; then
        log_info "部署验证通过"
        return 0
    else
        log_error "部署验证失败，发现 $errors 个错误"
        return 1
    fi
}

################################################################################
# 显示部署信息
################################################################################

show_deployment_info() {
    echo ""
    echo "=========================================="
    echo "  部署完成！"
    echo "=========================================="
    echo ""
    echo "访问地址:"
    if [ -n "$DOMAIN" ] && [ "$ENABLE_SSL" = true ]; then
        echo "  用户前端: https://${DOMAIN}"
        echo "  管理后台: https://admin.${DOMAIN}"
        echo "  API 服务: https://${DOMAIN}/api"
    else
        echo "  用户前端: http://$(hostname -I | awk '{print $1}'):50080"
        echo "  管理后台: http://$(hostname -I | awk '{print $1}'):50081"
        echo "  API 服务: http://$(hostname -I | awk '{print $1}'):50082"
    fi
    echo ""
    echo "默认管理员账号:"
    echo "  邮箱: admin@example.com"
    echo "  密码: admin123"
    echo "  ⚠️  请立即登录并修改密码！"
    echo ""
    echo "常用命令:"
    echo "  查看服务状态: docker compose ps"
    echo "  查看日志:     docker compose logs -f"
    echo "  停止服务:     docker compose down"
    echo "  重启服务:     docker compose restart"
    echo ""
    echo "配置文件位置:"
    echo "  环境变量: $(pwd)/.env"
    echo "  日志文件: $LOG_FILE"
    echo ""
    echo "下一步:"
    echo "  1. 访问管理后台并修改默认密码"
    echo "  2. 在管理后台创建节点"
    echo "  3. 使用节点部署脚本部署节点服务器"
    echo ""
    echo "节点部署命令:"
    echo "  curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/scripts/deploy_node.sh | bash"
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
    check_system_resources

    # 安装 Docker
    install_docker

    # 配置项目
    setup_project

    # 部署服务
    deploy_services

    # 配置防火墙
    configure_firewall

    # 配置 SSL
    configure_ssl

    # 验证部署
    if verify_deployment; then
        show_deployment_info
        exit 0
    else
        log_error "部署验证失败，请检查日志"
        exit 1
    fi
}

# 执行主函数
main "$@"
