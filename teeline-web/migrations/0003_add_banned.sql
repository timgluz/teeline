-- Account-level ban flag (operator-set). Banned users cannot log in and
-- their existing API keys stop verifying. Note: with open registration a
-- banned user could create a fresh account; a durable ban would additionally
-- deny-list credential IDs (follow-up).
ALTER TABLE users ADD COLUMN banned INTEGER NOT NULL DEFAULT 0;
