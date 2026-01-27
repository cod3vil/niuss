# 设计文档：节点一键部署功能

## 概述

节点一键部署功能通过自动化脚本 `deploy_node.sh` 实现从节点创建到服务启动的全流程自动化。该脚本将替代手动在管理后台创建节点的流程，通过调用 API 自动创建节点记录，生成安全的随机密钥，并完成 Xray-core 和 Node Agent 的安装配置。

设计目标：
- 简化节点部署流程，减少人工操作
- 提高部署效率和一致性
- 支持单节点和批量部署场景
- 提供完善的错误处理和回滚机制
- 确保部署过程的安全性和可靠性

## 架构

### 系统组件

```
┌─────────────────────────────────────────────────────────────┐
│                    Deployment Script                         │
│  (deploy_node.sh)                                           │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Parameter  │  │  Environment │  │   Batch      │     │
│  │   Parser     │  │   Detector   │  │   Config     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Secret     │  │  API Client  │  │  Installer   │     │
│  │  Generator   │  │              │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Service    │  │  Validator   │  │   Logger     │     │
│  │   Manager    │  │              │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ HTTPS
                            ▼
                ┌───────────────────────┐
                │   API Service         │
                │  /api/admin/nodes     │
                └───────────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │   PostgreSQL          │
                │   (Node Records)      │
                └───────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    Target Node Server                        │
│                                                              │
│  ┌──────────────┐         ┌──────────────┐                 │
│  │  Xray-core   │◄────────┤ Node Agent   │                 │
│  │  (Proxy)     │         │              │                 │
│  └──────────────┘         └──────────────┘                 │
│                                   │                          │
│                                   │ HTTPS                    │
│                                   ▼                          │
│                          ┌──────────────┐                   │
│                          │ API Service  │                   │
│                          └──────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

### 部署流程

```mermaid
sequenceDiagram
    participant User as 运维人员
    participant Script as deploy_node.sh
    participant API as API Service
    participant Server as 节点服务器
    
    User->>Script: 执行脚本（参数/配置文件）
    Script->>Script: 解析参数和验证环境
    Script->>Script: 生成随机 Node_Secret
    Script->>API: POST /api/admin/nodes
    API-->>Script: 返回 Node_ID 和 Secret
    Script->>Server: 安装 Xray-core
    Script->>Server: 安装 Node Agent
    Script->>Server: 创建配置文件
    Script->>Server: 启动服务
    Script->>Server: 验证服务状态
    Script->>API: 验证节点连接
    Script-->>User: 显示部署结果
```

## 组件和接口

### 1. 参数解析器 (Parameter Parser)

**职责：** 解析命令行参数和环境变量，构建部署配置。

**接口：**
```bash
# 命令行参数
--api-url <URL>           # API 服务地址（必需）
--admin-token <TOKEN>     # 管理员 JWT 令牌（必需）
--node-name <NAME>        # 节点名称（必需）
--node-host <HOST>        # 节点主机地址（可选，默认自动检测）
--node-port <PORT>        # 节点端口（可选，默认 443）
--node-protocol <PROTO>   # 协议类型（可选，默认 vless）
--node-config <JSON>      # 协议配置（可选）
--batch-config <FILE>     # 批量部署配置文件（可选）
--force                   # 强制重新部署（可选）
--verbose                 # 详细输出（可选）
--quiet                   # 静默模式（可选）
--rollback                # 回滚到上一版本（可选）

# 环境变量（作为备选）
API_URL
ADMIN_TOKEN
NODE_NAME
NODE_HOST
NODE_PORT
NODE_PROTOCOL
NODE_CONFIG
```

**实现细节：**
- 优先使用命令行参数，其次使用环境变量
- 验证必需参数是否提供
- 验证参数格式（URL、端口范围、协议类型）
- 支持从配置文件读取批量部署配置

### 2. 环境检测器 (Environment Detector)

**职责：** 检测系统环境，验证部署条件。

**检测项：**
- 操作系统类型和版本（Ubuntu/CentOS/Debian）
- Root 权限验证
- 必需命令可用性（curl、jq、systemctl、openssl）
- 网络连接性（能否访问 API 和下载源）
- 磁盘空间（至少 500MB 可用空间）
- 端口占用情况（检查目标端口是否已被占用）

**自动检测公网 IP：**
```bash
# 方法 1：使用 ipify API
curl -s https://api.ipify.org

# 方法 2：使用 icanhazip
curl -s https://icanhazip.com

