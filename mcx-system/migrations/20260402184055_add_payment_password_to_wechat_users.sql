-- 添加支付密码字段到微信用户表
ALTER TABLE wechat_users
ADD COLUMN payment_password VARCHAR(32) DEFAULT '' COMMENT '支付密码（健康卡密码）' AFTER id_card_number;

CREATE INDEX idx_wechat_users_payment_password ON wechat_users(payment_password);
