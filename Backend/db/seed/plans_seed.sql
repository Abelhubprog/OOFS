INSERT INTO plans (code, price_usd_dec, daily_wallets, backfill_days, cadence, alerts, api_rows, perks_json)
VALUES
  ('FREE', 0.00, 2, 180, 'manual', 0, 0, '{}'::jsonb),
  ('LITE', 2.00, 5, 365, 'weekly', 0, 10000, '{}'::jsonb),
  ('PRO', 10.00, 25, 730, 'daily', 10, 100000, '{"boosts":true}'::jsonb)
ON CONFLICT (code) DO UPDATE SET
  price_usd_dec = EXCLUDED.price_usd_dec,
  daily_wallets = EXCLUDED.daily_wallets,
  backfill_days = EXCLUDED.backfill_days,
  cadence = EXCLUDED.cadence,
  alerts = EXCLUDED.alerts,
  api_rows = EXCLUDED.api_rows,
  perks_json = EXCLUDED.perks_json;
