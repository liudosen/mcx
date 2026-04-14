CREATE TABLE IF NOT EXISTS balance_transactions (
    id                BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    openid            VARCHAR(64) NOT NULL   COMMENT '微信 openid',
    amount            BIGINT NOT NULL        COMMENT '变动金额（正=充值），单位分',
    balance_after     BIGINT NOT NULL        COMMENT '变动后余额，单位分',
    `type`            TINYINT NOT NULL       COMMENT '1=自动充值',
    external_order_no VARCHAR(128) NULL      COMMENT 'jk.cn 外部订单号',
    status            TINYINT NOT NULL DEFAULT 0 COMMENT '0=失败 1=成功',
    remark            VARCHAR(500) NULL      COMMENT '备注/失败原因',
    created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_balance_transactions_openid (openid),
    INDEX idx_balance_transactions_status (status)
);
