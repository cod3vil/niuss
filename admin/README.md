# VPN 管理后台

VPN 订阅服务平台的管理后台前端应用。

## 技术栈

- Vue 3 (Composition API)
- TypeScript
- Ant Design Vue (UI 组件库)
- Pinia (状态管理)
- Vue Router (路由)
- Axios (HTTP 客户端)
- Vite (构建工具)

## 功能模块

### 1. 登录认证
- 管理员登录页面
- JWT 令牌认证
- 自动跳转和权限验证

### 2. 仪表板
- 用户总数、活跃用户统计
- 总流量、总收入展示
- 收入趋势图
- 流量趋势图
- 快速操作入口

### 3. 节点管理
- 节点列表展示
- 创建新节点
- 编辑节点配置
- 删除节点
- 节点状态监控
- 支持多种协议（Shadowsocks、VMess、Trojan、Hysteria2、VLESS-Reality）

### 4. 用户管理
- 用户列表展示
- 用户详情查看
- 启用/禁用用户
- 调整用户金币余额
- 调整用户流量配额
- 流量使用进度展示

### 5. 订单管理
- 订单列表展示
- 订单筛选（按状态、时间范围）
- 订单详情查看
- 订单状态标识

### 6. 数据统计
- 收入统计图表
- 流量统计图表
- 节点流量统计表格
- 自定义时间范围查询

## 开发

### 安装依赖

```bash
npm install
```

### 启动开发服务器

```bash
npm run dev
```

开发服务器将在 `http://localhost:3001` 启动。

### 构建生产版本

```bash
npm run build
```

构建产物将输出到 `dist` 目录。

### 预览生产构建

```bash
npm run preview
```

## 项目结构

```
admin/
├── src/
│   ├── api/           # API 客户端配置
│   ├── router/        # 路由配置
│   ├── stores/        # Pinia 状态管理
│   │   ├── auth.ts    # 认证状态
│   │   ├── dashboard.ts # 仪表板数据
│   │   ├── nodes.ts   # 节点管理
│   │   ├── users.ts   # 用户管理
│   │   ├── orders.ts  # 订单管理
│   │   └── stats.ts   # 统计数据
│   ├── views/         # 页面组件
│   │   ├── Login.vue  # 登录页面
│   │   ├── Layout.vue # 布局组件
│   │   ├── Dashboard.vue # 仪表板
│   │   ├── Nodes.vue  # 节点管理
│   │   ├── Users.vue  # 用户管理
│   │   ├── Orders.vue # 订单管理
│   │   └── Stats.vue  # 数据统计
│   ├── App.vue        # 根组件
│   └── main.ts        # 入口文件
├── index.html
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## API 配置

API 请求通过 Axios 拦截器自动添加认证令牌。API 基础 URL 配置在 `vite.config.ts` 中：

```typescript
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:8080',
      changeOrigin: true
    }
  }
}
```

## 认证流程

1. 用户在登录页面输入邮箱和密码
2. 系统验证管理员权限（role 必须为 'admin'）
3. 登录成功后，JWT 令牌存储在 localStorage
4. 所有 API 请求自动携带令牌
5. 令牌过期或无效时，自动跳转到登录页面

## 路由守卫

路由配置了导航守卫，确保：
- 未登录用户访问受保护页面时跳转到登录页
- 已登录用户访问登录页时跳转到仪表板

## 部署

### Docker 部署

使用提供的 Dockerfile 构建镜像：

```bash
docker build -t vpn-admin .
docker run -p 8081:80 vpn-admin
```

### Nginx 配置

参考 `nginx.conf` 文件配置 Nginx 反向代理。

## 注意事项

1. 管理员账号需要在数据库中手动设置 `role = 'admin'`
2. 首次登录需要确保 API 服务正常运行
3. 生产环境建议使用 HTTPS
4. 定期备份管理员操作日志
