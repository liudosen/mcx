"""
（纯 requests，无需浏览器）
流程: 读取缓存 token → 若过期则重新登录(验证码+重试) → 获取订单号 → 验证健康卡 → 预结算 → 支付 → 查询结果

用法:
  python jk_order.py -u 卖家账号 -p 卖家密码 -c 健康卡号 -k 健康卡密码 -a 金额
  python jk_order.py -u jyzy2240 -p DUP3AEX2qijf -c 310115199011060935 -k 093538 -a 0.01
"""
import argparse
import requests
import ddddocr
import hashlib
import json
import time
import base64
import os
from Crypto.PublicKey import RSA
from Crypto.Cipher import PKCS1_v1_5

# ===== 固定配置 =====
APP_ID          = "9222001"
SELLER_ID       = "248933040709"
ID_TYPE         = "1"   # 1=身份证
PAY_CHANNEL     = 6     # 养老险健康卡

ITEM = {
    "itemCode": "PAJKPOS1169888",
    "itemName": "中医-其他",
    "category": "D04",
    "categoryDesp": "保健服务",
    "spec": "", "brand": "", "itemType": "",
    "itemBarCode": "", "itemApprovalNumber": "",
    "itemSubCategory": "", "itemManufacturer": "",
    "gmtModified": "2025-10-29 16:56:44",
    "lastModifier": "",
}

RSA_PUBLIC_KEY = (
    "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDKzDDsrhcP7iRsbbVhn30P/38R"
    "+b4DNmV0bhrxG7lm1kBdhk8+br7g42JCK5m7Vs50FWnSXWSkNoKT+fuzg23x3WpR"
    "xu6s84FSFj9Un6H4eRFSAOKyxTQuNftr4RYDFvkRsHlGGnhiHv7dXgufD7TfaTNr"
    "fI/K4pLZRhfzcqHecwIDAQAB"
)

TOKEN_DIR       = "C:/Users/songxiaobai"
MAX_LOGIN_RETRY = 10
# ===================


def _parse_args():
    parser = argparse.ArgumentParser(description="jk.cn 全自动下单脚本")
    parser.add_argument("-u", "--username",      required=True, help="卖家登录账号")
    parser.add_argument("-p", "--password",      required=True, help="卖家登录密码")
    parser.add_argument("-c", "--card-no",       required=True, help="健康卡卡号（身份证号）")
    parser.add_argument("-k", "--card-password", required=True, help="健康卡支付密码")
    parser.add_argument("-a", "--amount",        required=True, type=float, help="支付金额（元），如 0.01")
    return parser.parse_args()


# ----- 签名 & 加密工具 -----

def _md5(s: str) -> str:
    return hashlib.md5(s.encode()).hexdigest()


def _calc_sig(params: dict, wtk: str) -> str:
    """
    签名算法（逆向自 app.js）：
      将所有参数（排除 _sig 本身）按 key 字母升序拼接为 key=value，
      末尾追加 _wtk 值；若无 wtk 则追加 'jk.pingan.com'，再整体 MD5。
    注意：_mt 来自 URL query，必须包含在签名中。
    """
    keys = sorted(k for k in params if k != "_sig")
    r = "".join(k + "=" + str(params[k]) for k in keys)
    r += wtk if wtk else "jk.pingan.com"
    return _md5(r)


def _pwd_hash(password: str) -> str:
    """登录密码：md5(password + 'pajk.cn')"""
    return _md5(password + "pajk.cn")


def _rsa_encrypt(plain: str) -> str:
    """PKCS#1 v1.5 RSA 加密，返回 base64 字符串"""
    pem = (
        "-----BEGIN PUBLIC KEY-----\n"
        + "\n".join(RSA_PUBLIC_KEY[i:i+64] for i in range(0, len(RSA_PUBLIC_KEY), 64))
        + "\n-----END PUBLIC KEY-----"
    )
    key = RSA.import_key(pem)
    cipher = PKCS1_v1_5.new(key)
    return base64.b64encode(cipher.encrypt(plain.encode())).decode()


def _make_card_password(card_no: str, card_password: str) -> str:
    """每次加密时带最新时间戳（防重放）"""
    payload = json.dumps(
        {"cardNo": card_no, "pd": card_password, "timestamp": int(time.time() * 1000)},
        separators=(",", ":"),
    )
    return _rsa_encrypt(payload)


# ----- HTTP 客户端 -----

_session = requests.Session()
_session.headers.update({
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36"
    ),
    "Origin": "https://www.jk.cn",
    "Referer": "https://www.jk.cn/",
    "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
})


