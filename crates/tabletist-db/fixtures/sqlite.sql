-- The shared fixture: every awkward case a browser meets. Used by the tests
-- and by the app's demo mode. Keep it deterministic (no random data).

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    created_at DATETIME NOT NULL DEFAULT '2026-01-01 00:00:00',
    active BOOLEAN NOT NULL DEFAULT 1,
    meta JSON,
    avatar BLOB,
    score REAL
);
CREATE INDEX users_name_idx ON users (name);

INSERT INTO users (id, email, name, created_at, active, meta, avatar, score) VALUES
    (1, 'ada@example.com', 'Ada Lovelace', '2026-01-02 09:00:00', 1, '{"plan":"pro","tags":["math","engines"]}', X'89504E47', 99.5),
    (2, 'bob@example.com', 'Bob', '2026-01-03 10:30:00', 0, NULL, NULL, NULL),
    (3, 'zoe@example.com', 'Zoë 🚀', '2026-01-04 11:45:00', 1, '{"plan":"free"}', X'00FF10', 12.25),
    (4, 'percent@example.com', '50% off', '2026-01-05 12:00:00', 1, NULL, NULL, 0),
    (5, 'null@example.com', NULL, '2026-01-06 13:15:00', 1, 'not json', CAST(X'C328' AS TEXT), -1);

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    total NUMERIC(10, 2) NOT NULL,
    note TEXT
);
CREATE INDEX orders_user_total_idx ON orders (user_id, total);

INSERT INTO orders (id, user_id, total, note) VALUES
    (1, 1, '19.99', 'first'),
    (2, 1, '5.00', NULL),
    (3, 3, '120.50', 'bulk');

-- No primary key: paging order is not stable.
CREATE TABLE events (kind TEXT, payload TEXT);
INSERT INTO events (kind, payload) VALUES ('login', 'ada'), ('logout', 'ada'), ('login', 'zoe');

-- Identifiers that need quoting.
CREATE TABLE "weird ""name""" ("col with space" TEXT, "select" INTEGER);
INSERT INTO "weird ""name""" VALUES ('quoted', 1);

-- Enough rows to page, sort and cancel on.
CREATE TABLE big (id INTEGER PRIMARY KEY, label TEXT NOT NULL, bucket INTEGER NOT NULL);
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 100000)
INSERT INTO big (id, label, bucket) SELECT i, 'row ' || i, i % 10 FROM n;

CREATE VIEW active_users AS SELECT id, email FROM users WHERE active = 1;
