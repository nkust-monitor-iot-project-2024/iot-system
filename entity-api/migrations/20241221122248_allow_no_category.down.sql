-- Add down migration script here

ALTER TABLE entities ALTER COLUMN category_id SET NOT NULL;