def _api(mt: str, extra: dict, wtk: str) -> dict:
    """向 api.jk.cn 发一条 POST 请求，自动带 _mt / _sm / _aid / _wtk / _sig"""
    params = {"_mt": mt, "_sm": "md5", "_aid": APP_ID, **extra}
    if wtk:
        params["_wtk"] = wtk
    params["_sig"] = _calc_sig(params, wtk)
    resp = _session.post(f"https://api.jk.cn/m.api?_mt={mt}", data=params)
    resp.raise_for_status()
    return resp.json()


# ----- Token 缓存 -----

def _load_token(username: str) -> str | None:
    """从缓存文件恢复 wtk 和 session cookies，返回 wtk 或 None"""
    token_file = os.path.join(TOKEN_DIR, f"jk_token_{username}.json")
    if not os.path.exists(token_file):
        return None
    try:
        with open(token_file, encoding="utf-8") as f:
            obj = json.load(f)
        saved_at = obj.get("saved_at", 0)
        wtk = obj.get("wtk", "")
        cookies = obj.get("cookies", {})
        # token 有效期按 8 小时保守估计
        if wtk and time.time() - saved_at < 8 * 3600:
            # 恢复 session cookies
            _session.cookies.update(cookies)
            return wtk
    except Exception:
        pass
    return None


def _save_token(username: str, wtk: str) -> None:
    token_file = os.path.join(TOKEN_DIR, f"jk_token_{username}.json")
    cookies = dict(_session.cookies)
    with open(token_file, "w", encoding="utf-8") as f:
        json.dump({"wtk": wtk, "cookies": cookies, "saved_at": time.time()}, f)
    print(f"  Token 已缓存至 {token_file}")


def _verify_token(wtk: str) -> bool:
    """用 getUserAndSellerInfo 验证 token 是否仍有效"""
    try:
        data = _api("kylin.getUserAndSellerInfo", {}, wtk)
        return data.get("stat", {}).get("code") == 0
    except Exception:
        return False


# ----- 登录 -----

_ocr1 = ddddocr.DdddOcr()
_ocr2 = ddddocr.DdddOcr(beta=True)


def _recognize_captcha(img_bytes: bytes) -> str:
    r1 = _ocr1.classification(img_bytes)
    r2 = _ocr2.classification(img_bytes)
    return next((x for x in [r2, r1] if len(x) == 4), max([r1, r2], key=len))


def login(username: str, password: str) -> str:
    """登录（带重试），返回有效 _wtk"""
    for attempt in range(1, MAX_LOGIN_RETRY + 1):
        # 获取验证码
        cap_params = {"_mt": "kylin.requestCaptcha", "_sm": "md5", "_aid": APP_ID}
        cap_params["_sig"] = _calc_sig(cap_params, "")
        cap_data = _session.post(
            "https://api.jk.cn/m.api?_mt=kylin.requestCaptcha", data=cap_params
        ).json()
        content = cap_data["content"][0]
        img_url, cap_key = content["imgUrl"], content["key"]

        img_bytes = _session.get(img_url).content
        cap_text = _recognize_captcha(img_bytes)
        print(f"  [{attempt}/{MAX_LOGIN_RETRY}] 验证码={cap_text}")

        resp = _session.post(
            "https://jk.cn/login/loginname",
            data={
                "loginName": username,
                "password": _pwd_hash(password),
                "captcha": cap_text,
                "_cap": cap_key,
                "appId": APP_ID,
            },
        ).json()

        if resp.get("success"):
            wtk = resp["model"]["_wtk"]
            print(f"  登录成功！wtk={wtk}")
            _save_token(username, wtk)
            return wtk

        print(f"  失败: {resp.get('errorCode')} {resp.get('errorMessage', '')}")

    raise RuntimeError("登录失败，已超过最大重试次数")


def get_token(username: str, password: str) -> str:
    """优先使用缓存 token，失效则重新登录"""
    wtk = _load_token(username)
    if wtk:
        print(f"[Token] 读取缓存 wtk={wtk}")
        if _verify_token(wtk):
            print("[Token] 验证有效，跳过登录")
            return wtk
        print("[Token] 已过期，重新登录...")
    else:
        print("[Token] 无缓存，执行登录...")
    return login(username, password)


# ----- 下单主流程 -----

