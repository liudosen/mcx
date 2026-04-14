-- 订单主表
CREATE TABLE IF NOT EXISTS orders (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    order_no VARCHAR(64) NOT NULL UNIQUE,           -- 订单号，业务侧生成
    user_id BIGINT UNSIGNED NOT NULL,               -- 关联 wechat_users.id
    status TINYINT NOT NULL DEFAULT 0,
        -- 0=待付款, 1=待发货, 2=待收货, 3=已完成, 4=已取消
    total_amount BIGINT NOT NULL DEFAULT 0,         -- 应付金额，单位分
    paid_amount  BIGINT NOT NULL DEFAULT 0,         -- 实付金额，单位分
    discount_amount BIGINT NOT NULL DEFAULT 0,      -- 优惠金额，单位分
    remark VARCHAR(500),                            -- 买家备注
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_orders_user_id (user_id),
    INDEX idx_orders_order_no (order_no),
    INDEX idx_orders_status (status)
);

-- 订单明细表（一个订单对应多条 SKU 商品）
CREATE TABLE IF NOT EXISTS order_items (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    order_id BIGINT UNSIGNED NOT NULL,              -- 关联 orders.id
    order_no VARCHAR(64) NOT NULL,                  -- 冗余订单号，方便查询
    spu_id BIGINT UNSIGNED NOT NULL,                -- 关联 goods.id
    sku_id BIGINT UNSIGNED NOT NULL,                -- 关联 goods_skus.id
    goods_title VARCHAR(255) NOT NULL,              -- 下单时快照
    goods_image VARCHAR(1024) NOT NULL DEFAULT '',  -- 商品主图快照
    spec_info LONGTEXT NOT NULL,                    -- SKU 规格快照 JSON
    unit_price BIGINT NOT NULL DEFAULT 0,           -- 成交单价，单位分
    quantity INT NOT NULL DEFAULT 1,                -- 购买数量
    subtotal BIGINT NOT NULL DEFAULT 0,             -- 小计 = unit_price * quantity
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_order_items_order FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    INDEX idx_order_items_order_id (order_id),
    INDEX idx_order_items_order_no (order_no)
);
