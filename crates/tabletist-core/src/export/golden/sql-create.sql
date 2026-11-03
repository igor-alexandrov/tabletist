CREATE TABLE public.reviews (
  id bigint PRIMARY KEY
);

INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES
  (10, 4, 'The Tide Atlas', 'Maps the coast, one "harbor" at a time.', '{"tags": ["sea", "maps"], "helpful": 12}', E'\\x89504e47', '2026-02-06 08:05:00+00'),
  (11, 5, 'Harbor Lights', 'Lovely	second half
and a pipe | here', NULL, NULL, '2026-02-06 08:06:12.5+00'),
  (12, NULL, '', E'Café Ünïcödé 東京, shelved under C:\\fiction', '{"a": {"b": [1, 2.50]}, "z": null}', E'\\x', '2026-02-11 12:40:00+05:30');
