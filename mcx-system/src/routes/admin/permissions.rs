use crate::models::{PermissionCatalogResponse, PermissionGroup, PermissionItem};

pub const DASHBOARD_VIEW: &str = "dashboard:view";
pub const PRODUCT_LIST_VIEW: &str = "product:list:view";
pub const CATEGORY_LIST_VIEW: &str = "category:list:view";
pub const WECHAT_USER_LIST_VIEW: &str = "wechat-user:list:view";
pub const WECHAT_USER_PASSWORD_VIEW: &str = "wechat-user:password:view";
pub const ORDER_LIST_VIEW: &str = "order:list:view";
pub const ADMIN_USER_VIEW: &str = "admin:user:view";
pub const LOGS_VIEW: &str = "logs:view";
pub const GOODS_VIEW: &str = "goods:view";
pub const SUBSCRIPTION_VIEW: &str = "subscription:view";
pub const SUBSCRIPTION_RECORD_VIEW: &str = "subscription:record:view";

pub fn all_permission_codes() -> Vec<String> {
    [
        DASHBOARD_VIEW,
        PRODUCT_LIST_VIEW,
        CATEGORY_LIST_VIEW,
        WECHAT_USER_LIST_VIEW,
        WECHAT_USER_PASSWORD_VIEW,
        ORDER_LIST_VIEW,
        ADMIN_USER_VIEW,
        LOGS_VIEW,
        GOODS_VIEW,
        SUBSCRIPTION_VIEW,
        SUBSCRIPTION_RECORD_VIEW,
    ]
    .into_iter()
    .map(|code| code.to_string())
    .collect()
}

pub fn permission_catalog() -> PermissionCatalogResponse {
    PermissionCatalogResponse {
        groups: vec![
            PermissionGroup {
                name: "首页与系统".to_string(),
                items: vec![
                    PermissionItem {
                        code: DASHBOARD_VIEW.to_string(),
                        name: "首页".to_string(),
                        description: "访问管理后台首页仪表盘".to_string(),
                    },
                    PermissionItem {
                        code: ADMIN_USER_VIEW.to_string(),
                        name: "授权管理".to_string(),
                        description: "查看和修改管理账号权限".to_string(),
                    },
                    PermissionItem {
                        code: LOGS_VIEW.to_string(),
                        name: "日志中心".to_string(),
                        description: "查看系统日志与最近请求".to_string(),
                    },
                ],
            },
            PermissionGroup {
                name: "商品与内容".to_string(),
                items: vec![
                    PermissionItem {
                        code: PRODUCT_LIST_VIEW.to_string(),
                        name: "商品管理".to_string(),
                        description: "访问商品列表与维护入口".to_string(),
                    },
                    PermissionItem {
                        code: CATEGORY_LIST_VIEW.to_string(),
                        name: "分类管理".to_string(),
                        description: "访问商品分类列表与维护入口".to_string(),
                    },
                    PermissionItem {
                        code: GOODS_VIEW.to_string(),
                        name: "权益商品".to_string(),
                        description: "访问后台权益商品页".to_string(),
                    },
                    PermissionItem {
                        code: SUBSCRIPTION_VIEW.to_string(),
                        name: "订阅与充值".to_string(),
                        description: "访问订阅、充值和余额相关页面".to_string(),
                    },
                    PermissionItem {
                        code: SUBSCRIPTION_RECORD_VIEW.to_string(),
                        name: "订阅记录".to_string(),
                        description: "查看小程序用户订阅记录列表".to_string(),
                    },
                ],
            },
            PermissionGroup {
                name: "用户与订单".to_string(),
                items: vec![
                    PermissionItem {
                        code: WECHAT_USER_LIST_VIEW.to_string(),
                        name: "用户管理".to_string(),
                        description: "访问微信用户列表".to_string(),
                    },
                    PermissionItem {
                        code: WECHAT_USER_PASSWORD_VIEW.to_string(),
                        name: "查看密码".to_string(),
                        description: "单独控制用户列表中的查看密码按钮".to_string(),
                    },
                    PermissionItem {
                        code: ORDER_LIST_VIEW.to_string(),
                        name: "订单管理".to_string(),
                        description: "访问订单列表与订单处理页".to_string(),
                    },
                ],
            },
        ],
    }
}
