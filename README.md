# VPN 订阅服务平台

一个完整的 VPN 订阅服务平台，支持用户购买流量套餐、自动获取 Clash 订阅链接，并提供管理后台进行节点和用户管理。

## 技术栈

### 后端
- **Rust** - 编程语言
- **Axum** - Web 框架
- **SQLx** - 数据库驱动
- **PostgreSQL** - 主数据库
- **Redis** - 缓存和消息队列
- **JWT** - 认证
- **Argon2** - 密码哈希

### 前端
- **Vue 3** - 前端框架
- **TypeScript** - 类型安全
- **Vite** - 构建工具
- **Pinia** - 状态管理
- **TailwindCSS** - 用户前端样式
- **Ant Design Vue** - 管理后台 UI 组件

### 代理服务
- **Xray-core** - 代理核心
- 支持协议：Shadowsocks、VMess、Trojan、Hysteria2、VLESS-Reality

## 项目结构

```
.
├── api/                    # API 服务（Rust）
│   ├── src/
│   │   ├── main.rs        # 入口文件
│   │   ├── config.rs      # 配置管理
│   │   ├── models.rs      # 数据模型
│   │   ├── db.rs          # 数据库层
│   │   ├── handlers.rs    # 路由处理
│   │   ├── middleware.rs  # 中间件
│   │   └── utils.rs       # 工具函数
│   ├── Cargo.toml
│   └── Dockerfile
├── frontend/              # 用户前端（Vue 3）
│   ├── src/
│   │   ├── main.ts
│   │   ├── App.vue
│   │   ├── router/        # 路由配置
│   │   ├── stores/        # Pinia 状态管理
│   │   ├── views/         # 页面组件
│   │   └── api/           # API 客户端
│   ├── package.json
│   └── Dockerfile
├── admin/                 # 管理后台（Vue 3）
│   ├── src/
│   │   ├── main.ts
│   │   ├── App.vue
│   │   ├── router/
│   │   ├── stores/
│   │   ├── views/
│   │   └── api/
│   ├── package.json
│   └── Dockerfile
├── node-agent/            # 节点代理程序（Rust）
│   ├── src/
│   │   ├── main.rs
│   │   └── config.rs
│   ├── Cargo.toml
│   └── Dockerfile
├── migrations/            # 数据库迁移脚本
│   └── 001_init.sql
├── docker-compose.yml     # Docker Compose 配置
├── Cargo.toml            # Rust 工作空间配置
└── README.md
```

## 快速开始

### 前置要求

- Docker 和 Docker Compose
- Rust 1.75+ (本地开发)
- Node.js 20+ (本地开发)

### 使用 Docker Compose 部署

1. 克隆仓库：
```bash
git clone <repository-url>
cd vpn-subscription-platform
```

2. 创建环境变量文件：
```bash
cp .env.example .env
# 编辑 .env 文件，设置必要的环境变量
```

3. 启动所有服务：
```bash
docker-compose up -d
```

4. 访问服务：
- 用户前端: http://localhost
- 管理后台: http://localhost:8081
- API 服务: http://localhost:8080

### 本地开发

#### 启动数据库和 Redis

```bash
docker-compose up -d postgres redis
```

#### 运行 API 服务

```bash
cd api
export DATABASE_URL="postgres://vpn_user:vpn_password@localhost:5432/vpn_platform"
export REDIS_URL="redis://localhost:6379"
export JWT_SECRET="your-secret-key"
cargo run
```

#### 运行用户前端

```bash
cd frontend
npm install
npm run dev
```

#### 运行管理后台

```bash
cd admin
npm install
npm run dev
```

## 环境变量

### API 服务

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| DATABASE_URL | PostgreSQL 连接字符串 | 必需 |
| REDIS_URL | Redis 连接字符串 | redis://127.0.0.1:6379 |
| JWT_SECRET | JWT 签名密钥 | 必需 |
| JWT_EXPIRATION | JWT 过期时间（秒） | 86400 |
| API_HOST | API 监听地址 | 0.0.0.0 |
| API_PORT | API 监听端口 | 8080 |
| CORS_ORIGINS | 允许的 CORS 源 | http://localhost:3000 |

### Node Agent

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| API_URL | API 服务地址 | 必需 |
| NODE_ID | 节点 ID | 必需 |
| NODE_SECRET | 节点密钥 | 必需 |
| XRAY_API_PORT | Xray API 端口 | 10085 |
| TRAFFIC_REPORT_INTERVAL | 流量上报间隔（秒） | 30 |
| HEARTBEAT_INTERVAL | 心跳间隔（秒） | 60 |

## 数据库迁移

数据库迁移脚本位于 `migrations/` 目录。使用 Docker Compose 启动时会自动执行。

手动执行迁移：
```bash
psql -U vpn_user -d vpn_platform -f migrations/001_init.sql
```

## 测试

### 运行 Rust 测试

```bash
cargo test
```

### 运行前端测试

```bash
cd frontend
npm run test

cd admin
npm run test
```

## 默认账号

系统初始化后会创建一个默认管理员账号：

- 邮箱: admin@example.com
- 密码: admin123

**⚠️ 请在生产环境中立即修改默认密码！**

## 开发状态

当前实现了项目的基础设施搭建（任务 1）：

- ✅ 项目目录结构
- ✅ Rust 工作空间配置
- ✅ Vue 3 项目配置
- ✅ Docker Compose 配置
- ✅ 数据库迁移脚本

后续任务将逐步实现：
- 数据模型和数据库层
- 认证和授权
- 虚拟金币系统
- 流量套餐管理
- 推荐返利系统
- 订阅链接生成
- 节点管理
- 流量统计
- 管理后台
- Node Agent

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
