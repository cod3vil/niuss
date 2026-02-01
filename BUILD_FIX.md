# 构建错误修复说明

## 问题
构建 admin 前端时出现语法错误：
```
Attribute name cannot contain U+0022 ("), U+0027 ('), and U+003C (<).
```

## 原因
`admin/src/views/ClashConfig.vue` 第 7 行的 `description` 属性中包含了未转义的中文双引号：
```vue
description="...使用"包含在 Clash 中"开关..."
```

## 修复
将中文双引号 `"` 替换为中文书名号 `「」`：
```vue
description="...使用「包含在 Clash 中」开关..."
```

## 现在可以重新构建

```bash
# 重新构建
docker compose build

# 或者只构建 admin
docker compose build admin

# 启动服务
docker compose up -d
```

## 验证
```bash
# 检查构建状态
docker compose ps

# 查看日志
docker compose logs admin
```
