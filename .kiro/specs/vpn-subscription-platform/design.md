# 设计文档：VPN 订阅服务平台

## 概述

VPN 订阅服务平台是一个基于微服务架构的商业化代理服务系统。系统采用前后端分离架构，后端使用 Rust (Axum) 提供高性能 API 服务，前端使用 Vue 3 构建用户界面和管理后台。系统支持多种代理协议（Shadowsocks、VMess、Trojan、Hysteria2、VLESS-Reality），通过 Xray-core 提供代理服务，使用 Node Agent 实现分布式节点管理。

### 核心特性

- 用户注册、认证和授权
- 虚拟金币充值和消费系统
- 流量套餐购买和管理
- 推荐返利机制
- Clash 订阅链接生成
- 多节点管理和健康检查
- 实时流量统计和限制
- 管理后台（用户管理、节点管理、订单管理、数据统计）
- 支持多种代理协议（包括 Reality 抗封锁）

## 架构

### 系统架构图

```mermaid
graph TB
    subgraph "用户端"
        UserFrontend[用户前端<br/>Vue 3 + TypeScript]
        AdminFrontend[管理后台<br/>Vue 3 + TypeScript]
        ClashClient[Clash 客户端]
    end
    
    subgraph "API 层"
        APIService[API 服务<br/>Rust Axum]
        Redis[(Redis<br/>缓存 + 消息队列)]
    end
    
    subgraph "数据层"
        PostgreSQL[(PostgreSQL<br/>主数据库)]
    end
    
    subgraph "节点集群"
        NodeAgent1[Node Agent 1]
        NodeAgent2[Node Agent 2]
        NodeAgentN[Node Agent N]
        Xray1[Xray-core 1]
        Xray2[Xray-core 2]
        XrayN[Xray-core N]
    end

    UserFrontend -->|HTTPS| APIService
    AdminFrontend -->|HTTPS| APIService
    ClashClient -->|订阅请求| APIService
    ClashClient -->|代理流量| Xray1
    ClashClient -->|代理流量| Xray2
    ClashClient -->|代理流量| XrayN
    
    APIService -->|读写| PostgreSQL
    APIService -->|缓存/队列| Redis
    
    NodeAgent1 -->|配置同步| APIService
    NodeAgent2 -->|配置同步| APIService
    NodeAgentN -->|配置同步| APIService
    
    NodeAgent1 -->|流量上报| Redis
    NodeAgent2 -->|流量上报| Redis
    NodeAgentN -->|流量上报| Redis
    
    NodeAgent1 -->|管理| Xray1
    NodeAgent2 -->|管理| Xray2
    NodeAgentN -->|管理| XrayN
```

### 技术栈

**前端**：
- Vue 3 (Composition API)
- TypeScript
- Vite (构建工具)
- Pinia (状态管理)
- TailwindCSS (用户前端样式框架)
- Ant Design Vue (管理后台 UI 组件库)
- Axios (HTTP 客户端)

**后端**：
- Rust (编程语言)
- Axum (Web 框架)
- SQLx (数据库驱动)
- Redis (缓存和消息队列)
- JWT (认证)
- Serde (序列化)

**数据库**：
- PostgreSQL (主数据库)
- Redis (缓存、会话、消息队列)

**代理服务**：
- Xray-core (代理核心)
- 支持协议：Shadowsocks、VMess、Trojan、Hysteria2、VLESS-Reality

**部署**：
- Docker
- Docker Compose

## 组件和接口

### 1. API 服务 (API Service)

API 服务是系统的核心后端，负责处理所有业务逻辑。


#### 主要模块

**认证模块 (Auth Module)**：
- 用户注册、登录
- JWT 令牌生成和验证
- 密码哈希（使用 argon2）

**用户模块 (User Module)**：
- 用户信息管理
- 金币余额管理
- 流量配额管理
- 推荐关系管理

