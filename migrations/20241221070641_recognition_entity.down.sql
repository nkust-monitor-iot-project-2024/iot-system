-- Add down migration script here

DROP INDEX IF EXISTS idx_entity_label;
DROP TABLE IF EXISTS entities;
DROP TABLE IF EXISTS categories;
