# AI 记账本 / AI Personal Finance Tracker

一个前后端分离的个人财务管理系统，支持用户登录、账单管理、预算管理、月度统计、AI 规则分析，以及 CSV / Excel 表格导入导出。

## 项目概述

本项目用于个人记账与课程项目展示。系统采用 Rust 后端 + MySQL 数据库 + 原生 HTML/CSS/JavaScript 前端实现，前端通过 REST API 与后端通信。

当前版本已经完成前后端联调，支持从网页登录后直接读取和写入后端数据库。

## 技术栈

| 模块 | 技术 |
| --- | --- |
| 前端 | HTML, CSS, JavaScript, Fetch API |
| 后端 | Rust, Actix-web |
| 数据库 | MySQL |
| 认证 | JWT |
| 密码加密 | bcrypt |
| 数据访问 | sqlx |
| AI 功能 | 本地关键词规则 |

## 已实现功能

- 用户注册、登录、退出
- JWT 登录鉴权
- 账单新增、查询、删除
- 账单批量选择与批量删除
- 收入 / 支出分类
- 月度预算设置与进度展示
- 月度收入、支出、结余统计
- 分类支出统计
- AI 自动分类
- AI 月度财务报告
- CSV / Excel 可打开的 `.xls` 导入导出
- 支持喵喵记账 CSV 表头导入：`分类、时间、金额、账户、账本、货币、备注`

## 项目结构

```text
AI-Personal-Finance-Tracker/
├── backend/
│   ├── src/
│   │   ├── main.rs          # 后端入口
│   │   ├── config.rs        # 环境变量配置
│   │   ├── data.rs          # 数据库访问
│   │   ├── middleware.rs    # JWT 中间件
│   │   ├── response.rs      # 统一响应与错误处理
│   │   └── routes.rs        # API 路由
│   ├── init.sql             # 数据库初始化脚本
│   ├── Cargo.toml           # Rust 依赖配置
│   └── .env.example         # 环境变量示例
│
├── frontend/
│   ├── index.html           # 页面结构
│   ├── app.js               # 页面逻辑与 API 调用
│   └── styles.css           # 页面样式
│
├── docs/                    # 项目文档
└── README.md
```

## 环境要求

- Rust 1.75+，推荐使用 [rustup](https://rustup.rs/) 安装
- MySQL 5.7+ 或 MySQL 8.0+
- 一个静态文件服务器，例如 Python 自带的 `http.server`

## 快速启动

### 1. 克隆项目

```bash
git clone https://github.com/Ayintama/AI-Personal-Finance-Tracker
cd AI-Personal-Finance-Tracker
```

### 2. 初始化数据库

确认 MySQL 已启动，然后执行：

```bash
mysql -u root -p < backend/init.sql
```

初始化脚本会创建数据库 `ai_finance`，并写入默认分类和测试账号。

测试账号：

```text
用户名：test
密码：test123456
```

### 3. 配置后端环境变量

```bash
cd backend
cp .env.example .env
```

根据本机 MySQL 用户名和密码修改 `backend/.env`：

```env
DATABASE_URL=mysql://root:password@localhost:3306/ai_finance
JWT_SECRET=your-super-secret-jwt-key-change-in-production
JWT_EXPIRY_HOURS=24
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
AI_ENABLED=false
```

### 4. 启动后端

在 `backend/` 目录执行：

```bash
cargo run
```

后端默认运行在：

```text
http://127.0.0.1:8080
```

可以用下面命令检查后端是否启动成功：

```bash
curl http://127.0.0.1:8080/health
```

正常返回：

```json
{ "status": "ok" }
```

### 5. 启动前端

新开一个终端，回到项目根目录：

```bash
cd AI-Personal-Finance-Tracker
python3 -m http.server 8081 --bind 127.0.0.1
```

然后在浏览器访问：

```text
http://127.0.0.1:8081/frontend/index.html
```

也可以直接打开 `frontend/index.html`，但推荐通过静态服务器访问，便于浏览器正确加载资源。

## 使用说明

1. 启动 MySQL。
2. 启动后端服务。
3. 启动前端静态服务器。
4. 打开前端页面。
5. 使用测试账号登录，或注册新账号。
6. 在“账单”页面新增、导入、导出或批量删除账单。
7. 在“预算”页面设置月度预算。
8. 在“AI 报告”页面生成月度财务报告。

## 表格导入说明

当前支持两类表格：

- CSV 文件：`.csv`
- Excel 可打开的网页表格：`.xls`

推荐表头：

```text
日期, 类型, 分类, 金额, 备注
```

也兼容喵喵记账导出的 CSV 表头：

```text
分类, 时间, 金额, 账户, 账本, 货币, 备注
```

导入规则：

- `时间` 或 `日期` 会作为账单日期。
- `金额` 支持正数或负数，入库时会取绝对值。
- 如果表格没有 `类型`，系统会根据分类判断收入或支出。
- `账户`、`账本`、`货币` 当前会自动忽略。
- 如果分类无法匹配，系统会使用默认分类兜底。

## API 接口

| 方法 | 路径 | 需要登录 | 说明 |
| --- | --- | --- | --- |
| `GET` | `/health` | 否 | 健康检查 |
| `POST` | `/api/auth/register` | 否 | 用户注册 |
| `POST` | `/api/auth/login` | 否 | 用户登录 |
| `GET` | `/api/auth/profile` | 是 | 获取用户信息 |
| `GET` | `/api/categories` | 是 | 获取分类 |
| `GET` | `/api/transactions` | 是 | 获取账单 |
| `POST` | `/api/transactions` | 是 | 新增账单 |
| `PUT` | `/api/transactions/{id}` | 是 | 修改账单 |
| `DELETE` | `/api/transactions/{id}` | 是 | 删除账单 |
| `GET` | `/api/budgets` | 是 | 获取预算 |
| `POST` | `/api/budgets` | 是 | 设置或更新预算 |
| `DELETE` | `/api/budgets/{id}` | 是 | 删除预算 |
| `GET` | `/api/statistics/monthly` | 是 | 月度统计 |
| `POST` | `/api/ai/classify` | 是 | AI 自动分类 |
| `POST` | `/api/ai/report` | 是 | AI 月度报告 |

## 统一响应格式

成功响应：

```json
{
  "code": 0,
  "message": "success",
  "data": {}
}
```

失败响应：

```json
{
  "code": 4001,
  "message": "错误信息",
  "data": null
}
```

## 常见问题

### 登录后提示连接后端失败

请确认后端已经启动，并且端口是 `8080`：

```bash
curl http://127.0.0.1:8080/health
```

### 导入后页面看不到账单

请检查页面右上角选择的月份。导入的账单会按实际日期保存，如果导入的是历史账单，需要切换到对应月份查看。

### 修改代码后页面没有变化

浏览器可能缓存了旧的 `app.js` 或 `styles.css`，请强制刷新页面。

## 项目成员职责

| 成员 | 职责 |
| --- | --- |
| 许铭睿 | 后端接口与数据库 |
| 凌英竣 | 统计分析与图表 |
| 周雨昇 | AI 模块 |
| 戴豪 | 前端、测试与文档 |

## 安全说明

- 密码使用 bcrypt 哈希存储。
- 后端使用 JWT 进行接口鉴权。
- SQL 查询使用参数化方式，避免 SQL 注入。
- `.env` 文件包含敏感配置，不应提交到 GitHub。

## 当前状态

当前项目已完成基础可运行版本，适合本地运行、课程演示和继续扩展。后续可以继续补充分类管理页面、真实大模型 API、图表库和部署配置。
