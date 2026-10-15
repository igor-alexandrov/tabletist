-- The Bookshop: the made-up shop of the app's screenshots, as a database.
-- What an agent may read through the MCP server is tested on it, on each
-- engine. The three files hold the same rows. Keep them deterministic.
-- Loaded into a database of its own, `bookshop`, which the tests make.

DROP VIEW IF EXISTS customer_tally;
DROP FUNCTION IF EXISTS customer_count();
DROP VIEW IF EXISTS recent_orders, customer_names;
DROP TABLE IF EXISTS order_items, orders, customers, book_covers, books, authors, publishers;

CREATE TABLE publishers (
    id integer PRIMARY KEY,
    name text NOT NULL,
    city text
);
CREATE TABLE authors (
    id integer PRIMARY KEY,
    name text NOT NULL
);
CREATE TABLE books (
    id integer PRIMARY KEY,
    title text NOT NULL,
    author_id integer NOT NULL REFERENCES authors (id),
    publisher_id integer NOT NULL REFERENCES publishers (id),
    price numeric(8, 2) NOT NULL,
    published date,
    in_print boolean NOT NULL DEFAULT true
);
CREATE TABLE book_covers (
    id integer PRIMARY KEY,
    book_id integer NOT NULL REFERENCES books (id),
    kind text NOT NULL,
    image bytea
);
CREATE TABLE customers (
    id integer PRIMARY KEY,
    name text NOT NULL,
    email text NOT NULL,
    phone text
);
CREATE TABLE orders (
    id integer PRIMARY KEY,
    customer_id integer NOT NULL REFERENCES customers (id),
    email text NOT NULL,
    phone text,
    placed_at timestamp NOT NULL,
    total numeric(10, 2) NOT NULL
);
CREATE TABLE order_items (
    id integer PRIMARY KEY,
    order_id integer NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    book_id integer NOT NULL REFERENCES books (id),
    quantity integer NOT NULL,
    price numeric(8, 2) NOT NULL
);

INSERT INTO publishers (id, name, city) VALUES
    (1, 'Northlight Books', 'Bergen'),
    (2, 'Harbor Press', 'Halifax'),
    (3, 'Harbor Lights', 'Lisbon'),
    (4, 'Studio 91', 'Kyoto'),
    (5, '100% Books', NULL);

INSERT INTO authors (id, name) VALUES
    (1, 'Ines Calloway'),
    (2, 'Tomas Varga'),
    (3, 'Mei Arakawa'),
    (4, 'Rufus Oyelaran'),
    (5, 'Zoë Lindqvist'),
    (6, 'N. K. O''Dell');

INSERT INTO books (id, title, author_id, publisher_id, price, published, in_print) VALUES
    (1, 'The Salt Road', 1, 1, 18.50, '2019-03-14', TRUE),
    (2, 'A Field Guide to Fog', 2, 1, 24.00, '2021-09-02', TRUE),
    (3, 'Lanterns', 3, 2, 12.99, '2017-11-20', TRUE),
    (4, 'The Cartographer''s Daughter', 6, 2, 21.75, '2022-05-05', TRUE),
    (5, 'Small Engines', 4, 3, 15.00, '2015-01-30', FALSE),
    (6, 'Winter Harbour', 1, 3, 19.99, '2023-10-10', TRUE),
    (7, 'Notes on Bread', 5, 4, 30.00, '2020-02-29', TRUE),
    (8, 'Ninety-One Doors', 3, 4, 16.40, '2018-07-07', TRUE),
    (9, 'The Quiet Ledger', 2, 5, 9.99, '2012-12-12', FALSE),
    (10, 'Tidewater', 4, 2, 22.00, '2024-04-18', TRUE),
    (11, 'An Atlas of Small Things', 5, 1, 27.30, '2025-06-01', TRUE),
    (12, '100% Rain', 6, 5, 11.25, '2016-08-23', TRUE);

INSERT INTO book_covers (id, book_id, kind, image) VALUES
    (1, 1, 'front', '\x89504e470d0a1a0a'),
    (2, 1, 'back', '\xffd8ffe000104a46'),
    (3, 7, 'front', '\x47494638396101');

INSERT INTO customers (id, name, email, phone) VALUES
    (1, 'Ada Brandt', 'ada.brandt@example.com', '+1 555 0101'),
    (2, 'Bo Castellan', 'bo@example.com', '+1 555 0102'),
    (3, 'Chidi Okafor', 'chidi.okafor@example.com', NULL),
    (4, 'Dana Whitlock', 'dana@example.org', '+44 20 7946 0104'),
    (5, 'Emil Sørensen', 'emil@example.com', '+45 55 50 01 05'),
    (6, 'Farah Qureshi', 'farah.q@example.net', NULL),
    (7, 'Greta O''Malley', 'greta@example.com', '+353 1 555 0107'),
    (8, 'Hiro Tanabe', 'hiro@example.org', '+81 3 5550 0108');

