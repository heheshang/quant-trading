-- Seed data for symbol configs
-- Runs after 02_order_schema.sql via docker-entrypoint-initdb.d

-- Symbol configs for major trading pairs
INSERT INTO symbol_configs (id, symbol, base_currency, quote_currency, price_precision, quantity_precision, min_quantity, max_quantity, min_notional, fee_rate, enabled, created_at)
VALUES
    (gen_random_uuid(), 'BTC/USDT', 'BTC', 'USDT', 2, 4, 0.001, 1000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'ETH/USDT', 'ETH', 'USDT', 2, 4, 0.01, 1000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'SOL/USDT', 'SOL', 'USDT', 3, 2, 0.1, 10000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'BNB/USDT', 'BNB', 'USDT', 2, 3, 0.01, 1000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'XRP/USDT', 'XRP', 'USDT', 4, 1, 1, 100000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'ADA/USDT', 'ADA', 'USDT', 5, 0, 10, 1000000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'DOGE/USDT', 'DOGE', 'USDT', 5, 0, 100, 1000000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'MATIC/USDT', 'MATIC', 'USDT', 4, 1, 1, 100000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'DOT/USDT', 'DOT', 'USDT', 3, 2, 0.1, 10000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'AVAX/USDT', 'AVAX', 'USDT', 3, 2, 0.1, 10000, 10, 0.001, true, NOW())
ON CONFLICT (symbol) DO NOTHING;