# 方法 3：使用 ifconfig.me
curl -s https://ifconfig.me

# 备选：从网络接口获取
ip addr show | grep 'inet ' | grep -v '127.0.0.1' | awk '{print $2}' | cut -d/ -f1
```

### 3. 密钥生成器 (Secret Generator)

**职责：** 生成加密安全的随机节点密钥。

**实现：**
```bash
generate_node_secret() {
    # 使用 openssl 生成 32 字节随机数据，转换为 base64
    openssl rand -base64 32 | tr -d '/+=' | head -c 32
}
```

**安全要求：**
- 使用加密安全的随机源（/dev/urandom）
- 密钥长度至少 32 字符
- 仅包含字母数字字符（避免特殊字符导致的转义问题）
- 在日志中隐藏完整密钥，仅显示前 8 个字符

### 4. API 客户端 (API Client)

**职责：** 与 API 服务通信，创建和管理节点记录。

**节点创建接口：**
```bash
create_node() {
    local api_url="$1"
    local admin_token="$2"
    local node_name="$3"
    local node_host="$4"
    local node_port="$5"
    local node_protocol="$6"
    local node_config="$7"
    
    # 构建 JSON 请求体
    local request_body=$(cat <<EOF
{
    "name": "$node_name",
    "host": "$node_host",
    "port": $node_port,
    "protocol": "$node_protocol",
    "config": $node_config
}
EOF
)
    
    # 发送 POST 请求
    local response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $admin_token" \
        -d "$request_body" \
        "$api_url/api/admin/nodes")
    
    # 解析响应
    local http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" -eq 200 ] || [ "$http_code" -eq 201 ]; then
        # 提取 node_id 和 secret
        local node_id=$(echo "$body" | jq -r '.id')
        local node_secret=$(echo "$body" | jq -r '.secret')
        echo "$node_id|$node_secret"
        return 0
    else
        echo "API Error: HTTP $http_code - $body" >&2
        return 1
    fi
}
```

**错误处理：**
- HTTP 401/403：认证失败，提示检查 Admin_Token
- HTTP 400：请求参数错误，显示详细错误信息
- HTTP 409：节点已存在，询问是否更新
- HTTP 500：服务器错误，建议稍后重试
- 网络错误：检查网络连接和 API_URL

### 5. 安装器 (Installer)

**职责：** 安装和配置 Xray-core 和 Node Agent。

**Xray-core 安装：**
```bash
install_xray() {
    log_info "Installing Xray-core..."
    
    # 使用官方安装脚本
    bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
    
    # 验证安装
    if ! command -v xray &> /dev/null; then
        log_error "Xray-core installation failed"
        return 1
    fi
    
    # 启用服务（但不启动，由 Node Agent 管理）
    systemctl enable xray
    
    log_info "Xray-core installed successfully"
}
```

**Node Agent 安装：**
```bash
install_node_agent() {
    log_info "Installing Node Agent..."
    
    # 检测架构
    local arch=$(uname -m)
    case $arch in
        x86_64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) log_error "Unsupported architecture: $arch"; return 1 ;;
    esac
    
    # 下载二进制文件
    local download_url="https://github.com/your-org/vpn-platform/releases/latest/download/node-agent-${arch}"
    
    if wget -O /usr/local/bin/node-agent "$download_url"; then
        chmod +x /usr/local/bin/node-agent
        log_info "Node Agent downloaded successfully"
    else
        log_error "Failed to download Node Agent"
        return 1
    fi
}
```

**配置文件生成：**
```bash
create_node_config() {
    local api_url="$1"
    local node_id="$2"
    local node_secret="$3"
    
    mkdir -p /etc/node-agent
    
    cat > /etc/node-agent/config.env <<EOF
API_URL=${api_url}
NODE_ID=${node_id}
NODE_SECRET=${node_secret}
XRAY_API_PORT=10085
TRAFFIC_REPORT_INTERVAL=30
HEARTBEAT_INTERVAL=60
RUST_LOG=info
EOF
    
    chmod 600 /etc/node-agent/config.env
    log_info "Configuration created at /etc/node-agent/config.env"
}
```

**Systemd 服务单元：**
```bash
create_systemd_service() {
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
}
```

### 6. 服务管理器 (Service Manager)

**职责：** 启动、停止和管理系统服务。

**服务启动：**
```bash
start_services() {
    log_info "Starting services..."
    
    # 启动 Node Agent
    systemctl enable node-agent
    systemctl start node-agent
    
    # 等待服务启动
    sleep 3
    
    # 验证服务状态
    if ! systemctl is-active --quiet node-agent; then
        log_error "Node Agent failed to start"
        journalctl -u node-agent -n 50 --no-pager
        return 1
    fi
    
    log_info "Node Agent started successfully"
}
```

**服务验证：**
```bash
verify_deployment() {
    log_info "Verifying deployment..."
    
    # 检查服务状态
    if ! systemctl is-active --quiet node-agent; then
        log_error "Node Agent is not running"
        return 1
    fi
    
    # 检查端口监听
    if ! netstat -tuln | grep -q ":${NODE_PORT}"; then
        log_warn "Port ${NODE_PORT} is not listening yet"
    fi
    
    # 检查日志中的错误
    if journalctl -u node-agent -n 20 --no-pager | grep -qi "error"; then
        log_warn "Errors found in Node Agent logs"
        journalctl -u node-agent -n 20 --no-pager | grep -i "error"
    fi
    
    log_info "Deployment verification completed"
}
```

### 7. 批量部署配置 (Batch Config)

**配置文件格式（YAML）：**
```yaml
# batch_deploy.yaml
api_url: https://api.example.com
admin_token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

