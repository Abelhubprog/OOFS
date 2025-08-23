-- name: insert_moment
INSERT INTO oof_moments (id, wallet, mint, kind, t_event, window, pct_dec, missed_usd_dec, severity_dec, sig_ref, slot_ref, version, explain_json, preview_png_url)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
ON CONFLICT (id) DO UPDATE SET
  preview_png_url = COALESCE(EXCLUDED.preview_png_url, oof_moments.preview_png_url);
