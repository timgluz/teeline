-- Per-IP fixed-window rate limiting for the auth endpoints.
-- Rows are keyed by '{scope}:{ip}:{windowStart}'; the counter is incremented
-- atomically per request. Old windows are cleaned up opportunistically.
CREATE TABLE IF NOT EXISTS rate_limits (
  key          TEXT PRIMARY KEY,
  count        INTEGER NOT NULL,
  window_start INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_rate_limits_window ON rate_limits(window_start);