nodes:
  - name: node-hk-01
    host: 1.2.3.4
    port: 443
    protocol: vless
    config:
      flow: xtls-rprx-vision
      
  - name: node-us-01
    host: 5.6.7.8
    port: 8443
    protocol: vmess
    config:
      alterId: 0
      
  - name: node-jp-01
    host: auto  # 自动检测
    port: 443
    protocol: trojan
```

**批量部署逻辑：**
```bash
batch_deploy() {
    local config_file="$1"
    
    # 解析配置文件（使用 yq 或 jq）
    local api_url=$(yq eval '.api_url' "$config_file")
    local admin_token=$(yq eval '.admin_token' "$config_file")
    local node_count=$(yq eval '.nodes | length' "$config_file")
    
    log_info "Starting batch deployment for $node_count nodes"
    
    local success_count=0
    local fail_count=0
    
    for i in $(seq 0 $((node_count - 1))); do
        local node_name=$(yq eval ".nodes[$i].name" "$config_file")
        log_info "Deploying node $((i + 1))/$node_count: $node_name"
        
        if deploy_single_node "$config_file" "$i"; then
            ((success_count++))
            log_info "✓ $node_name deployed successfully"
        else
            ((fail_count++))
            log_error "✗ $node_name deployment failed"
        fi
    done
    
    log_info "Batch deployment completed: $success_count succeeded, $fail_count failed"
}
```

### 8. 日志记录器 (Logger)

**职责：** 记录部署过程和错误信息。

**日志级别：**
- INFO：正常操作信息
- WARN：警告信息（不影响部署）
- ERROR：错误信息（导致部署失败）
- DEBUG：详细调试信息（--verbose 模式）

**日志输出：**
```bash
LOG_FILE="/var/log/node-deployment.log"
VERBOSE=false
QUIET=false

log_info() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [INFO] $msg" >> "$LOG_FILE"
    if [ "$QUIET" != "true" ]; then
        echo -e "${GREEN}[INFO]${NC} $msg"
    fi
}

log_warn() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [WARN] $msg" >> "$LOG_FILE"
    if [ "$QUIET" != "true" ]; then
        echo -e "${YELLOW}[WARN]${NC} $msg"
    fi
}

log_error() {
    local msg="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [ERROR] $msg" >> "$LOG_FILE"
    echo -e "${RED}[ERROR]${NC} $msg" >&2
}

log_debug() {
    local msg="$1"
    if [ "$VERBOSE" = "true" ]; then
        local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        echo "[$timestamp] [DEBUG] $msg" >> "$LOG_FILE"
        echo -e "${BLUE}[DEBUG]${NC} $msg"
    fi
}

