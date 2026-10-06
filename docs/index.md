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
    src: /assets/images/macos.png
    alt: "Tabletist in its macOS look: a table's data grid, the sidebar of tables on the left and the row panel on the right"
    width: 2880
    height: 1800

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
    details: Sort, filter and page on the server. Open a row in full, follow its foreign keys, and read the table's structure.
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

<div class="look-showcase">
  <img src="{{ '/assets/images/omarchy.png' | relative_url }}" alt="Tabletist in its Omarchy look: the same table in a dark, square, keyboard-first layout with key hints along the bottom" width="3840" height="2160" loading="lazy">
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
  .look-showcase img {
    display: block;
    max-width: 100%;
    height: auto;
    border-radius: 12px;
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.35);
  }
  @media (max-width: 959px) {
    .VPHero .image {
      margin: 0 0 24px !important;
    }
  }
</style>
