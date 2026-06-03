-- P3-A: Admin IP 白名单 — per-user CIDR allow-list
-- P3-A: Admin IP whitelist — per-user CIDR allow-list for admin routes.
--
-- 中文：
--   该表为每个 admin 用户保存一组允许访问 admin 路由的 CIDR 段。中间件
--   `admin_ip_check` 在每次 admin 路由请求时检查请求方 IP 是否被该 admin
--   的任一 entry 覆盖，未覆盖则 403。这是"网络层 + 凭证"双因素之外的
--   第三层"地理/网络位置"因子。
--
--   设计要点：
--     - 一行一 CIDR：拆开存比塞进一个 JSON 数组更便于做 (user_id, ip_cidr)
--       UNIQUE 索引、便于逐条审计与撤销。
--     - 索引：建 (user_id) 索引加速"列出该 user 的所有白名单"和"白名单匹配"
--       两条主要查询路径。
--     - 标签字段：便于运维记录来源（"Office VPN", "Home", "K8s pod CIDR"），
--       无业务依赖，默认空串。
-- English:
--   Stores CIDR allow-list entries per admin user. The `admin_ip_check`
--   middleware reads this on every admin request and 403s requests whose
--   peer IP is not contained in any entry. Adds a third "network location"
--   factor on top of credentials + TOTP.
--
--   Design notes:
--     - One row per CIDR: storing a flat list (vs JSON array) makes the
--       (user_id, ip_cidr) UNIQUE constraint trivial, simplifies per-entry
--       audit & revocation, and matches the access pattern (always a
--       point-in-time membership check).
--     - Index on (user_id) is the access path for both list and check.
--     - `label` is metadata-only; no business logic depends on it.

CREATE TABLE IF NOT EXISTS admin_ip_whitelist (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ip_cidr     VARCHAR(64)  NOT NULL,
    label       VARCHAR(120) NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Lookup by user: "list entries for user X" + "find any entry for user X"
-- are the two hot paths, both served by this index.
CREATE INDEX IF NOT EXISTS idx_admin_ip_whitelist_user_id
    ON admin_ip_whitelist (user_id);

-- Prevent a user from inserting the same CIDR twice (defence in depth;
-- the handler also rejects dupes with 409).
CREATE UNIQUE INDEX IF NOT EXISTS uq_admin_ip_whitelist_user_cidr
    ON admin_ip_whitelist (user_id, ip_cidr);
