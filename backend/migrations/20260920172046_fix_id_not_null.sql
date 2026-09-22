

CREATE TABLE messages_new (
    id           TEXT PRIMARY KEY NOT NULL,
    received_at  TEXT NOT NULL,
    from_addr    TEXT NOT NULL,
    to_addrs     TEXT NOT NULL,
    cc_addrs     TEXT,
    subject      TEXT,
    body_text    TEXT,
    body_html    TEXT,
    raw_size     INTEGER NOT NULL,
    raw_source   BLOB NOT NULL
);

INSERT INTO messages_new SELECT * FROM messages;

DROP TABLE messages;

ALTER TABLE messages_new RENAME TO messages;

CREATE TABLE attachments_new (
    id            TEXT PRIMARY KEY NOT NULL,
    message_id    TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    filename      TEXT,
    content_type  TEXT,
    size          INTEGER NOT NULL,
    data          BLOB NOT NULL
);

INSERT INTO attachments_new SELECT * FROM attachments;

DROP TABLE attachments;

ALTER TABLE attachments_new RENAME TO attachments;

-- DROP TABLE removes any indexes defined on it, so these need recreating.
CREATE INDEX idx_messages_received_at ON messages(received_at);
CREATE INDEX idx_messages_from ON messages(from_addr);
CREATE INDEX idx_messages_to ON messages(to_addrs);
