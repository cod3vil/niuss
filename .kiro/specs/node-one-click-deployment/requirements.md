# 需求文档：节点一键部署功能

## 简介

本文档定义了 VPN 订阅平台节点一键部署功能的需求。该功能旨在简化节点部署流程，通过自动化脚本实现从节点创建到服务启动的全流程自动化，消除手动在管理后台创建节点的步骤，提高运维效率。

## 术语表

- **Deployment_Script**: 节点部署脚本，负责自动化部署流程的 Shell 脚本
- **Node_Agent**: 运行在节点服务器上的代理程序，负责与中心 API 通信
- **Xray_Core**: 代理服务核心程序，处理实际的流量转发
- **API_Service**: 中心 API 服务，提供节点管理接口
- **Node_Secret**: 节点密钥，用于节点与 API 服务之间的认证
- **Admin_Token**: 管理员 JWT 令牌，用于调用管理 API
- **Node_ID**: 节点唯一标识符，由 API 服务生成
- **Deployment_Config**: 部署配置文件，用于批量部署场景

## 需求

### 需求 1：脚本创建与参数处理

**用户故事：** 作为运维人员，我希望有一个统一的部署脚本，能够接受必要的参数来配置节点部署。

#### 验收标准

1. THE Deployment_Script SHALL 接受 API_URL 参数作为 API 服务地址
2. THE Deployment_Script SHALL 接受 ADMIN_TOKEN 参数作为管理员认证令牌
3. THE Deployment_Script SHALL 接受 NODE_NAME 参数作为节点名称
4. WHEN NODE_HOST 参数未提供时，THE Deployment_Script SHALL 自动检测服务器的公网 IP 地址
5. WHEN NODE_PORT 参数未提供时，THE Deployment_Script SHALL 使用默认端口 443
6. WHEN NODE_PROTOCOL 参数未提供时，THE Deployment_Script SHALL 使用默认协议 vless
7. THE Deployment_Script SHALL 支持通过命令行参数或环境变量接收配置

### 需求 2：环境检测与依赖验证

**用户故事：** 作为运维人员，我希望脚本能够自动检测系统环境，确保满足部署条件。

#### 验收标准

1. WHEN 脚本启动时，THE Deployment_Script SHALL 检测操作系统类型（Ubuntu/CentOS/Debian）
2. WHEN 脚本启动时，THE Deployment_Script SHALL 验证是否具有 root 权限
3. WHEN 脚本启动时，THE Deployment_Script SHALL 检查必需的系统命令是否可用（curl、jq、systemctl）
4. IF 缺少必需依赖，THEN THE Deployment_Script SHALL 尝试自动安装或提示用户安装
5. WHEN 检测到不支持的操作系统时，THE Deployment_Script SHALL 终止执行并显示错误信息

### 需求 3：节点密钥生成

**用户故事：** 作为运维人员，我希望脚本能够自动生成安全的随机密钥，无需手动创建。

#### 验收标准

1. THE Deployment_Script SHALL 生成长度至少为 32 字符的随机 Node_Secret
2. THE Deployment_Script SHALL 使用加密安全的随机源（/dev/urandom 或 openssl）
3. THE Deployment_Script SHALL 确保生成的 Node_Secret 包含字母数字字符
4. THE Deployment_Script SHALL 在日志中隐藏完整的 Node_Secret，仅显示前 8 个字符

### 需求 4：API 节点创建

**用户故事：** 作为运维人员，我希望脚本能够自动调用 API 创建节点记录，获取节点 ID。

#### 验收标准

1. WHEN 调用节点创建 API 时，THE Deployment_Script SHALL 使用 POST 方法访问 /api/admin/nodes 端点
2. WHEN 调用节点创建 API 时，THE Deployment_Script SHALL 在请求头中包含 Admin_Token 进行认证
3. WHEN 调用节点创建 API 时，THE Deployment_Script SHALL 发送包含 name、host、port、protocol 和 config 的 JSON 请求体
4. WHEN API 返回成功响应时，THE Deployment_Script SHALL 解析并保存返回的 Node_ID 和 Node_Secret
5. IF API 调用失败，THEN THE Deployment_Script SHALL 记录错误信息并终止部署
6. WHEN API 返回 401 或 403 错误时，THE Deployment_Script SHALL 提示 Admin_Token 无效或权限不足

### 需求 5：Xray-core 安装

**用户故事：** 作为运维人员，我希望脚本能够自动安装和配置 Xray-core。

#### 验收标准

1. THE Deployment_Script SHALL 下载最新稳定版本的 Xray-core
2. THE Deployment_Script SHALL 验证下载文件的完整性
3. THE Deployment_Script SHALL 将 Xray-core 安装到标准系统路径（/usr/local/bin）
4. THE Deployment_Script SHALL 创建 Xray-core 的 systemd 服务单元文件
5. THE Deployment_Script SHALL 根据节点协议类型生成相应的 Xray-core 配置文件

### 需求 6：Node Agent 安装

**用户故事：** 作为运维人员，我希望脚本能够自动安装和配置 Node Agent。

#### 验收标准

