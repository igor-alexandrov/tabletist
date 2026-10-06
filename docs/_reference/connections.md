---
title: Connections
description: Every field of a connection, connection URLs, TLS modes, SSH tunnels, host keys, passwords and read-only connections.
nav_order: 0
---

## The picker

Cmd/Ctrl+O opens the picker with your saved connections. Each row shows
the connection's environment and when it was last used, and
**Find connection…** searches by name or by `user@host:port/database`.

Connections with exactly the same name form a group under that name, which
suits one database in several environments. The rest are listed under
**Local** or **Remote**.

A connection can be open more than once. Enter shows a connection that is
already open, and Shift+Enter opens it again.

## Fields

| Field | Notes |
| --- | --- |
| **Name** | Left empty, it becomes `user@host:port/database` or the file's name |
| **Type** | SQLite, PostgreSQL or MySQL |
| **Environment** | local, dev, staging, production or none. Sets the connection's color |
| **File** | SQLite only |
| **Host**, **Port** | The port defaults to 5432 for PostgreSQL and 3306 for MySQL |
| **Database** | Left empty, PostgreSQL uses the user's name and MySQL selects no database |
| **User**, **Password** | See [Passwords](#passwords) |
| **SSL mode**, **CA certificate** | See [TLS](#tls) |
| **Connect through SSH tunnel** | See [SSH tunnels](#ssh-tunnels) |
| **Open read-only** | See [Open read-only](#open-read-only) |

**Test** checks the connection without saving it.

## Connection URLs

The **URL** tab of the dialog (`u` in the Omarchy look) takes a connection
URL and fills the fields from it. A password in the URL moves to the
password field.

```text
postgres://app@db.example.com:5432/shop?sslmode=verify-full
mysql://app@127.0.0.1:3306/shop
sqlite:///home/me/data/shop.db
```

- The schemes are `postgres://`, `postgresql://`, `mysql://`, `mariadb://`
  and `sqlite:`.
- `sslmode` (also `ssl-mode` or `ssl_mode`) sets the SSL mode, and
  `sslrootcert` (also `ssl-ca`) the CA certificate. Other parameters are
  ignored.
- A URL cannot be given on the command line.

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
| `Include` | Other files to read |

Without an `IdentityAgent`, the agent is the one in `SSH_AUTH_SOCK`. On
Windows it is the OpenSSH agent, then Pageant.

`Match` blocks are ignored. `ProxyJump` and `ProxyCommand` are not
supported yet: a host that needs one cannot be used for a tunnel.

### Host keys

The first time you connect to an SSH server, Tabletist shows its key's
fingerprint and asks **Trust this SSH host?** Compare the fingerprint
with the server's before you trust it.

Trusted keys are kept in Tabletist's own `known_hosts.json`, not in
OpenSSH's `known_hosts`. If a server's key changes later, Tabletist
refuses to connect, because that can mean someone is intercepting the
connection. If the key really did change, remove the server's line from
[`known_hosts.json`]({% link _reference/settings-and-files.md %}#files).

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

## Open read-only

A connection with **Open read-only** ticked blocks every write from
Tabletist: the session itself is read-only on PostgreSQL and MySQL, and a
SQLite file is opened read-only.

The box is ticked by default for production, and not for the other
environments. Turn it off to edit. A change applies from the next time
you connect.

Production stays careful with the box off too:

- A save asks first, with its statements on screen.
- The SQL editor still runs read-only.
