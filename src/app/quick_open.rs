//! The reducer's part in quick open, and the search of a connection's
//! objects that it shares with the start screen: what matches what is
//! typed, in what order, what quick open lists, and what its actions do.
//! How one name matches is `crate::search`'s.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use tabletist_db::{ObjectKind, ObjectRef};

use super::App;
use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen, RunMode, StartRecent};
use crate::search::{self, Tier};

/// How many results a group shows before its "more" row (macOS).
pub const GROUP_SHOWN: usize = 3;

/// A table or view that matches what is typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The index's own: a search that matches thousands copies no name.
    pub object: Rc<ObjectRef>,
    pub kind: ObjectKind,
    /// The catalog's estimate of its rows, where it has one.
    pub estimate: Option<u64>,
    pub tier: Tier,
    /// The runs of the name that matched, as byte ranges of
    /// `object.name`.
    pub spans: Vec<Range<usize>>,
}

/// The index of a connection's objects: each with its name taken apart for
/// matching once, in order by name and then schema (the last of the
/// ranking's ties).
pub(super) struct Index {
    entries: Vec<Entry>,
    /// The schemas listed: a query's schema part is matched once each.
    schemas: Vec<String>,
}

struct Entry {
    object: Rc<ObjectRef>,
    /// Its schema's place in `Index::schemas`.
    schema: usize,
    kind: ObjectKind,
    estimate: Option<u64>,
    name: search::Name,
}

impl Index {
    fn of(workspace: &crate::model::Workspace) -> Self {
        let listed: Vec<(&String, &Vec<tabletist_db::ObjectInfo>)> = workspace
            .tree
            .nodes
            .iter()
            .filter_map(|(schema, node)| Some((schema, node.objects.value.as_ref()?)))
            .collect();
        let schemas = listed.iter().map(|(schema, _)| (*schema).clone()).collect();
        let mut entries: Vec<Entry> = listed
            .into_iter()
            .enumerate()
            .flat_map(|(at, (schema, objects))| {
                objects.iter().map(move |info| Entry {
                    object: Rc::new(ObjectRef::new(schema, &info.name)),
                    schema: at,
                    kind: info.kind,
                    estimate: info.estimated_rows,
                    name: search::Name::new(&info.name),
                })
            })
            .collect();
        entries.sort_by(|a, b| {
            (a.object.name.as_str(), a.object.schema.as_str())
                .cmp(&(b.object.name.as_str(), b.object.schema.as_str()))
        });
        Self { entries, schemas }
    }

    /// What says the index is still the catalog's: its generation, and
    /// each schema's list as it stands (a list read again is a list of its
    /// own).
    fn key(workspace: &crate::model::Workspace) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut key = workspace.catalog_generation;
        for (schema, node) in &workspace.tree.nodes {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            schema.hash(&mut hasher);
            if let Some(objects) = &node.objects.value {
                (objects.as_ptr() as usize, objects.len()).hash(&mut hasher);
            }
            // In whatever order the schemas come.
            key = key.wrapping_add(hasher.finish());
        }
        key
    }
}

/// A match with what it ranks by, packed in one number so that the sort
/// compares numbers alone: its tier (best first), its place among the
/// recents, its length, and its place in the index, which is by name.
struct Ranked {
    key: u64,
    tier: Tier,
    at: usize,
    spans: Vec<Range<usize>>,
}

impl Ranked {
    fn new(tier: Tier, rank: usize, len: usize, at: usize, spans: Vec<Range<usize>>) -> Self {
        let worse = 3 - tier as u64;
        let rank = rank.min(0xff) as u64;
        let len = len.min(0x3f_ffff) as u64;
        let key = worse << 62 | rank << 54 | len << 32 | (at as u64 & 0xffff_ffff);
        Self {
            key,
            tier,
            at,
            spans,
        }
    }
}

/// The index of each open connection, as last built.
#[derive(Default)]
pub struct Indexes(RefCell<HashMap<ConnTabId, (u64, Rc<Index>)>>);

/// A row of a window's start screen list, as quick open shows it with
/// nothing typed: a recent, or on a first visit one of the largest tables.
#[derive(Debug, Clone, PartialEq)]
pub enum StartRow {
    Recent(StartRecent),
    Largest(ObjectRef, Option<u64>),
}

