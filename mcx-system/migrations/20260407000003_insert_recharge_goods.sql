-- 充值虚拟商品（status=0 不上架，仅供充值订单使用）
-- 使用固定 ID=1 占位，通过 INSERT IGNORE 避免重复执行
INSERT IGNORE INTO goods (id, title, primary_image, images, desc_images, spec_list,
    min_sale_price, max_line_price, spu_tag_list, is_sold_out, spu_stock_quantity, status)
VALUES (1, '储值充值', '', '[]', '[]', '[]', 0, 0, '[]', 0, 999999, 0);

-- 充值商品对应的 SKU（sale_price=0，实际金额在创建订单时按充值金额写入快照）
INSERT IGNORE INTO goods_skus (id, spu_id, spec_info, sale_price, line_price, stock_quantity)
VALUES (1, 1, '[]', 0, 0, 999999);
