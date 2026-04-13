-- 将nickname字段改为real_name
ALTER TABLE wechat_users CHANGE COLUMN nickname real_name VARCHAR(64) DEFAULT '' COMMENT '真实姓名';
