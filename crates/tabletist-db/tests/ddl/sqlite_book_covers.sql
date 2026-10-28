CREATE TABLE book_covers (
    id INTEGER PRIMARY KEY,
    book_id INTEGER NOT NULL REFERENCES books (id),
    kind TEXT NOT NULL,
    image BLOB
);

CREATE INDEX book_covers_book_id ON book_covers (book_id);

CREATE UNIQUE INDEX book_covers_book_id_kind ON book_covers (book_id, kind);

CREATE TRIGGER book_covers_keep_kind AFTER UPDATE OF kind ON book_covers
BEGIN
    UPDATE book_covers SET kind = lower(NEW.kind) WHERE id = NEW.id;
END;

CREATE TRIGGER book_covers_lower_kind AFTER INSERT ON book_covers
BEGIN
    UPDATE book_covers SET kind = lower(NEW.kind) WHERE id = NEW.id;
END;
