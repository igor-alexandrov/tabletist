---
title: Connections
description: Every field of a connection, connection URLs, TLS modes, SSH tunnels, host keys, passwords and read-only connections.
nav_order: 0
---

## The picker

Cmd/Ctrl+O (Ctrl+Shift+C in the Omarchy look) opens the picker with your
saved connections. Each row shows
the connection's environment and when it was last used, and
**Find connection…** searches by name, by the group you gave it, or by
`user@host:port/database`.

{% include shot.html file="macos-connections" alt="The connection picker: three connections named Bookshop grouped under that name, and two local ones" %}

A connection you put in a group, with **Group** in its form, is listed
under that group: production and staging, one group per product, or
whatever suits you. Of the others, those with exactly the same name form
a group under that name, which suits one database in several
environments, and the rest are listed under **Local** or **Remote**.
Groups are listed by name, with Local and Remote last, and a group
written in another case is the same group.

A connection can be open more than once. Enter shows a connection that is
already open, and Shift+Enter, or **Open another** in its row's menu,
opens one more window of it, on its saved database.

Cmd/Ctrl+Backspace (`dd` in the Omarchy look), **Delete** in a row's menu,
or **Delete** in the connection's form asks at the bottom of the picker
before it deletes the connection. Where the keyring holds a secret of
the connection, it names the one that goes with it: the password, the
key's passphrase, the SSH password or SSH secret, or the password and
the SSH one. Keeping a connection asked
about from its form opens the form again, edits and all. Deleting cannot
be undone: the
connection goes with its saved password and SSH secret, its labels, and
what was last open on it.

## Another database of the server

