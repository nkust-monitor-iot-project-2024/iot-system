-- Add up migration script here

ALTER TABLE entities ALTER COLUMN category_id DROP NOT NULL;
