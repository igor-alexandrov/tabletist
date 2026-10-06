---
layout: home
title: Tabletist
description: A fast, native database client for PostgreSQL, MySQL and SQLite.
permalink: /
hero:
  name: Tabletist
  text: Your databases, native and fast
  tagline: A database client for PostgreSQL, MySQL and SQLite. Written in Rust, for Linux, macOS and Windows.
  actions:
    - theme: brand
      text: Download
      link: /download/
    - theme: alt
      text: What is Tabletist?
      link: /what-is-tabletist/
    - theme: alt
      text: GitHub
      link: https://github.com/igor-alexandrov/tabletist
  image:
    light: /assets/images/macos-table.webp
    dark: /assets/images/macos-table-dark.webp
    alt: "Tabletist in its macOS look: a table's data grid, the sidebar of tables on the left and the row panel on the right"
    width: 2560
    height: 1600

features:
  - icon: ⚡
    title: Native
    details: Written in Rust, with no browser inside. One small program for Linux, macOS and Windows.
  - icon: 🗄️
    title: Three databases
    details: PostgreSQL, MySQL and SQLite, with the same grid, row panel and SQL editor for each.
  - icon: 🔐
    title: Connections that fit your setup
    details: TLS, SSH tunnels with a password, a key file or your agent, and passwords kept in the system keyring.
    link: /connections/
    link_text: How connections work
  - icon: 🛡️
    title: Careful with production
    details: A production connection opens read-only unless you say otherwise, and a save to production asks first, with its SQL on screen.
  - icon: ✏️
    title: Edit in the grid
    details: Changes wait until you save and go in as one transaction. When someone else changed the same row, the save stops and asks you.
    link: /editing-data/
    link_text: How editing works
  - icon: 🧭
    title: Made for browsing
    details: Sort, filter and page on the server. Open a row to read every field, follow its foreign keys, and read the table's structure.
    link: /browsing-data/
    link_text: Browse a table
  - icon: ⌨️
    title: SQL editor
    details: Run the statement at the cursor or the whole script, with completion, a formatter, a row limit and a timeout.
    link: /sql-editor/
    link_text: Open the guide
  - icon: 🔓
    title: Open source
    details: Free to use, study and improve, under the MIT license.
    link: https://github.com/igor-alexandrov/tabletist
    link_text: Read the source
---

## Two looks, one app

The picture above is the macOS look, and Windows gets a close relative of
it. On Linux, Tabletist takes the Omarchy look: square, keyboard first,
with vim keys and the desktop's monospace font throughout. On Omarchy it
follows the desktop's theme and recolors when you switch themes.
Read about [the Omarchy look]({% link _guide/omarchy.md %}) and
[Tabletist on macOS]({% link _guide/macos.md %}).

{% include shot.html file="omarchy-table" alt="Tabletist in its Omarchy look: the same table in a dark, square, keyboard-first layout with key hints along the bottom" width="2880" height="1800" %}

## A closer look

<div class="shot-grid">
  <figure>
    {% include shot.html file="macos-edit-review" alt="Review SQL open above the bar of pending changes" %}
    <figcaption>Read the SQL of your changes before it runs.</figcaption>
  </figure>
  <figure>
    {% include shot.html file="macos-edit-conflict" alt="The question about a row that changed on the server" %}
    <figcaption>See what changed on the server before you decide.</figcaption>
  </figure>
  <figure>
    {% include shot.html file="macos-sql-complete" alt="The completion list in the SQL editor" %}
    <figcaption>Write SQL with completion for tables and columns.</figcaption>
  </figure>
  <figure>
    {% include shot.html file="macos-connections" alt="The connection picker with connections tagged by environment" %}
    <figcaption>Keep every connection, tagged with its environment.</figcaption>
  </figure>
</div>

## Try it without a database

Start Tabletist with `--demo` and it opens a sample database, so you can
look around before you connect to your own.
[Get started]({% link _guide/getting-started.md %}).

<style>
  /* The hero image slot is sized for a square logo; the screenshot needs the
     room. Page-scoped overrides, so the theme stays untouched. */
  .VPHero .image-container {
    width: 100% !important;
    height: auto !important;
    transform: none !important;
  }
  .VPHero .image-src {
    position: relative !important;
    top: auto !important;
    left: auto !important;
    transform: none !important;
    width: 100% !important;
    height: auto !important;
    max-width: 100% !important;
    max-height: none !important;
    padding: 0 !important;
    border-radius: 12px;
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.25);
  }
  .shot-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 24px;
  }
  .shot-grid figure {
    margin: 0;
  }
  .shot-grid a.shot {
    margin: 0 0 8px;
  }
  .shot-grid figcaption {
    color: var(--vp-c-text-2);
    font-size: 14px;
  }
  @media (max-width: 767px) {
    .shot-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (max-width: 959px) {
    .VPHero .image {
      margin: 0 0 24px !important;
    }
  }
</style>
