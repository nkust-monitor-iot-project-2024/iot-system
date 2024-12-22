-- Add down migration script here

DROP INDEX IF EXISTS idx_entities_monitor_id;
ALTER TABLE entities DROP COLUMN monitor_id;
DROP TABLE IF EXISTS monitors;
