-- 修改 addresses 表的 user_id 列类型为 VARCHAR(64) 以存储 openid
ALTER TABLE addresses MODIFY COLUMN user_id VARCHAR(64) NOT NULL COMMENT '用户ID（openid）';