**套餐模块 (Package Module)**：
- 套餐列表查询
- 套餐购买
- 订单创建和管理

**订阅模块 (Subscription Module)**：
- 订阅链接生成
- Clash 配置生成
- 节点列表过滤（根据用户套餐）

**节点模块 (Node Module)**：
- 节点 CRUD 操作
- 节点健康检查
- 节点配置分发

**流量模块 (Traffic Module)**：
- 流量数据接收（从 Redis Streams）
- 流量统计聚合
- 流量限制检查

**管理模块 (Admin Module)**：
- 用户管理
- 订单管理
- 数据统计

#### API 端点设计

**认证相关**：
```
POST   /api/auth/register          # 用户注册
POST   /api/auth/login             # 用户登录
POST   /api/auth/refresh           # 刷新令牌
```

**用户相关**：
```
GET    /api/user/profile           # 获取用户信息
GET    /api/user/balance           # 获取金币余额
GET    /api/user/traffic           # 获取流量使用情况
GET    /api/user/referral          # 获取推荐链接
GET    /api/user/referral/stats    # 获取推荐统计
```

**套餐相关**：
```
GET    /api/packages               # 获取套餐列表
POST   /api/packages/:id/purchase  # 购买套餐
GET    /api/orders                 # 获取订单列表
GET    /api/orders/:id             # 获取订单详情
```

**订阅相关**：
```
GET    /api/subscription/link      # 获取订阅链接
GET    /sub/:token                 # Clash 订阅端点（公开）
```

**节点相关（管理员）**：
```
GET    /api/admin/nodes            # 获取节点列表
POST   /api/admin/nodes            # 创建节点
PUT    /api/admin/nodes/:id        # 更新节点
DELETE /api/admin/nodes/:id        # 删除节点
GET    /api/admin/nodes/:id/stats  # 获取节点统计
```

**用户管理（管理员）**：
```
GET    /api/admin/users            # 获取用户列表
GET    /api/admin/users/:id        # 获取用户详情
PUT    /api/admin/users/:id/status # 启用/禁用用户
PUT    /api/admin/users/:id/balance # 调整用户金币
PUT    /api/admin/users/:id/traffic # 调整用户流量
```

**统计相关（管理员）**：
```
GET    /api/admin/stats/overview   # 获取总览统计
GET    /api/admin/stats/revenue    # 获取收入统计
GET    /api/admin/stats/traffic    # 获取流量统计
```

### 2. 用户前端 (User Frontend)

用户前端提供用户注册、登录、购买套餐、获取订阅链接等功能。

#### 页面结构

```
/                       # 首页
/register               # 注册页面
/login                  # 登录页面
/dashboard              # 用户仪表板
  /dashboard/profile    # 个人信息
  /dashboard/packages   # 套餐购买
  /dashboard/orders     # 订单历史
  /dashboard/subscription # 订阅管理
  /dashboard/referral   # 推荐返利
```

#### 状态管理（Pinia Stores）

**authStore**：
- 用户认证状态
- 登录/登出操作
- 令牌管理

**userStore**：
- 用户信息
- 金币余额
- 流量配额

**packageStore**：
- 套餐列表
- 购买操作

**subscriptionStore**：
- 订阅链接
- 订阅配置

### 3. 管理后台 (Admin Frontend)

管理后台提供节点管理、用户管理、订单管理、数据统计等功能。

#### 页面结构

```
/admin                  # 管理后台首页
/admin/login            # 管理员登录
/admin/dashboard        # 仪表板（数据总览）
/admin/nodes            # 节点管理
/admin/users            # 用户管理
/admin/orders           # 订单管理
/admin/packages         # 套餐管理
/admin/stats            # 数据统计
```


### 4. Node Agent

Node Agent 部署在每个 VPN 节点上，负责配置同步、流量上报、健康检查。

#### 功能模块