/// One of the commands quick open offers in the macOS and Windows looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCommand {
    /// A new SQL editor. `read_only` where a production connection's new
    /// editors open read-only, which the row says.
    NewSql { read_only: bool },
    /// The menu of the server's other databases. Offered only where there
    /// is another.
    SwitchDatabase,
    /// The window's Agents tab. Offered only where agents can reach the
    /// app.
    Agents,
    /// The window's History tab.
    History,
}

impl QuickCommand {
    /// The command's name, as the row writes it and what is typed
    /// matches.
    pub fn name(self) -> &'static str {
        match self {
            Self::NewSql { .. } => "New SQL query",
            Self::SwitchDatabase => "Switch database",
            Self::Agents => "Agents",
            Self::History => "History",
        }
    }

    /// The keymap's command it is, whose key the row writes.
    pub fn command(self) -> crate::keymap::Command {
        use crate::keymap::Command;
        match self {
            Self::NewSql { .. } => Command::NewSqlTab,
            Self::SwitchDatabase => Command::SwitchDatabase,
            Self::Agents => Command::ShowAgents,
            Self::History => Command::History,
        }
    }
}

/// One of quick open's results: what can be picked.
#[derive(Debug, Clone, PartialEq)]
pub enum QuickItem {
    /// A table or view of the window `tab`'s connection.
    Object { tab: ConnTabId, found: Found },
    /// The row of `tab`'s start screen list numbered `index` there.
    Start {
        tab: ConnTabId,
        index: usize,
        row: StartRow,
    },
    /// A command, and the runs of its name that matched.
    Command {
        command: QuickCommand,
        spans: Vec<Range<usize>>,
    },
    /// The row after a group's first results: the rest are `hidden`.
    More { group: QuickGroup, hidden: usize },
}

/// What heads a group of quick open's results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickHeading {
    Recent,
    /// The start screen's first visit lists the largest tables.
    Largest,
    Tables,
    Views,
    Commands,
    /// One connection's results, where every open one is searched.
    Connection(ConnTabId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuickRow {
    /// A heading: a label, not a result.
    Heading(QuickHeading),
    Item(QuickItem),
}

/// What quick open lists: its rows in order, and how many objects
/// matched (more than are listed where a group is folded).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuickList {
    pub rows: Vec<QuickRow>,
    pub matches: usize,
}

impl QuickList {
    /// The results, in order: what the pick is an index into.
    pub fn items(&self) -> impl Iterator<Item = &QuickItem> {
        self.rows.iter().filter_map(|row| match row {
            QuickRow::Item(item) => Some(item),
            QuickRow::Heading(_) => None,
        })
    }

    /// Whether nothing at all is listed.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// A group's results: the first [`GROUP_SHOWN`] and a "more" row,
    /// unless `whole`.
    fn group(&mut self, group: QuickGroup, items: Vec<QuickItem>, whole: bool) {
        let count = items.len();
        let shown = if whole { count } else { count.min(GROUP_SHOWN) };
        self.rows
            .extend(items.into_iter().take(shown).map(QuickRow::Item));
        if shown < count {
            let hidden = count - shown;
            self.rows
                .push(QuickRow::Item(QuickItem::More { group, hidden }));
        }
    }
}

impl App {
    /// The index of `tab`'s connection: what the sidebar has listed, which
    /// the SQL editor's completion reads too, each name taken apart once.
    /// Built anew when the catalog changes, and kept until it does.
    fn index(&self, tab: ConnTabId) -> Option<Rc<Index>> {
        let workspace = self.workspace(tab)?;
        let key = Index::key(workspace);
        let mut kept = self.indexes.0.borrow_mut();
        if let Some((built, index)) = kept.get(&tab)
            && *built == key
        {
            return Some(index.clone());
        }
        let index = Rc::new(Index::of(workspace));
        kept.insert(tab, (key, index.clone()));
        // The windows closed since keep none.
        kept.retain(|tab, _| self.workspace(*tab).is_some());
        Some(index)
    }

