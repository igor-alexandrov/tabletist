# Connection groups the user names (#367)

From user feedback: "I want to group by production/staging or even different
products. Need flexibility." Let the user put saved connections in groups they
name, and let the picker group by environment instead.

## What exists

- The picker groups by a rule (`groups`, `src/ui/picker.rs`): a name two or more
  connections share heads a group (sorted by title), the rest go under Local,
  then Remote. Groups fold by title (`picker.folded`, in memory only).
  Search (`SavedConnections::search`) matches name and summary.
- `SavedConnection` has `environment` (colour, read-only default). No group.
- `connections.toml` (`crates/tabletist-core/src/connections/file.rs`) is read
  strictly (`deny_unknown_fields`), version 8, written as the oldest version
  that holds what it has; a newer version is refused with "written by a
  newer Tabletist".
- The form has a `select` dropdown (SSH authentication, `sheet.rs`) and
  `menu::value` choices; the row's More actions menu and the right-click menu
  have Duplicate, Delete and an Agents section (heading + choices).
- Omarchy picker keys: j/k, enter, shift+enter, e, n, yy p, dd, /, a. Free: m, g, R.

## The design canvas

Before this design the canvas already drew a Group field in both edit forms
(MacConnectionEdit / MacConnectionEditNext: Name and Group side by side, a
dropdown of Bookshop, Local, New group…; OmarchyConnectionEdit / Next: a
`group` row between name and environment). It was never built.

Added for this design (2026-11-21, Connections page, section "Connection
groups"):

- **macOS – Connection groups (#367)** (`MacConnectionGroups`): the picker by
  group and by environment, the row menu's Group section, the heading's
  Rename group… and its field, the form's Group dropdown open, and the field
  after New group….
- **Omarchy – Connection groups (#367)** (`OmarchyConnectionGroups`): the
  picker with the `groups env` toggle, the `m` list, the New group and `R`
  status-line prompts, the environment view, the form's group row open, and
  the keys.
- **macOS – Connections** and **Omarchy – Connections** now show the switch
  (Groups | Environment; `groups env g`).

## The design

### Groups

- A group is a title. A connection shows under its saved `group`, else under
  its automatic one: a name it shares with another connection, else Local,
  else Remote. Titles compare trimmed and case-insensitively, so a saved
  "Local" and the automatic Local are one heading. An empty title is no group.
  A group exists while a connection is in it; no list of groups is kept on its
  own.
- Order: named and shared-name groups sorted by title together, then Local,
  then Remote. Nothing about order is stored.
- Search matches the group title too.
- Duplicate keeps the group. A new connection starts with none.

### Choosing a group

- **Form** (both looks, as the canvas draws it): a dropdown of every group the
  picker shows (named, shared-name, Local, Remote) and New group…, which turns
  the box into a text field in place (Esc goes back to the list; empty means no
  group).
- **Row menus** (More actions and right-click, macOS and Windows): a Group
  section between Delete and Agents, with the same choices, the row's ticked.
  New group… opens the connection's form with the Group field ready to type.
- **Omarchy `m`**: the same list as a floating list (j/k, enter, esc), as `a`
  opens agent levels; new group… asks in the status line.
- What is saved: `group` only when the choice differs from the automatic
  group; choosing the automatic one clears it. No drag.

### Renaming

- macOS and Windows: right-click a group heading, Rename group… turns the
  title into a field; Enter renames, Esc keeps.
- Omarchy: `R` renames the selected row's group in the status line.
- Offered where at least one connection under the heading saves the title; it
  writes the new title to every connection under that heading. Renaming onto
  an existing title merges the two. On a row with no saved group, `R` says so
  in the status line for 4 s.

### By environment

- A switch after the search: the Segmented control Groups | Environment
  (macOS and Windows); `groups env` in the filter row, switched with `g`
  (Omarchy). Remembered in `settings.toml`.
- Headings: Production, Staging, Dev, Local, No environment (Omarchy:
  lower case); empty ones hidden; rows keep the file's order. The row menus
  still change the group. Folding is kept per view, so a group named
  "Production" and the Production environment fold apart.

### Storage

- `group = "Clients"` on the connection in `connections.toml`, as version 9,
  written only once some connection has a group; files with none stay
  version 8. An older Tabletist refuses a version 9 file, as it does for the
  earlier additions.

### Unchanged

Start screen (one connection), quick open (open connections' objects), menu
bar (lists no connections), MCP `list_connections`.

New strings get Russian and Brazilian Portuguese in the same run (the
every-entry-translated tests).

## Departures from the canvas as it was

| The canvas | Built | Why |
| --- | --- | --- |
| No grouping switch in the picker header | Segmented Groups / Environment; Omarchy `groups env`, `g` | Decision 2 |
| No Group section in the row menus | Group section above Agents, Omarchy `m` | Decision 4 |
| No rename | Heading right-click Rename group…, Omarchy `R` | Decision 7 |
| Dropdown lists Bookshop, Local, New group… | Also lists Remote and every other group shown | Decision 1 |

All four are now drawn on the canvas (decision 9).

## Runs (a PR each)

1. Storage (`group`, version 9), the grouping rule, search by group, the Group
   field in both forms with New group…. Tests: the rule; a harness test that a
   connection given a group shows under it after a save and a relaunch.
2. Moving and renaming: Group section in both menus, Omarchy `m` and `R`,
   heading right-click Rename group….
3. Environment view: the switch in both looks, `g`, the setting.

## Overlap with #366

#366 may add "open another database" to the row menus (the right-click menu
already has Show and Connect again for an open connection). Run 2 puts the
Group section between Delete and Agents; #366's item belongs with Show and
Connect again at the top, so the two do not collide.

## Decisions

1. Group field: "Groups shown + New group…". The dropdown lists every group
   the picker shows and New group…; only a choice that differs from the
   automatic group is saved. Follows the canvas.
2. Environment: "Picker switch: Group / Environment". Igor chose the switch
   over leaving environment to named groups.
3. Order: "Alphabetical, Local and Remote last". Nothing to store or reorder.
4. Moving: "Row menus + Omarchy key m". Like the Agents section and `a`; no
   drag.
5. Switch: "Segmented after search". The canvas's Segmented control; Omarchy
   toggles like the start screen's tree/flat, key `g`.
6. Environment order: "Production first". The riskiest at the top.
7. Rename: "Heading menu + Omarchy R". Only named groups rename.
8. Old versions: "Yes, version 9". The file's existing rule.
9. Approval: "Looks good. Update the design artifact and always keep the
   design in sync with the code." The design was drawn on the canvas the same
   day, and later runs update the boards with the code.