**配置同步模块**：
- 定期从 API 服务拉取节点配置
- 监听配置更新通知（通过 Redis Pub/Sub）
- 应用配置到 Xray-core

**流量上报模块**：
- 从 Xray-core API 获取用户流量数据
- 批量上报流量数据到 Redis Streams
- 每 30 秒上报一次

**健康检查模块**：
- 检查 Xray-core 进程状态
- 发送心跳到 API 服务
- 异常时尝试重启 Xray-core

**用户管理模块**：
- 从 API 服务获取活跃用户列表
- 动态添加/删除 Xray-core 用户配置
- 检查用户流量配额

#### 与 API 服务的通信

**配置拉取**：
```
GET /api/node/config?node_id={id}&secret={secret}
Response: {
  "protocol": "vless",
  "port": 443,
  "users": [...],
  "reality_config": {...},
  ...
}
```

**心跳上报**：
```
POST /api/node/heartbeat
Body: {
  "node_id": "xxx",
  "secret": "xxx",
  "status": "online",
  "cpu_usage": 45.2,
  "memory_usage": 60.5,
  "active_connections": 123
}
```

**流量上报**（通过 Redis Streams）：
```
XADD traffic_stream * 
  node_id xxx 
  user_id yyy 
  upload 1024000 
  download 2048000 
  timestamp 1234567890
```

### 5. Xray-core

Xray-core 是实际的代理服务，由 Node Agent 管理。

#### 配置结构

**VLESS-Reality 配置示例**：
```json
{
  "inbounds": [{
    "port": 443,
    "protocol": "vless",
    "settings": {
      "clients": [
        {
          "id": "uuid-here",
          "email": "user@example.com",
          "flow": "xtls-rprx-vision"
        }
      ],
      "decryption": "none"
    },
    "streamSettings": {
      "network": "tcp",
      "security": "reality",
      "realitySettings": {
        "show": false,
        "dest": "www.microsoft.com:443",
        "xver": 0,
        "serverNames": ["www.microsoft.com"],
        "privateKey": "private-key-here",
        "shortIds": [""]
      }
    }
  }],
  "outbounds": [{
    "protocol": "freedom"
  }]
}
```

#### API 接口

Xray-core 提供 API 用于流量统计：
```
GET /stats/user/{email}/uplink
GET /stats/user/{email}/downlink
```

## 数据模型

### 数据库表结构

#### users 表
```sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    coin_balance BIGINT DEFAULT 0,
    traffic_quota BIGINT DEFAULT 0,  -- 字节
    traffic_used BIGINT DEFAULT 0,   -- 字节
    referral_code VARCHAR(32) UNIQUE,
    referred_by BIGINT REFERENCES users(id),
    status VARCHAR(20) DEFAULT 'active',  -- active, disabled
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_referral_code ON users(referral_code);
```

#### packages 表
```sql
CREATE TABLE packages (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    traffic_amount BIGINT NOT NULL,  -- 字节
    price BIGINT NOT NULL,           -- 金币
    duration_days INT NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

#### orders 表
```sql
CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    order_no VARCHAR(64) UNIQUE NOT NULL,
    user_id BIGINT REFERENCES users(id),
    package_id BIGINT REFERENCES packages(id),
    amount BIGINT NOT NULL,          -- 金币
    status VARCHAR(20) DEFAULT 'pending',  -- pending, completed, failed
    created_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP
);

