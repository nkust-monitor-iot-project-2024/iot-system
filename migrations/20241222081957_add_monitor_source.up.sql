-- Add up migration script here

CREATE TABLE monitors (
    id VARCHAR(255) PRIMARY KEY
);

ALTER TABLE entities ADD COLUMN monitor_id VARCHAR(
    255
) REFERENCES monitors (id);

CREATE INDEX idx_entities_monitor_id ON entities (monitor_id);
