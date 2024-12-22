-- Add up migration script here

ALTER TABLE entities ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'UTC';