CREATE INDEX idx_orders_user_id ON orders(user_id);
CREATE INDEX idx_orders_created_at ON orders(created_at);
```


#### user_packages 表
```sql
CREATE TABLE user_packages (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(id),
    package_id BIGINT REFERENCES packages(id),
    order_id BIGINT REFERENCES orders(id),
    traffic_quota BIGINT NOT NULL,
    traffic_used BIGINT DEFAULT 0,
    expires_at TIMESTAMP NOT NULL,
    status VARCHAR(20) DEFAULT 'active',  -- active, expired, exhausted
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_user_packages_user_id ON user_packages(user_id);
CREATE INDEX idx_user_packages_expires_at ON user_packages(expires_at);
```

#### nodes 表
```sql
CREATE TABLE nodes (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    host VARCHAR(255) NOT NULL,
    port INT NOT NULL,
    protocol VARCHAR(20) NOT NULL,  -- shadowsocks, vmess, trojan, hysteria2, vless
    secret VARCHAR(255) NOT NULL,   -- Node Agent 认证密钥
    config JSONB NOT NULL,          -- 协议特定配置
    status VARCHAR(20) DEFAULT 'offline',  -- online, offline, maintenance
    max_users INT DEFAULT 1000,
    current_users INT DEFAULT 0,
    total_upload BIGINT DEFAULT 0,
    total_download BIGINT DEFAULT 0,
    last_heartbeat TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_nodes_status ON nodes(status);
```

#### traffic_logs 表
```sql
CREATE TABLE traffic_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(id),
    node_id BIGINT REFERENCES nodes(id),
    upload BIGINT NOT NULL,
    download BIGINT NOT NULL,
    recorded_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_traffic_logs_user_id ON traffic_logs(user_id);
CREATE INDEX idx_traffic_logs_recorded_at ON traffic_logs(recorded_at);
```

#### subscriptions 表
```sql
CREATE TABLE subscriptions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(id) UNIQUE,
    token VARCHAR(64) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    last_accessed TIMESTAMP
);

CREATE INDEX idx_subscriptions_token ON subscriptions(token);
```

#### coin_transactions 表
```sql
CREATE TABLE coin_transactions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES users(id),
    amount BIGINT NOT NULL,          -- 正数为增加，负数为减少
    type VARCHAR(20) NOT NULL,       -- recharge, purchase, referral
    description TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_coin_transactions_user_id ON coin_transactions(user_id);
```

#### admin_logs 表
```sql
CREATE TABLE admin_logs (
    id BIGSERIAL PRIMARY KEY,
    admin_id BIGINT REFERENCES users(id),
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(50),
    target_id BIGINT,
    details JSONB,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_admin_logs_admin_id ON admin_logs(admin_id);
CREATE INDEX idx_admin_logs_created_at ON admin_logs(created_at);
```

### Redis 数据结构

#### 缓存键

**用户套餐缓存**：
```
Key: user:package:{user_id}
Type: Hash
Fields:
  - traffic_quota: 总流量配额
  - traffic_used: 已使用流量
  - expires_at: 过期时间
  - status: 状态
TTL: 300 秒
```

**节点列表缓存**：
```
Key: nodes:active
Type: List (JSON strings)
TTL: 60 秒
```

**订阅配置缓存**：
```
Key: subscription:{token}
Type: String (Clash YAML)
TTL: 300 秒
```

#### 消息队列

**流量上报流**：
```
Stream: traffic_stream
Consumer Group: traffic_processor
Fields:
  - node_id
  - user_id
  - upload
  - download
  - timestamp
```

**配置更新通知**：
```
Channel: node:config:update
Message: {
  "node_id": "xxx",
  "action": "reload"
}
```

### 数据模型类（Rust）

```rust
// User 模型
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub coin_balance: i64,
    pub traffic_quota: i64,
    pub traffic_used: i64,
    pub referral_code: Option<String>,
    pub referred_by: Option<i64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Package 模型
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Package {
    pub id: i64,
    pub name: String,
    pub traffic_amount: i64,
    pub price: i64,
    pub duration_days: i32,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Node 模型
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub protocol: String,
    pub secret: String,
    pub config: serde_json::Value,
    pub status: String,
    pub max_users: i32,
    pub current_users: i32,
    pub total_upload: i64,
    pub total_download: i64,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Order 模型
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Order {
    pub id: i64,
    pub order_no: String,
    pub user_id: i64,
    pub package_id: i64,
    pub amount: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```


## 正确性属性

属性是系统在所有有效执行中应保持为真的特征或行为——本质上是关于系统应该做什么的形式化陈述。属性作为人类可读规范和机器可验证正确性保证之间的桥梁。

### 属性 1：用户注册唯一性

*对于任何*有效的邮箱和密码，如果使用该邮箱注册成功，则使用相同邮箱再次注册应该被拒绝并返回错误。

**验证需求：1.1, 1.2**

### 属性 2：认证令牌有效性

*对于任何*用户，使用正确的凭证登录应该返回有效的 JWT 令牌，使用该令牌访问受保护资源应该成功；使用错误的凭证登录应该被拒绝。

**验证需求：1.3, 1.4, 14.2, 14.3**

### 属性 3：金币余额一致性

*对于任何*用户和金币操作序列（充值、购买），执行操作后的余额应该等于初始余额加上所有充值金额减去所有购买金额。

**验证需求：2.2, 2.3, 2.4**

### 属性 4：金币余额不足拒绝

*对于任何*用户，如果金币余额小于套餐价格，则购买请求应该被拒绝并返回错误，余额保持不变。

**验证需求：2.5**

### 属性 5：套餐购买流量增加

*对于任何*用户和套餐，成功购买套餐后，用户的流量配额应该增加套餐的流量数量。

**验证需求：3.3**

### 属性 6：推荐链接唯一性

*对于任何*用户，生成的推荐链接应该是唯一的，不同用户的推荐链接应该不同。

**验证需求：4.1**

### 属性 7：推荐返利正确性

*对于任何*推荐关系，当被推荐用户完成首次购买时，推荐人的金币余额应该增加预定的返利金额。

**验证需求：4.3**

### 属性 8：订阅链接唯一性

*对于任何*用户，生成的订阅 URL 应该是唯一的，不同用户的订阅 URL 应该不同。

**验证需求：5.1**

### 属性 9：订阅配置生成正确性

*对于任何*拥有有效套餐的用户，访问其订阅链接应该返回有效的 Clash YAML 配置，配置中应包含所有在线节点的连接信息。

**验证需求：5.2, 5.3, 5.5**

### 属性 10：节点配置往返一致性

*对于任何*节点配置，保存到数据库后再读取应该得到相同的配置内容（JSON 序列化往返）。

**验证需求：6.1**

### 属性 11：流量统计累加正确性

*对于任何*用户和流量上报序列，用户的总流量使用应该等于所有上报的上传流量和下载流量之和。

**验证需求：7.1, 7.2, 7.4**

### 属性 12：流量超额拒绝服务

*对于任何*用户，如果已使用流量大于或等于流量配额，则代理服务应该拒绝该用户的连接请求。

**验证需求：7.3**

### 属性 13：用户状态管理往返

*对于任何*用户，禁用后再启用应该恢复用户的访问权限，用户应该能够正常登录和使用服务。

**验证需求：8.2, 8.3**

### 属性 14：订单号唯一性

*对于任何*订单创建操作，生成的订单号应该是唯一的，不同订单的订单号应该不同。

**验证需求：9.5**

### 属性 15：订单统计聚合正确性

*对于任何*时间范围，该时间范围内的订单总额应该等于该范围内所有订单金额之和，订单数量应该等于该范围内订单的计数。

**验证需求：9.4**

### 属性 16：多协议配置生成正确性

*对于任何*支持的代理协议（Shadowsocks、VMess、Trojan、Hysteria2、VLESS-Reality），系统应该能够生成符合 Clash 格式规范的节点配置，配置应包含协议特定的所有必需参数。

**验证需求：11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7**

### 属性 17：Reality 配置完整性

*对于任何* VLESS-Reality 节点配置，生成的配置应该包含所有必需的 Reality 参数（dest、serverNames、publicKey、privateKey、shortIds）。

**验证需求：11.6**

### 属性 18：密码哈希不可逆性

*对于任何*用户密码，存储在数据库中的应该是密码的哈希值而非明文，且哈希值应该无法反向推导出原始密码。

**验证需求：14.1**

### 属性 19：审计日志完整性

*对于任何*敏感操作（删除节点、禁用用户、调整余额），系统应该在 admin_logs 表中记录操作日志，包含操作者、操作类型、目标和时间戳。

**验证需求：8.5, 14.4**

### 属性 20：环境变量配置正确性

*对于任何*配置参数（数据库连接、Redis 连接等），如果通过环境变量提供，系统应该正确读取并使用该值而非默认值。

**验证需求：13.4**


## 错误处理

### API 错误响应格式

所有 API 错误响应应遵循统一格式：

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "人类可读的错误消息",
    "details": {}
  }
}
```

### 错误代码定义

**认证错误（4xx）**：
- `INVALID_CREDENTIALS`: 无效的登录凭证
- `EMAIL_ALREADY_EXISTS`: 邮箱已存在
- `TOKEN_EXPIRED`: 令牌已过期
- `TOKEN_INVALID`: 令牌无效
- `UNAUTHORIZED`: 未授权访问

**业务逻辑错误（4xx）**：
- `INSUFFICIENT_BALANCE`: 金币余额不足
- `PACKAGE_NOT_FOUND`: 套餐不存在
- `PACKAGE_EXPIRED`: 套餐已过期
- `TRAFFIC_EXHAUSTED`: 流量已耗尽
- `NODE_NOT_FOUND`: 节点不存在
- `USER_DISABLED`: 用户已被禁用
- `INVALID_REFERRAL`: 无效的推荐链接
- `SELF_REFERRAL_NOT_ALLOWED`: 不允许自我推荐

**服务器错误（5xx）**：
- `INTERNAL_SERVER_ERROR`: 内部服务器错误
- `DATABASE_ERROR`: 数据库错误
- `REDIS_ERROR`: Redis 错误
- `NODE_AGENT_ERROR`: 节点代理错误

### 错误处理策略

**数据库错误**：
- 使用事务确保数据一致性
- 失败时回滚事务
- 记录错误日志
- 返回通用错误消息（不暴露内部细节）

**Redis 错误**：
- 缓存失败时降级到数据库查询
- 消息队列失败时重试（最多 3 次）
- 记录错误日志

**Node Agent 通信错误**：
- 心跳超时标记节点为离线
- 配置同步失败时重试
- 流量上报失败时缓存本地，稍后重试

**外部服务错误**：
- 支付服务失败时返回友好错误消息
- 实现重试机制（指数退避）
- 记录详细错误日志用于排查

### 输入验证

**用户输入验证**：
- 邮箱格式验证（RFC 5322）
- 密码强度验证（最少 8 字符，包含字母和数字）
- 金额范围验证（正数，不超过最大值）
- SQL 注入防护（使用参数化查询）
- XSS 防护（前端输入清理）

**API 请求验证**：
- Content-Type 验证
- 请求体大小限制（最大 1MB）
- 速率限制（每用户每分钟最多 60 请求）
- CSRF 保护（使用 CSRF 令牌）

## 测试策略

### 双重测试方法

系统测试采用单元测试和基于属性的测试相结合的方法：

**单元测试**：
- 验证特定示例和边界情况
- 测试错误条件和异常处理
- 测试组件之间的集成点
- 使用 Rust 的 `#[cfg(test)]` 模块

**基于属性的测试**：
- 验证跨所有输入的通用属性
- 通过随机化实现全面的输入覆盖
- 每个属性测试最少运行 100 次迭代
- 使用 `proptest` 或 `quickcheck` 库

两者互补且都是全面覆盖所必需的：单元测试捕获具体错误，属性测试验证通用正确性。

### 测试库选择

**Rust 后端**：
- 单元测试：内置 `#[test]` 框架
- 基于属性的测试：`proptest` crate
- HTTP 测试：`axum-test` 或 `reqwest`
- 数据库测试：`sqlx` 的测试工具

**前端（Vue 3 + TypeScript）**：
- 单元测试：Vitest
- 组件测试：Vue Test Utils
- E2E 测试：Playwright

### 属性测试配置

每个属性测试必须：
1. 运行最少 100 次迭代（由于随机化）
2. 使用注释引用设计文档中的属性
3. 标签格式：`Feature: vpn-subscription-platform, Property {number}: {property_text}`

示例：
```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    
    // Feature: vpn-subscription-platform, Property 1: 用户注册唯一性
    proptest! {
        #[test]
        fn test_user_registration_uniqueness(
            email in "[a-z]{5,10}@[a-z]{3,8}\\.com",
            password in "[A-Za-z0-9]{8,16}"
        ) {
            // 测试逻辑
        }
    }
}
```

### 测试覆盖目标

**代码覆盖率**：
- 核心业务逻辑：>90%
- API 端点：>85%
- 数据模型：>80%
- 工具函数：>95%

**功能覆盖**：
- 所有 API 端点都有单元测试
- 所有正确性属性都有属性测试
- 所有错误情况都有测试用例
- 关键用户流程有 E2E 测试

### 测试环境

**开发环境**：
- 使用 Docker Compose 启动测试数据库和 Redis
- 测试数据库与生产隔离
- 每次测试运行前重置数据库

**CI/CD 环境**：
- GitHub Actions 或 GitLab CI
- 自动运行所有测试
- 代码覆盖率报告
- 测试失败时阻止合并

### 集成测试

**API 集成测试**：
- 测试完整的请求-响应流程
- 验证数据库状态变化
- 验证 Redis 缓存行为
- 模拟外部服务（支付、邮件）

**Node Agent 集成测试**：
- 模拟 API 服务
- 测试配置同步流程
- 测试流量上报流程
- 测试心跳机制

### 性能测试

**负载测试**：
- 使用 `wrk` 或 `k6` 进行压力测试
- 测试目标：10000 并发用户
- 监控响应时间、吞吐量、错误率

**数据库性能测试**：
- 测试大数据量下的查询性能
- 优化慢查询
- 验证索引效果

## 部署架构

### Docker Compose 部署

**服务组成**：
```yaml
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: vpn_platform
      POSTGRES_USER: vpn_user
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
  
  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
  
  api:
    build: ./api
    environment:
      DATABASE_URL: postgres://vpn_user:${DB_PASSWORD}@postgres/vpn_platform
      REDIS_URL: redis://redis:6379
      JWT_SECRET: ${JWT_SECRET}
    depends_on:
      - postgres
      - redis
    ports:
      - "8080:8080"
  
  frontend:
    build: ./frontend
    ports:
      - "80:80"
    depends_on:
      - api
  
  admin:
    build: ./admin
    ports:
      - "8081:80"
    depends_on:
      - api

volumes:
  postgres_data:
  redis_data:
```

### 节点部署脚本

节点一键部署脚本 `install_node.sh`：

```bash
#!/bin/bash

# 安装 Xray-core
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install

# 下载 Node Agent
wget https://github.com/your-org/node-agent/releases/latest/download/node-agent
chmod +x node-agent

# 配置 Node Agent
cat > /etc/node-agent/config.toml <<EOF
api_url = "${API_URL}"
node_id = "${NODE_ID}"
node_secret = "${NODE_SECRET}"
xray_api_port = 10085
EOF

# 创建 systemd 服务
cat > /etc/systemd/system/node-agent.service <<EOF
[Unit]
Description=VPN Node Agent
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/node-agent
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# 启动服务
systemctl daemon-reload
systemctl enable node-agent
systemctl start node-agent

echo "Node Agent installed successfully"
```

### 环境变量配置

**API 服务环境变量**：
```
DATABASE_URL=postgres://user:pass@host/db
REDIS_URL=redis://host:6379
JWT_SECRET=your-secret-key
JWT_EXPIRATION=86400
BCRYPT_COST=12
API_PORT=8080
CORS_ORIGINS=http://localhost:3000
```

**Node Agent 环境变量**：
```
API_URL=https://api.example.com
NODE_ID=node-001
NODE_SECRET=node-secret-key
XRAY_API_PORT=10085
TRAFFIC_REPORT_INTERVAL=30
HEARTBEAT_INTERVAL=60
```

### 监控和日志

**日志收集**：
- 使用结构化日志（JSON 格式）
- 日志级别：DEBUG、INFO、WARN、ERROR
- 集中式日志收集（可选：ELK Stack）

**监控指标**：
- API 响应时间
- 数据库连接池状态
- Redis 内存使用
- 节点在线状态
- 用户活跃数
- 流量使用量

**告警规则**：
- API 错误率 > 5%
- 数据库连接池耗尽
- Redis 内存使用 > 80%
- 节点离线超过 5 分钟
- 磁盘空间 < 10%

## 安全考虑

### 认证和授权

**JWT 令牌**：
- 使用 HS256 算法签名
- 包含用户 ID、角色、过期时间
- 刷新令牌机制（7 天有效期）
- 令牌黑名单（用于登出）

**密码安全**：
- 使用 argon2id 算法哈希
- 密码强度要求：最少 8 字符，包含大小写字母、数字
- 登录失败限制：5 次失败后锁定 15 分钟

**API 授权**：
- 基于角色的访问控制（RBAC）
- 角色：user、admin
- 管理员端点需要 admin 角色

### 数据安全

**敏感数据加密**：
- 数据库连接使用 SSL/TLS
- Redis 连接使用 TLS（生产环境）
- 节点密钥加密存储

**数据备份**：
- 每日自动备份数据库
- 备份保留 30 天
- 定期测试恢复流程

### 网络安全

**HTTPS**：
- 所有前端-后端通信使用 HTTPS
- 使用 Let's Encrypt 证书
- 强制 HTTPS 重定向

**防火墙规则**：
- 仅开放必要端口（80、443、代理端口）
- 限制数据库和 Redis 访问（仅内网）
- 使用 fail2ban 防止暴力破解

**DDoS 防护**：
- 使用 Cloudflare 或类似 CDN
- 速率限制
- IP 黑名单

### 审计和合规

**审计日志**：
- 记录所有管理员操作
- 记录敏感数据访问
- 日志不可篡改（只追加）

**数据隐私**：
- 遵守 GDPR/CCPA（如适用）
- 用户数据删除功能
- 隐私政策和服务条款

## 扩展性考虑

### 水平扩展

**API 服务**：
- 无状态设计，支持多实例部署
- 使用负载均衡器（Nginx、HAProxy）
- Session 存储在 Redis

**数据库**：
- 读写分离（主从复制）
- 连接池优化
- 查询优化和索引

**Redis**：
- Redis Cluster 用于高可用
- 数据分片
- 持久化配置（AOF + RDB）

### 性能优化

**缓存策略**：
- 热点数据缓存（用户套餐、节点列表）
- 缓存失效策略（TTL + 主动失效）
- 缓存预热

**数据库优化**：
- 索引优化
- 查询优化（避免 N+1 查询）
- 分页查询
- 定期清理历史数据

**异步处理**：
- 流量统计异步处理（Redis Streams）
- 邮件发送异步处理
- 报表生成异步处理

### 未来扩展

**功能扩展**：
- 多币种支持
- 更多支付方式
- 移动应用
- 自定义域名
- 白标解决方案

**技术扩展**：
- 微服务架构
- Kubernetes 部署
- 服务网格（Istio）
- 消息队列（RabbitMQ、Kafka）
