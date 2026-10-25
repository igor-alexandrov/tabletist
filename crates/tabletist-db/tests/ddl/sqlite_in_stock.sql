CREATE VIEW in_stock AS
    SELECT b.id AS book_id, b.title, p.name AS publisher
    FROM books b JOIN publishers p ON p.id = b.publisher_id
    WHERE b.in_print;
