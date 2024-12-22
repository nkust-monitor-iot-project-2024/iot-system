-- Add down migration script here

ALTER TABLE entities ALTER COLUMN created_at TYPE TIMESTAMP USING created_at AT TIME ZONE 'UTC';
