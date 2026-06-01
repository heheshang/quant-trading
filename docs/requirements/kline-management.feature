@kline-management
Feature: K线数据管理模块

  Background:
    Given 用户已登录
      And kline_data 表已存在 TimescaleDB hypertable 配置

  # ============================================================
  # US-KM-01: K线数据导入
  # ============================================================

  Scenario: CSV 上传成功
    Given 用户准备 CSV 文件，格式为: timestamp,open,high,low,close,volume
      And CSV 包含 1000 条有效数据
    When 用户上传 CSV 文件并指定 symbol="BTCUSDT" interval="1h"
    Then 系统解析文件并写入 kline_data 表
      And 返回导入结果：总条数=1000, 成功数=1000, 失败数=0, 重复忽略数=0
      And 前端显示导入成功 toast

  Scenario: CSV 文件格式错误（缺少必需列）
    Given 用户上传的 CSV 文件缺少必需列 "close"
    When 系统解析文件
    Then 返回 400 错误
      And 提示 "缺少必需列: close"
      And kline_data 表无任何写入

  Scenario: CSV 文件格式错误（无效时间戳）
    Given 用户上传的 CSV 文件 timestamp 列为非数字字符串
    When 系统解析文件
    Then 返回 400 错误
      And 提示 "timestamp 格式无效"
      And kline_data 表无任何写入

  Scenario: CSV 文件过大
    Given 用户上传超过 100MB 的 CSV 文件
    When 系统接收文件
    Then 返回 413 错误
      And 提示 "文件大小超过 100MB 限制，请分批上传"

  Scenario: API 批量导入
    Given 用户拥有有效的 JWT token
    When POST /api/v1/kline/import 发送 JSON 数据（1000条）
    Then 返回 200 状态码
      And 返回 {"imported": 1000, "duplicates": 0, "failed": 0}
      And 数据已写入 kline_data 表

  Scenario: 导入重复数据（UPSERT 跳过）
    Given kline_data 已存在 symbol="BTCUSDT" interval="1h" open_time=1704067200000 的记录
    When 用户上传包含相同 open_time 的 CSV（共 100 条，含 10 条重复）
    Then 系统跳过重复记录（不报错，不覆盖）
      And 返回 {"imported": 90, "duplicates": 10, "failed": 0}
      And 已存在数据未被修改

  Scenario: 交易所直采（pro-trader）
    Given 当前用户角色为 pro-trader
      And symbol="ETHUSDT" interval="15m" 在 Binance 有数据
    When 用户选择交易所 "binance" 并指定范围 2024-01-01 至 2024-01-31
    Then 系统从 Binance API 获取数据
      And 数据写入 kline_data 表
      And 返回导入条数

  Scenario: 交易所直采权限拦截
    Given 当前用户角色为 trader（无 pro-trader 权限）
    When 用户尝试访问交易所直采功能
    Then 返回 403 Forbidden
      And 提示 "该功能仅对专业交易员开放"

  # ============================================================
  # US-KM-02: K线数据查询
  # ============================================================

  Scenario: 按时间范围查询
    Given 用户指定 symbol="BTCUSDT" interval="1h"
      And start_time=1704067200000 (2024-01-01 00:00 UTC)
      And end_time=1706745599000 (2024-01-31 23:59 UTC)
    When 发起 GET /api/v1/kline/query
    Then 返回该时间范围内的 K线数据
      And 数据按 open_time ASC 排序
      And 每页默认 1000 条
      And 返回 meta.total 字段表示总条数

  Scenario: 查询结果分页
    Given 用户指定 symbol="BTCUSDT" interval="1h"
      And 时间范围内共有 5000 条记录
    When 发起 GET /api/v1/kline/query?page=2&page_size=1000
    Then 返回第 1001-2000 条记录
      And meta.total = 5000
      And meta.page = 2
      And meta.page_size = 1000

  Scenario: 按交易对和周期查询最新数据
    Given kline_data 存在 symbol="BTCUSDT" interval="1h" 的记录
    When 发起 GET /api/v1/kline/latest?symbol=BTCUSDT&interval=1h
    Then 返回该交易对最新的 1 条 K线数据

  Scenario: 查询无数据
    Given 用户指定 symbol="INVALID99" interval="1h"
    When 发起 GET /api/v1/kline/query
    Then 返回 200 状态码
      And data 字段为空数组 []
      And meta.total = 0

  Scenario: 查询时间范围过大
    Given 用户指定时间范围超过 10 年
    When 发起 GET /api/v1/kline/query
    Then 返回 400 错误
      And 提示 "查询范围过大，请分段查询（最大 10 年）"

  Scenario: 跨周末缺口检测（1h 周期）
    Given 用户查询 symbol="BTCUSDT" interval="1h" 范围跨周末
      And 数据中存在周末超过 8 小时无数据
    When 发起 GET /api/v1/kline/query
    Then meta.gap_detected = true
      And 返回数据中周末缺口位置有标记

  Scenario: 缺失 interval 参数
    Given 用户查询时缺少 interval 参数
    When 发起 GET /api/v1/kline/query?symbol=BTCUSDT
    Then 返回 400 错误
      And 提示 "缺少必需参数: interval"

  # ============================================================
  # US-KM-03: 数据质量检测
  # ============================================================

  Scenario: 检测缺尖并标记
    Given kline_data 中 symbol="BTCUSDT" interval="1h" 存在 5 处连续缺尖
    When 用户点击"数据质量检测"
    Then 系统返回检测报告
      And gap_count = 5
      And gap_positions 包含缺尖的 open_time 列表
      And 建议包含 "自动插值填充"

  Scenario: 检测异常价格（偏离均值 ±15%）
    Given kline_data 中存在 1 根 K线 close=100000（其他均约 50000）
    When 触发数据质量检测
    Then 该 K线被标记为 suspicious
      And anomaly_count = 1
      And 报告显示异常值位置和偏离幅度

  Scenario: 检测重复数据
    Given kline_data 中存在 3 条完全重复的 K线（symbol/interval/open_time 相同）
    When 触发数据质量检测
    Then 重复数据自动去重为 1 条
      And 报告显示: "去重 3 条 → 保留 1 条"
      And duplicate_count = 2

  Scenario: 检测零成交量
    Given kline_data 中存在 volume=0 的 K线
    When 触发数据质量检测
    Then 该 K线被标记为 suspicious
      And 提示 "检测到零成交量数据"

  Scenario: 检测价格反向（high < low）
    Given kline_data 中存在 1 根 corrupted K线（high < low）
    When 触发数据质量检测
    Then 该 K线被标记为 corrupted
      And anomaly_count 包含此条
      And 建议 "需要手动修正"

  Scenario: 数据覆盖率统计
    Given kline_data 中 symbol="BTCUSDT" interval="1h" 共 1000 条
      And 其中 950 条有效、30 条缺尖、20 条异常
    When 用户请求质量报告
    Then 返回:
      And total_rows = 1000
      And valid_rows = 950
      And coverage_rate = 95%

  # ============================================================
  # US-KM-04: 数据清洗
  # ============================================================

  Scenario: 自动清洗执行
    Given kline_data 存在 5 处缺尖和 10 条重复
    When 用户选择"自动清洗"并点击执行
    Then 系统执行清洗操作：
      And 缺尖处使用线性插值填充
      And 重复数据去重（保留第一条）
      And 异常数据标记为 suspicious（不自动删除）
      And 返回清洗报告（填充数、去重数、保留异常数）

  Scenario: 手动删除异常数据
    Given 质量检测报告显示 3 条异常数据
    When 用户勾选这 3 条并选择"删除"
    Then 这 3 条数据被物理删除
      And 返回 deleted = 3

  Scenario: 手动修正异常数据
    Given 质量检测报告显示 1 条 corrupted K线（high < low）
    When 用户将该 K线 high/low 值修正为正确值
    Then 数据被更新
      And 该 K线取消 suspicious/corrupted 标记

  Scenario: 清洗前自动创建备份
    Given 用户执行清洗操作
    When 系统开始清洗
    Then 自动创建数据快照（kline_backup 表）
      And 快照保留 7 天
      And 返回 backup_id

  Scenario: 回滚至清洗前状态
    Given 用户执行过清洗并拥有有效快照
    When 用户点击"回滚"
    Then 数据恢复至清洗前状态
      And 返回 restored_rows

  Scenario: 回滚时快照已过期
    Given 用户的快照已超过 7 天保留期
    When 用户尝试回滚
    Then 返回 400 错误
      And 提示 "快照已过期（保留期 7 天）"

  # ============================================================
  # US-KM-05: 数据存储格式
  # ============================================================

  Scenario: TimescaleDB 超表创建
    Given 系统执行数据库初始化
    When 创建 kline_data 表
    Then 自动转换为 TimescaleDB hypertable
      And 按 open_time 月度分区

  Scenario: 1m 数据 30 天后自动压缩
    Given 用户已导入 1m 周期 K线数据 100 万条
    When 30 天后数据超过保留期
    Then TimescaleDB 自动压缩该数据
      And 存储空间节省 ≥ 60%

  Scenario: 压缩数据查询透明解压
    Given 存在已压缩的 1m 数据（15 天前）
    When 用户查询该时间范围
    Then 查询结果正确返回
      And 延迟不受压缩影响

  Scenario: TimescaleDB 扩展未安装（降级）
    Given PostgreSQL 未安装 TimescaleDB 扩展
    When 创建 kline_data 表
    Then 系统降级为普通 PostgreSQL 表
      And 提示 "TimescaleDB 未安装，手动分区请参考文档"

  # ============================================================
  # US-KM-06: 数据导出
  # ============================================================

  Scenario: 导出 CSV
    Given 用户拥有 symbol="BTCUSDT" interval="1h" 时间范围 2024-01 的数据
    When 用户点击"导出 CSV"
    Then 浏览器下载 CSV 文件
      And 文件名格式: kline_BTCUSDT_1h_20240101_20240131.csv
      And 包含表头: timestamp,open,high,low,close,volume
      And 数据按 open_time ASC 排列

  Scenario: 导出 JSON
    Given 用户拥有 symbol="BTCUSDT" interval="1h" 时间范围 2024-01 的数据
    When 用户点击"导出 JSON"
    Then 浏览器下载 JSON 文件
      And 格式: {"symbol":"BTCUSDT","interval":"1h","data":[...]}
      And data 数组每项包含 open_time, open, high, low, close, volume

  Scenario: 导出字段筛选
    Given 用户导出时取消勾选 "high" 和 "low"
    When 用户导出 CSV
    Then 文件仅包含: timestamp,open,close,volume

  Scenario: 导出无数据
    Given 用户选择无数据的 symbol
    When 用户点击导出
    Then 导出空文件（仅含表头或空 data 数组）
      And 不报错误

  # ============================================================
  # US-KM-07: 权限控制
  # ============================================================

  Scenario: 用户只能查询自己的数据
    Given 用户 A 导入了 symbol="BTCUSDT" interval="1h" 数据
      And 用户 B 导入了 symbol="ETHUSDT" interval="1h" 数据
      And 用户 B 已登录
    When 用户 B 查询 symbol="BTCUSDT"
    Then 仅返回用户 B 自己导入的数据（ETHUSDT 相关，非 BTCUSDT）
      And 用户 A 的 BTCUSDT 数据对 B 完全不可见

  Scenario: 用户无法访问他人数据（构造查询）
    Given 攻击者尝试通过修改 user_id 参数访问其他用户数据
    When 发起 /api/v1/kline/query
    Then 权限中间件强制使用当前登录用户的 user_id
      And 无法通过参数注入访问他人数据

  Scenario: admin 跨用户审计（只读）
    Given 管理员已登录
    When 管理员查询任意用户的数据（带 user_id 参数）
    Then 返回该用户的数据
      And admin 角色不可执行导入/清洗/删除操作（403）

  Scenario: 未授权用户无法导入
    Given 未登录用户尝试 POST /api/v1/kline/import
    Then 返回 401 Unauthorized

  Scenario: SQL 注入攻击防御
    Given 攻击者尝试在 symbol 参数中注入 SQL
    When 发起 /api/v1/kline/query?symbol=BTCUSDT'; DROP TABLE kline_data;--
    Then 参数被正确转义
      And kline_data 表未被影响
      And 返回 400 参数格式错误
