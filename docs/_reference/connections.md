---
title: Connections
description: Every field of a connection, connection URLs, Redis, TLS modes, SSH tunnels, host keys, passwords and read-only connections.
nav_order: 0
---

## The picker

Cmd/Ctrl+O (Ctrl+Shift+C in the Omarchy look) opens the picker with your
saved connections. Each row shows
the connection's environment and when it was last used, and
**Find connection…** searches by name or by `user@host:port/database`.

{% include shot.html file="macos-connections" alt="The connection picker: three connections named Bookshop grouped under that name, and two local ones" %}

Connections with exactly the same name form a group under that name, which
suits one database in several environments. The rest are listed under
**Local** or **Remote**.

A connection can be open more than once. Enter shows a connection that is
already open, and Shift+Enter opens it again.

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

## Fields

| Field | Notes |
| --- | --- |
| **Name** | Left empty, it becomes `user@host:port/database`, the file's name or the BigQuery project |
| **Type** | SQLite, PostgreSQL, MySQL, Redis or BigQuery |
| **Environment** | local, dev, staging, production or none. Sets the connection's color |
| **File** | SQLite only |
| **Project**, **Key file** | BigQuery only. See [BigQuery](#bigquery) |
| **Host**, **Port** | The port defaults to 5432 for PostgreSQL, 3306 for MySQL and 6379 for Redis |
| **Database**, **Username**, **Use TLS**, **Browsing** | Redis only. See [Redis](#redis) |
| **Database** | Left empty, PostgreSQL uses the user's name and MySQL selects no database |
| **User**, **Password** | See [Passwords](#passwords) |
| **SSL mode**, **CA certificate** | See [TLS](#tls) |
| **Connect through SSH tunnel** | See [SSH tunnels](#ssh-tunnels) |
| **Open read-only** | See [Open read-only](#open-read-only) |
| **Agents** | See [Agents](#agents) |

**Test** checks the connection without saving it.

{% include shot.html file="macos-connection-edit" alt="The connection dialog with every field filled in for a production database behind an SSH tunnel" width="2880" height="1800" %}

## Connection URLs

The **URL** tab of the dialog (`u` in the Omarchy look) takes a connection
URL and fills the fields from it. A password in the URL moves to the
password field.

```text
postgres://app@db.example.com:5432/shop?sslmode=verify-full
mysql://app@127.0.0.1:3306/shop
sqlite:///home/me/data/shop.db
bigquery:my-project?credentials=/home/me/keys/reader.json
rediss://reader@cache.example.com:6380/2
redis-cli -h cache.example.com -p 6380 -n 2 --user reader --tls
```

- The schemes are `postgres://`, `postgresql://`, `mysql://`, `mariadb://`,
  `sqlite:`, `bigquery:`, `redis://` and `rediss://`. A `redis-cli` line
  is read too: its `-h`, `-p`, `-n`, `-a`, `--user`, `--pass`, `--tls`,
  `--cacert`, `--cert`, `--key` and `-u`.
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

## Redis

A Redis connection is to a standalone server (Redis, or one that speaks
its protocol: Valkey, KeyDB, Dragonfly), with TLS and an SSH tunnel as
for the others. Sentinel, Cluster and Unix sockets are not supported.

- **Database:** the one the window opens on, 0 to 15. After a Test the
  list says what each holds ("0 · 1,204 keys"), and lists every database
  where the server has more than 16 and the user may ask how many.
- **Username:** empty signs in as the default user, with the password
  alone. Leave both empty for a server without a password. Tabletist
  signs in with `HELLO 3` where the server has it, else with `AUTH`.
- **Use TLS (rediss://):** the server's certificate is checked against
  the CA certificate, else the system's, and its name unless
  **Verify the server's hostname** is off. A client certificate and its
  key go together.
- **Browsing:** the separator keys are grouped by (`:`), how many keys
  are read at a time (500, 1,000 or 5,000), the pattern of the keys
  listed (`*`), and the most keys held (100,000).

The **URL** tab of a Redis connection stays in step with the fields: it
shows them as a URL, and what you type there fills them. The password
shows as dots; it is kept in the Keychain or keyring, never in the URL or
in `connections.toml`.

**Test** says what it reached: "Connected · Redis 7.4.1 · db 0 · 1,204
keys · 2 ms", with "Redis-compatible" after another server's name. A
failure names its cause and keeps the server's code: a wrong password
(`WRONGPASS`), a server that needs one (`NOAUTH`), a server that closed
the connection (it may need TLS), nothing listening, no answer in 10
seconds. A user that may not run `INFO` or `SCAN` connects, and Test
says that its keys can't be listed (`NOPERM`).

### The window

The sidebar lists the database's keys grouped by the separator:
`book:reviews:12` is under `book:` › `reviews:`, a key without it is at
the top, and a group of one key shows as that key. Each key has its type.
A group shows a page of its keys, then a row that shows the next page.
**Filter keys…** takes a pattern (`book:*`) and scans again with it. The
connection's chip switches between databases, listed with their keys.

The keys are listed with `SCAN`, never `KEYS`, so a large database never
blocks the server. `DBSIZE` gives the total first, and the tree grows as
names arrive, with counts that grow ("31+") and **Stop** under it. On
production, Tabletist waits 20 ms between two steps. A database with more
keys than the limit stops there: the groups count what was held ("≈"),
and **Scan all** goes on, after saying how much it would hold. Types are
read only for the keys on screen. Reload scans again from the start;
keys added or removed during a scan may or may not show until then.

A key opens in a tab with its type, size, encoding and expiry, and its
value: a string as text (JSON pretty-printed, a binary one as hex, a long
one cut at 256 KiB), a hash, list, set, sorted set or stream as a table
read a page at a time, a RedisJSON value as JSON. **Copy value** copies
the string, or the rest as JSON; **Copy key** copies its name.

### Read-only

This version only reads. Every command Tabletist sends is checked against
a list, and any other is refused before it reaches the server, whatever
**Open read-only** says: `AUTH`, `HELLO`, `PING`, `INFO`, `SELECT`,
`DBSIZE`, `SCAN`, `HSCAN`, `SSCAN`, `TYPE`, `PTTL`, `MEMORY USAGE`,
`OBJECT ENCODING`, `GET`, `GETRANGE`, `STRLEN`, `HGETALL`, `HLEN`,
`LRANGE`, `LLEN`, `SCARD`, `ZRANGE`, `ZCARD`, `XRANGE`, `XLEN`,
`JSON.GET`, `JSON.TYPE`, `CONFIG GET databases` and
`CLIENT SETNAME tabletist` (so `CLIENT LIST` names it). For the server
itself to refuse writes, sign in as an ACL user without write permission.
Agents cannot reach a Redis connection yet.

## TLS

The SSL modes are libpq's:

| Mode | PostgreSQL | MySQL |
| --- | --- | --- |
| `disable` | No TLS | No TLS |
| `prefer` (the default) | Encrypts when the server can. The certificate is not checked | The same |
| `require` | Encrypts. The certificate is not checked, unless you give a CA certificate | Encrypts. The certificate is not checked |
| `verify-ca` | Checks the certificate against your CA certificate, which is required | Not available yet |
| `verify-full` | Checks the certificate and the host name, against your CA certificate or the system's | Checks the certificate and the host name, against your CA certificate or a built-in list of public CAs (not the system's) |

In a URL, `allow` is read as `prefer`.

Unless the mode checks the certificate, someone on the network between
you and a remote server can read the password. To prevent that, use
`verify-full` or an SSH tunnel. The dialog warns you when this applies.

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
connection starts with agents **Off**: they do not see it at all.
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
