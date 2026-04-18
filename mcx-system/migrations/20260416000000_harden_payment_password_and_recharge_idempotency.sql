ALTER TABLE wechat_users
    MODIFY payment_password VARCHAR(255) NOT NULL DEFAULT '' COMMENT '支付密码（加密存储）';

DROP INDEX idx_wechat_users_payment_password ON wechat_users;

ALTER TABLE orders
    ADD COLUMN request_hash VARCHAR(64) NULL COMMENT '充值请求幂等哈希' AFTER external_order_no;

CREATE UNIQUE INDEX idx_orders_request_hash ON orders(request_hash);

ALTER TABLE balance_transactions
    ADD COLUMN request_hash VARCHAR(64) NULL COMMENT '充值请求幂等哈希' AFTER external_order_no;

CREATE INDEX idx_balance_transactions_request_hash ON balance_transactions(request_hash);