    /// The tables, views and materialized views of `tab`'s connection that
    /// match `query`, best first: by tier, then the recently opened ones
    /// (the newest first), then the shorter names, then by name and
    /// schema. From the connection's index: nothing is queried. A query
    /// with a `.` in it is a schema and a name: what stands before the dot
    /// matches the schema, in any tier, and what stands after it the name,
    /// whose tier ranks it (`main.` lists the schema's objects, in the
    /// schema's tier). Empty where nothing is typed, or only a dot.
    pub fn search_objects(&self, tab: ConnTabId, query: &str) -> Vec<Found> {
        let (Some(workspace), Some(index)) = (self.workspace(tab), self.index(tab)) else {
            return Vec::new();
        };
        let typed = search::folded(query);
        if typed.is_empty() {
            return Vec::new();
        }
        let (schema_typed, name_typed) = match typed.iter().position(|char| *char == '.') {
            Some(at) => (&typed[..at], &typed[at + 1..]),
            None => (&typed[..0], &typed[..]),
        };
        if name_typed.is_empty() && schema_typed.is_empty() {
            return Vec::new();
        }
        // Each schema's match, worked out once: a few schemas hold many
        // objects.
        let schemas: Vec<Option<Tier>> = if schema_typed.is_empty() {
            Vec::new()
        } else {
            let tier = |schema: &String| search::find(schema_typed, schema).map(|hit| hit.tier);
            index.schemas.iter().map(tier).collect()
        };
        let database = crate::session::database_of(&workspace.spec);
        let recent: Vec<ObjectRef> = self
            .recents
            .of(&workspace.conn_id.0, &database)
            .into_iter()
            .map(crate::recents::Recent::object)
            .collect();
        let mut hits: Vec<Ranked> = Vec::new();
        for (at, entry) in index.entries.iter().enumerate() {
            let schema = if schema_typed.is_empty() {
                None
            } else {
                let Some(tier) = schemas[entry.schema] else {
                    continue;
                };
                Some(tier)
            };
            let hit = if name_typed.is_empty() {
                schema.map(|tier| search::Hit {
                    tier,
                    spans: Vec::new(),
                })
            } else {
                entry.name.find(name_typed)
            };
            let Some(hit) = hit else {
                continue;
            };
            let rank = recent
                .iter()
                .position(|object| *object == *entry.object)
                .unwrap_or(usize::MAX);
            hits.push(Ranked::new(hit.tier, rank, entry.name.len(), at, hit.spans));
        }
        hits.sort_unstable_by_key(|hit| hit.key);
        hits.into_iter()
            .map(|hit| {
                let entry = &index.entries[hit.at];
                Found {
                    object: entry.object.clone(),
                    kind: entry.kind,
                    estimate: entry.estimate,
                    tier: hit.tier,
                    spans: hit.spans,
                }
            })
            .collect()
    }

    /// What `tab`'s start screen lists with nothing typed, as quick open
    /// shows it: the recents, or on a first visit the largest tables, and
    /// the heading that says which.
    fn start_rows(&self, tab: ConnTabId) -> (QuickHeading, Vec<StartRow>) {
        let recents = self.start_recents(tab);
        if recents.is_empty() {
            let largest = self.start_largest(tab).into_iter();
            let rows = largest.map(|(object, estimate)| StartRow::Largest(object, estimate));
            (QuickHeading::Largest, rows.collect())
        } else {
            let rows = recents.into_iter().map(StartRow::Recent);
            (QuickHeading::Recent, rows.collect())
        }
    }

    /// The windows quick open searches: its own, then where every open
    /// connection is searched, the others in the order the bar shows them.
    pub fn quick_open_windows(&self, open: &QuickOpen) -> Vec<ConnTabId> {
        let mut windows = vec![open.tab];
        if open.everywhere {
            let others = self.open_connections().map(|(tab, _)| tab);
            windows.extend(others.filter(|tab| *tab != open.tab));
        }
        windows
    }

    /// The commands quick open offers in the macOS and Windows looks, in
    /// order, those that can do something now: none in the terminal look,
    /// whose commands are on `:`.
    fn quick_commands(&self, tab: ConnTabId) -> Vec<QuickCommand> {
        let Some(workspace) = self.workspace(tab) else {
            return Vec::new();
        };
        if self.look.terminal {
            return Vec::new();
        }
        let read_only = workspace.environment.is_production()
            && (self.settings.new_tab == crate::settings::NewTab::ReadOnly
                || RunMode::of_new_tab(workspace.sql_writes(), workspace.environment)
                    == RunMode::ReadOnly);
        let switchable = workspace
            .databases
            .value
            .as_ref()
            .is_some_and(|databases| databases.len() > 1);
        let mut commands = vec![QuickCommand::NewSql { read_only }];
        if switchable {
            commands.push(QuickCommand::SwitchDatabase);
        }
        if crate::endpoint::AVAILABLE {
            commands.push(QuickCommand::Agents);
        }
        commands.push(QuickCommand::History);
        commands
    }

