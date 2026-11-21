---
title: Speed and security
description: Why Tabletist stays quick on a far-away server, and how it keeps your passwords, your data and its own updates safe.
nav_order: 1
---

Tabletist is made to be quick over a slow network and careful with what
you trust it with. This page gathers what that means, with links to the
details.

## Fast

### A native program

Tabletist is written in Rust and draws its own window, so there is no
browser inside. Connecting, queries, exports and downloads run away from
the window, so it keeps answering while the database works, and
Cmd/Ctrl+. (Ctrl+C in the Omarchy look) cancels a running query.

Sorting, filtering and paging are done by the server, and only the page
you look at comes over the network, so a table of millions of rows opens
without loading them.

### Few round trips

On a remote server most of the wait is the network, not the database.
Each time a client sends something and waits for the answer is a round
trip, and a server in another region or behind a VPN can take 100 to
200 ms for each. Tabletist sends what a step needs together, so on
PostgreSQL, MySQL and MariaDB the steps you take most wait on the server
once:

| What you do | Round trips |
| --- | --- |
| List the schemas and the default schema's tables, once connected | 1 |
| Open a table from the sidebar: its structure and its first page | 1 |
| Load a page: the next one, or after a sort or a filter | 1 |
| Count a table's rows | 1 |
| Run a read-only `SELECT` in the SQL editor | 1 |

Through a network that adds 200 ms to every round trip, a table opens in
about 0.2 seconds. From pressing Connect to seeing the tables takes about
1.2 seconds on PostgreSQL and 0.9 on MySQL and MariaDB, a little more
with TLS, whose handshake has round trips of its own.

A few steps take more: a run that writes, a table whose key changed
since the sidebar listed it, a table whose structure the sidebar's
listing did not read (opening it waits once for the structure and again
for the first page), and on MySQL and MariaDB a table with a `FLOAT`
column. A SQLite database is a file on your computer, with no
network in between.

An export opens a connection of its own. On PostgreSQL it then waits on
the server three times for a table and twice for a SQL result, and on
MySQL and MariaDB twice for either (a few times more for a view, or
for one with a `FLOAT` column), however many rows it writes, and the
rows come as fast as the network carries them; through an SSH tunnel,
up to about 80 MB a second at that distance. Through a network that
adds 200 ms to every round trip, 200,000 rows of ten columns are written
to a CSV file in about 1.8 seconds, the connection included.

## Secure

### Connections

- **TLS.** Choose `verify-full` and Tabletist checks the server's
  certificate and its host name. When the mode you chose would let
  someone on the network read the password, the connection dialog says
  so. [Read about TLS]({% link _reference/connections.md %}#tls).
- **No password in plain text.** When a MySQL server asks for the
  password in plain text, as AWS RDS IAM authentication does, Tabletist
  sends it only over TLS. Without TLS the connection fails instead.
- **SSH tunnels.** Sign in with a password, a key file or your SSH agent,
  1Password's included, and go through the `ProxyJump` and `ProxyCommand`
  hosts your `~/.ssh/config` names. For a `ProxyCommand`, a host name or
  user with a character the shell would read is refused, not put into
  the command.
  [Read about SSH tunnels]({% link _reference/connections.md %}#ssh-tunnels).
- **Host keys.** The first time you connect to an SSH server, Tabletist
  shows its key's fingerprint and asks whether to trust it. If the key
  changes later, Tabletist refuses to connect, since that can mean
  someone is in the middle.
  [Read about host keys]({% link _reference/connections.md %}#host-keys).

### Passwords

Passwords are never written to Tabletist's files or its log. They are
kept in the system keyring (the Keychain on macOS, Credential Manager on
Windows, the Secret Service on Linux), or asked for each time you
connect. A password you type when asked is saved only once the server
accepted it, and agents never get it.
[Read about passwords]({% link _reference/connections.md %}#passwords).

### Read-only, kept by the database

Read-only is the database's to keep, not a guess from a statement's
words. With **Open read-only** the session itself is read-only on
PostgreSQL and MySQL, and a SQLite file is opened read-only. A run in the
SQL editor's Read-only mode happens in a read-only transaction that is
rolled back. BigQuery has no transactions, so a BigQuery connection is
always read-only and runs only the statements Tabletist can tell are
reads.

Production is careful by default:

- A production connection opens read-only unless you turn that off.
- With it off, a save asks first, with its statements on screen, and the
  SQL editor still runs read-only.

On every connection, a save is one transaction, and when someone else
changed the same row it writes nothing and asks you what to do. In the
SQL editor, a run that would change every row of a table (an `UPDATE` or
`DELETE` with no `WHERE`, or a `TRUNCATE`) asks before it runs.

[Read about Open read-only]({% link _reference/connections.md %}#open-read-only).

### Agents

Agents read only the connections you allow, within the rows and the
time you set: on PostgreSQL, MySQL and SQLite in a read-only transaction
that is rolled back, and on BigQuery only the statements Tabletist can
tell are reads. They never get
a connection's password. Tables you hide and the values of columns you
mask are kept from them, and so are views that would read what they may
not. Every call goes into a log you can read, which keeps what was asked
and never a row. On Linux and macOS, an administrator can set limits in a
policy file that an agent running as you cannot change.
[Connect an agent]({% link _guide/agents.md %}) and read about
[the administrator's policy]({% link _reference/settings-and-files.md %}#the-administrators-policy).

### Releases and updates

- Each release lists its files in a `checksums.txt` signed with the
  Tabletist release key. The macOS app is signed by its developer and
  notarized by Apple.
  [See the download page]({% link _guide/download.md %}).
- Before an update is installed, Tabletist checks the signature of the
  release's checksums, the file's checksum, and that the new version
  starts; on macOS also that it is Tabletist, signed by its developer and
  approved by Gatekeeper. A file that fails is deleted, and nothing is
  installed.
  [Read about updates]({% link _reference/settings-and-files.md %}#the-settings-window)
  (the Updates pane).

### The code

- Tabletist is open source under the MIT license, so anyone can read
  what it does with your connections. The code is on
  [GitHub](https://github.com/igor-alexandrov/tabletist).
- Rust keeps memory safe. The code forbids `unsafe`, apart from three
  small crates that call macOS's AppKit, the menu bar and SQLite, each
  behind a safe interface.
- The dependencies are checked against the RustSec advisory database
  every day, and on every change to them.
