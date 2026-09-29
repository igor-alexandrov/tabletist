-- The PostgreSQL fixture. Loaded by the integration tests through an admin
-- connection (the adapter is read-only). Safe to run more than once.

DROP SCHEMA IF EXISTS billing CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_counts;
DROP VIEW IF EXISTS public.active_users;
DROP TABLE IF EXISTS public.orders, public.users, public.events, public."weird ""name""", public.big CASCADE;
DROP SEQUENCE IF EXISTS public.tick;
DROP TYPE IF EXISTS public.mood;

CREATE TYPE mood AS ENUM ('happy', 'sad');
CREATE SEQUENCE tick;

CREATE TABLE users (
    id integer PRIMARY KEY,
    email text NOT NULL UNIQUE,
    name text,
    created_at timestamptz NOT NULL DEFAULT '2026-01-01 00:00:00+00',
    active boolean NOT NULL DEFAULT true,
    meta jsonb,
    avatar bytea,
    score double precision,
    balance numeric(14, 2),
    tags text[],
    mood mood,
    uid uuid
);
COMMENT ON COLUMN users.email IS 'Login address';
CREATE INDEX users_name_idx ON users (name);

INSERT INTO users VALUES
    (1, 'ada@example.com', 'Ada Lovelace', '2026-01-02 09:00:00+00', true, '{"plan": "pro"}', '\x89504e47', 99.5, 123456789012.34, '{math,engines}', 'happy', '00000000-0000-0000-0000-000000000001'),
    (2, 'bob@example.com', 'Bob', '2026-01-03 10:30:00+00', false, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    (3, 'zoe@example.com', 'Zoë 🚀', '2026-01-04 11:45:00+00', true, '{"plan": "free"}', '\x00ff10', 12.25, 0, '{}', 'sad', NULL),
    (4, 'percent@example.com', '50% off', '2026-01-05 12:00:00+00', true, NULL, NULL, 0, NULL, NULL, NULL, NULL),
    (5, 'quote@example.com', 'O''Brien C:\temp', '2026-01-06 13:15:00+00', true, NULL, NULL, -1, NULL, NULL, NULL, NULL);

CREATE TABLE orders (
    id integer PRIMARY KEY,
    user_id integer NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    total numeric(10, 2) NOT NULL,
    note text
);
CREATE INDEX orders_user_total_idx ON orders (user_id, total);
INSERT INTO orders VALUES (1, 1, 19.99, 'first'), (2, 1, 5.00, NULL), (3, 3, 120.50, 'bulk');

CREATE TABLE events (kind text, payload text);
INSERT INTO events VALUES ('login', 'ada'), ('logout', 'ada'), ('login', 'zoe');

CREATE TABLE "weird ""name""" ("col with space" text, "select" integer);
INSERT INTO "weird ""name""" VALUES ('quoted', 1);

CREATE TABLE big (id integer PRIMARY KEY, label text NOT NULL, bucket integer NOT NULL);
INSERT INTO big SELECT i, 'row ' || i, i % 10 FROM generate_series(1, 100000) AS i;
ANALYZE big;

CREATE VIEW active_users AS SELECT id, email FROM users WHERE active;
CREATE MATERIALIZED VIEW user_counts AS SELECT active, count(*) AS n FROM users GROUP BY active;

CREATE SCHEMA billing;
CREATE TABLE billing.invoices (
    id integer PRIMARY KEY,
    user_id integer NOT NULL REFERENCES public.users (id) ON UPDATE RESTRICT ON DELETE SET NULL,
    amount numeric(10, 2) NOT NULL
);