# 隐藏敏感信息
mask_sensitive() {
    local value="$1"
    local visible_chars=8
    if [ ${#value} -gt $visible_chars ]; then
        echo "${value:0:$visible_chars}..."
    else
        echo "***"
    fi
}
```

### 9. 验证器 (Validator)

**职责：** 验证部署结果和配置正确性。

**验证项：**
```bash
validate_deployment() {
    local errors=0
    
    # 1. 服务状态验证
    if ! systemctl is-active --quiet node-agent; then
        log_error "Node Agent service is not active"
        ((errors++))
    fi
    
    # 2. 配置文件验证
    if [ ! -f /etc/node-agent/config.env ]; then
        log_error "Configuration file not found"
        ((errors++))
    fi
    
    # 3. 二进制文件验证
    if [ ! -x /usr/local/bin/node-agent ]; then
        log_error "Node Agent binary not found or not executable"
        ((errors++))
    fi
    
    # 4. 网络连接验证
    if ! curl -s -f -H "Authorization: Bearer $NODE_SECRET" \
         "$API_URL/api/nodes/$NODE_ID/health" > /dev/null; then
        log_error "Cannot connect to API service"
        ((errors++))
    fi
    
    # 5. 端口监听验证
    if ! ss -tuln | grep -q ":${NODE_PORT}"; then
        log_warn "Port $NODE_PORT is not listening (may take a moment)"
    fi
    
    if [ $errors -eq 0 ]; then
        log_info "All validation checks passed"
        return 0
    else
        log_error "$errors validation check(s) failed"
        return 1
    fi
}
```

## 数据模型

### 部署配置

```bash
# 单节点部署配置
declare -A DEPLOY_CONFIG=(
    [api_url]=""
    [admin_token]=""
    [node_name]=""
    [node_host]=""
    [node_port]=443
    [node_protocol]="vless"
    [node_config]="{}"
    [node_id]=""
    [node_secret]=""
)
```

### 部署状态

```bash
# 部署状态跟踪
declare -A DEPLOY_STATE=(
    [phase]="init"              # init, env_check, api_call, install, config, start, verify, complete
    [start_time]=""
    [end_time]=""
    [errors]=0
    [warnings]=0
)
```

### 批量部署记录

```bash
# 批量部署结果
declare -a BATCH_RESULTS=()

record_batch_result() {
    local node_name="$1"
    local status="$2"  # success, failed
    local message="$3"
    
    BATCH_RESULTS+=("$node_name|$status|$message")
}

generate_batch_report() {
    echo "=========================================="
    echo "Batch Deployment Report"
    echo "=========================================="
    echo ""
    
    for result in "${BATCH_RESULTS[@]}"; do
        IFS='|' read -r name status message <<< "$result"
        if [ "$status" = "success" ]; then
            echo "✓ $name: $message"
        else
            echo "✗ $name: $message"
        fi
    done
}
```

## 正确性属性

*属性是一个特征或行为，应该在系统的所有有效执行中保持为真——本质上是关于系统应该做什么的形式化陈述。属性作为人类可读规范和机器可验证正确性保证之间的桥梁。*


### 属性反思

在编写正确性属性之前，让我审查预分析中识别的所有可测试属性，以消除冗余：

**识别的属性：**
1. 参数解析（1.1-1.3, 1.7）：可以合并为一个综合属性
2. 默认值（1.4-1.6）：这些是具体示例，不是属性
3. 操作系统检测（2.1）：通用属性
4. 依赖检查（2.3）：通用属性
5. 密钥生成长度（3.1）：通用属性
6. 密钥格式（3.3）：通用属性
7. API 响应解析（4.4）：通用属性
8. 协议配置生成（5.5）：通用属性
9. 配置文件解析（8.1）：通用属性
10. 密钥唯一性（8.3）：通用属性
11. 幂等性（11.1）：通用属性
12. Token 验证（12.1）：通用属性
13. 日志屏蔽（12.3）：通用属性

**冗余分析：**
- 属性 1（参数解析）可以涵盖 1.1、1.2、1.3 和 1.7，因为它们都是关于参数解析的
- 属性 5 和 6（密钥生成）可以合并为一个综合属性，因为它们都是关于密钥生成的要求
- 大多数其他属性提供独特的验证价值，应该保留

### 正确性属性

基于预分析，以下是可以通过自动化测试验证的正确性属性：

**属性 1：参数解析一致性**
*对于任何*有效的参数集（通过命令行或环境变量提供），脚本应该正确解析并存储所有提供的参数值，且命令行参数应该优先于环境变量。
**验证需求：1.1, 1.2, 1.3, 1.7**

**属性 2：操作系统检测准确性**
*对于任何*支持的 Linux 发行版（Ubuntu/CentOS/Debian），脚本应该正确识别操作系统类型和版本。
**验证需求：2.1**

**属性 3：依赖检查完整性**
*对于任何*必需的系统命令列表，脚本应该检查所有命令的可用性，并报告任何缺失的命令。
**验证需求：2.3**

**属性 4：密钥生成安全性**
*对于任何*生成的节点密钥，密钥长度应该至少为 32 个字符，且仅包含字母数字字符。
**验证需求：3.1, 3.3**

**属性 5：API 响应解析正确性**
*对于任何*来自 API 的成功响应（HTTP 200/201），脚本应该能够正确解析并提取 node_id 和 secret 字段。
**验证需求：4.4**

**属性 6：协议配置生成正确性**
*对于任何*支持的协议类型（shadowsocks/vmess/trojan/hysteria2/vless），脚本应该生成符合该协议规范的 Xray-core 配置文件。
**验证需求：5.5**

**属性 7：批量配置解析正确性**
*对于任何*有效的批量部署配置文件，脚本应该正确解析所有节点配置项，且解析后的节点数量应该等于配置文件中定义的节点数量。
**验证需求：8.1**

**属性 8：密钥唯一性**
*对于任何*批量部署操作，为每个节点生成的密钥应该是唯一的，不应该有重复。
**验证需求：8.3**

**属性 9：幂等性**
*对于任何*已完成的部署，重复执行脚本应该检测到已安装的组件，跳过重复安装，且最终状态应该与首次部署相同。
**验证需求：11.1**

**属性 10：Token 格式验证**
*对于任何*提供的 Admin_Token，如果 token 不符合 JWT 格式（三个由点分隔的 base64 编码段），脚本应该拒绝该 token 并显示错误消息。
**验证需求：12.1**

**属性 11：敏感信息屏蔽**
*对于任何*包含敏感信息（token、secret、密码）的日志条目，日志输出应该屏蔽完整值，仅显示前 8 个字符或使用占位符。
**验证需求：12.3**

## 错误处理

### 错误分类

**1. 参数错误**
- 缺少必需参数（API_URL、ADMIN_TOKEN、NODE_NAME）
- 参数格式错误（无效的 URL、端口范围）
- 处理：显示使用说明，退出代码 1

**2. 环境错误**
- 非 root 用户执行
- 不支持的操作系统
- 缺少必需依赖
- 处理：显示错误信息和解决建议，退出代码 2

**3. 网络错误**
- 无法连接到 API 服务
- 下载失败（Xray-core、Node Agent）
- 处理：重试机制（最多 3 次），显示网络诊断建议，退出代码 3

**4. API 错误**
- 401/403：认证失败
- 400：请求参数错误
- 409：节点已存在
- 500：服务器错误
- 处理：显示详细错误信息和 API 响应，提供解决建议，退出代码 4

**5. 安装错误**
- 文件下载失败
- 文件完整性验证失败
- 安装路径权限不足
- 处理：清理部分安装的文件，显示错误日志，退出代码 5

**6. 服务错误**
- 服务启动失败
- 服务状态异常
- 端口被占用
- 处理：显示服务日志，提供故障排查步骤，退出代码 6

### 错误恢复策略

```bash
# 错误处理框架
set -e  # 遇到错误立即退出
trap cleanup EXIT  # 确保清理函数总是被调用

cleanup() {
    local exit_code=$?
    
    if [ $exit_code -ne 0 ]; then
        log_error "Deployment failed with exit code $exit_code"
        
        # 根据部署阶段执行清理
        case $DEPLOY_STATE[phase] in
            install)
                log_info "Cleaning up partial installation..."
                stop_services
                ;;
            config)
                log_info "Removing incomplete configuration..."
                remove_config_files
                ;;
            *)
                log_info "No cleanup needed for phase: ${DEPLOY_STATE[phase]}"
                ;;
        esac
        
        # 显示故障排查建议
        show_troubleshooting_tips $exit_code
    fi
}

