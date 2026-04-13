use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ─── DB row structs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct GoodsRow {
    pub id: u64,
    pub store_id: String,
    pub saas_id: String,
    pub title: String,
    pub primary_image: String,
    pub images: String,      // JSON
    pub desc_images: String, // JSON
    pub spec_list: String,   // JSON
    pub min_sale_price: i64,
    pub max_line_price: i64,
    pub spu_tag_list: String, // JSON
    pub is_sold_out: bool,
    pub spu_stock_quantity: i32,
    pub sold_num: i32,
    pub category_id: Option<String>,
    pub status: bool,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct GoodsSkuRow {
    pub id: u64,
    pub spu_id: u64,
    pub sku_image: Option<String>,
    pub spec_info: String, // JSON
    pub sale_price: i64,
    pub line_price: i64,
    pub stock_quantity: i32,
}

// ─── API response types (matches mini-app spec) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpuTag {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecValue {
    #[serde(rename = "specValueId")]
    pub spec_value_id: String,
    #[serde(rename = "specValue")]
    pub spec_value: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    #[serde(rename = "specId")]
    pub spec_id: String,
    pub title: String,
    #[serde(rename = "specValueList")]
    pub spec_value_list: Vec<SpecValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuSpecInfo {
    #[serde(rename = "specId")]
    pub spec_id: String,
    #[serde(rename = "specValueId")]
    pub spec_value_id: String,
    #[serde(rename = "specValue")]
    pub spec_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuPriceInfo {
    #[serde(rename = "priceType")]
    pub price_type: i32, // 1 = 销售价, 2 = 划线价
    pub price: String, // 单位分，整数字符串
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuStockInfo {
    #[serde(rename = "stockQuantity")]
    pub stock_quantity: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GoodsSku {
    #[serde(rename = "skuId")]
    pub sku_id: String,
    #[serde(rename = "skuImage")]
    pub sku_image: Option<String>,
    #[serde(rename = "specInfo")]
    pub spec_info: Vec<SkuSpecInfo>,
    #[serde(rename = "priceInfo")]
    pub price_info: Vec<SkuPriceInfo>,
    #[serde(rename = "stockInfo")]
    pub stock_info: SkuStockInfo,
}

/// 商品列表项（小程序列表接口返回）
#[derive(Debug, Clone, Serialize)]
pub struct GoodsListItem {
    #[serde(rename = "spuId")]
    pub spu_id: String,
    #[serde(rename = "storeId")]
    pub store_id: String,
    pub title: String,
    #[serde(rename = "primaryImage")]
    pub primary_image: String,
    pub images: Vec<String>,
    #[serde(rename = "minSalePrice")]
    pub min_sale_price: i64,
    #[serde(rename = "maxLinePrice")]
    pub max_line_price: i64,
    #[serde(rename = "spuTagList")]
    pub spu_tag_list: Vec<SpuTag>,
    #[serde(rename = "isSoldOut")]
    pub is_sold_out: bool,
    #[serde(rename = "spuStockQuantity")]
    pub spu_stock_quantity: i32,
    #[serde(rename = "soldNum")]
    pub sold_num: i32,
}

/// 商品详情（在列表基础上追加）
#[derive(Debug, Clone, Serialize)]
pub struct GoodsDetail {
    #[serde(rename = "spuId")]
    pub spu_id: String,
    #[serde(rename = "storeId")]
    pub store_id: String,
    #[serde(rename = "saasId")]
    pub saas_id: String,
    pub title: String,
    #[serde(rename = "primaryImage")]
    pub primary_image: String,
    pub images: Vec<String>,
    #[serde(rename = "minSalePrice")]
    pub min_sale_price: i64,
    #[serde(rename = "maxLinePrice")]
    pub max_line_price: i64,
    #[serde(rename = "spuTagList")]
    pub spu_tag_list: Vec<SpuTag>,
    #[serde(rename = "isSoldOut")]
    pub is_sold_out: bool,
    #[serde(rename = "spuStockQuantity")]
    pub spu_stock_quantity: i32,
    #[serde(rename = "soldNum")]
    pub sold_num: i32,
    pub desc: Vec<String>,
    #[serde(rename = "specList")]
    pub spec_list: Vec<Spec>,
    #[serde(rename = "skuList")]
    pub sku_list: Vec<GoodsSku>,
}

// ─── Admin request / response types ──────────────────────────────────────────

/// SKU 创建/更新请求
#[derive(Debug, Clone, Deserialize)]
pub struct SkuRequest {
    pub sku_image: Option<String>,
    pub spec_info: Vec<SkuSpecInfo>,
    pub sale_price: i64,
    pub line_price: i64,
    pub stock_quantity: i32,
}

/// 商品创建请求（管理后台）
#[derive(Debug, Clone, Deserialize)]
pub struct CreateGoodsRequest {
    pub store_id: Option<String>,
    pub saas_id: Option<String>,
    pub title: String,
    pub primary_image: String,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub desc_images: Vec<String>,
    #[serde(default)]
    pub spec_list: Vec<Spec>,
    #[serde(default)]
    pub spu_tag_list: Vec<SpuTag>,
    pub category_id: Option<String>,
    #[serde(default)]
    pub skus: Vec<SkuRequest>,
}

/// 商品更新请求（管理后台）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGoodsRequest {
    pub store_id: Option<String>,
    pub saas_id: Option<String>,
    pub title: Option<String>,
    pub primary_image: Option<String>,
    pub images: Option<Vec<String>>,
    pub desc_images: Option<Vec<String>>,
    pub spec_list: Option<Vec<Spec>>,
    pub spu_tag_list: Option<Vec<SpuTag>>,
    pub category_id: Option<String>,
    pub status: Option<bool>,
    /// 如果提供，会完整替换 SKU 列表
    pub skus: Option<Vec<SkuRequest>>,
}

/// 管理后台查看的商品详情（包含完整字段）
#[derive(Debug, Clone, Serialize)]
pub struct AdminGoodsDetail {
    pub spu_id: String,
    pub store_id: String,
    pub saas_id: String,
    pub title: String,
    pub primary_image: String,
    pub images: Vec<String>,
    pub desc_images: Vec<String>,
    pub spec_list: Vec<Spec>,
    pub min_sale_price: i64,
    pub max_line_price: i64,
    pub spu_tag_list: Vec<SpuTag>,
    pub is_sold_out: bool,
    pub spu_stock_quantity: i32,
    pub sold_num: i32,
    pub category_id: Option<String>,
    pub status: bool,
    pub skus: Vec<AdminSkuDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminSkuDetail {
    pub sku_id: String,
    pub sku_image: Option<String>,
    pub spec_info: Vec<SkuSpecInfo>,
    pub sale_price: i64,
    pub line_price: i64,
    pub stock_quantity: i32,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

pub fn build_goods_list_item(row: &GoodsRow) -> GoodsListItem {
    GoodsListItem {
        spu_id: row.id.to_string(),
        store_id: row.store_id.clone(),
        title: row.title.clone(),
        primary_image: row.primary_image.clone(),
        images: serde_json::from_str(&row.images).unwrap_or_default(),
        min_sale_price: row.min_sale_price,
        max_line_price: row.max_line_price,
        spu_tag_list: serde_json::from_str(&row.spu_tag_list).unwrap_or_default(),
        is_sold_out: row.is_sold_out,
        spu_stock_quantity: row.spu_stock_quantity,
        sold_num: row.sold_num,
    }
}

pub fn build_goods_detail(row: &GoodsRow, skus: Vec<GoodsSkuRow>) -> GoodsDetail {
    let spec_list: Vec<Spec> = serde_json::from_str(&row.spec_list).unwrap_or_default();
    let sku_list = skus.iter().map(build_goods_sku).collect();
    GoodsDetail {
        spu_id: row.id.to_string(),
        store_id: row.store_id.clone(),
        saas_id: row.saas_id.clone(),
        title: row.title.clone(),
        primary_image: row.primary_image.clone(),
        images: serde_json::from_str(&row.images).unwrap_or_default(),
        min_sale_price: row.min_sale_price,
        max_line_price: row.max_line_price,
        spu_tag_list: serde_json::from_str(&row.spu_tag_list).unwrap_or_default(),
        is_sold_out: row.is_sold_out,
        spu_stock_quantity: row.spu_stock_quantity,
        sold_num: row.sold_num,
        desc: serde_json::from_str(&row.desc_images).unwrap_or_default(),
        spec_list,
        sku_list,
    }
}

pub fn build_goods_sku(row: &GoodsSkuRow) -> GoodsSku {
    let spec_info: Vec<SkuSpecInfo> = serde_json::from_str(&row.spec_info).unwrap_or_default();
    GoodsSku {
        sku_id: row.id.to_string(),
        sku_image: row.sku_image.clone(),
        spec_info,
        price_info: vec![
            SkuPriceInfo {
                price_type: 1,
                price: row.sale_price.to_string(),
            },
            SkuPriceInfo {
                price_type: 2,
                price: row.line_price.to_string(),
            },
        ],
        stock_info: SkuStockInfo {
            stock_quantity: row.stock_quantity,
        },
    }
}

pub fn build_admin_goods_detail(row: &GoodsRow, skus: Vec<GoodsSkuRow>) -> AdminGoodsDetail {
    let admin_skus = skus
        .iter()
        .map(|s| AdminSkuDetail {
            sku_id: s.id.to_string(),
            sku_image: s.sku_image.clone(),
            spec_info: serde_json::from_str(&s.spec_info).unwrap_or_default(),
            sale_price: s.sale_price,
            line_price: s.line_price,
            stock_quantity: s.stock_quantity,
        })
        .collect();
    AdminGoodsDetail {
        spu_id: row.id.to_string(),
        store_id: row.store_id.clone(),
        saas_id: row.saas_id.clone(),
        title: row.title.clone(),
        primary_image: row.primary_image.clone(),
        images: serde_json::from_str(&row.images).unwrap_or_default(),
        desc_images: serde_json::from_str(&row.desc_images).unwrap_or_default(),
        spec_list: serde_json::from_str(&row.spec_list).unwrap_or_default(),
        min_sale_price: row.min_sale_price,
        max_line_price: row.max_line_price,
        spu_tag_list: serde_json::from_str(&row.spu_tag_list).unwrap_or_default(),
        is_sold_out: row.is_sold_out,
        spu_stock_quantity: row.spu_stock_quantity,
        sold_num: row.sold_num,
        category_id: row.category_id.clone(),
        status: row.status,
        skus: admin_skus,
    }
}
