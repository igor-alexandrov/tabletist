# Promotional video

A 15 second, 1920x1080, 60 fps, silent video for social posts (X, Mastodon):
large captions, a title card first and an end card with the name and the URL.

## How it is made

1. `src/promo.rs` (an ignored test behind the `shots` feature) drives the app
   through the headless harness and writes one PNG per state to
   `target/promo/`. What the app asks its backend is answered by the real
   SQLite driver reading the file `PROMO_DB` names, in a read-only session.
   Nothing is written to that file: the edit is shown up to its review, and
   the SQL runs in a read-only transaction.
2. `contrib/promo/index.html` is the composition: the window with its camera
   moves, the captions, the title and end cards. It draws any time of the
   video on request, with no clock of its own.
3. `contrib/promo/render.cjs` asks a headless Chrome for every frame and has
   ffmpeg encode them to `target/tabletist-promo.mp4`.

## Storyboard

| Seconds | Shot | Caption |
|---|---|---|
| 0.0 to 1.3 | Icon, name, "A small, fast, native database client." | |
| 1.3 to 4.0 | The file opens, `cards` fills, the cursor walks down the titles | Open a table. It's just there. |
| 4.0 to 6.7 | A filter typed on `title`, five rows left | Filter without writing SQL. |
| 6.7 to 9.6 | A title typed over, the pending change, Review SQL | Edit. Review. Then save. |
| 9.6 to 13.5 | A join typed in a SQL editor, a completion taken, its rows | Or just write the SQL. |
| 13.5 to 15.0 | Icon, name, engines, platforms, igor.works/tabletist | |

## The data

The frames show Fizzy's development database (its seed data), by request,
and only its `cards` and `boards` tables. The site's screenshots keep the
Bookshop data.
