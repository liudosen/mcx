-- 修改 addresses 表的 user_id 为 open_id
ALTER TABLE addresses CHANGE COLUMN user_id open_id VARCHAR(64) NOT NULL COMMENT '用户openid';