# 重试机制
retry() {
    local max_attempts=3
    local delay=5
    local attempt=1
    local command="$@"
    
    while [ $attempt -le $max_attempts ]; do
        log_info "Attempt $attempt/$max_attempts: $command"
        
        if eval "$command"; then
            return 0
        fi
        
        if [ $attempt -lt $max_attempts ]; then
            log_warn "Command failed, retrying in ${delay}s..."
            sleep $delay
        fi
        
        ((attempt++))
    done
    
    log_error "Command failed after $max_attempts attempts"
    return 1
}
```

### 故障排查建议

```bash
show_troubleshooting_tips() {
    local exit_code=$1
    
    echo ""
    echo "=========================================="
    echo "Troubleshooting Tips"
    echo "=========================================="
    
    case $exit_code in
        1)
            echo "Parameter Error:"
            echo "  - Check if all required parameters are provided"
            echo "  - Verify parameter formats (URL, port, protocol)"
            echo "  - Run with --help to see usage examples"
            ;;
        2)
            echo "Environment Error:"
            echo "  - Ensure you are running as root (sudo)"
            echo "  - Check if your OS is supported (Ubuntu/CentOS/Debian)"
            echo "  - Install missing dependencies manually"
            ;;
        3)
            echo "Network Error:"
            echo "  - Check your internet connection"
            echo "  - Verify API_URL is correct and accessible"
            echo "  - Check firewall settings"
            echo "  - Try using a different DNS server"
            ;;
        4)
            echo "API Error:"
            echo "  - Verify ADMIN_TOKEN is valid and not expired"
            echo "  - Check if you have admin permissions"
            echo "  - Review API error message above"
            echo "  - Contact system administrator if needed"
            ;;
        5)
            echo "Installation Error:"
            echo "  - Check disk space (need at least 500MB)"
            echo "  - Verify write permissions to /usr/local/bin"
            echo "  - Check installation logs for details"
            echo "  - Try manual installation if automated fails"
            ;;
        6)
            echo "Service Error:"
            echo "  - Check service logs: journalctl -u node-agent -n 50"
            echo "  - Verify configuration file: /etc/node-agent/config.env"
            echo "  - Check if port is already in use: netstat -tuln | grep $NODE_PORT"
            echo "  - Ensure Xray-core is properly installed"
            ;;
    esac
    
    echo ""
    echo "For more help:"
    echo "  - View full logs: cat /var/log/node-deployment.log"
    echo "  - Check service status: systemctl status node-agent"
    echo "  - Run with --verbose for detailed output"
    echo ""
}
```

## 测试策略

### 双重测试方法

本功能将采用单元测试和集成测试相结合的方法：

**单元测试：**
- 测试各个函数的正确性（参数解析、密钥生成、配置生成等）
- 测试边界条件和错误处理
- 使用 Bash 测试框架（如 bats-core）
- 快速执行，适合开发过程中的频繁测试

**集成测试：**
- 测试完整的部署流程
- 使用 Docker 容器模拟不同的操作系统环境
- 使用 mock API 服务器测试 API 交互
- 验证实际的文件创建、服务启动等操作

### 测试框架选择

**Bats (Bash Automated Testing System)：**
- 专为 Bash 脚本设计的测试框架
- 简单易用，语法类似于 RSpec
- 支持 setup/teardown 钩子
- 提供丰富的断言函数

**安装 Bats：**
```bash
# Ubuntu/Debian
apt-get install bats

