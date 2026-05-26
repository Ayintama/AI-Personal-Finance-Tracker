# AI 记账本 / 个人财务管理系统

一个面向个人用户的智能财务管理系统，目标是帮助用户完成日常收支记录、分类管理、预算控制、数据统计和 AI 财务分析。

项目计划采用前后端分离架构：前端使用 Vue3 展示页面和图表，后端使用 Rust + Actix-web 提供 RESTful API，数据库使用 MySQL 存储用户、账单、分类和预算数据。AI 部分采用“本地规则 + 可选大模型”的方式实现，保证系统在没有外部 AI 服务时也能稳定运行。

## 项目文档

- [需求文档](./需求文档.docx)
- [设计文档](./设计文档.docx)

## 核心功能

- 用户管理：注册、登录、JWT 鉴权、密码加密存储、用户信息维护
- 收支管理：新增、删除、修改、查询收支记录，支持按时间、类型、分类筛选
- 分类管理：收入分类、支出分类、默认分类和用户自定义分类
- 预算管理：月度预算、分类预算、预算使用进度和超预算提醒
- 数据统计：月度统计、季度统计、分类统计和图表展示
- AI 分析：自动分类、月度财务报告、消费分析和省钱建议

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 前端 | Vue3, ECharts |
| 后端 | Rust, Actix-web |
| 数据库 | MySQL |
| 认证 | JWT, bcrypt |
| AI | 关键词规则, OpenAI API（可选） |
| 部署 | Nginx, Rust Server |

## AI 模块设计

AI 模块建议分为两部分：

1. 自动分类：优先使用本地关键词规则，根据账单备注、金额和收支类型推荐分类。
2. 财务报告：后端先完成月度统计，再将脱敏后的统计摘要交给大模型生成自然语言分析。

自动分类示例：

```text
餐饮：饭、外卖、奶茶、火锅、早餐、午餐、晚餐
交通：地铁、公交、打车、滴滴、高铁、机票
购物：淘宝、京东、衣服、数码、超市
收入：工资、奖金、兼职、报销
```

AI 服务不可用时，系统应降级为本地规则分析和基础统计报告，避免核心功能失效。

## 推荐接口设计

```text
POST   /api/auth/register
POST   /api/auth/login

GET    /api/transactions
POST   /api/transactions
PUT    /api/transactions/{id}
DELETE /api/transactions/{id}

GET    /api/categories
POST   /api/categories
PUT    /api/categories/{id}
DELETE /api/categories/{id}

POST   /api/budgets
GET    /api/statistics/monthly

POST   /api/ai/classify
POST   /api/ai/report
```

统一响应格式建议：

```json
{
  "code": 0,
  "message": "success",
  "data": {}
}
```

## 数据库核心表

- `users`：用户信息，包含用户名、邮箱、密码哈希等
- `categories`：收入/支出分类，支持系统默认分类和用户自定义分类
- `transactions`：收支记录，包含金额、类型、分类、时间和备注
- `budgets`：预算记录，包含用户、月份、分类和预算金额

## 开发计划

| 阶段 | 内容 |
| --- | --- |
| 第 1 周 | 数据库设计、项目结构搭建 |
| 第 2 周 | 用户、账单、分类、预算 CRUD |
| 第 3 周 | 统计接口和图表展示 |
| 第 4 周 | AI 自动分类和月度报告 |
| 第 5 周 | 前后端联调、错误处理、测试 |
| 第 6 周 | 部署演示、答辩材料和最终文档 |

## 当前状态

目录结构：

```text
.
├── backend/      # Rust + Actix-web 后端
├── frontend/     # Vue3 前端
├── docs/         # 项目文档
├── README.md
├── 需求文档.docx
└── 设计文档.docx
```

## 小组分工

| 成员 | 职责 |
| --- | --- |
| 许铭睿 | 后端接口与数据库 |
| 凌英竣 | 统计分析与图表 |
| 周雨昇 | AI 模块 |
| 戴豪 | 前端、测试与文档 |
