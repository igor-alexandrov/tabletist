-- The MySQL fixture. Loaded by the integration tests through a normal
-- (writable) connection; the adapter itself is read-only. Safe to rerun.
SET FOREIGN_KEY_CHECKS = 0;
DROP TABLE IF EXISTS billing.invoices;
DROP VIEW IF EXISTS active_users;
DROP TABLE IF EXISTS orders, users, events, `weird "name"`, big, digits;
SET FOREIGN_KEY_CHECKS = 1;

CREATE TABLE users (
    id INT PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE COMMENT 'Login address',
    name VARCHAR(255),
    created_at DATETIME(6) NOT NULL DEFAULT '2026-01-01 00:00:00',
    active TINYINT(1) NOT NULL DEFAULT 1,
    meta JSON,
    avatar VARBINARY(16),
    score DOUBLE,
    balance DECIMAL(14, 2),
    counter BIGINT UNSIGNED,
    mood ENUM('happy', 'sad'),
    birthday DATE,
    alarm TIME
);
CREATE INDEX users_name_idx ON users (name);
INSERT INTO users VALUES
    (1, 'ada@example.com', 'Ada Lovelace', '2026-01-02 09:00:00', 1, '{"plan": "pro"}', X'89504E47', 99.5, 123456789012.34, 18446744073709551615, 'happy', '1815-12-10', '07:30:00'),
    (2, 'bob@example.com', 'Bob', '2026-01-03 10:30:00', 0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    (3, 'zoe@example.com', 'Zoë 🚀', '2026-01-04 11:45:00.000120', 1, '{"plan": "free"}', X'00FF10', 12.25, 0, 5, 'sad', NULL, '-25:15:00'),
    (4, 'percent@example.com', '50% off_now', '2026-01-05 12:00:00', 1, NULL, NULL, 0, NULL, NULL, NULL, NULL, NULL),
    (5, 'quote@example.com', 'O''Brien C:\\temp', '2026-01-06 13:15:00', 1, NULL, NULL, -1, NULL, NULL, NULL, NULL, NULL);

CREATE TABLE orders (
    id INT PRIMARY KEY,
    user_id INT NOT NULL,
    total DECIMAL(10, 2) NOT NULL,
    note TEXT,
    CONSTRAINT orders_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    INDEX orders_user_total_idx (user_id, total)
);
INSERT INTO orders VALUES (1, 1, 19.99, 'first'), (2, 1, 5.00, NULL), (3, 3, 120.50, 'bulk');

CREATE TABLE events (kind TEXT, payload TEXT);
INSERT INTO events VALUES ('login', 'ada'), ('logout', 'ada'), ('login', 'zoe');

CREATE TABLE `weird "name"` (`col with space` TEXT, `select` INT);
INSERT INTO `weird "name"` VALUES ('quoted', 1);

CREATE TABLE digits (d INT);
INSERT INTO digits VALUES (0), (1), (2), (3), (4), (5), (6), (7), (8), (9);
CREATE TABLE big (id INT PRIMARY KEY, label VARCHAR(32) NOT NULL, bucket INT NOT NULL);
INSERT INTO big SELECT n, CONCAT('row ', n), n % 10 FROM (SELECT a.d + 10 * b.d + 100 * c.d + 1000 * e.d + 10000 * f.d + 1 AS n FROM digits a, digits b, digits c, digits e, digits f) AS numbers;
DROP TABLE digits;
ANALYZE TABLE big;

CREATE VIEW active_users AS SELECT id, email FROM users WHERE active = 1;

CREATE TABLE billing.invoices (
    id INT PRIMARY KEY,
    user_id INT NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    CONSTRAINT invoices_user_fkey FOREIGN KEY (user_id) REFERENCES tabletist.users (id) ON UPDATE RESTRICT ON DELETE CASCADE
);
