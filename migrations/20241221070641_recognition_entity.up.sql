-- Add up migration script here

CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE entities (
    id SERIAL PRIMARY KEY,
    image_id VARCHAR(255) NOT NULL,
    confidence DECIMAL(3, 2) NOT NULL,
    label VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    category_id INTEGER NOT NULL REFERENCES categories (id)
);

CREATE INDEX idx_entity_label ON entities (label);
