-- 添加身份证号字段到微信用户表
ALTER TABLE wechat_users ADD COLUMN id_card_number VARCHAR(18) DEFAULT '' COMMENT '身份证号';

-- 为身份证号字段添加索引（用于查询去重）
CREATE INDEX idx_wechat_users_id_card_number ON wechat_users(id_card_number);
