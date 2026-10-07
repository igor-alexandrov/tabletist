CREATE DATABASE billing;
GRANT ALL PRIVILEGES ON billing.* TO 'tabletist'@'%';
-- A user who writes and is shown no triggers: the server lists a table's
-- triggers only to who has TRIGGER on it. One test of a save needs one.
CREATE USER 'writer'@'%' IDENTIFIED BY 'writer';
GRANT SELECT, INSERT, UPDATE, DELETE ON tabletist.* TO 'writer'@'%';
