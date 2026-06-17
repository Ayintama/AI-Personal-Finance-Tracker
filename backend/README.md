# AI 记账本后端

Rust + Actix-web + MySQL 个人财务管理系统后端

## 环境要求

- Rust 1.70+
- MySQL 8.0+
- OpenSSL (Windows 上通过 vcpkg 或安装)

## 快速开始

### 1. 初始化数据库

```bash
mysql -u root -p < init.sql
```

或者手动执行 `init.sql` 中的 SQL 语句。

### 2. 配置环境

复制 `.env.example` 为 `.env`，修改数据库连接信息：

```env
DATABASE_URL=mysql://root:your_password@localhost:3306/ai_finance
JWT_SECRET=change-this-to-a-very-long-random-secret
```

### 3. 编译运行

```bash
cargo build --release
cargo run --release
```

服务器将在 `http://127.0.0.1:8080` 启动。

## API 接口

### 认证接口

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /api/auth/register | 用户注册 |
| POST | /api/auth/login | 用户登录 |
| GET | /api/auth/profile | 获取用户信息 |
| PUT | /api/auth/profile | 更新用户信息 |

### 账单接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/transactions | 获取账单列表 |
| POST | /api/transactions | 新增账单 |
| PUT | /api/transactions/{id} | 修改账单 |
| DELETE | /api/transactions/{id} | 删除账单 |

### 分类接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/categories | 获取分类列表 |
| POST | /api/categories | 新增分类 |
| PUT | /api/categories/{id} | 修改分类 |
| DELETE | /api/categories/{id} | 删除分类 |

### 预算接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/budgets | 获取预算列表 |
| POST | /api/budgets | 设置/更新预算 |
| PUT | /api/budgets/{id} | 修改预算 |
| DELETE | /api/budgets/{id} | 删除预算 |

### 统计接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /api/statistics/monthly | 获取月度统计 |

### AI 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | /api/ai/classify | 自动分类 |
| POST | /api/ai/report | 生成月度报告 |

## 统一响应格式

```json
{
  "code": 0,
  "message": "success",
  "data": {...}
}
```

错误响应示例：

```json
{
  "code": 4001,
  "message": "Token expired",
  "data": null
}
```

## 开发

```bash
# debug 模式运行
cargo run

# 运行测试
cargo test

# 检查代码
cargo check
```
