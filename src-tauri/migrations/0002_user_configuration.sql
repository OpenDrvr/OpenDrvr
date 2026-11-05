-- Structured user configuration (migration 2).
--
-- `app_preferences` stores lightweight key/value UI preferences. This table is
-- reserved for structured configuration documents that may evolve independently
-- of the preference schema. Values are JSON text and are always written through
-- parameterized Rust queries.

CREATE TABLE user_configuration (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_user_configuration_updated_at ON user_configuration (updated_at DESC);
