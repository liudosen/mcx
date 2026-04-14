-- 商品SPU表（Standard Product Unit，标准化产品单元）
CREATE TABLE IF NOT EXISTS goods (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    store_id VARCHAR(64) NOT NULL DEFAULT '',
    saas_id VARCHAR(64) NOT NULL DEFAULT '',
    title VARCHAR(255) NOT NULL,
    primary_image VARCHAR(1024) NOT NULL DEFAULT '',
    images LONGTEXT NOT NULL,      -- JSON string[]
    desc_images LONGTEXT NOT NULL, -- JSON string[] 详情图
    spec_list LONGTEXT NOT NULL,   -- JSON specList
    min_sale_price BIGINT NOT NULL DEFAULT 0,   -- 最低销售价，单位分
    max_line_price BIGINT NOT NULL DEFAULT 0,   -- 最高划线价，单位分
    spu_tag_list LONGTEXT NOT NULL, -- JSON [{id, title}]
    is_sold_out TINYINT(1) NOT NULL DEFAULT 0,
    spu_stock_quantity INT NOT NULL DEFAULT 0,
    sold_num INT NOT NULL DEFAULT 0,
    category_id VARCHAR(64),
    status TINYINT(1) NOT NULL DEFAULT 1,       -- 1=上架, 0=下架
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

-- 商品SKU表
CREATE TABLE IF NOT EXISTS goods_skus (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    spu_id BIGINT UNSIGNED NOT NULL,
    sku_image VARCHAR(1024),
    spec_info LONGTEXT NOT NULL, -- JSON [{specId, specValueId, specValue}]
    sale_price BIGINT NOT NULL DEFAULT 0,     -- 销售价，单位分
    line_price BIGINT NOT NULL DEFAULT 0,     -- 划线价，单位分
    stock_quantity INT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    CONSTRAINT fk_goods_skus_spu FOREIGN KEY (spu_id) REFERENCES goods(id) ON DELETE CASCADE
);

CREATE INDEX idx_goods_category_id ON goods(category_id);
CREATE INDEX idx_goods_status ON goods(status);
CREATE INDEX idx_goods_skus_spu_id ON goods_skus(spu_id);
