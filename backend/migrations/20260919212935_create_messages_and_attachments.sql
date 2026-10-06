CREATE TABLE messages (
    id           TEXT PRIMARY KEY NOT NULL,  -- uuid
    received_at  TEXT NOT NULL,              -- RFC3339
    from_addr    TEXT NOT NULL,
    to_addrs     TEXT NOT NULL,              -- JSON array
    cc_addrs     TEXT,                       -- JSON array, nullable
    subject      TEXT,
    body_text    TEXT,
    body_html    TEXT,
    raw_size     INTEGER NOT NULL,
    raw_source   BLOB NOT NULL,              -- full .eml, for raw view + re-parsing if schema evolves
    headers      TEXT                        -- JSON array of {name, value}, all headers in order
);

CREATE TABLE attachments (
    id            TEXT PRIMARY KEY NOT NULL, -- uuid
    message_id    TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    filename      TEXT,
    content_type  TEXT,
    size          INTEGER NOT NULL,
    data          BLOB NOT NULL
);

CREATE INDEX idx_messages_received_at ON messages(received_at);
CREATE INDEX idx_messages_from ON messages(from_addr);
CREATE INDEX idx_messages_to ON messages(to_addrs);
