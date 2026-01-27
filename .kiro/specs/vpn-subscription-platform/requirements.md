# 需求文档：VPN 订阅服务平台

## 简介

本系统是一个完整的 VPN 订阅服务平台，支持用户购买流量套餐、自动获取 Clash 订阅链接，并提供管理后台进行节点和用户管理。系统采用 Shadowsocks/VMess/Trojan/Hysteria2 等代理协议，基于 Xray-core 实现代理服务，支持多节点部署和流量统计。

## 术语表

- **System**: VPN 订阅服务平台系统
- **User_Frontend**: 用户前端应用
- **Admin_Backend**: 管理后台应用
- **API_Service**: API 服务后端
- **Node_Agent**: 节点代理程序
- **Xray_Core**: Xray 代理核心服务
- **User**: 平台用户
- **Admin**: 平台管理员
- **Node**: VPN 服务器节点
- **Subscription_Link**: Clash 兼容的订阅链接
- **Traffic_Package**: 流量套餐
- **Virtual_Coin**: 虚拟金币
- **Referral_Link**: 推荐链接
- **Traffic_Quota**: 流量配额
- **Proxy_Protocol**: 代理协议（Shadowsocks/VMess/Trojan/Hysteria2/VLESS-Reality）
- **Reality**: Xray 的 Reality 协议，提供强抗封锁能力

## 需求

### 需求 1：用户注册与认证

**用户故事**：作为用户，我想要注册账号并登录系统，以便使用 VPN 订阅服务。

#### 验收标准

1. WHEN 用户提交注册信息（邮箱、密码），THE System SHALL 验证信息格式并创建用户账号
2. WHEN 用户提交的邮箱已存在，THE System SHALL 返回错误提示并拒绝注册
3. WHEN 用户提交登录凭证，THE System SHALL 验证凭证并返回认证令牌
4. WHEN 用户提交无效的登录凭证，THE System SHALL 返回错误提示并拒绝登录
5. WHEN 认证令牌过期，THE System SHALL 要求用户重新登录

### 需求 2：虚拟金币系统

**用户故事**：作为用户，我想要购买和使用虚拟金币，以便购买流量套餐。

#### 验收标准

1. WHEN 用户选择金币充值金额，THE System SHALL 生成支付订单并返回支付信息
2. WHEN 支付成功，THE System SHALL 增加用户的金币余额
3. WHEN 用户查询金币余额，THE System SHALL 返回当前可用金币数量
4. WHEN 用户使用金币购买套餐，THE System SHALL 扣除相应金币并更新余额
5. IF 用户金币余额不足，THEN THE System SHALL 拒绝购买请求并返回错误提示

### 需求 3：流量套餐管理

**用户故事**：作为用户，我想要购买流量套餐，以便获得 VPN 服务访问权限。

#### 验收标准

1. WHEN 用户查询可用套餐，THE System SHALL 返回所有有效的流量套餐列表（包含流量大小、价格、有效期）
2. WHEN 用户使用金币购买套餐，THE System SHALL 验证金币余额并创建订单
3. WHEN 套餐购买成功，THE System SHALL 增加用户的流量配额并记录有效期
4. WHEN 用户流量配额耗尽，THE System SHALL 阻止该用户继续使用代理服务
5. WHEN 套餐有效期到期，THE System SHALL 将该套餐标记为过期并停止提供服务

### 需求 4：推荐返利系统

**用户故事**：作为用户，我想要分享推荐链接获得返利，以便获得额外的金币奖励。

#### 验收标准

1. WHEN 用户请求推荐链接，THE System SHALL 生成唯一的推荐链接
2. WHEN 新用户通过推荐链接注册，THE System SHALL 记录推荐关系
3. WHEN 被推荐用户完成首次购买，THE System SHALL 向推荐人账户增加返利金币
4. WHEN 用户查询推荐记录，THE System SHALL 返回推荐人数和累计返利金额
5. THE System SHALL 防止用户自我推荐获取返利

### 需求 5：订阅链接生成

**用户故事**：作为用户，我想要获取 Clash 订阅链接，以便在客户端配置 VPN 连接。

#### 验收标准

1. WHEN 用户请求订阅链接，THE System SHALL 生成唯一的订阅 URL
2. WHEN 客户端访问订阅链接，THE System SHALL 验证用户身份和套餐有效性
3. WHEN 用户拥有有效套餐，THE System SHALL 返回 Clash 格式的节点配置
4. WHEN 用户套餐已过期或流量耗尽，THE System SHALL 返回空节点列表或错误信息
5. THE System SHALL 在订阅配置中包含所有可用节点的连接信息

### 需求 6：节点管理

**用户故事**：作为管理员，我想要管理 VPN 节点，以便提供稳定的代理服务。

#### 验收标准

1. WHEN 管理员添加新节点，THE System SHALL 验证节点信息并保存到数据库
2. WHEN 管理员更新节点配置，THE System SHALL 通知对应的 Node_Agent 同步配置
3. WHEN 管理员删除节点，THE System SHALL 标记节点为不可用并停止分配给用户
4. WHEN 管理员查询节点列表，THE System SHALL 返回所有节点的状态信息（在线/离线、负载、流量）
5. THE System SHALL 定期检查节点健康状态并更新节点状态

### 需求 7：流量统计与限制

**用户故事**：作为系统，我需要统计和限制用户流量，以便确保服务公平使用。

#### 验收标准