def order(wtk: str, card_no: str, card_password: str, amount: float) -> None:
    line = {
        **ITEM,
        "qty": 1,
        "price": amount,
        "amount": amount,
        "approvalNumber": "",
        "barcode": "",
        "manufacturer": "",
        "subCategory": "",
    }

    # Step 1: 获取订单号
    print("\n== Step 1: 获取订单号 ==")
    data = _api("baize.getStoreAndOrderNo", {"req": "{}", "sellerId": SELLER_ID}, wtk)
    if not data.get("content"):
        raise RuntimeError(f"获取订单号失败: {data}")
    info = data["content"][0]
    order_no = info["orderNo"]
    store_id = info["storeId"]
    print(f"  订单号={order_no}  门店={store_id}")

    # Step 2: 查询支付渠道（验证健康卡）
    print("\n== Step 2: 查询支付渠道 ==")
    enc_pwd = _make_card_password(card_no, card_password)
    channel_req = {
        "idType": ID_TYPE,
        "cardNo": card_no,
        "password": enc_pwd,
        "queryBalance": True,
    }
    data = _api(
        "baize.queryPayChannelByEntityCard",
        {"req": json.dumps(channel_req, separators=(",", ":")), "sellerId": SELLER_ID},
        wtk,
    )
    if not data.get("content") or not data["content"][0].get("success"):
        raise RuntimeError(f"健康卡验证失败: {data}")
    channels = data["content"][0].get("payChannels", [])
    for ch in channels:
        print(f"  渠道: {ch.get('payChannelName')}  余额={ch.get('balance')} 分")

    # Step 3: 预结算
    print("\n== Step 3: 预结算 ==")
    enc_pwd2 = _make_card_password(card_no, card_password)
    precalc_req = {
        "cardNo": card_no,
        "password": enc_pwd2,
        "idType": ID_TYPE,
        "xrefNo": order_no,
        "amount": amount,
        "lines": [line],
        "payChannel": PAY_CHANNEL,
    }
    data = _api(
        "baize.drugCardPreCalc",
        {"req": json.dumps(precalc_req, separators=(",", ":")), "sellerId": SELLER_ID},
        wtk,
    )
    if not data.get("content") or not data["content"][0].get("success"):
        raise RuntimeError(f"预结算失败: {data}")
    pr = data["content"][0]
    if not pr.get("fundAmount"):
        raise RuntimeError(f"预结算业务失败: {pr.get('returnMsg', '未知错误')}")
    print(f"  预计扣款={pr.get('fundAmount')} 元  渠道={pr.get('payChannelName')}")
    # Step 3b: pollReadyPlan（如需）
    if pr.get("pollReadyPlan"):
        print("\n== Step 3b: pollReadyPlan ==")
        for i in range(15):
            poll_data = _api(
                "baize.pollReadyPlan",
                {
                    "req": json.dumps({"xrefNo": order_no, "payChannel": PAY_CHANNEL}, separators=(",", ":")),
                    "sellerId": SELLER_ID,
                },
                wtk,
            )
            pr2 = poll_data.get("content", [{}])[0]
            print(f"  poll {i+1}: finish={pr2.get('finish')} fundAmount={pr2.get('fundAmount')}")
            if pr2.get("finish"):
                break
            time.sleep(1)

    # Step 4: 正式支付
    print("\n== Step 4: 正式支付 ==")
    pay_req = {
        "idType": ID_TYPE,
        "xrefNo": order_no,
        "amount": amount,
        "lines": [line],
        "payChannel": PAY_CHANNEL,
    }
    data = _api(
        "baize.drugCardPay",
        {"req": json.dumps(pay_req, separators=(",", ":")), "sellerId": SELLER_ID},
        wtk,
    )

    # Step 5: 查询订单状态
    print("\n== Step 5: 查询订单状态 ==")
    status_data = _api(
        "baize.drugCardQueryOrderStatus",
        {"req": json.dumps({"xrefNo": order_no, "type": 20}, separators=(",", ":")), "sellerId": SELLER_ID},
        wtk,
    )

    # 打印结果
    print("\n" + "=" * 50)
    r = data.get("content", [{}])[0]
    s = status_data.get("content", [{}])[0]
    if data.get("content") and r.get("success") and r.get("deductAmount", 0) > 0:
        print("[OK] 支付成功！")
        print(f"  订单号    : {order_no}")
        print(f"  实付金额  : {r.get('totalAmount')} 元")
        print(f"  扣款金额  : {r.get('deductAmount')} 元")
        print(f"  订单状态  : status={s.get('status')}")
    else:
        msg = r.get("returnMsg") or data.get("stat", {}).get("stateList")
        print(f"[FAIL] 支付失败: {msg}")
        print(f"  完整响应: {json.dumps(data, ensure_ascii=False)[:400]}")


def main():
    args = _parse_args()
    print(f"卖家账号: {args.username}  金额: {args.amount} 元  健康卡: {args.card_no}")
    wtk = get_token(args.username, args.password)
    order(wtk, args.card_no, args.card_password, args.amount)


if __name__ == "__main__":
    main()
