CREATE TABLE IF NOT EXISTS wechat_users (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    openid VARCHAR(64) NOT NULL UNIQUE COMMENT '微信openid',
    nickname VARCHAR(64) DEFAULT '' COMMENT '用户昵称',
    avatar_url VARCHAR(512) DEFAULT '' COMMENT '头像URL',
    phone VARCHAR(20) DEFAULT '' COMMENT '手机号',
    country VARCHAR(50) DEFAULT '' COMMENT '国家',
    province VARCHAR(50) DEFAULT '' COMMENT '省份',
    city VARCHAR(50) DEFAULT '' COMMENT '城市',
    gender TINYINT(1) DEFAULT 0 COMMENT '性别 0未知 1男 2女',
    status TINYINT(1) DEFAULT 1 COMMENT '状态 0禁用 1正常',
    last_login_at DATETIME DEFAULT NULL COMMENT '最后登录时间',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE INDEX idx_wechat_users_openid ON wechat_users(openid);
CREATE INDEX idx_wechat_users_phone ON wechat_users(phone);
CREATE INDEX idx_wechat_users_status ON wechat_users(status);
CREATE INDEX idx_wechat_users_created_at ON wechat_users(created_at);