INSERT INTO orders (id, customer_id, email, phone, placed_at, total) VALUES
    (1, 6, 'farah.q@example.net', NULL, '2025-02-08 09:13:00', 176.25),
    (2, 3, 'chidi.okafor@example.com', NULL, '2025-04-15 10:26:00', 201.70),
    (3, 8, 'hiro@example.org', '+81 3 5550 0108', '2025-06-22 11:39:00', 94.19),
    (4, 5, 'emil@example.com', '+45 55 50 01 05', '2025-08-02 12:52:00', 192.50),
    (5, 2, 'bo@example.com', '+1 555 0102', '2025-10-09 13:05:00', 116.00),
    (6, 7, 'greta@example.com', '+353 1 555 0107', '2026-12-16 14:18:00', 73.72),
    (7, 4, 'dana@example.org', '+44 20 7946 0104', '2026-02-23 15:31:00', 144.50),
    (8, 1, 'ada.brandt@example.com', '+1 555 0101', '2026-04-03 16:44:00', 124.50),
    (9, 6, 'farah.q@example.net', NULL, '2026-06-10 17:57:00', 71.46),
    (10, 3, 'chidi.okafor@example.com', NULL, '2026-08-17 08:10:00', 121.25),
    (11, 8, 'hiro@example.org', '+81 3 5550 0108', '2026-10-24 09:23:00', 135.90),
    (12, 5, 'emil@example.com', '+45 55 50 01 05', '2026-12-04 10:36:00', 97.20);

INSERT INTO order_items (id, order_id, book_id, quantity, price) VALUES
    (1, 1, 1, 2, 18.50),
    (2, 1, 4, 3, 21.75),
    (3, 1, 7, 1, 30.00),
    (4, 1, 10, 2, 22.00),
    (5, 2, 2, 3, 24.00),
    (6, 2, 5, 1, 15.00),
    (7, 2, 8, 2, 16.40),
    (8, 2, 11, 3, 27.30),
    (9, 3, 3, 1, 12.99),
    (10, 3, 6, 2, 19.99),
    (11, 3, 9, 3, 9.99),
    (12, 3, 12, 1, 11.25),
    (13, 4, 4, 2, 21.75),
    (14, 4, 7, 3, 30.00),
    (15, 4, 10, 1, 22.00),
    (16, 4, 1, 2, 18.50),
    (17, 5, 5, 3, 15.00),
    (18, 5, 8, 1, 16.40),
    (19, 5, 11, 2, 27.30),
    (20, 6, 6, 1, 19.99),
    (21, 6, 9, 2, 9.99),
    (22, 6, 12, 3, 11.25),
    (23, 7, 7, 2, 30.00),
    (24, 7, 10, 3, 22.00),
    (25, 7, 1, 1, 18.50),
    (26, 8, 8, 3, 16.40),
    (27, 8, 11, 1, 27.30),
    (28, 8, 2, 2, 24.00),
    (29, 9, 9, 1, 9.99),
    (30, 9, 12, 2, 11.25),
    (31, 9, 3, 3, 12.99),
    (32, 10, 10, 2, 22.00),
    (33, 10, 1, 3, 18.50),
    (34, 10, 4, 1, 21.75),
    (35, 11, 11, 3, 27.30),
    (36, 11, 2, 1, 24.00),
    (37, 11, 5, 2, 15.00),
    (38, 12, 12, 1, 11.25),
    (39, 12, 3, 2, 12.99),
    (40, 12, 6, 3, 19.99);

-- Two ways to a table's rows that do not name it: what hiding a table and
-- masking a column must also cover.
CREATE VIEW recent_orders AS
    SELECT id, email, placed_at, total FROM orders WHERE placed_at >= '2026-01-01';
CREATE VIEW customer_names AS SELECT id, name FROM customers;

-- And a third, a function of the database's own: what it reads, no check
-- can see. Here alone: SQLite has none to write, and MySQL, which logs its
-- changes, lets only a user with SUPER write one.
CREATE FUNCTION customer_count() RETURNS bigint LANGUAGE sql STABLE
    AS 'SELECT count(*) FROM customers';
-- A view that calls it: it reads `customers`, and its catalog says only
-- that it calls the function.
CREATE VIEW customer_tally AS SELECT customer_count() AS customers;