# 或从源码安装
git clone https://github.com/bats-core/bats-core.git
cd bats-core
./install.sh /usr/local
```

### 测试用例示例

```bash
#!/usr/bin/env bats

# tests/test_deploy_node.bats

setup() {
    # 加载脚本函数
    source ../scripts/deploy_node.sh
    
    # 设置测试环境
    export TEST_MODE=true
    export LOG_FILE="/tmp/test-deployment.log"
}

teardown() {
    # 清理测试环境
    rm -f "$LOG_FILE"
}

# 单元测试示例

@test "generate_node_secret produces 32+ character secret" {
    # Feature: node-one-click-deployment, Property 4: 密钥生成安全性
    local secret=$(generate_node_secret)
    [ ${#secret} -ge 32 ]
}

@test "generate_node_secret produces alphanumeric only" {
    # Feature: node-one-click-deployment, Property 4: 密钥生成安全性
    local secret=$(generate_node_secret)
    [[ "$secret" =~ ^[a-zA-Z0-9]+$ ]]
}

@test "parse_parameters accepts command line arguments" {
    # Feature: node-one-click-deployment, Property 1: 参数解析一致性
    parse_parameters --api-url "https://api.test.com" \
                    --admin-token "test-token" \
                    --node-name "test-node"
    
    [ "${DEPLOY_CONFIG[api_url]}" = "https://api.test.com" ]
    [ "${DEPLOY_CONFIG[admin_token]}" = "test-token" ]
    [ "${DEPLOY_CONFIG[node_name]}" = "test-node" ]
}

@test "parse_parameters prefers command line over environment" {
    # Feature: node-one-click-deployment, Property 1: 参数解析一致性
    export API_URL="https://env.test.com"
    
    parse_parameters --api-url "https://cli.test.com"
    
    [ "${DEPLOY_CONFIG[api_url]}" = "https://cli.test.com" ]
}

@test "detect_os identifies Ubuntu correctly" {
    # Feature: node-one-click-deployment, Property 2: 操作系统检测准确性
    # 需要在 Ubuntu 环境中运行
    skip "Requires Ubuntu environment"
    
    detect_os
    [ "$OS" = "ubuntu" ]
}

@test "mask_sensitive hides full value" {
    # Feature: node-one-click-deployment, Property 11: 敏感信息屏蔽
    local secret="abcdefghijklmnopqrstuvwxyz123456"
    local masked=$(mask_sensitive "$secret")
    
    [ "$masked" = "abcdefgh..." ]
    [[ ! "$masked" =~ "ijklmnop" ]]
}

@test "validate_jwt_format accepts valid JWT" {
    # Feature: node-one-click-deployment, Property 10: Token 格式验证
    local valid_jwt="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"
    
    validate_jwt_format "$valid_jwt"
    [ $? -eq 0 ]
}

@test "validate_jwt_format rejects invalid JWT" {
    # Feature: node-one-click-deployment, Property 10: Token 格式验证
    local invalid_jwt="not-a-valid-jwt"
    
    run validate_jwt_format "$invalid_jwt"
    [ $status -ne 0 ]
}

# 集成测试示例

@test "full deployment workflow with mock API" {
    skip "Requires Docker environment and mock API"
    
    # 启动 mock API 服务器
    docker run -d --name mock-api -p 8080:8080 mock-api-server
    
    # 运行部署脚本
    run ./deploy_node.sh \
        --api-url "http://localhost:8080" \
        --admin-token "test-token" \
        --node-name "test-node" \
        --node-host "127.0.0.1"
    
    [ $status -eq 0 ]
    
    # 验证服务已启动
    systemctl is-active --quiet node-agent
    
    # 清理
    docker stop mock-api
    docker rm mock-api
}

@test "idempotency: second run skips installation" {
    # Feature: node-one-click-deployment, Property 9: 幂等性
    skip "Requires full environment setup"
    
    # 第一次运行
    run ./deploy_node.sh --api-url "http://test" --admin-token "token" --node-name "node1"
    [ $status -eq 0 ]
    
    # 第二次运行
    run ./deploy_node.sh --api-url "http://test" --admin-token "token" --node-name "node1"
    [ $status -eq 0 ]
    
    # 验证日志中包含 "already installed" 消息
    grep -q "already installed" "$LOG_FILE"
}

@test "batch deployment generates unique secrets" {
    # Feature: node-one-click-deployment, Property 8: 密钥唯一性
    skip "Requires batch config file"
    
    # 创建批量配置
    cat > /tmp/batch_config.yaml <<EOF
api_url: http://test
admin_token: token
nodes:
  - name: node1
  - name: node2
  - name: node3
EOF
    
    # 运行批量部署
    run ./deploy_node.sh --batch-config /tmp/batch_config.yaml
    
    # 提取所有生成的密钥
    local secrets=$(grep "Generated secret:" "$LOG_FILE" | awk '{print $NF}')
    
    # 验证密钥数量
    local count=$(echo "$secrets" | wc -l)
    [ $count -eq 3 ]
    
    # 验证密钥唯一性
    local unique_count=$(echo "$secrets" | sort -u | wc -l)
    [ $unique_count -eq 3 ]
}
```

### 测试覆盖率目标

- 单元测试覆盖率：80% 以上
- 关键路径（参数解析、API 调用、服务启动）：100%
- 错误处理路径：90% 以上
- 集成测试：覆盖所有主要部署场景

### 持续集成

```yaml
# .github/workflows/test.yml
name: Test Deploy Script

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Bats
        run: |
          sudo apt-get update
          sudo apt-get install -y bats
      
      - name: Run unit tests
        run: |
          cd tests
          bats test_deploy_node.bats
      
      - name: Run integration tests
        run: |
          cd tests
          bats test_integration.bats
```

### 测试数据管理

**Mock API 响应：**
```bash
# tests/fixtures/api_responses.json
{
  "create_node_success": {
    "id": "node-test-001",
    "name": "test-node",
    "host": "127.0.0.1",
    "port": 443,
    "protocol": "vless",
    "secret": "test-secret-12345678901234567890",
    "status": "active",
    "created_at": "2024-01-01T00:00:00Z"
  },
  "create_node_error_401": {
    "error": "Unauthorized",
    "message": "Invalid or expired token"
  },
  "create_node_error_409": {
    "error": "Conflict",
    "message": "Node with this name already exists"
  }
}
```

**测试配置文件：**
```yaml
# tests/fixtures/batch_config_valid.yaml
api_url: http://localhost:8080
admin_token: test-token-123

nodes:
  - name: test-node-1
    host: 192.168.1.1
    port: 443
    protocol: vless
    
  - name: test-node-2
    host: 192.168.1.2
    port: 8443
    protocol: vmess
```

## 实现注意事项

### 安全考虑

1. **密钥管理：**
   - 使用加密安全的随机源
   - 在日志和输出中屏蔽敏感信息
   - 配置文件权限设置为 600
   - 完成后清理临时文件

2. **API 通信：**
   - 强制使用 HTTPS
   - 验证 SSL 证书
   - 实现请求超时
   - 限制重试次数

3. **输入验证：**
   - 验证所有用户输入
   - 防止命令注入
   - 验证文件路径
   - 检查文件权限

### 性能优化

1. **并行下载：**
   - 同时下载 Xray-core 和 Node Agent
   - 使用后台任务加速批量部署

2. **缓存机制：**
   - 缓存已下载的二进制文件
   - 避免重复下载相同版本

3. **快速失败：**
   - 尽早验证参数和环境
   - 避免不必要的操作

### 可维护性

1. **模块化设计：**
   - 每个功能独立函数
   - 清晰的函数接口
   - 便于单元测试

2. **配置管理：**
   - 集中配置变量
   - 支持配置文件
   - 环境变量覆盖

3. **文档完善：**
   - 详细的使用说明
   - 故障排查指南
   - API 文档引用

### 兼容性

1. **操作系统：**
   - 支持 Ubuntu 18.04+
   - 支持 CentOS 7+
   - 支持 Debian 10+

2. **架构：**
   - x86_64
   - aarch64 (ARM64)

3. **依赖版本：**
   - Bash 4.0+
   - curl 7.0+
   - jq 1.5+
   - systemd 219+

## 部署流程图

```mermaid
flowchart TD
    Start([开始]) --> ParseArgs[解析参数]
    ParseArgs --> ValidateArgs{参数有效?}
    ValidateArgs -->|否| ShowHelp[显示帮助信息]
    ShowHelp --> End([结束])
    
    ValidateArgs -->|是| CheckRoot{Root权限?}
    CheckRoot -->|否| ErrorRoot[错误: 需要Root权限]
    ErrorRoot --> End
    
    CheckRoot -->|是| DetectOS[检测操作系统]
    DetectOS --> CheckDeps[检查依赖]
    CheckDeps --> InstallDeps{依赖完整?}
    InstallDeps -->|否| TryInstall[尝试安装依赖]
    TryInstall --> InstallDeps
    
    InstallDeps -->|是| GenSecret[生成节点密钥]
    GenSecret --> CallAPI[调用API创建节点]
    CallAPI --> APISuccess{API成功?}
    
    APISuccess -->|否| HandleAPIError[处理API错误]
    HandleAPIError --> Cleanup[清理]
    Cleanup --> End
    
    APISuccess -->|是| ParseResponse[解析API响应]
    ParseResponse --> DownloadXray[下载Xray-core]
    DownloadXray --> DownloadAgent[下载Node Agent]
    DownloadAgent --> CreateConfig[创建配置文件]
    CreateConfig --> CreateService[创建Systemd服务]
    CreateService --> StartServices[启动服务]
    StartServices --> Verify[验证部署]
    Verify --> VerifyOK{验证通过?}
    
    VerifyOK -->|否| ShowLogs[显示错误日志]
    ShowLogs --> Rollback[回滚]
    Rollback --> End
    
    VerifyOK -->|是| Success[显示成功信息]
    Success --> End
```

## 总结

本设计文档详细描述了节点一键部署功能的架构、组件、接口和实现细节。通过模块化设计和完善的错误处理机制，该脚本将大大简化节点部署流程，提高运维效率。

关键设计决策：
1. 使用 Bash 脚本实现，确保在所有 Linux 系统上的兼容性
2. 模块化函数设计，便于测试和维护
3. 完善的错误处理和回滚机制，确保部署可靠性
4. 支持单节点和批量部署，满足不同规模需求
5. 详细的日志记录和验证机制，便于问题排查
6. 安全优先，保护敏感信息，验证所有输入
