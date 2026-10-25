-- public.in_stock · built from the catalog · PostgreSQL
CREATE OR REPLACE VIEW public.in_stock AS
 SELECT b.id AS book_id,
    b.title,
    p.name AS publisher
   FROM public.books b
     JOIN public.publishers p ON p.id = b.publisher_id
  WHERE b.in_print;

-- Comments
COMMENT ON VIEW public.in_stock IS 'Books in print, with their publisher''s name';