    /// What quick open lists for `open`. With nothing typed: each searched
    /// window's start screen list (headed by its connection where every
    /// open one is searched), then the commands. With a query: the tables
    /// and then the views that match, or where every open connection is
    /// searched, each connection's matches under it; then the commands
    /// whose names match. In the macOS and Windows looks each group shows
    /// its first three and a "more" row until that is opened; the terminal
    /// look lists every match, in one list with no kind headings, and no
    /// commands. After `>` it lists the commands alone, by what follows
    /// it: `> History`.
    pub fn quick_open_list(&self, open: &QuickOpen) -> QuickList {
        let terminal = self.look.terminal;
        let (commands_only, query) = match open.query.trim().strip_prefix('>') {
            Some(rest) => (true, rest.trim()),
            None => (false, open.query.trim()),
        };
        let mut list = QuickList::default();
        let whole = |group| terminal || open.expanded.contains(&group);
        let windows = match commands_only {
            true => Vec::new(),
            false => self.quick_open_windows(open),
        };
        for tab in windows {
            if query.is_empty() {
                let (heading, rows) = self.start_rows(tab);
                if rows.is_empty() {
                    continue;
                }
                let heading = if open.everywhere {
                    QuickHeading::Connection(tab)
                } else {
                    heading
                };
                list.rows.push(QuickRow::Heading(heading));
                let rows = rows.into_iter().enumerate();
                list.rows.extend(
                    rows.map(|(index, row)| QuickRow::Item(QuickItem::Start { tab, index, row })),
                );
                continue;
            }
            let found = self.search_objects(tab, query);
            list.matches += found.len();
            let item = |found| QuickItem::Object { tab, found };
            if open.everywhere {
                if found.is_empty() {
                    continue;
                }
                list.rows
                    .push(QuickRow::Heading(QuickHeading::Connection(tab)));
                let group = QuickGroup::Connection(tab);
                list.group(group, found.into_iter().map(item).collect(), whole(group));
                continue;
            }
            let (tables, views): (Vec<Found>, Vec<Found>) = found
                .into_iter()
                .partition(|found| found.kind == ObjectKind::Table);
            let groups = [
                (QuickGroup::Tables, QuickHeading::Tables, tables),
                (QuickGroup::Views, QuickHeading::Views, views),
            ];
            for (group, heading, found) in groups {
                if found.is_empty() {
                    continue;
                }
                if !terminal {
                    list.rows.push(QuickRow::Heading(heading));
                }
                list.group(group, found.into_iter().map(item).collect(), whole(group));
            }
        }
        let typed = search::folded(query);
        let mut commands: Vec<(Tier, QuickItem)> = self
            .quick_commands(open.tab)
            .into_iter()
            .filter_map(|command| {
                let name = crate::i18n::gettext(self.locale, command.name());
                if typed.is_empty() {
                    let spans = Vec::new();
                    return Some((Tier::Exact, QuickItem::Command { command, spans }));
                }
                let hit = search::find(&typed, &name)?;
                let spans = hit.spans;
                Some((hit.tier, QuickItem::Command { command, spans }))
            })
            .collect();
        // Best first; a sort that keeps their order within a tier.
        commands.sort_by_key(|(tier, _)| std::cmp::Reverse(*tier));
        if !commands.is_empty() {
            list.rows.push(QuickRow::Heading(QuickHeading::Commands));
            list.rows
                .extend(commands.into_iter().map(|(_, item)| QuickRow::Item(item)));
        }
        list
    }

    /// `OpenQuickOpen`: quick open for the window in front, searching its
    /// connection, where it is a connection's and no dialog is open. While
    /// quick open is open, its field's text is selected instead: one panel
    /// per window.
    pub(super) fn open_quick_open(&mut self) {
        if let Some(Dialog::QuickOpen(open)) = &mut self.dialog {
            open.caret = Some(crate::model::QuickCaret::All);
            return;
        }
        let tab = self.active_tab_id();
        if self.dialog.is_none() && self.workspace(tab).is_some() {
            self.dialog = Some(Dialog::QuickOpen(Box::new(QuickOpen {
                tab,
                query: String::new(),
                selected: 0,
                everywhere: false,
                expanded: Vec::new(),
                caret: None,
                scrolled_to: None,
                scroll_offset: 0.0,
            })));
        }
    }