1. THE Deployment_Script SHALL 下载或编译 Node_Agent 程序
2. THE Deployment_Script SHALL 将 Node_Agent 安装到标准系统路径
3. THE Deployment_Script SHALL 创建 Node_Agent 的配置文件，包含 API_URL、Node_ID 和 Node_Secret
4. THE Deployment_Script SHALL 创建 Node_Agent 的 systemd 服务单元文件
5. THE Deployment_Script SHALL 配置 Node_Agent 开机自启动

### 需求 7：服务启动与验证

**用户故事：** 作为运维人员，我希望脚本能够启动服务并验证部署是否成功。

#### 验收标准

1. WHEN 安装完成后，THE Deployment_Script SHALL 启动 Xray_Core 服务
2. WHEN 安装完成后，THE Deployment_Script SHALL 启动 Node_Agent 服务
3. WHEN 服务启动后，THE Deployment_Script SHALL 检查服务状态是否为 active (running)
4. WHEN 服务启动后，THE Deployment_Script SHALL 验证服务端口是否正在监听
5. WHEN 服务启动后，THE Deployment_Script SHALL 检查 Node_Agent 是否成功连接到 API_Service
6. IF 任何服务启动失败，THEN THE Deployment_Script SHALL 显示详细的错误日志并提供故障排查建议

### 需求 8：批量部署支持

**用户故事：** 作为运维人员，我希望能够通过配置文件批量部署多个节点。

#### 验收标准

1. THE Deployment_Script SHALL 支持从 Deployment_Config 文件读取多个节点配置
2. WHEN 使用批量部署模式时，THE Deployment_Script SHALL 按顺序部署每个节点
3. WHEN 批量部署时，THE Deployment_Script SHALL 为每个节点生成独立的 Node_Secret
4. WHEN 批量部署时，THE Deployment_Script SHALL 记录每个节点的部署结果（成功/失败）
5. WHEN 某个节点部署失败时，THE Deployment_Script SHALL 继续部署剩余节点
6. WHEN 批量部署完成后，THE Deployment_Script SHALL 生成部署摘要报告

### 需求 9：节点更新与重新部署

**用户故事：** 作为运维人员，我希望能够更新现有节点的配置或重新部署节点。

#### 验收标准

1. WHEN 检测到节点已存在时，THE Deployment_Script SHALL 询问用户是否更新现有节点
2. WHEN 用户选择更新时，THE Deployment_Script SHALL 保留现有的 Node_ID 和 Node_Secret
3. WHEN 用户选择重新部署时，THE Deployment_Script SHALL 停止现有服务
4. WHEN 重新部署时，THE Deployment_Script SHALL 备份现有配置文件
5. WHEN 更新完成后，THE Deployment_Script SHALL 重启相关服务
6. THE Deployment_Script SHALL 支持 --force 参数强制重新部署而不询问

### 需求 10：日志记录与错误处理

**用户故事：** 作为运维人员，我希望脚本能够提供详细的日志输出，便于问题排查。

#### 验收标准

1. THE Deployment_Script SHALL 将所有操作记录到日志文件（/var/log/node-deployment.log）
2. THE Deployment_Script SHALL 在控制台输出彩色的进度信息（INFO/WARN/ERROR）
3. WHEN 发生错误时，THE Deployment_Script SHALL 记录详细的错误堆栈和上下文信息
4. WHEN 发生错误时，THE Deployment_Script SHALL 提供可能的解决方案建议
5. THE Deployment_Script SHALL 支持 --verbose 参数启用详细调试输出
6. THE Deployment_Script SHALL 支持 --quiet 参数仅输出错误信息

### 需求 11：幂等性与回滚机制

**用户故事：** 作为运维人员，我希望脚本可以安全地重复执行，并在失败时能够回滚。

#### 验收标准

1. WHEN 脚本重复执行时，THE Deployment_Script SHALL 检测已安装的组件并跳过重复安装
2. WHEN 脚本重复执行时，THE Deployment_Script SHALL 更新配置文件而不是覆盖
3. IF 部署过程中发生错误，THEN THE Deployment_Script SHALL 停止所有已启动的服务
4. IF 部署过程中发生错误，THEN THE Deployment_Script SHALL 恢复备份的配置文件
5. IF 部署过程中发生错误，THEN THE Deployment_Script SHALL 提供清理命令以完全移除部署
6. THE Deployment_Script SHALL 支持 --rollback 参数回滚到上一个稳定版本

### 需求 12：安全性要求

**用户故事：** 作为系统管理员，我希望部署过程是安全的，不会泄露敏感信息。

#### 验收标准

1. THE Deployment_Script SHALL 验证 Admin_Token 的格式和有效性
2. THE Deployment_Script SHALL 确保配置文件的权限设置为 600（仅所有者可读写）
3. THE Deployment_Script SHALL 在日志中屏蔽敏感信息（token、secret、密码）
4. THE Deployment_Script SHALL 使用 HTTPS 与 API_Service 通信
5. WHEN 使用配置文件时，THE Deployment_Script SHALL 验证文件权限不得过于宽松
6. THE Deployment_Script SHALL 在完成后清理临时文件中的敏感信息
