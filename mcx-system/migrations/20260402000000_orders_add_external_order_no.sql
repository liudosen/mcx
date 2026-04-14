ALTER TABLE orders ADD COLUMN external_order_no VARCHAR(64) NULL COMMENT 'jk.cn 外部订单号' AFTER order_no;