    /// The quick open that is open, if one is.
    fn quick_open_mut(&mut self) -> Option<&mut QuickOpen> {
        match &mut self.dialog {
            Some(Dialog::QuickOpen(open)) => Some(open),
            _ => None,
        }
    }

    /// `QuickOpenChanged`: what is typed changed.
    pub(super) fn quick_open_changed(&mut self) {
        if let Some(open) = self.quick_open_mut() {
            open.selected = 0;
            open.expanded.clear();
        }
    }

    /// `QuickOpenScope`: this connection, or every open one.
    pub(super) fn quick_open_scope(&mut self) {
        if let Some(open) = self.quick_open_mut() {
            open.everywhere = !open.everywhere;
            open.selected = 0;
            open.expanded.clear();
        }
    }

    /// `QuickOpenMove`: the pick moves by `step`, and stops at the ends.
    pub(super) fn quick_open_move(&mut self, step: isize) {
        let count = match &self.dialog {
            Some(Dialog::QuickOpen(open)) => self.quick_open_list(open).items().count(),
            _ => return,
        };
        if let Some(open) = self.quick_open_mut() {
            // From the pick as it shows: clamped, as the list may have
            // shrunk under it (a relist, a window closed).
            let last = count.saturating_sub(1);
            let from = open.selected.min(last) as isize;
            open.selected = (from + step).clamp(0, last as isize) as usize;
        }
    }

    /// `QuickOpenPick`: opens the picked result, or the one at `index`.
    /// An object opens as a tab, or the tab it has is shown, in its own
    /// connection's window, which comes to the front; `in_editor` opens a
    /// new SQL editor reading its first rows instead. A start screen row
    /// opens as the start screen opens it. A command does what it does in
    /// the window quick open was opened in. A "more" row shows its group
    /// in full and quick open stays; the pick stays where it was, on the
    /// first result that row hid. Anything else closes quick open.
    pub(super) fn quick_open_pick(&mut self, index: Option<usize>, in_editor: bool) {
        let (here, item) = match &self.dialog {
            Some(Dialog::QuickOpen(open)) => {
                let list = self.quick_open_list(open);
                let count = list.items().count();
                // The pick is clamped: the list may have shrunk since it
                // moved (a relist under a typed query).
                let picked = index.unwrap_or(open.selected.min(count.saturating_sub(1)));
                (open.tab, list.items().nth(picked).cloned())
            }
            _ => return,
        };
        let Some(item) = item else {
            return;
        };
        if let QuickItem::More { group, .. } = item {
            if let Some(open) = self.quick_open_mut() {
                open.expanded.push(group);
                if let Some(index) = index {
                    open.selected = index;
                }
            }
            return;
        }
        self.dialog = None;
        let front = |app: &mut App, tab: ConnTabId| {
            if tab != here {
                app.apply(Action::ActivateConnTab(tab));
            }
        };
        match item {
            QuickItem::Object { tab, found } => {
                front(self, tab);
                if in_editor {
                    self.open_sql_for(tab, &found.object);
                } else {
                    self.apply(Action::OpenObject {
                        tab,
                        object: Rc::unwrap_or_clone(found.object),
                        kind: found.kind,
                        pin: true,
                    });
                }
            }
            QuickItem::Start { tab, index, row } => {
                front(self, tab);
                let object = match row {
                    StartRow::Recent(StartRecent::Object { object, .. })
                    | StartRow::Largest(object, _) => Some(object),
                    StartRow::Recent(StartRecent::Sql { .. }) => None,
                };
                match object {
                    Some(object) if in_editor => self.open_sql_for(tab, &object),
                    _ => self.apply(Action::OpenRecent { tab, index }),
                }
            }
            QuickItem::Command { command, .. } => self.apply(match command {
                QuickCommand::NewSql { .. } => Action::NewSqlTab(here),
                QuickCommand::SwitchDatabase => Action::ShowDatabaseMenu(here),
                QuickCommand::Agents => Action::ShowAgents(here),
                QuickCommand::History => Action::ShowHistory(here),
            }),
            QuickItem::More { .. } => {}
        }
    }
}
