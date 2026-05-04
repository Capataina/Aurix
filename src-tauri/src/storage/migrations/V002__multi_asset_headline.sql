-- Extend the M2.8 headline tables with multi-asset comparisons (S&P 500
-- via VOO, gold via the LBMA London PM fix, 3-month T-bills) so the
-- verdict can score LP against more than just stable lending.
--
-- SQLite ALTER TABLE only supports ADD COLUMN; we add nullable columns
-- so existing runs persisted under V001 remain readable.

ALTER TABLE headline_runs ADD COLUMN months_lp_beat_sp500 INTEGER;
ALTER TABLE headline_runs ADD COLUMN months_lp_beat_gold  INTEGER;
ALTER TABLE headline_runs ADD COLUMN months_lp_beat_tbill INTEGER;
ALTER TABLE headline_runs ADD COLUMN median_sp500_spread  REAL;
ALTER TABLE headline_runs ADD COLUMN median_gold_spread   REAL;
ALTER TABLE headline_runs ADD COLUMN median_tbill_spread  REAL;

ALTER TABLE headline_monthly ADD COLUMN sp500_return REAL;
ALTER TABLE headline_monthly ADD COLUMN gold_return  REAL;
ALTER TABLE headline_monthly ADD COLUMN tbill_return REAL;