1. WHEN 用户通过代理传输数据，THE Node_Agent SHALL 记录上传和下载流量
2. WHEN Node_Agent 上报流量数据，THE System SHALL 更新用户的流量使用记录
3. WHEN 用户流量使用超过配额，THE System SHALL 阻止该用户继续使用代理服务
4. WHEN 用户查询流量使用情况，THE System SHALL 返回已使用流量和剩余流量
5. THE System SHALL 每小时汇总流量数据并持久化到数据库

### 需求 8：用户管理

**用户故事**：作为管理员，我想要管理用户账号，以便维护平台秩序。

#### 验收标准

1. WHEN 管理员查询用户列表，THE System SHALL 返回用户信息（邮箱、注册时间、套餐状态、流量使用）
2. WHEN 管理员禁用用户账号，THE System SHALL 阻止该用户登录和使用服务
3. WHEN 管理员启用被禁用的账号，THE System SHALL 恢复该用户的访问权限
4. WHEN 管理员重置用户流量，THE System SHALL 更新用户的流量配额
5. WHEN 管理员调整用户金币余额，THE System SHALL 更新用户的金币数量并记录操作日志

### 需求 9：订单管理

**用户故事**：作为管理员，我想要查看和管理订单，以便跟踪平台收入和用户购买行为。

#### 验收标准

1. WHEN 管理员查询订单列表，THE System SHALL 返回所有订单信息（用户、套餐、金额、时间、状态）
2. WHEN 管理员筛选订单，THE System SHALL 支持按时间范围、用户、状态进行过滤
3. WHEN 管理员查看订单详情，THE System SHALL 返回完整的订单信息和支付记录
4. THE System SHALL 自动统计每日、每月的订单总额和订单数量
5. WHEN 订单创建，THE System SHALL 记录订单时间戳并生成唯一订单号

### 需求 10：Node Agent 配置同步

**用户故事**：作为节点代理程序，我需要同步配置并上报状态，以便提供代理服务。

#### 验收标准

1. WHEN Node_Agent 启动，THE Node_Agent SHALL 向 API_Service 注册并获取初始配置
2. WHEN API_Service 更新节点配置，THE Node_Agent SHALL 接收配置更新通知
3. WHEN Node_Agent 接收配置更新，THE Node_Agent SHALL 应用新配置到 Xray_Core
4. WHEN Node_Agent 检测到 Xray_Core 异常，THE Node_Agent SHALL 尝试重启服务并上报错误
5. THE Node_Agent SHALL 每分钟向 API_Service 发送心跳包以维持在线状态

### 需求 11：代理协议支持

**用户故事**：作为系统，我需要支持多种代理协议，以便满足不同用户需求和抗封锁场景。

#### 验收标准

1. THE System SHALL 支持 Shadowsocks 协议的节点配置和连接
2. THE System SHALL 支持 VMess 协议的节点配置和连接
3. THE System SHALL 支持 Trojan 协议的节点配置和连接
4. THE System SHALL 支持 Hysteria2 协议的节点配置和连接
5. THE System SHALL 支持 VLESS-Reality 协议的节点配置和连接
6. WHEN 配置 Reality 节点，THE System SHALL 支持配置目标网站（dest）、服务器名称（serverNames）、公钥（publicKey）等 Reality 特定参数
7. WHEN 生成订阅配置，THE System SHALL 根据节点协议类型生成对应的 Clash 配置格式
8. THE System SHALL 允许管理员为每个节点选择最适合的协议类型

### 需求 12：数据统计与报表

**用户故事**：作为管理员，我想要查看平台数据统计，以便了解运营状况。

#### 验收标准

1. WHEN 管理员查询统计数据，THE System SHALL 返回用户总数、活跃用户数、总流量使用量
2. WHEN 管理员查询收入统计，THE System SHALL 返回指定时间段的总收入和订单数量
3. WHEN 管理员查询节点统计，THE System SHALL 返回每个节点的流量使用量和在线用户数
4. THE System SHALL 生成每日流量使用趋势图数据
5. THE System SHALL 生成每日收入趋势图数据

### 需求 13：系统部署与配置

**用户故事**：作为运维人员，我想要快速部署系统，以便启动服务。

#### 验收标准

1. THE System SHALL 提供 Docker Compose 配置文件用于一键部署所有服务
2. THE System SHALL 提供节点一键部署脚本用于快速安装 Node_Agent 和 Xray_Core
3. WHEN 执行部署脚本，THE System SHALL 自动安装依赖、配置服务并启动
4. THE System SHALL 支持通过环境变量配置数据库连接、Redis 连接等参数
5. THE System SHALL 在首次启动时自动初始化数据库表结构

### 需求 14：安全性要求

**用户故事**：作为系统，我需要保护用户数据和服务安全，以便防止未授权访问。

#### 验收标准

1. THE System SHALL 使用 bcrypt 或 argon2 算法存储用户密码哈希
2. THE System SHALL 使用 JWT 令牌进行 API 认证，令牌有效期不超过 24 小时
3. WHEN API 接收请求，THE System SHALL 验证请求来源和令牌有效性
4. THE System SHALL 对敏感操作（删除节点、禁用用户）记录审计日志
5. THE System SHALL 使用 HTTPS 加密所有前端与后端之间的通信

### 需求 15：性能要求

**用户故事**：作为系统，我需要高效处理请求，以便支持大量并发用户。

#### 验收标准

1. WHEN 系统处理订阅请求，THE System SHALL 在 200ms 内返回响应
2. WHEN 系统处理流量上报，THE System SHALL 使用 Redis 缓存减少数据库写入频率
3. THE System SHALL 支持至少 10000 个并发用户同时使用代理服务
4. THE System SHALL 使用连接池管理数据库连接以提高性能
5. THE System SHALL 使用 Redis 缓存热点数据（用户套餐状态、节点列表）以减少数据库查询
