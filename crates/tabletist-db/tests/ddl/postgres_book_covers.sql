-- public.book_covers · built from the catalog · PostgreSQL
CREATE TABLE public.book_covers (
    id integer NOT NULL,
    book_id integer NOT NULL,
    kind text NOT NULL,
    image bytea,
    CONSTRAINT book_covers_pkey PRIMARY KEY (id),
    CONSTRAINT book_covers_book_id_fkey FOREIGN KEY (book_id) REFERENCES public.books(id)
);

-- Indexes
CREATE INDEX book_covers_book_id ON public.book_covers USING btree (book_id);
CREATE UNIQUE INDEX book_covers_book_id_kind ON public.book_covers USING btree (book_id, kind);

-- Triggers
CREATE TRIGGER book_covers_audit AFTER DELETE ON public.book_covers FOR EACH ROW EXECUTE FUNCTION public.book_covers_check();
ALTER TABLE public.book_covers DISABLE TRIGGER book_covers_audit;
CREATE TRIGGER book_covers_check BEFORE INSERT OR UPDATE OF image ON public.book_covers FOR EACH ROW EXECUTE FUNCTION public.book_covers_check();

-- Comments
COMMENT ON TABLE public.book_covers IS 'Cover images, one for each side of a book';
COMMENT ON COLUMN public.book_covers.kind IS 'front or back, as the shop''s scanner names it';
