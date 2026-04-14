CREATE TABLE IF NOT EXISTS subscription_records (
    id         BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    openid     VARCHAR(64) NOT NULL COMMENT '微信 openid',
    action     TINYINT NOT NULL DEFAULT 0 COMMENT '0=关闭 1=开启',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_subscription_records_openid (openid),
    INDEX idx_subscription_records_created_at (created_at)
);
