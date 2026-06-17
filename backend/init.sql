-- AI 记账本数据库初始化脚本
CREATE DATABASE IF NOT EXISTS ai_finance CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
USE ai_finance;

CREATE TABLE IF NOT EXISTS users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    username VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    email VARCHAR(100) UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_username (username)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS categories (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_id BIGINT NULL,
    name VARCHAR(50) NOT NULL,
    category_type VARCHAR(20) NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_user_id (user_id),
    INDEX idx_category_type (category_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS transactions (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_id BIGINT NOT NULL,
    category_id BIGINT NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    transaction_type VARCHAR(20) NOT NULL,
    occurred_at DATETIME NOT NULL,
    remark VARCHAR(255),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_user_id (user_id),
    INDEX idx_category_id (category_id),
    INDEX idx_transaction_type (transaction_type),
    INDEX idx_occurred_at (occurred_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS budgets (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_id BIGINT NOT NULL,
    category_id BIGINT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    month CHAR(7) NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_user_id (user_id),
    INDEX idx_category_id (category_id),
    INDEX idx_month (month),
    UNIQUE KEY uk_user_category_month (user_id, category_id, month)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 默认支出分类
INSERT INTO categories (user_id, name, category_type) VALUES
(NULL, '餐饮', 'expense'),
(NULL, '交通', 'expense'),
(NULL, '购物', 'expense'),
(NULL, '学习', 'expense'),
(NULL, '娱乐', 'expense'),
(NULL, '医疗', 'expense'),
(NULL, '住房', 'expense'),
(NULL, '其他', 'expense');

-- 默认收入分类
INSERT INTO categories (user_id, name, category_type) VALUES
(NULL, '工资', 'income'),
(NULL, '奖金', 'income'),
(NULL, '兼职', 'income'),
(NULL, '报销', 'income'),
(NULL, '理财', 'income'),
(NULL, '其他收入', 'income');

-- 测试账号（密码: test123456）
INSERT INTO users (username, password_hash, email) VALUES
('test', '$2b$12$s3VzbkNeZMeJG.jxSYS11upqGRX8A5WKJO2zvTj9BKETMktaWZcJC', 'test@example.com');