The chip of the connection you are in (the start screen's **Switch
database** and quick open's open the same menu) lists the server's other
databases, where it has more than one. Picking one switches the window to
it: the tables close, and the SQL editors stay. Cmd-click or Cmd+Enter
(Ctrl on Windows and Linux, Ctrl+Enter in the Omarchy look) opens it in a
new window instead, right after this one and in front, with a session of
its own. The row under the pointer or the keyboard names the key. The new
window connects with the password this one connected with, so nothing is
asked again, and this one keeps its tables, editors and edits. Do it on
the database you are on for a second window on it. The chips tell the
windows apart by their database. The next time Tabletist starts, each
window opens again on its own database, with its own tables where
**Restore tabs and filters** is on.

## Fields

| Field | Notes |
| --- | --- |
| **Name** | Left empty, it becomes `user@host:port/database`, the file's name or the BigQuery project |
| **Type** | SQLite, PostgreSQL, MySQL, BigQuery or Files, as radio buttons in rows above the name. In the Omarchy look each type is a Tab stop and Enter or Space picks it; in the other looks the type is one Tab stop and the arrow keys choose. MySQL is also for [MariaDB](#mariadb). Changing it keeps the name, the environment and whatever was typed for the other types until the dialog closes. Saving a connection you have used under another type asks first |
| **Group** | The group the picker lists it under: one the picker shows, or **New group…**, which turns the box into a field for a new group's name (Esc goes back). A connection put in the group it would be in anyway, the name it shares, Local or Remote, has none of its own, and follows its name and server. See [The picker](#the-picker) |
| **Environment** | local, dev, staging, production or none. Sets the connection's color |
| **File** | SQLite only |
| **Project**, **Key file** | BigQuery only. See [BigQuery](#bigquery) |
| **Source** | Files only. See [Files](#files) |
| **Host**, **Port** | The port defaults to 5432 for PostgreSQL and 3306 for MySQL |
| **Database** | Left empty, PostgreSQL uses the user's name and MySQL selects no database |
| **User**, **Password** | See [Passwords](#passwords) |
| **SSL mode**, **CA certificate** | See [TLS](#tls) |
| **Connect through SSH tunnel** | See [SSH tunnels](#ssh-tunnels) |
| **Open read-only** | See [Open read-only](#open-read-only) |
| **Agents** | See [Agents](#agents) |

**Test** checks the connection without saving it.

{% include shot.html file="macos-connection-edit" alt="The connection dialog with every field filled in for a production database behind an SSH tunnel" width="2880" height="1800" %}

### MariaDB

A MariaDB server is opened as MySQL, and a `mariadb://` URL means the
same as `mysql://`. The footer names the server: `MariaDB 11.4.5`. Where
MariaDB is not MySQL:

- A JSON column is text to MariaDB (`LONGTEXT` with a check), and shows
  as text.
- Integer types keep their display width: `int(11)`.
- MariaDB does not list what a view reads, so Tabletist reads its
  definition. While anything is hidden or masked, agents may not read a
  view whose definition you may not see (without SHOW VIEW) or that
  Tabletist cannot read for its tables (see [Agents](#agents)).

## Files

A Files connection opens data files as tables: CSV, TSV, Parquet, JSON
(an array), NDJSON (`.ndjson`, `.jsonl`), Excel (`.xlsx`, its first sheet)
and DuckDB files (`.duckdb`). DuckDB reads them, inside Tabletist.

- **Source** is a **Folder** or **Files**. Every supported file directly
  in a folder becomes a table; subfolders, hidden files and other files
  are left out, and a file added later shows the next time the connection
  opens. **Files** is a list you add files to (**Add files…**); × takes a
  file off the list and never off the disk.
- In the Omarchy look the folder is typed, or picked with Ctrl+O, and the
  list has an **add** line under it for a file's path. While you type a
  path that starts with `/` or `~`, the names of its folder that start
  with its last part are listed under the line: folders for the folder,
  and folders and the files a Files connection reads for the add line.
  Ctrl+N and Ctrl+P (or Down and Up) move, Enter takes one (a folder
  lists its own names next, a file is added), and Esc closes the list.
  Tab moves on and never completes. Enter in the add line with nothing
  listed adds the path as typed, if it names a file from `/` or `~`.
- Under the source, Tabletist says what it found: `4 files · 2.5 GB · CSV,
  Parquet, NDJSON`, and the tables they become.
- A table's name is the file's name without its extension, in lower case,
  with anything but letters, digits and `_` turned into `_`. Two files of
  the same name each take their extension: `orders_csv`,
  `orders_parquet`. A `.duckdb` file is a schema, its tables inside it.
- **Reading CSV**, closed until you open it, says how the connection's
  CSV and TSV files are read: **Header row** (Detect, Yes, No),
  **Delimiter** (Detect, `,`, `;`, Tab, `|`; a `.tsv` file is always
  read with a tab) and **Column types** (from the first 20,000 rows, from
  all rows, which is slower, or everything as text). Closed, it says the
  three on one line, and how many overrides there are. A change for one
  file, such as a column read as text, shows under them with **Reset**
  (`R` in the Omarchy look, on the override's row); for now an override
  is written in `connections.toml` (see [Settings and
  Files]({% link _reference/settings-and-files.md %}#the-connections-file)).
  A connection that is open keeps reading its files as it did when it
  opened, Reconnect included: close it and open it again from the
  connections list to read them with the new settings.
- **Join with saved connections**, closed until you open it, lists your
  saved PostgreSQL, MySQL and SQLite connections. Tick one to attach it:
  its tables are then in the Files connection too, under a name of its
  own, so one query can join a file to them (`SELECT … FROM returns r
  JOIN bookshop_development.publishers p ON …`). The name is its
  database's (a SQLite file's name without its extension) and you can
  type over it: lowercase letters, digits and `_`, and not a schema the
  connection has already. In the sidebar each attached connection is a
  schema, listing the tables of its default schema (PostgreSQL's
  `public`, MySQL's database, SQLite's own); its other schemas are
  reached in SQL, as `bookshop_development.billing.invoices`. An attached
  connection is always read-only. Its password comes from the keyring as
  the Files connection opens, and its SSH tunnel, if it has one, is
  opened then too; a connection whose password is asked for on every
  connect, or a MySQL one that names no database, cannot be attached. If
  one cannot be attached when the Files connection opens or is tested,
  that says which and why; an SSH host whose key is not trusted yet can
  be trusted from the Test. A MySQL connection reached through an SSH
  tunnel with TLS **verify-full** checks the server's certificate against
  `127.0.0.1` and fails: attach it with **verify-ca** and its CA file
  instead, which checks the certificate's issuer but not its name. In the Omarchy look each row is a Tab stop,
  Space ticks it and Up and Down move between the rows.
- The first time a Files connection needs DuckDB's reader for Excel,
  PostgreSQL, MySQL or SQLite, DuckDB downloads it, and Test says so
  while it does ("Downloading the PostgreSQL reader for DuckDB…"). Once
  downloaded, it is used offline.
- A new Files connection is **local**, and lets agents read. Files open
  read-only: nothing writes to them, and a statement that would read
  another file on the computer, write a file or install anything is
  refused. Agents can query the files, never change them, and cannot
  hide tables or mask columns on a Files connection. Its **Agents**
  section is closed, and says the access on one line.
- **Test** reads each file's columns and its first rows, never a whole
  file to the end. With **Column types** from all rows, DuckDB reads a
  CSV in full to find its types, which takes longer on a large file.
  After a Test that read, the CSV, TSV and NDJSON files are read to
  their end in the background while the dialog is open, one at a time:
  the footer adds "checking 2 files…". That is how a value past the
  first rows is found, such as an `N/A` on row 48,112 of a date column.
- A file that cannot be read makes the Test **partial**: the footer says
  "Read 3 of 4 files · returns.csv failed" (with several, how many), and
  a banner over it names the file, the row, the value and the column.
  For a value its column cannot take it offers three fixes: **Read
  returned_at as text**, **Read 'N/A' as empty** (that value becomes
  NULL in that column) and **Skip returns.csv**; for anything else, such
  as a row with more values than the others or a line of NDJSON that is
  not JSON, it offers to skip the file. In the Omarchy look `t`, `e` and
  `s` run them. Each fix is an override under Reading CSV, and the Test
  runs again; it is saved with the connection. A file that cannot be
  opened at all, a file in a folder Tabletist is not allowed to read, or
  a listed file that is gone, still stops the connection from opening:
  Test shows it as partial, with its fixes. The Excel reader is
  downloaded the first time a connection opens an Excel file.
- The same banner shows on a table's view when its rows cannot be read
  while you browse: a page, a sort or a filter that reaches a row such
  as 48,112 says which file, row, value and column, in place of the
  error card, with the same fixes. In the Omarchy look `t`, `e` and `x`
  run them, and `s` still shows the structure. A fix is saved with the
  connection at once and the connection opens again with it: its tabs
  stay and the table reads again. **Skip** closes the table's tab, and
  the table leaves the sidebar. Reset under Reading CSV in the
  connection's dialog takes a fix off.
- On macOS, a folder that macOS's privacy settings have not let
  Tabletist open (one in Documents, Desktop or Downloads, or on an
  external disk, until you allow it) makes the Test fail with "Can't read
  ~/Data/bookshop-exports. macOS hasn't allowed Tabletist to open this
  folder." and **Open Privacy & Security**, which opens System Settings
  at Privacy & Security › Files and Folders. Opening the connection says
  the same on its window, with the same button first. macOS keeps what
  you allow there, so Tabletist keeps no bookmark of the folder.
- The SQL editor runs DuckDB's SQL, and the Structure view shows each
  file's path, size and format beside its columns, and what it is read
  with.
- A connection has no URL: paste a folder's or a file's path into the
  **URL** field of a new connection instead, and it becomes a Files
  connection over it. A SQLite database's path (`.db`, `.sqlite`,
  `.sqlite3`, `.db3`) opens as SQLite.

## Connection URLs

The **URL** tab of the dialog (`u` in the Omarchy look) takes a connection
URL and fills the fields from it. A password in the URL moves to the
password field.

```text
postgres://app@db.example.com:5432/shop?sslmode=verify-full
mysql://app@127.0.0.1:3306/shop
sqlite:///home/me/data/shop.db
bigquery:my-project?credentials=/home/me/keys/reader.json
```

- The schemes are `postgres://`, `postgresql://`, `mysql://`, `mariadb://`,
  `sqlite:` and `bigquery:`.
- `sslmode` (also `ssl-mode` or `ssl_mode`) sets the SSL mode, and
  `sslrootcert` (also `ssl-ca`) the CA certificate. For BigQuery,
  `credentials` sets the key file. Other parameters are ignored.
- A URL cannot be given on the command line.

## BigQuery

A BigQuery connection names a **Project**, the Google Cloud project its
queries are billed to, and how it signs in:

- **Key file:** a service-account key (JSON). The file stays where it is,
  and the connection keeps its path.
- **Left empty:** Google Cloud's application default credentials. These
  are the file `GOOGLE_APPLICATION_CREDENTIALS` names, else the one
  `gcloud auth application-default login` saved, else the service account
  of the Google Cloud machine Tabletist runs on.

The sidebar lists the project's datasets as schemas, with their tables
and views. A BigQuery connection always opens read-only, whatever
[Open read-only](#open-read-only) says: BigQuery has no transactions, so
Tabletist runs only statements it can tell are reads. Rows cannot be
edited, and a table has no DDL view. Agents can read a BigQuery
connection only while it hides no table and masks no column.

## TLS

The SSL modes are libpq's:

| Mode | PostgreSQL | MySQL |
| --- | --- | --- |
| `disable` | No TLS | No TLS |
| `prefer` (the default) | Encrypts when the server can. The certificate is not checked | The same |
| `require` | Encrypts. The certificate is not checked, unless you give a CA certificate | Encrypts. The certificate is not checked |
| `verify-ca` | Checks the certificate against your CA certificate, which is required | Not available yet |
| `verify-full` | Checks the certificate and the host name, against your CA certificate or the system's | Checks the certificate and the host name, against your CA certificate or the system's |

In a URL, `allow` is read as `prefer`.

The CA certificate file can be PEM (`.pem`, `.crt`) or one certificate
in binary DER, as most `.cer` files are.

Unless the mode checks the certificate, someone on the network between
you and a remote server can read the password. To prevent that, use
`verify-full` or an SSH tunnel. The dialog warns you when this applies.

A MySQL server can ask for the password in plain text
(`mysql_clear_password`), as AWS RDS IAM authentication does: type the
token from `aws rds generate-db-auth-token` as the password. Tabletist
sends it only over an encrypted connection. Choose `require` or
`verify-full`; with `disable`, or when `prefer` finds no TLS, the
connection fails instead.

## SSH tunnels

Tick **Connect through SSH tunnel** and Tabletist reaches the database
through an SSH server. You can sign in to that server with:

- **Password.**
- **Key file.** An encrypted key asks for its passphrase.
- **Agent.** Your running SSH agent, including 1Password's.

### ~/.ssh/config

Tabletist reads `~/.ssh/config`. Type a host alias as the **SSH host**, or
pick one from the list, and the rest is filled in from the file. A value
you type wins over the file.

| Directive | Used for |
| --- | --- |
| `Host` | Aliases and patterns, with `*`, `?` and `!` |
| `HostName` | The server to connect to |
| `Port`, `User` | The port and the login name |
| `IdentityFile` | The key file. The first one is used |
| `IdentityAgent` | The agent's socket. This is how 1Password is found |
| `ProxyJump` | The jump hosts the tunnel goes through |
| `ProxyCommand` | A command the tunnel goes through |
| `Include` | Other files to read |

Without an `IdentityAgent`, the agent is the one in `SSH_AUTH_SOCK`. On
Windows it is the OpenSSH agent, then Pageant.

`Match` blocks are ignored.

### ProxyJump and ProxyCommand

A host with a `ProxyCommand` is reached through that command, as `ssh`
reaches it: Tabletist runs it in your shell and talks to the SSH server
over its input and output. On Windows, Tabletist starts the command's
program itself with no shell in between, as Windows' own `ssh` does. It takes the tokens `%h`
(the host name), `%p` (the port), `%r` (the user), `%n` (the host as you
typed it) and `%%`. A host name or user with a space, a quote or another
character the shell would read is refused rather than put into the
command. Started from the Dock on macOS, the command has the `PATH`
macOS gives apps, not the one your shell sets up: if a program is not
found, write its whole path (`/opt/homebrew/bin/cloudflared`).

A host with a `ProxyJump` is reached the way `ssh` reaches it: Tabletist
runs `ssh -W` through the jump hosts. The jump hosts are logged in to by
your `ssh`, with its config, your agent and its `known_hosts`, so a jump
host you can reach from a terminal works here too. They cannot ask for a
password or for trust in a new key: connect to each one from a terminal
once first. The `ssh` command has to be installed and on the `PATH`.

Either way, the SSH server at the end is logged in to with the method you
chose, and its key is checked against Tabletist's trusted keys, as for a
direct connection. The login through a proxy has two minutes, time
enough to approve an agent's prompt. If the command fails, or the login
runs out of time, the error shows the last lines the command wrote.

### Host keys

The first time you connect to an SSH server, Tabletist shows its key's
fingerprint and asks **Trust this SSH host?** Compare the fingerprint
with the server's before you trust it.

Trusted keys are kept in Tabletist's own `known_hosts.json`, not in
OpenSSH's `known_hosts`. If a server's key changes later, Tabletist
refuses to connect, because that can mean someone is intercepting the
connection. If the key really did change:

1. Quit Tabletist. It reads the file when it starts and writes it when
   you trust a host, so an edit made while it runs can be lost.
2. Open [`known_hosts.json`]({% link _reference/settings-and-files.md %}#files)
   and remove the server's entry from `hosts`. Keep the file valid JSON:
   the last entry has no comma after it.
3. Start Tabletist and connect. It asks about the new key.

If the file cannot be read, Tabletist leaves it as it is and trusts no
new host until it is fixed.

## Passwords

Passwords are never written to Tabletist's own files. Each password field
has a keyring checkbox:

- **Ticked:** the password is kept in the system keyring. That is the
  Keychain on macOS, Credential Manager on Windows, and the Secret Service
  (GNOME Keyring, KWallet) on Linux.
- **Not ticked:** Tabletist asks for the password when you connect, and
  remembers it only for that connection tab.

A password typed in the connection dialog goes to the keyring when you
save the connection. One typed when Tabletist asks for it is saved only
after the server accepted it. The same goes for an SSH password and a
key's passphrase.

When you edit a connection whose password is in the keyring, the
Password field is empty and says it is saved there. To connect without a
password (a server with trust or peer authentication), put the cursor in
that field and press Backspace or Delete: the note goes, and the
connection is saved as needing no password (`password = "none"`), with
its keyring entry deleted. **Test** then tries the server with no
password. Type a password instead to replace the saved one.

When the keyring has no password for a connection, Tabletist asks for
one. If you leave it blank and the server accepts that (trust or peer
authentication), the connection is saved as needing no password
(`password = "none"`), and Tabletist and agents connect without asking
from then on. Should the server start asking for a password, Tabletist
asks you for it again, and saves it in the keyring once it is accepted.

## Agents

What AI agents may do on a connection, through `tabletist mcp`. Every
connection starts with agents **Off**: they do not see it at all. A new
[Files](#files) connection starts with **Read**, and with **Off** on
production.
**Read** lets them read it, in read-only transactions, while the agent
server is on.

- In the picker, the **Agents** column says each connection's access, and
  its "…" menu changes it. Its **Tables, columns and limits…** opens the
  connection's **Agents** section, and **Show Agents tab** shows its
  Agents tab.
- In the connection's form, the **Agents** section has the access,
  **Tables**, **Masked columns**, **Rows per answer** (1 to 5000, 500 at
  first) and the **Statement timeout** in seconds (1 to 60, 10 at first).
- **Tables** is **All**, **All except** or **Only**. With **All except**,
  agents may read every table but the ones listed; with **Only**, the ones
  listed and no others, so that a table made later is hidden too. **Add table…** opens
  a field for a name, `customers` or `sales.customers`: Enter adds it and
  leaves the field open for the next, Escape closes the field, and **×**
  takes a name out. While a window of the connection is open, the field
  lists the tables its sidebar has loaded that match what you type: Up and
  Down move in the list, and Enter, Tab or a click adds the one chosen. A
  listed name that the loaded schemas show is not a table is marked **not
  in this database**, and under **All except** agents may not use the
  connection until it is taken out or put right. A name in a schema the
  sidebar has not loaded is not marked. A name still in the field when you save, or choose
  another of the three, is added with the rest. Each of the two lists is kept while you try the other, and
  what you save is the one chosen. Agents are told the names in the list,
  never what is in the hidden tables: not their rows, their columns, or
  the views over them. A function from an extension can still read them;
  for strict isolation, use a database user without access.
- **Masked columns** lists the columns whose values agents get as `•••`.
  **Add column…** opens a field for a table and a column, `orders.email`
  or `sales.orders.email`, which works as Add table…'s does: Enter adds
  it, Escape closes the field, **×** takes one out, and one still in the
  field when you save is added. While a window of the connection is open,
  the field lists the tables its sidebar has loaded; Enter or a click on
  one puts it in the field, and the list then shows its columns, which the
  window is asked for. A masked column that the loaded schemas and fetched
  columns show is not in the database is marked **not in this database**,
  and agents may not use the connection until it is taken out or put
  right. Agents see a masked column's name and type, and a query that
  filters, joins, sorts or computes with it is refused, in any table with
  a column of that name.
- If `connections.toml` hides more tables while the form is open, saving
  a change of yours to them is refused, and the form shows them as they
  are now, for you to make your change again. A change in the file that
  hides less is held until you answer it, and is not in force: a save that
  changes the tables goes through over it, and one that leaves them as
  shown keeps it, still held. Masked columns are kept the same way: if
  the file masks more while the form is open, saving a change of yours to
  them is refused and the form shows them as they are now.
- An Agents tab that says agents can't use a connection has **Allow
  agents…**, which opens that section with **Read** chosen. Nothing
  changes until you save.

The file keeps this in the connection's `[connections.agents]` table: see
[Settings and Files]({% link _reference/settings-and-files.md %}#the-connections-file).

## Open read-only

A connection with **Open read-only** ticked blocks every write from
Tabletist: the session itself is read-only on PostgreSQL and MySQL, and a
SQLite file is opened read-only. A BigQuery connection is read-only with
the box off too: see [BigQuery](#bigquery).

The box is ticked by default for production, and not for the other
environments. Turn it off to edit. A change applies from the next time
you connect. Where a new production connection's box starts is
**New production connections open read-only** in
[Settings]({% link _reference/settings-and-files.md %}#the-settings-window).

Production stays careful with the box off too:

- A save asks first, with its statements on screen.
- The SQL editor still runs read-only.
