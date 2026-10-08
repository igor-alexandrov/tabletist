//! Every key the app answers to, as data: the one place a binding is
//! written. The keys' handler reads its chords from here, and so does
//! everything that names a key: the `?` list, the status line's hints, a
//! button's shortcut, a tooltip, the Keyboard page of the Settings.
//!
//! A binding is spelled once, the way the Omarchy look writes it
//! (`ctrl+shift+c`, `yy p`, `:w`), and each layout writes it its own way
//! from that: `⇧⌘F` on macOS, `Ctrl+Shift+F` on Windows.
//!
//! A command that is not built yet is here all the same, with its keys:
//! they are claimed, so nothing else takes them, and nothing answers them.

use egui::Key;

use crate::theme::{Faces, Look};

/// How a look binds its keys and writes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// The macOS look: its own keys, written with glyphs.
    Mac,
    /// The Windows look: the Mac's keys with Ctrl for Cmd, written out.
    Windows,
    /// The Omarchy look: vim keys and the `:` prompt, in lower case.
    Omarchy,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Mac, Layout::Windows, Layout::Omarchy];

    pub fn of(look: &Look) -> Self {
        if look.terminal {
            Self::Omarchy
        } else if look.faces == Faces::Plex {
            Self::Mac
        } else {
            Self::Windows
        }
    }
}

/// Where a binding holds: what has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Everywhere but under a prompt.
    Global,
    /// The connection picker.
    Connections,
    /// The tree of tables.
    Sidebar,
    /// A table's rows.
    Grid,
    /// An open editor on a cell or on a field of the inspector.
    CellEditor,
    /// The fields of the row inspector.
    Inspector,
    /// A SQL editor's tab.
    SqlEditor,
    /// A SQL editor's result.
    Results,
    /// A prompt or a dialog: nothing under it has the keys.
    Prompt,
}

impl Scope {
    pub const ALL: [Scope; 9] = [
        Scope::Global,
        Scope::Connections,
        Scope::Sidebar,
        Scope::Grid,
        Scope::CellEditor,
        Scope::Inspector,
        Scope::SqlEditor,
        Scope::Results,
        Scope::Prompt,
    ];

    /// The scope's name in `settings.toml` (`[keys.<scope>]`).
    pub fn id(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Connections => "connections",
            Self::Sidebar => "sidebar",
            Self::Grid => "grid",
            Self::CellEditor => "cell-editor",
            Self::Inspector => "inspector",
            Self::SqlEditor => "sql-editor",
            Self::Results => "results",
            Self::Prompt => "prompt",
        }
    }

    /// Whether a key of `self` and a key of `other` can be read in the same
    /// moment: the same scope, the global one with any but a prompt, an
    /// editor with what it was opened from, a result with its editor's tab.
    pub fn alongside(self, other: Scope) -> bool {
        use Scope::{CellEditor, Global, Grid, Inspector, Prompt, Results, SqlEditor};
        let pair = |a, b| (self == a && other == b) || (self == b && other == a);
        self == other
            || (self == Global && other != Prompt)
            || (other == Global && self != Prompt)
            || pair(Grid, CellEditor)
            || pair(Inspector, CellEditor)
            || pair(SqlEditor, Results)
    }
}

/// What must also be on screen for a binding to hold. A binding with one
/// of these takes its key before the same key of its scope with none: that
/// is no conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    Always,
    /// A connect is under way.
    Connecting,
    /// The cursor is on the grid's last row.
    LastRow,
    /// The row inspector shows.
    InspectorOpen,
    /// The SQL of the pending changes shows.
    Review,
    /// What the last save came to is still said.
    Note,
    /// The large editor is open.
    LargeEditor,
    /// The completion list is open.
    Completion,
    /// An answer of the assistant is offered.
    Suggestion,
    /// The card of a refused write shows in Messages.
    RefusedWrite,
    /// The connection form.
    ConnectionForm,
    /// The Settings screen.
    SettingsScreen,
    /// Quick open.
    QuickOpen,
    /// The question before a save to production.
    ConfirmWrite,
    /// The question about a row that changed on the server.
    Conflict,
    /// The question about leaving with pending changes.
    Leaving,
}

/// The mode a chord holds in: with the keyboard in a text field (insert),
/// or out of one (normal). Typed letters and `:` commands are normal mode's
/// whatever this says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// By the scope: insert in an editor, normal on rows and lists, any
    /// for the global scope and for prompts.
    Auto,
    Normal,
    Insert,
    Any,
}

impl Mode {
    /// Whether a chord of this mode is read with the keyboard out of a
    /// text field (`normal`) or in one.
    pub fn holds(self, normal: bool) -> bool {
        match self {
            Mode::Auto | Mode::Any => true,
            Mode::Normal => normal,
            Mode::Insert => !normal,
        }
    }

    /// Whether a chord of this mode and one of `other` could be read in
    /// the same moment.
    pub fn overlaps(self, other: Mode) -> bool {
        !matches!(
            (self, other),
            (Mode::Normal, Mode::Insert) | (Mode::Insert, Mode::Normal)
        )
    }
}

/// What a spelling is to the command it stands under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The command's own key.
    Own,
    /// Another command's key that ends up here (`ctrl+l` moves a pane to
    /// the right, which from the grid is the inspector). Written beside the
    /// command's keys, and bound where it is that command's own.
    Via,
    /// A key the command will have once what it needs is built. Claimed,
    /// written nowhere, answered by nothing.
    Claimed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub keys: &'static str,
    pub kind: Kind,
}

const fn k(keys: &'static str) -> Chord {
    Chord {
        keys,
        kind: Kind::Own,
    }
}

const fn via(keys: &'static str) -> Chord {
    Chord {
        keys,
        kind: Kind::Via,
    }
}

const fn claimed(keys: &'static str) -> Chord {
    Chord {
        keys,
        kind: Kind::Claimed,
    }
}

/// A command's keys in one layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bound {
    pub chords: &'static [Chord],
    /// How the keys are written together where that is shorter than one
    /// after the other (`j/k`). Empty: one after the other.
    pub shown: &'static str,
    /// What does the command where no key does (`Review SQL button`).
    pub control: &'static str,
    pub mode: Mode,
}

/// No key, and none meant.
const NONE: Bound = Bound {
    chords: &[],
    shown: "",
    control: "",
    mode: Mode::Auto,
};

const fn keys(chords: &'static [Chord]) -> Bound {
    Bound { chords, ..NONE }
}

const fn control(control: &'static str) -> Bound {
    Bound { control, ..NONE }
}

impl Bound {
    const fn shown(self, shown: &'static str) -> Self {
        Self { shown, ..self }
    }

    const fn normal(self) -> Self {
        Self {
            mode: Mode::Normal,
            ..self
        }
    }

    const fn any(self) -> Self {
        Self {
            mode: Mode::Any,
            ..self
        }
    }
}

/// The groups the Keyboard page lists the commands in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Window,
    Navigation,
    Editing,
    Sql,
    Ai,
    /// The keys of a prompt or a dialog: its own, and not rebound.
    Prompts,
}

macro_rules! commands {
    ($($name:ident),* $(,)?) => {
        /// Everything a key can ask for.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Command {
            $($name),*
        }

        impl Command {
            pub const ALL: &'static [Command] = &[$(Command::$name),*];
        }
    };
}

commands! {
    // Window and connections.
    Connections,
    GoToConnectionWindow,
    SwitchDatabase,
    Settings,
    CloseConnection,
    NextConnection,
    PreviousConnection,
    CancelConnecting,
    NewConnection,
    EditConnection,
    DuplicateConnection,
    DeleteConnection,
    Connect,
    ConnectAgain,
    MoveInConnections,
    FilterConnections,
    // Navigation.
    FindObject,
    NewSqlTab,
    ToggleSidebar,
    PaneLeft,
    PaneDown,
    PaneUp,
    PaneRight,
    NextPart,
    PreviousPart,
    SwitchTab,
    CloseTab,
    PreviousTab,
    NextTab,
    MoveInTree,
    OpenObject,
    ToggleTree,
    Reload,
    FilterBar,
    WhereFilter,
    PreviousPage,
    NextPage,
    Help,
    KeysDefault,
    // Grid and editing.
    MoveRow,
    MoveColumn,
    MovePage,
    OpenInspector,
    ToggleInspector,
    CloseInspector,
    BackToGrid,
    MoveField,
    PreviousRow,
    NextRow,
    FoldDocuments,
    EditCell,
    ReplaceCell,
    CommitDown,
    CommitNext,
    CommitPrevious,
    KeepEdit,
    CancelEdit,
    OpenLargeEditor,
    ApplyLargeEditor,
    KeepLargeEditor,
    CancelLargeEditor,
    OpenInEditor,
    SetNull,
    SetDefault,
    UndoCell,
    RedoCell,
    CopyCells,
    CopyRow,
    PasteCells,
    PasteNewRows,
    AddRow,
    AddRowAbove,
    AddRowAtEnd,
    DuplicateRow,
    DuplicateRowAbove,
    DeleteRow,
    OpenReferencedRow,
    ShowSavedRow,
    ReviewSql,
    NextChange,
    CopyReviewSql,
    CloseReview,
    DismissNote,
    SaveChanges,
    DiscardChanges,
    NextError,
    PreviousError,
    BooleanCycle,
    StepUp,
    StepDown,
    Format,
    // SQL editor.
    RunStatement,
    RunAll,
    Explain,
    ExplainAnalyze,
    ComparePreviousRun,
    CancelQuery,
    FocusResults,
    LeaveEditor,
    Complete,
    InsertCompletion,
    NextCompletion,
    PreviousCompletion,
    CloseCompletion,
    GoToLine,
    History,
    ReadOnlyTab,
    ReadWriteTab,
    ToggleSqlMode,
    Commit,
    Rollback,
    NextPlanTip,
    EditRefusedConnection,
    AllowRefusedWrite,
    // AI assistant.
    AskAi,
    AcceptAi,
    AcceptAiAndRun,
    DiscardAi,
    // Prompts and dialogs.
    ConfirmProductionWrite,
    CancelProductionWrite,
    ScrollStatements,
    LeaveWrite,
    LeaveDiscard,
    LeaveStay,
    ConflictOverwrite,
    ConflictUseServer,
    ConflictKeepMine,
    ConflictDiscard,
    FormPasteUrl,
    FormNextField,
    FormTest,
    FormSave,
    FormSaveAndConnect,
    FormCancel,
    SettingsMove,
    SettingsChange,
    SettingsToggle,
    SettingsOpenFile,
    SettingsReset,
    SettingsClose,
    QuickOpenMove,
    QuickOpenPick,
    QuickOpenClose,
}

/// What is said of a command wherever it is listed.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    /// The command's own name.
    pub name: &'static str,
    /// The row of the settled keymap it belongs to, by the row's name. A
    /// row can hold several commands (`Undo / redo one cell`). Empty: a key
    /// the app had before the keymap was settled, kept beside it.
    pub row: &'static str,
    pub group: Group,
    /// Whether anything answers the command yet.
    pub built: bool,
}

const fn info(name: &'static str, row: &'static str, group: Group) -> Info {
    Info {
        name,
        row,
        group,
        built: true,
    }
}

impl Info {
    const fn reserved(self) -> Self {
        Self {
            built: false,
            ..self
        }
    }
}

impl Command {
    pub fn info(self) -> Info {
        use Command as C;
        use Group::{Ai, Editing, Navigation, Prompts, Sql, Window};
        const PANES: &str = "Move between panes";
        const INSPECTOR_ROWS: &str = "Previous / next row in inspector";
        const COMMIT: &str = "Commit cell and move";
        const UNDO: &str = "Undo / redo one cell";
        const COPY: &str = "Copy / paste cell values";
        const ERRORS: &str = "Next / previous error";
        const LARGE: &str = "Apply / cancel large editor";
        const STEP: &str = "Step number / date part";
        const MODE: &str = "Read-only / read-write tab";
        const ANSWER: &str = "Accept / accept and run / discard AI";
        match self {
            C::Connections => info("Connections", "Connections", Window),
            C::GoToConnectionWindow => {
                info("Go to connection window", "Go to connection window", Window)
            }
            C::SwitchDatabase => info("Switch database", "Switch database", Window).reserved(),
            C::Settings => info("Settings", "Settings", Window),
            C::CloseConnection => info("Close connection", "", Window),
            C::NextConnection => info("Next connection", "", Window),
            C::PreviousConnection => info("Previous connection", "", Window),
            C::CancelConnecting => info("Cancel connecting", "", Window),
            C::NewConnection => info("New connection", "New connection", Window),
            C::EditConnection => info("Edit connection", "Edit connection", Window),
            C::DuplicateConnection => info("Duplicate connection", "Duplicate connection", Window),
            C::DeleteConnection => info("Delete connection", "Delete connection", Window),
            C::Connect => info("Connect / go to window", "Connect / go to window", Window),
            C::ConnectAgain => info("Open the connection again", "", Window),
            C::MoveInConnections => info("Move in the connections", "", Window),
            C::FilterConnections => info("Filter the connections", "", Window),
            C::FindObject => info("Find any object", "Find any object", Navigation),
            C::NewSqlTab => info("New SQL editor tab", "New SQL editor tab", Navigation),
            C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
            C::PaneLeft => info("Pane to the left", PANES, Navigation),
            C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
            C::PaneUp => info("Pane above", PANES, Navigation).reserved(),
            C::PaneRight => info("Pane to the right", PANES, Navigation),
            C::NextPart => info("Next part of the window", "", Navigation),
            C::PreviousPart => info("Previous part of the window", "", Navigation),
            C::SwitchTab => info("Switch tab", "Switch tab", Navigation),
            C::CloseTab => info("Close tab", "", Navigation),
            C::PreviousTab => info("Previous tab", "", Navigation),
            C::NextTab => info("Next tab", "", Navigation),
            C::MoveInTree => info("Move in the sidebar", "", Navigation),
            C::OpenObject => info("Open the table or view", "", Navigation),
            C::ToggleTree => info("Tree or flat list", "", Navigation),
            C::Reload => info("Reload", "Reload", Navigation),
            C::FilterBar => info("Filter bar", "", Navigation),
            C::WhereFilter => info("Filter with a WHERE line", "", Navigation),
            C::PreviousPage => info("Previous page", "", Navigation),
            C::NextPage => info("Next page", "", Navigation),
            C::Help => info("Help, all keys", "Help, all keys", Navigation),
            C::KeysDefault => info("Write the full list of keys", "", Navigation).reserved(),
            C::MoveRow => info("Move by row", "", Editing),
            C::MoveColumn => info("Move by column", "", Editing),
            C::MovePage => info("Move by page", "", Editing),
            C::OpenInspector => info(
                "Open inspector / focus fields",
                "Open inspector / focus fields",
                Editing,
            ),
            C::ToggleInspector => info("Show or hide the inspector", "", Editing),
            C::CloseInspector => info("Close the inspector", "", Editing),
            C::BackToGrid => info(
                "Back to grid from inspector",
                "Back to grid from inspector",
                Editing,
            ),
            C::MoveField => info("Move by field", "", Editing),
            C::PreviousRow => info("Previous row in inspector", INSPECTOR_ROWS, Editing),
            C::NextRow => info("Next row in inspector", INSPECTOR_ROWS, Editing),
            C::FoldDocuments => info("Fold the documents", "", Editing),
            C::EditCell => info("Edit cell", "Edit cell", Editing),
            C::ReplaceCell => info("Edit cell from nothing", "Edit cell", Editing),
            C::CommitDown => info("Commit cell and move down", COMMIT, Editing),
            C::CommitNext => info("Commit cell and move to the next", COMMIT, Editing),
            C::CommitPrevious => info("Commit cell and move to the previous", COMMIT, Editing),
            C::KeepEdit => info("Leave the editor and keep the edit", COMMIT, Editing),
            C::CancelEdit => info("Cancel cell edit", "Cancel cell edit", Editing),
            C::OpenLargeEditor => info("Open the large editor", "", Editing),
            C::ApplyLargeEditor => info("Apply large editor", LARGE, Editing),
            C::KeepLargeEditor => info("Leave the large editor and keep the text", "", Editing),
            C::CancelLargeEditor => info("Cancel large editor", LARGE, Editing),
            C::OpenInEditor => {
                info("Open value in $EDITOR", "Open value in $EDITOR", Editing).reserved()
            }
            C::SetNull => info("Set NULL", "Set NULL", Editing),
            C::SetDefault => info("Set DEFAULT", "Set DEFAULT", Editing).reserved(),
            C::UndoCell => info("Undo one cell", UNDO, Editing),
            C::RedoCell => info("Redo one cell", UNDO, Editing).reserved(),
            C::CopyCells => info("Copy cell values", COPY, Editing),
            C::CopyRow => info("Copy row", "", Editing),
            C::PasteCells => info("Paste cell values", COPY, Editing).reserved(),
            C::PasteNewRows => info("Paste as new rows", "Paste as new rows", Editing).reserved(),
            C::AddRow => info("Add row", "Add row", Editing),
            C::AddRowAbove => info("Add row above", "Add row", Editing),
            C::AddRowAtEnd => info("Add row after the last", "Add row", Editing).reserved(),
            C::DuplicateRow => info("Duplicate row", "Duplicate row", Editing).reserved(),
            C::DuplicateRowAbove => {
                info("Duplicate row above", "Duplicate row", Editing).reserved()
            }
            C::DeleteRow => info("Delete row", "Delete row", Editing),
            C::OpenReferencedRow => info("Open referenced row", "Open referenced row", Editing),
            C::ShowSavedRow => info(
                "Show saved row in sorted position",
                "Show saved row in sorted position",
                Editing,
            )
            .reserved(),
            C::ReviewSql => info("Review pending SQL", "Review pending SQL", Editing),
            C::NextChange => info("Next change", "Review pending SQL", Editing).reserved(),
            C::CopyReviewSql => info("Copy the pending SQL", "", Editing),
            C::CloseReview => info("Close the pending SQL", "", Editing),
            C::DismissNote => info("Dismiss what the last save came to", "", Editing),
            C::SaveChanges => info("Save changes", "Save changes", Editing),
            C::DiscardChanges => info("Discard all pending", "Discard all pending", Editing),
            C::NextError => info("Next error", ERRORS, Editing).reserved(),
            C::PreviousError => info("Previous error", ERRORS, Editing).reserved(),
            C::BooleanCycle => info("Boolean cycle", "Boolean cycle", Editing).reserved(),
            C::StepUp => info("Step up", STEP, Editing).reserved(),
            C::StepDown => info("Step down", STEP, Editing).reserved(),
            C::Format => info("Format SQL or JSON", "Format SQL or JSON", Sql),
            C::RunStatement => info("Run statement", "Run statement", Sql),
            C::RunAll => info("Run all", "Run all", Sql),
            C::Explain => info("Explain", "Explain", Sql).reserved(),
            C::ExplainAnalyze => info("Explain analyze", "Explain analyze", Sql).reserved(),
            C::ComparePreviousRun => info(
                "Compare with previous run",
                "Compare with previous run",
                Sql,
            )
            .reserved(),
            C::CancelQuery => info("Cancel running query", "Cancel running query", Sql),
            C::FocusResults => info("Focus results", "Focus results", Sql).reserved(),
            C::LeaveEditor => info("Leave the editor", "", Sql),
            C::Complete => info("Completion", "Completion", Sql),
            C::InsertCompletion => info("Insert the completion", "Completion", Sql),
            C::NextCompletion => info("Next completion", "Completion", Sql),
            C::PreviousCompletion => info("Previous completion", "Completion", Sql),
            C::CloseCompletion => info("Close the completion list", "", Sql),
            C::GoToLine => info("Go to line", "Go to line", Sql).reserved(),
            C::History => info("History", "History", Sql).reserved(),
            C::ReadOnlyTab => info("Read-only tab", MODE, Sql),
            C::ReadWriteTab => info("Read-write tab", MODE, Sql),
            C::ToggleSqlMode => info("Read-only or read-write runs", "", Sql),
            C::Commit => info("Commit", "Commit", Sql).reserved(),
            C::Rollback => info("Rollback", "Rollback", Sql).reserved(),
            C::NextPlanTip => info("Next plan tip", "Next plan tip", Sql).reserved(),
            C::EditRefusedConnection => info("Edit the connection that refused", "", Prompts),
            C::AllowRefusedWrite => info("Allow writes, or run again", "", Prompts),
            C::AskAi => info("Ask AI", "Ask AI", Ai).reserved(),
            C::AcceptAi => info("Accept AI", ANSWER, Ai).reserved(),
            C::AcceptAiAndRun => info("Accept AI and run", ANSWER, Ai).reserved(),
            C::DiscardAi => info("Discard AI", ANSWER, Ai).reserved(),
            C::ConfirmProductionWrite => info(
                "Confirm production write",
                "Confirm production write",
                Prompts,
            ),
            C::CancelProductionWrite => info("Cancel the production write", "", Prompts),
            C::ScrollStatements => info("Scroll the statements", "", Prompts),
            C::LeaveWrite => info("Write and leave", "", Prompts),
            C::LeaveDiscard => info("Discard and leave", "", Prompts),
            C::LeaveStay => info("Stay", "", Prompts),
            C::ConflictOverwrite => info("Overwrite", "", Prompts),
            C::ConflictUseServer => info("Use the server's values", "", Prompts),
            C::ConflictKeepMine => info("Keep mine and reload", "", Prompts),
            C::ConflictDiscard => info("Discard my changes", "", Prompts),
            C::FormPasteUrl => info("Paste a URL", "", Prompts),
            C::FormNextField => info("Next field", "", Prompts),
            C::FormTest => info("Test the connection", "", Prompts),
            C::FormSave => info("Save the connection", "", Prompts),
            C::FormSaveAndConnect => info("Save and connect", "", Prompts),
            C::FormCancel => info("Cancel", "", Prompts),
            C::SettingsMove => info("Move in the settings", "", Prompts),
            C::SettingsChange => info("Change the option", "", Prompts),
            C::SettingsToggle => info("Toggle the option", "", Prompts),
            C::SettingsOpenFile => info("Open the settings file in $EDITOR", "", Prompts),
            C::SettingsReset => info("Reset the option", "", Prompts),
            C::SettingsClose => info("Close the settings", "", Prompts),
            C::QuickOpenMove => info("Move in quick open", "", Prompts),
            C::QuickOpenPick => info("Open the one selected", "", Prompts),
            C::QuickOpenClose => info("Close quick open", "", Prompts),
        }
    }
}

/// One line of the keymap: a command, where it holds, and its keys in the
/// Mac's layout and in Omarchy's. Windows has the Mac's with Ctrl for Cmd.
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub command: Command,
    pub scopes: &'static [Scope],
    pub when: When,
    pub mac: Bound,
    pub omarchy: Bound,
}

impl Binding {
    pub fn bound(&self, layout: Layout) -> &Bound {
        match layout {
            Layout::Mac | Layout::Windows => &self.mac,
            Layout::Omarchy => &self.omarchy,
        }
    }
}

impl Binding {
    /// The mode `spelled`, one of this line's keys, holds in where the
    /// line is read in `scope`. Never `Auto`.
    pub fn mode(&self, scope: Scope, layout: Layout, spelled: &Spelled) -> Mode {
        let typed = match spelled {
            Spelled::Ex(_) => true,
            Spelled::Digits(_) => false,
            Spelled::Strokes(strokes) => strokes
                .iter()
                .any(|stroke| matches!(stroke, Stroke::Text(_))),
        };
        // A prompt has no modes: its letters are its own.
        if scope == Scope::Prompt {
            return Mode::Any;
        }
        // Typed letters and the lines of the prompt are normal mode's.
        if typed {
            return Mode::Normal;
        }
        match self.bound(layout).mode {
            Mode::Auto => match scope {
                Scope::CellEditor | Scope::SqlEditor => Mode::Insert,
                Scope::Global | Scope::Prompt => Mode::Any,
                Scope::Connections
                | Scope::Sidebar
                | Scope::Grid
                | Scope::Inspector
                | Scope::Results => Mode::Normal,
            },
            set => set,
        }
    }
}

const fn bind(command: Command, scopes: &'static [Scope], mac: Bound, omarchy: Bound) -> Binding {
    Binding {
        command,
        scopes,
        when: When::Always,
        mac,
        omarchy,
    }
}

impl Binding {
    const fn when(self, when: When) -> Self {
        Self { when, ..self }
    }
}

const GLOBAL: &[Scope] = &[Scope::Global];
const CONNECTIONS: &[Scope] = &[Scope::Connections];
const SIDEBAR: &[Scope] = &[Scope::Sidebar];
const GRID: &[Scope] = &[Scope::Grid];
const CELL_EDITOR: &[Scope] = &[Scope::CellEditor];
const INSPECTOR: &[Scope] = &[Scope::Inspector];
const SQL_EDITOR: &[Scope] = &[Scope::SqlEditor];
const RESULTS: &[Scope] = &[Scope::Results];
const PROMPT: &[Scope] = &[Scope::Prompt];
const ROWS: &[Scope] = &[Scope::Grid, Scope::Results];
const CELLS: &[Scope] = &[Scope::Grid, Scope::Inspector];

/// The keymap. The rows of the settled table first in each part, then the
/// keys the app had beside them.
pub const BINDINGS: &[Binding] = {
    use Command as C;
    &[
        // Window and connections.
        bind(
            C::Connections,
            GLOBAL,
            keys(&[k("cmd+o")]),
            keys(&[k("ctrl+shift+c")]),
        ),
        bind(
            C::GoToConnectionWindow,
            GLOBAL,
            keys(&[k("cmd+1..9")]),
            keys(&[k("ctrl+shift+1..9")]),
        ),
        bind(C::SwitchDatabase, GLOBAL, NONE, keys(&[k("ctrl+shift+d")])),
        bind(
            C::Settings,
            GLOBAL,
            keys(&[k("cmd+,")]),
            keys(&[k("ctrl+,")]),
        ),
        bind(
            C::CloseConnection,
            GLOBAL,
            keys(&[k("cmd+shift+w")]),
            keys(&[k("ctrl+shift+w")]),
        ),
        bind(
            C::NextConnection,
            GLOBAL,
            keys(&[k("ctrl+tab")]),
            keys(&[k("ctrl+tab")]),
        ),
        bind(
            C::PreviousConnection,
            GLOBAL,
            keys(&[k("ctrl+shift+tab")]),
            keys(&[k("ctrl+shift+tab")]),
        ),
        bind(
            C::CancelConnecting,
            GLOBAL,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::Connecting),
        bind(
            C::NewConnection,
            CONNECTIONS,
            keys(&[k("cmd+n")]).any(),
            keys(&[k("n")]),
        ),
        bind(
            C::EditConnection,
            CONNECTIONS,
            keys(&[k("cmd+e")]),
            keys(&[k("e")]),
        ),
        bind(
            C::DuplicateConnection,
            CONNECTIONS,
            keys(&[k("cmd+d")]),
            keys(&[k("yy p")]),
        ),
        bind(
            C::DeleteConnection,
            CONNECTIONS,
            keys(&[k("backspace")]),
            keys(&[k("dd")]),
        ),
        bind(
            C::Connect,
            CONNECTIONS,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        ),
        bind(
            C::ConnectAgain,
            CONNECTIONS,
            keys(&[k("shift+enter")]),
            keys(&[k("shift+enter")]),
        ),
        bind(
            C::MoveInConnections,
            CONNECTIONS,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        ),
        bind(C::FilterConnections, CONNECTIONS, NONE, keys(&[k("/")])),
        // Navigation.
        bind(C::FindObject, GLOBAL, keys(&[k("cmd+p")]), NONE),
        bind(C::FindObject, SIDEBAR, NONE, keys(&[k("/")])),
        bind(
            C::NewSqlTab,
            GLOBAL,
            keys(&[k("cmd+t")]),
            keys(&[k("ctrl+t")]),
        ),
        bind(
            C::ToggleSidebar,
            GLOBAL,
            keys(&[k("cmd+b")]),
            keys(&[k("ctrl+b")]),
        ),
        bind(C::PaneLeft, GLOBAL, NONE, keys(&[k("ctrl+h")]).normal()),
        bind(C::PaneDown, GLOBAL, NONE, keys(&[k("ctrl+j")]).normal()),
        bind(C::PaneUp, GLOBAL, NONE, keys(&[k("ctrl+k")]).normal()),
        bind(C::PaneRight, GLOBAL, NONE, keys(&[k("ctrl+l")]).normal()),
        bind(C::NextPart, GLOBAL, keys(&[k("f6")]), keys(&[k("f6")])),
        bind(
            C::PreviousPart,
            GLOBAL,
            keys(&[k("shift+f6")]),
            keys(&[k("shift+f6")]),
        ),
        bind(C::SwitchTab, GLOBAL, NONE, keys(&[k("alt+1..9")])),
        bind(
            C::CloseTab,
            GLOBAL,
            keys(&[k("cmd+w")]),
            // In insert mode the chord is vim's: it deletes a word.
            keys(&[k("ctrl+w")]).normal(),
        ),
        bind(
            C::PreviousTab,
            GLOBAL,
            keys(&[k("cmd+shift+[")]),
            keys(&[k("ctrl+shift+[")]),
        ),
        bind(
            C::NextTab,
            GLOBAL,
            keys(&[k("cmd+shift+]")]),
            keys(&[k("ctrl+shift+]")]),
        ),
        bind(
            C::MoveInTree,
            SIDEBAR,
            keys(&[
                k("up"),
                k("down"),
                k("left"),
                k("right"),
                k("home"),
                k("end"),
            ]),
            keys(&[
                k("j"),
                k("k"),
                k("up"),
                k("down"),
                k("left"),
                k("right"),
                k("home"),
                k("end"),
            ])
            .shown("j/k"),
        ),
        bind(
            C::OpenObject,
            SIDEBAR,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        ),
        bind(C::ToggleTree, SIDEBAR, NONE, keys(&[k("t")])),
        bind(
            C::Reload,
            &[Scope::Grid, Scope::Sidebar],
            keys(&[k("cmd+r")]).any(),
            keys(&[k("R")]),
        ),
        bind(
            C::FilterBar,
            GRID,
            keys(&[k("cmd+f")]).any(),
            keys(&[k("ctrl+f")]).any(),
        ),
        bind(C::WhereFilter, GRID, NONE, keys(&[k("/")])),
        bind(
            C::PreviousPage,
            GRID,
            keys(&[k("cmd+alt+left")]).any(),
            keys(&[k("ctrl+alt+left")]).any(),
        ),
        bind(
            C::NextPage,
            GRID,
            keys(&[k("cmd+alt+right")]).any(),
            keys(&[k("ctrl+alt+right")]).any(),
        ),
        bind(C::Help, GLOBAL, keys(&[k("?")]), keys(&[k("?")])),
        bind(C::KeysDefault, GLOBAL, NONE, keys(&[k(":keys default")])),
        // Grid and editing.
        bind(
            C::MoveRow,
            ROWS,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        ),
        bind(
            C::MoveColumn,
            ROWS,
            keys(&[k("left"), k("right")]),
            keys(&[k("h"), k("l"), k("left"), k("right")]).shown("h/l"),
        ),
        bind(
            C::MovePage,
            ROWS,
            keys(&[k("pgup"), k("pgdn"), k("home"), k("end")]),
            keys(&[k("pgup"), k("pgdn"), k("home"), k("end")]),
        ),
        bind(
            C::OpenInspector,
            GRID,
            keys(&[k("cmd+i")]).any(),
            keys(&[k("enter"), via("ctrl+l")]),
        ),
        bind(C::OpenInspector, RESULTS, NONE, keys(&[k("enter")])),
        bind(
            C::ToggleInspector,
            ROWS,
            keys(&[k("cmd+shift+r")]).any(),
            keys(&[k("ctrl+shift+r")]).any(),
        ),
        bind(C::CloseInspector, ROWS, NONE, keys(&[k("esc")])).when(When::InspectorOpen),
        bind(
            C::BackToGrid,
            INSPECTOR,
            keys(&[k("esc")]),
            keys(&[via("ctrl+h"), k("esc")]),
        ),
        bind(
            C::MoveField,
            INSPECTOR,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k")]).shown("j/k"),
        ),
        bind(
            C::PreviousRow,
            INSPECTOR,
            control("↑↓ in header"),
            keys(&[k("[")]),
        ),
        bind(
            C::NextRow,
            INSPECTOR,
            control("↑↓ in header"),
            keys(&[k("]")]),
        ),
        bind(
            C::FoldDocuments,
            &[Scope::Grid, Scope::Inspector, Scope::Results],
            NONE,
            keys(&[k("za")]),
        ),
        bind(
            C::EditCell,
            GRID,
            keys(&[k("enter"), k("f2")]),
            keys(&[k("i")]),
        ),
        bind(
            C::EditCell,
            INSPECTOR,
            keys(&[k("enter"), k("f2")]),
            keys(&[k("enter"), k("i")]),
        ),
        bind(C::ReplaceCell, CELLS, NONE, keys(&[k("cc"), k("s")])),
        bind(
            C::CommitDown,
            CELL_EDITOR,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        ),
        bind(
            C::CommitNext,
            CELL_EDITOR,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        ),
        bind(
            C::CommitPrevious,
            CELL_EDITOR,
            keys(&[k("shift+tab")]),
            keys(&[k("shift+tab")]),
        ),
        bind(C::KeepEdit, CELL_EDITOR, NONE, keys(&[k("esc")])),
        bind(
            C::CancelEdit,
            CELL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("ctrl+c")]),
        ),
        bind(
            C::OpenLargeEditor,
            CELL_EDITOR,
            keys(&[k("alt+enter")]),
            keys(&[k("alt+enter")]),
        ),
        bind(
            C::ApplyLargeEditor,
            CELL_EDITOR,
            keys(&[k("cmd+enter")]),
            // `:wq` needs a normal mode in the editor, which it has not.
            keys(&[claimed(":wq"), k("ctrl+enter")]),
        )
        .when(When::LargeEditor),
        bind(C::KeepLargeEditor, CELL_EDITOR, NONE, keys(&[k("esc")])).when(When::LargeEditor),
        bind(
            C::CancelLargeEditor,
            CELL_EDITOR,
            keys(&[k("esc")]),
            keys(&[claimed(":q!"), k("ctrl+c")]),
        )
        .when(When::LargeEditor),
        bind(C::OpenInEditor, CELL_EDITOR, NONE, keys(&[k("ctrl+e")])),
        bind(
            C::SetNull,
            CELLS,
            // Only on a selected cell that is not being edited: in a text
            // field the chord deletes to the start of the line.
            keys(&[k("cmd+backspace")]).normal(),
            keys(&[k("x")]),
        ),
        bind(C::SetDefault, CELLS, keys(&[k("cmd+'")]), keys(&[k("D")])),
        bind(C::UndoCell, CELLS, keys(&[k("cmd+z")]), keys(&[k("u")])),
        bind(
            C::RedoCell,
            CELLS,
            keys(&[k("cmd+shift+z")]),
            keys(&[k("ctrl+r")]).normal(),
        ),
        bind(C::CopyCells, GRID, keys(&[k("cmd+c")]), keys(&[k("v y")])),
        bind(C::CopyRow, GRID, keys(&[k("cmd+shift+c")]), NONE),
        bind(C::PasteCells, GRID, keys(&[k("cmd+v")]), keys(&[k("v p")])),
        bind(
            C::PasteNewRows,
            GRID,
            keys(&[k("cmd+shift+v")]),
            keys(&[k("\"+p"), k("ctrl+shift+v")]),
        ),
        bind(C::AddRow, GRID, keys(&[k("cmd+n")]).any(), keys(&[k("o")])),
        bind(C::AddRowAbove, GRID, NONE, keys(&[k("O")])),
        bind(C::AddRowAtEnd, GRID, keys(&[k("down")]), NONE).when(When::LastRow),
        bind(
            C::DuplicateRow,
            GRID,
            keys(&[k("cmd+d")]),
            keys(&[k("yy p")]),
        ),
        bind(C::DuplicateRowAbove, GRID, NONE, keys(&[k("yy P")])),
        bind(
            C::DeleteRow,
            GRID,
            keys(&[k("backspace"), k("delete")]),
            keys(&[k("dd")]),
        ),
        bind(C::OpenReferencedRow, GRID, NONE, keys(&[k("gd")])),
        bind(C::ShowSavedRow, GRID, NONE, keys(&[k("gs")])),
        bind(
            C::ReviewSql,
            GRID,
            keys(&[k("cmd+shift+d")]).any(),
            keys(&[k(":diff")]),
        ),
        bind(C::NextChange, GRID, NONE, keys(&[k("]c")])),
        bind(C::CopyReviewSql, GRID, NONE, keys(&[k("Y")])).when(When::Review),
        bind(C::CloseReview, GRID, NONE, keys(&[k("esc")])).when(When::Review),
        bind(C::DismissNote, GRID, NONE, keys(&[k("esc")])).when(When::Note),
        bind(
            C::SaveChanges,
            GRID,
            keys(&[k("cmd+s")]).any(),
            keys(&[k(":w"), k("ctrl+s")]).any(),
        ),
        bind(
            C::DiscardChanges,
            GRID,
            keys(&[k("cmd+alt+backspace")]),
            keys(&[k(":e!")]),
        ),
        bind(
            C::NextError,
            GRID,
            keys(&[k("cmd+alt+down")]),
            keys(&[k("]e")]),
        ),
        bind(
            C::PreviousError,
            GRID,
            keys(&[k("cmd+alt+up")]),
            keys(&[k("[e")]),
        ),
        bind(
            C::NextError,
            SQL_EDITOR,
            keys(&[k("cmd+alt+down")]).any(),
            keys(&[k("]e")]),
        ),
        bind(
            C::PreviousError,
            SQL_EDITOR,
            keys(&[k("cmd+alt+up")]).any(),
            keys(&[k("[e")]),
        ),
        bind(
            C::BooleanCycle,
            GRID,
            keys(&[k("space")]),
            keys(&[k("space")]),
        ),
        bind(
            C::StepUp,
            CELL_EDITOR,
            keys(&[k("up")]),
            keys(&[k("ctrl+a")]),
        ),
        bind(
            C::StepDown,
            CELL_EDITOR,
            keys(&[k("down")]),
            keys(&[k("ctrl+x")]),
        ),
        bind(
            C::Format,
            SQL_EDITOR,
            keys(&[k("cmd+shift+f")]).any(),
            keys(&[k("=")]),
        ),
        bind(
            C::Format,
            CELL_EDITOR,
            keys(&[k("cmd+shift+f")]),
            // `=` needs a normal mode in the editor, which it has not.
            keys(&[claimed("="), k("ctrl+shift+f")]),
        )
        .when(When::LargeEditor),
        // SQL editor.
        bind(
            C::RunStatement,
            SQL_EDITOR,
            keys(&[k("cmd+enter")]).any(),
            keys(&[k("ctrl+enter")]).any(),
        ),
        bind(
            C::RunAll,
            SQL_EDITOR,
            keys(&[k("cmd+shift+enter")]).any(),
            keys(&[k("ctrl+shift+enter")]).any(),
        ),
        bind(
            C::Explain,
            SQL_EDITOR,
            keys(&[k("cmd+e")]).any(),
            keys(&[k("ctrl+e")]).any(),
        ),
        bind(C::ExplainAnalyze, SQL_EDITOR, NONE, NONE),
        bind(
            C::ComparePreviousRun,
            SQL_EDITOR,
            keys(&[k("cmd+alt+e")]).any(),
            NONE,
        ),
        bind(
            C::CancelQuery,
            SQL_EDITOR,
            keys(&[k("cmd+.")]).any(),
            // While a query runs. With none running the chord is the text
            // field's own.
            keys(&[k("ctrl+c")]).any(),
        ),
        bind(
            C::CancelQuery,
            GRID,
            keys(&[k("cmd+.")]).any(),
            // Normal mode only: in a cell's editor the chord drops the
            // edit, and in another field it is the copy.
            keys(&[k("ctrl+c")]),
        ),
        bind(C::FocusResults, SQL_EDITOR, NONE, keys(&[k("ctrl+j")])),
        bind(
            C::LeaveEditor,
            SQL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        ),
        bind(
            C::Complete,
            SQL_EDITOR,
            keys(&[k("ctrl+space"), k("cmd+i")]),
            keys(&[k("ctrl+space")]),
        ),
        bind(
            C::InsertCompletion,
            SQL_EDITOR,
            keys(&[k("enter"), k("tab")]),
            keys(&[k("tab"), k("enter")]),
        )
        .when(When::Completion),
        bind(
            C::NextCompletion,
            SQL_EDITOR,
            keys(&[k("down")]),
            keys(&[k("ctrl+n"), k("down")]),
        )
        .when(When::Completion),
        bind(
            C::PreviousCompletion,
            SQL_EDITOR,
            keys(&[k("up")]),
            keys(&[k("ctrl+p"), k("up")]),
        )
        .when(When::Completion),
        bind(
            C::CloseCompletion,
            SQL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::Completion),
        bind(
            C::GoToLine,
            SQL_EDITOR,
            keys(&[k("cmd+l")]),
            keys(&[k(":N")]),
        ),
        bind(C::History, SQL_EDITOR, NONE, keys(&[k(":history")])),
        bind(
            C::ReadOnlyTab,
            SQL_EDITOR,
            control("segmented switch"),
            keys(&[k(":ro")]),
        ),
        bind(
            C::ReadWriteTab,
            SQL_EDITOR,
            control("segmented switch"),
            keys(&[k(":rw")]),
        ),
        bind(
            C::ToggleSqlMode,
            SQL_EDITOR,
            keys(&[k("cmd+shift+m")]).any(),
            NONE,
        ),
        bind(
            C::Commit,
            SQL_EDITOR,
            keys(&[k("cmd+s")]).any(),
            keys(&[k(":commit"), k("ctrl+s")]).any(),
        ),
        bind(
            C::Rollback,
            SQL_EDITOR,
            keys(&[k("cmd+alt+backspace")]).any(),
            keys(&[k(":rollback")]),
        ),
        bind(C::NextPlanTip, RESULTS, NONE, keys(&[k("]t")])),
        bind(C::EditRefusedConnection, SQL_EDITOR, NONE, keys(&[k("e")])).when(When::RefusedWrite),
        bind(C::AllowRefusedWrite, SQL_EDITOR, NONE, keys(&[k("w")])).when(When::RefusedWrite),
        // AI assistant.
        bind(
            C::AskAi,
            SQL_EDITOR,
            keys(&[k("cmd+k")]),
            keys(&[k("ctrl+k")]),
        ),
        bind(
            C::AcceptAi,
            SQL_EDITOR,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        )
        .when(When::Suggestion),
        bind(
            C::AcceptAiAndRun,
            SQL_EDITOR,
            keys(&[k("cmd+enter")]),
            keys(&[k("ctrl+enter")]),
        )
        .when(When::Suggestion),
        bind(
            C::DiscardAi,
            SQL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::Suggestion),
        // Prompts and dialogs.
        bind(
            C::ConfirmProductionWrite,
            PROMPT,
            control("dialog button"),
            keys(&[k("enter")]).shown("type the database name, then enter"),
        )
        .when(When::ConfirmWrite),
        bind(
            C::CancelProductionWrite,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::ConfirmWrite),
        bind(
            C::ScrollStatements,
            PROMPT,
            keys(&[k("pgup"), k("pgdn")]),
            keys(&[k("pgup"), k("pgdn")]).shown("pgup/pgdn"),
        )
        .when(When::ConfirmWrite),
        bind(C::LeaveWrite, PROMPT, NONE, keys(&[k("w")])).when(When::Leaving),
        bind(C::LeaveDiscard, PROMPT, NONE, keys(&[k("d")])).when(When::Leaving),
        bind(C::LeaveStay, PROMPT, keys(&[k("esc")]), keys(&[k("esc")])).when(When::Leaving),
        bind(C::ConflictOverwrite, PROMPT, NONE, keys(&[k("o")])).when(When::Conflict),
        bind(C::ConflictUseServer, PROMPT, NONE, keys(&[k("s")])).when(When::Conflict),
        bind(
            C::ConflictKeepMine,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("k"), k("esc")]),
        )
        .when(When::Conflict),
        bind(C::ConflictDiscard, PROMPT, NONE, keys(&[k("d")])).when(When::Conflict),
        bind(C::FormPasteUrl, PROMPT, NONE, keys(&[k("u")])).when(When::ConnectionForm),
        bind(
            C::FormNextField,
            PROMPT,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        )
        .when(When::ConnectionForm),
        bind(
            C::FormTest,
            PROMPT,
            keys(&[k("cmd+t")]),
            keys(&[k("ctrl+t")]),
        )
        .when(When::ConnectionForm),
        bind(
            C::FormSave,
            PROMPT,
            keys(&[k("cmd+s")]),
            keys(&[k("ctrl+s")]),
        )
        .when(When::ConnectionForm),
        bind(
            C::FormSaveAndConnect,
            PROMPT,
            keys(&[k("cmd+enter")]),
            keys(&[k("ctrl+enter")]),
        )
        .when(When::ConnectionForm),
        bind(C::FormCancel, PROMPT, keys(&[k("esc")]), keys(&[k("esc")]))
            .when(When::ConnectionForm),
        bind(
            C::SettingsMove,
            PROMPT,
            NONE,
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        )
        .when(When::SettingsScreen),
        bind(
            C::SettingsChange,
            PROMPT,
            NONE,
            keys(&[k("h"), k("l"), k("left"), k("right")]).shown("h/l"),
        )
        .when(When::SettingsScreen),
        bind(C::SettingsToggle, PROMPT, NONE, keys(&[k("space")])).when(When::SettingsScreen),
        bind(C::SettingsOpenFile, PROMPT, NONE, keys(&[k("ctrl+e")])).when(When::SettingsScreen),
        bind(C::SettingsReset, PROMPT, NONE, keys(&[k("R")])).when(When::SettingsScreen),
        bind(
            C::SettingsClose,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::SettingsScreen),
        bind(
            C::QuickOpenMove,
            PROMPT,
            keys(&[k("up"), k("down")]),
            keys(&[k("up"), k("down")]),
        )
        .when(When::QuickOpen),
        bind(
            C::QuickOpenPick,
            PROMPT,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        )
        .when(When::QuickOpen),
        bind(
            C::QuickOpenClose,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::QuickOpen),
    ]
};

/// The modifiers of a chord, as a spelling names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub fn none(self) -> bool {
        self == Self::default()
    }
}

/// One press: a key with what is held, or a character as it is typed (a
/// vim letter is its character on every keyboard layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stroke {
    Key { mods: Mods, key: Key },
    Text(char),
}

/// A spelling, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spelled {
    /// One press, or a few after one another (`yy p`).
    Strokes(Vec<Stroke>),
    /// The digits `1` to `9` with these held (`alt+1..9`).
    Digits(Mods),
    /// A line of the `:` prompt, without its colon.
    Ex(&'static str),
}

/// Why a spelling could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unread(pub String);

const NAMED: &[(&str, Key)] = &[
    ("enter", Key::Enter),
    ("esc", Key::Escape),
    ("tab", Key::Tab),
    ("space", Key::Space),
    ("backspace", Key::Backspace),
    ("delete", Key::Delete),
    ("up", Key::ArrowUp),
    ("down", Key::ArrowDown),
    ("left", Key::ArrowLeft),
    ("right", Key::ArrowRight),
    ("home", Key::Home),
    ("end", Key::End),
    ("pgup", Key::PageUp),
    ("pgdn", Key::PageDown),
    ("f2", Key::F2),
    ("f6", Key::F6),
];

const MARKS: &[(char, Key)] = &[
    (',', Key::Comma),
    ('.', Key::Period),
    ('\'', Key::Quote),
    ('[', Key::OpenBracket),
    (']', Key::CloseBracket),
    ('/', Key::Slash),
    ('=', Key::Equals),
    ('-', Key::Minus),
    (';', Key::Semicolon),
    ('`', Key::Backtick),
    ('\\', Key::Backslash),
];

fn named(name: &str) -> Option<Key> {
    NAMED
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, key)| *key)
}

/// The key a chord names by one character: a letter, a digit or a mark.
fn key_of(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    let (Some(char), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if char.is_ascii_lowercase() || char.is_ascii_digit() {
        return Key::from_name(&char.to_ascii_uppercase().to_string());
    }
    MARKS
        .iter()
        .find(|(mark, _)| *mark == char)
        .map(|(_, key)| *key)
}

/// The character a key types on a US keyboard, with Shift or without: a
/// letter, a digit or a mark. For a key that came with no text (a test's
/// bare press).
pub fn char_of(key: Key, shift: bool) -> Option<char> {
    if let Some(mark) = key_mark(key) {
        return (!shift).then_some(mark);
    }
    let mut chars = key.name().chars();
    let (Some(char), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if char.is_ascii_uppercase() {
        return Some(if shift {
            char
        } else {
            char.to_ascii_lowercase()
        });
    }
    (char.is_ascii_digit() && !shift).then_some(char)
}

/// Reads a spelling. `super` is no modifier here: Hyprland owns it.
pub fn spell(keys: &'static str) -> Result<Spelled, Unread> {
    let unread = |why: &str| Unread(format!("{keys}: {why}"));
    if let Some(line) = keys.strip_prefix(':') {
        if line.is_empty() {
            return Err(unread("a command without a name"));
        }
        return Ok(Spelled::Ex(line));
    }
    if let Some(key) = named(keys) {
        let mods = Mods::default();
        return Ok(Spelled::Strokes(vec![Stroke::Key { mods, key }]));
    }
    let parts: Vec<&str> = keys.split('+').collect();
    let modifier = |part: &str| matches!(part, "cmd" | "ctrl" | "alt" | "shift");
    if parts.len() > 1 && modifier(parts[0]) {
        let (last, held) = parts.split_last().expect("more than one part");
        let mut mods = Mods::default();
        for part in held {
            let flag = match *part {
                "cmd" => &mut mods.cmd,
                "ctrl" => &mut mods.ctrl,
                "alt" => &mut mods.alt,
                "shift" => &mut mods.shift,
                other => return Err(unread(&format!("{other} is no modifier"))),
            };
            if std::mem::replace(flag, true) {
                return Err(unread("a modifier twice"));
            }
        }
        if *last == "1..9" {
            return Ok(Spelled::Digits(mods));
        }
        let key = named(last)
            .or_else(|| key_of(last))
            .ok_or_else(|| unread("no such key"))?;
        return Ok(Spelled::Strokes(vec![Stroke::Key { mods, key }]));
    }
    // A word before a `+` that is no modifier (`super+t`) is a mistake,
    // not five letters: only `"+p` has a `+` among typed characters.
    if parts.len() > 1 && parts.iter().any(|part| part.chars().count() > 1) {
        return Err(unread("no such modifier"));
    }
    // Typed characters, one after another. A space only sets them apart.
    let strokes: Vec<Stroke> = keys
        .chars()
        .filter(|char| *char != ' ')
        .map(Stroke::Text)
        .collect();
    if strokes.is_empty() || keys.chars().any(char::is_control) {
        return Err(unread("nothing to press"));
    }
    Ok(Spelled::Strokes(strokes))
}

fn key_word(key: Key) -> &'static str {
    NAMED
        .iter()
        .find(|(_, known)| *known == key)
        .map_or_else(|| key.name(), |(name, _)| name)
}

fn key_mark(key: Key) -> Option<char> {
    MARKS
        .iter()
        .find(|(_, known)| *known == key)
        .map(|(mark, _)| *mark)
}

/// A key as `layout` writes it.
fn written_key(layout: Layout, key: Key) -> String {
    if let Some(mark) = key_mark(key) {
        return mark.to_string();
    }
    match layout {
        Layout::Omarchy => key_word(key).to_lowercase(),
        Layout::Mac => match key {
            Key::Enter => "↩".to_owned(),
            Key::Backspace => "⌫".to_owned(),
            Key::Delete => "⌦".to_owned(),
            Key::Tab => "⇥".to_owned(),
            Key::Escape => "esc".to_owned(),
            Key::Space => "Space".to_owned(),
            Key::ArrowUp => "↑".to_owned(),
            Key::ArrowDown => "↓".to_owned(),
            Key::ArrowLeft => "←".to_owned(),
            Key::ArrowRight => "→".to_owned(),
            Key::PageUp => "Page Up".to_owned(),
            Key::PageDown => "Page Down".to_owned(),
            Key::Home => "Home".to_owned(),
            Key::End => "End".to_owned(),
            other => other.name().to_owned(),
        },
        Layout::Windows => match key {
            Key::Escape => "Esc".to_owned(),
            Key::ArrowUp => "Up".to_owned(),
            Key::ArrowDown => "Down".to_owned(),
            Key::ArrowLeft => "Left".to_owned(),
            Key::ArrowRight => "Right".to_owned(),
            Key::PageUp => "Page Up".to_owned(),
            Key::PageDown => "Page Down".to_owned(),
            other => other.name().to_owned(),
        },
    }
}

/// What is held, as `layout` writes it before the key.
fn written_mods(layout: Layout, mods: Mods) -> String {
    let mut written = String::new();
    match layout {
        // Apple's order: Control, Option, Shift, Command.
        Layout::Mac => {
            for (held, glyph) in [
                (mods.ctrl, "⌃"),
                (mods.alt, "⌥"),
                (mods.shift, "⇧"),
                (mods.cmd, "⌘"),
            ] {
                if held {
                    written.push_str(glyph);
                }
            }
        }
        Layout::Windows => {
            for (held, word) in [
                (mods.cmd || mods.ctrl, "Ctrl+"),
                (mods.alt, "Alt+"),
                (mods.shift, "Shift+"),
            ] {
                if held {
                    written.push_str(word);
                }
            }
        }
        Layout::Omarchy => {
            for (held, word) in [
                (mods.cmd, "cmd+"),
                (mods.ctrl, "ctrl+"),
                (mods.alt, "alt+"),
                (mods.shift, "shift+"),
            ] {
                if held {
                    written.push_str(word);
                }
            }
        }
    }
    written
}

/// A spelling as `layout` writes it: the one way a key is written there.
pub fn written(layout: Layout, keys: &'static str) -> String {
    let Ok(spelled) = spell(keys) else {
        return keys.to_owned();
    };
    match spelled {
        Spelled::Ex(_) => keys.to_owned(),
        Spelled::Digits(mods) => {
            let range = match layout {
                Layout::Omarchy => "1..9",
                Layout::Mac | Layout::Windows => "1…9",
            };
            format!("{}{range}", written_mods(layout, mods))
        }
        Spelled::Strokes(strokes) => match strokes.as_slice() {
            [Stroke::Key { mods, key }] => {
                format!(
                    "{}{}",
                    written_mods(layout, *mods),
                    written_key(layout, *key)
                )
            }
            // Typed characters are written as they are spelled, the space
            // that sets them apart too.
            _ => keys.to_owned(),
        },
    }
}

/// A key as a layout writes it. Only the keymap makes one, so what paints
/// a key beside a word (a hint, a button's shortcut) can take nothing
/// written by hand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written(String);

impl Written {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Written {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Written {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl PartialEq<&str> for Written {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// What typed characters, or a line of the `:` prompt, come to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typed {
    /// They are this command's keys.
    Command(Command),
    /// They are the keys of something that is not built yet: taken, and
    /// nothing happens.
    Claimed,
    /// They begin a command's keys: the next character says which.
    Waiting,
    /// They are nobody's.
    Nothing,
}

/// The keymap in force: the defaults. What the user rebinds comes to stand
/// over them here.
#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            bindings: BINDINGS.to_vec(),
        }
    }
}

impl Keymap {
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The lines of `command`, in the keymap's order.
    pub fn of(&self, command: Command) -> impl Iterator<Item = &Binding> {
        self.bindings
            .iter()
            .filter(move |binding| binding.command == command)
    }

    /// The spellings that answer `command` in `scope`: its own keys, and
    /// none that is only claimed.
    pub fn chords(
        &self,
        layout: Layout,
        command: Command,
        scope: Scope,
    ) -> impl Iterator<Item = &'static str> + '_ {
        let built = command.info().built;
        self.of(command)
            .filter(move |binding| built && binding.scopes.contains(&scope))
            .flat_map(move |binding| binding.bound(layout).chords.iter())
            .filter(|chord| chord.kind == Kind::Own)
            .map(|chord| chord.keys)
    }

    /// The spellings that answer `command` in `scope` now: with the
    /// keyboard out of a text field (`normal`) or in one. A chord the
    /// keymap gives to normal mode is the field's own while one has it.
    pub fn chords_now(
        &self,
        layout: Layout,
        command: Command,
        scope: Scope,
        normal: bool,
    ) -> impl Iterator<Item = &'static str> + '_ {
        let built = command.info().built;
        self.of(command)
            .filter(move |binding| built && binding.scopes.contains(&scope))
            .flat_map(move |binding| {
                binding
                    .bound(layout)
                    .chords
                    .iter()
                    .map(move |chord| (binding, chord))
            })
            .filter(move |(binding, chord)| {
                chord.kind == Kind::Own
                    && spell(chord.keys)
                        .is_ok_and(|spelled| binding.mode(scope, layout, &spelled).holds(normal))
            })
            .map(|(_, chord)| chord.keys)
    }

    /// Every key of `command` as `layout` writes them, for a list of keys:
    /// each line's keys once, `, ` between them. Empty where it has none.
    pub fn label(&self, layout: Layout, command: Command) -> Written {
        let built = command.info().built;
        let mut all: Vec<String> = Vec::new();
        for binding in self.of(command) {
            let bound = binding.bound(layout);
            let mut line: Vec<String> = Vec::new();
            if bound.shown.is_empty() {
                for chord in bound.chords {
                    // A claimed key of a command that answers its others is
                    // written nowhere: it would be offered and do nothing.
                    if chord.kind != Kind::Claimed || !built {
                        line.push(written(layout, chord.keys));
                    }
                }
            } else {
                line.push(bound.shown.to_owned());
            }
            for keys in line {
                if !all.contains(&keys) {
                    all.push(keys);
                }
            }
        }
        Written(all.join(", "))
    }

    /// The lines that can answer in `scope`: its own and the global ones
    /// (a prompt has only its own), with what `holds` says is on screen.
    fn held<'a>(
        &'a self,
        scope: Scope,
        holds: &'a dyn Fn(When) -> bool,
    ) -> impl Iterator<Item = &'a Binding> {
        self.bindings.iter().filter(move |binding| {
            let global = scope != Scope::Prompt && binding.scopes.contains(&Scope::Global);
            (global || binding.scopes.contains(&scope))
                && (binding.when == When::Always || holds(binding.when))
        })
    }

    /// What `spelled` of a line that holds comes to, where `matches` says
    /// whether it is the keys asked about and `begins` whether they begin
    /// it. A line with a `when` takes its key before one without.
    fn read(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        matches: &dyn Fn(&Spelled) -> bool,
        begins: &dyn Fn(&Spelled) -> bool,
    ) -> Typed {
        let mut found = Typed::Nothing;
        let mut qualified = false;
        for binding in self.held(scope, holds) {
            for chord in binding.bound(layout).chords {
                let Ok(spelled) = spell(chord.keys) else {
                    continue;
                };
                if chord.kind == Kind::Via {
                    continue;
                }
                let own = binding.when != When::Always;
                let answered = matches!(found, Typed::Command(_) | Typed::Claimed);
                if matches(&spelled) {
                    if answered && qualified && !own {
                        continue;
                    }
                    let answers = binding.command.info().built && chord.kind == Kind::Own;
                    found = if answers {
                        Typed::Command(binding.command)
                    } else {
                        Typed::Claimed
                    };
                    qualified = own;
                } else if begins(&spelled) && !answered {
                    found = Typed::Waiting;
                }
            }
        }
        found
    }

    /// What the characters typed so far (`typed`: `y`, then `yy`, then
    /// `yyp`) come to in `scope`.
    pub fn typed(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        typed: &str,
    ) -> Typed {
        let letters = |spelled: &Spelled| -> Option<String> {
            let Spelled::Strokes(strokes) = spelled else {
                return None;
            };
            strokes
                .iter()
                .map(|stroke| match stroke {
                    Stroke::Text(char) => Some(*char),
                    Stroke::Key { .. } => None,
                })
                .collect()
        };
        self.read(
            layout,
            scope,
            holds,
            &|spelled| letters(spelled).is_some_and(|letters| letters == typed),
            &|spelled| letters(spelled).is_some_and(|letters| letters.starts_with(typed)),
        )
    }

    /// What a line of the `:` prompt (`line`, without its colon) comes to
    /// in `scope`. A line of digits is the one spelled `:N`.
    pub fn ex(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        line: &str,
    ) -> Typed {
        let number = !line.is_empty() && line.chars().all(|char| char.is_ascii_digit());
        self.read(
            layout,
            scope,
            holds,
            &|spelled| match spelled {
                Spelled::Ex(name) => *name == line || (number && *name == "N"),
                Spelled::Strokes(_) | Spelled::Digits(_) => false,
            },
            &|_| false,
        )
    }

    /// The first key of `command` as `layout` writes it, for a hint beside
    /// a control. Empty where it has none.
    pub fn key(&self, layout: Layout, command: Command) -> Written {
        let label = self.label(layout, command);
        let first = label.split(", ").next().unwrap_or_default();
        Written(first.to_owned())
    }

    /// The first keys of `commands` written as one hint: `[ ]` for the
    /// previous row and the next, `ctrl+n/p` where they are held alike.
    pub fn together(&self, layout: Layout, commands: &[Command]) -> Written {
        let keys: Vec<Written> = commands
            .iter()
            .map(|command| self.key(layout, *command))
            .filter(|key| !key.is_empty())
            .collect();
        let Some(first) = keys.first() else {
            return Written::default();
        };
        // What is held, up to its last `+`: alike for all, it is written
        // once.
        let held = first.rfind('+').map_or("", |plus| &first[..=plus]);
        let alike = !held.is_empty() && keys.iter().all(|key| key.starts_with(held));
        let written = if alike {
            let rest: Vec<&str> = keys.iter().map(|key| &key[held.len()..]).collect();
            format!("{held}{}", rest.join("/"))
        } else {
            let all: Vec<&str> = keys.iter().map(Written::as_str).collect();
            all.join(" ")
        };
        Written(written)
    }

    /// The key that goes to the `digit`th (from one) of what `command`
    /// has a range of digits for: `⌘2`, `ctrl+shift+2`.
    pub fn digit_label(&self, layout: Layout, command: Command, digit: usize) -> Written {
        let held = self
            .of(command)
            .flat_map(|binding| binding.bound(layout).chords.iter())
            .find_map(|chord| match spell(chord.keys) {
                Ok(Spelled::Digits(mods)) => Some(mods),
                _ => None,
            });
        match held {
            Some(mods) if (1..=9).contains(&digit) => {
                Written(format!("{}{digit}", written_mods(layout, mods)))
            }
            _ => Written::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The presses of a spelling, each digit of a range apart.
    fn presses(layout: Layout, spelled: &Spelled) -> Vec<Vec<Stroke>> {
        // Windows has one Ctrl where the Mac has Cmd and Control.
        let fold = |mods: Mods| match layout {
            Layout::Windows => Mods {
                cmd: false,
                ctrl: mods.cmd || mods.ctrl,
                ..mods
            },
            Layout::Mac | Layout::Omarchy => mods,
        };
        match spelled {
            Spelled::Ex(_) => Vec::new(),
            Spelled::Digits(mods) => (1..=9)
                .map(|digit| {
                    let key = Key::from_name(&digit.to_string()).expect("a digit");
                    vec![Stroke::Key {
                        mods: fold(*mods),
                        key,
                    }]
                })
                .collect(),
            Spelled::Strokes(strokes) => vec![
                strokes
                    .iter()
                    .map(|stroke| match stroke {
                        Stroke::Key { mods, key } => Stroke::Key {
                            mods: fold(*mods),
                            key: *key,
                        },
                        text => *text,
                    })
                    .collect(),
            ],
        }
    }

    /// Whether two spellings would be read as one another: the same line
    /// of the prompt, the same presses, or presses the other begins with
    /// (after `y` nothing can tell `y` from `yy p`).
    fn clash(layout: Layout, a: &Spelled, b: &Spelled) -> bool {
        if let (Spelled::Ex(a), Spelled::Ex(b)) = (a, b) {
            return a == b;
        }
        presses(layout, a).iter().any(|a| {
            presses(layout, b).iter().any(|b| {
                let shared = a.len().min(b.len());
                a[..shared] == b[..shared]
            })
        })
    }

    struct Entry {
        command: Command,
        scope: Scope,
        when: When,
        mode: Mode,
        keys: &'static str,
        spelled: Spelled,
    }

    /// Every key of the keymap that is bound in `layout`, claimed ones and
    /// those of commands not built yet too. Not the ones written beside a
    /// command that are another's.
    fn entries(keymap: &Keymap, layout: Layout) -> Vec<Entry> {
        let mut entries = Vec::new();
        for binding in keymap.bindings() {
            for chord in binding.bound(layout).chords {
                if chord.kind == Kind::Via {
                    continue;
                }
                let spelled = spell(chord.keys).expect("a spelling that reads");
                for scope in binding.scopes {
                    entries.push(Entry {
                        command: binding.command,
                        scope: *scope,
                        when: binding.when,
                        mode: binding.mode(*scope, layout, &spelled),
                        keys: chord.keys,
                        spelled: spelled.clone(),
                    });
                }
            }
        }
        entries
    }

    #[test]
    fn every_spelling_reads() {
        for binding in BINDINGS {
            for bound in [&binding.mac, &binding.omarchy] {
                for chord in bound.chords {
                    assert!(spell(chord.keys).is_ok(), "{:?}", spell(chord.keys));
                }
            }
        }
        for unread in ["super+t", "ctrl+ctrl+t", "ctrl+nosuchkey", ":", ""] {
            assert!(spell(unread).is_err(), "{unread}");
        }
    }

    #[test]
    fn a_key_clashes_with_itself_and_with_what_begins_with_it() {
        let same = |a, b| {
            let (a, b) = (spell(a).unwrap(), spell(b).unwrap());
            Layout::ALL.into_iter().all(|layout| clash(layout, &a, &b))
        };
        assert!(same("ctrl+s", "ctrl+s"));
        assert!(same("y", "yy p"));
        assert!(same("yy p", "y"));
        assert!(same("]", "]e"));
        assert!(same("alt+3", "alt+1..9"));
        assert!(same(":w", ":w"));
        let apart = |a, b| {
            let (a, b) = (spell(a).unwrap(), spell(b).unwrap());
            Layout::ALL.into_iter().all(|layout| !clash(layout, &a, &b))
        };
        assert!(apart("yy p", "yy P"));
        assert!(apart("d", "D"));
        assert!(apart("ctrl+s", "ctrl+shift+s"));
        assert!(apart(":w", ":wq"));
        assert!(apart("]e", "]c"));
        assert!(apart("alt+3", "ctrl+shift+1..9"));
        // Windows has one Ctrl for the Mac's Cmd and Control.
        let (cmd, control) = (spell("cmd+t").unwrap(), spell("ctrl+t").unwrap());
        assert!(clash(Layout::Windows, &cmd, &control));
        assert!(!clash(Layout::Mac, &cmd, &control));
    }

    #[test]
    fn no_two_commands_share_a_key_where_both_could_be_read() {
        let keymap = Keymap::default();
        for layout in Layout::ALL {
            let entries = entries(&keymap, layout);
            for (index, a) in entries.iter().enumerate() {
                for b in &entries[index + 1..] {
                    let together =
                        a.scope.alongside(b.scope) && a.when == b.when && a.mode.overlaps(b.mode);
                    assert!(
                        a.command == b.command
                            || !together
                            || !clash(layout, &a.spelled, &b.spelled),
                        "{layout:?}: {} ({:?}, {}) and {} ({:?}, {})",
                        a.keys,
                        a.command,
                        a.scope.id(),
                        b.keys,
                        b.command,
                        b.scope.id(),
                    );
                }
            }
        }
    }

    #[test]
    fn every_command_has_its_keys_or_none_on_purpose() {
        let keymap = Keymap::default();
        for command in Command::ALL {
            let lines: Vec<&Binding> = keymap.of(*command).collect();
            assert!(!lines.is_empty(), "{command:?} is in no line of the keymap");
            assert!(!command.info().name.is_empty(), "{command:?}");
            for line in lines {
                assert!(!line.scopes.is_empty(), "{command:?}");
                for bound in [&line.mac, &line.omarchy] {
                    // Keys, or what does it in their place, never both.
                    assert!(
                        bound.chords.is_empty() || bound.control.is_empty(),
                        "{command:?}"
                    );
                    assert!(
                        bound.shown.is_empty() || !bound.chords.is_empty(),
                        "{command:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_keymap_has_every_row_of_the_settled_table_by_its_name() {
        let mut rows: Vec<&str> = Vec::new();
        for command in Command::ALL {
            let row = command.info().row;
            if !row.is_empty() && !rows.contains(&row) {
                rows.push(row);
            }
        }
        let mut settled = vec![
            "Connections",
            "Go to connection window",
            "Switch database",
            "Settings",
            "Find any object",
            "New SQL editor tab",
            "Toggle sidebar",
            "Move between panes",
            "Switch tab",
            "New connection",
            "Edit connection",
            "Duplicate connection",
            "Delete connection",
            "Connect / go to window",
            "Reload",
            "Open inspector / focus fields",
            "Back to grid from inspector",
            "Previous / next row in inspector",
            "Edit cell",
            "Commit cell and move",
            "Cancel cell edit",
            "Set NULL",
            "Set DEFAULT",
            "Undo / redo one cell",
            "Copy / paste cell values",
            "Paste as new rows",
            "Add row",
            "Duplicate row",
            "Delete row",
            "Open referenced row",
            "Show saved row in sorted position",
            "Review pending SQL",
            "Save changes",
            "Discard all pending",
            "Next / previous error",
            "Open value in $EDITOR",
            "Apply / cancel large editor",
            "Format SQL or JSON",
            "Boolean cycle",
            "Step number / date part",
            "Run statement",
            "Run all",
            "Explain",
            "Explain analyze",
            "Compare with previous run",
            "Cancel running query",
            "Focus results",
            "Completion",
            "Go to line",
            "History",
            "Read-only / read-write tab",
            "Commit",
            "Rollback",
            "Ask AI",
            "Accept / accept and run / discard AI",
            "Next plan tip",
            "Confirm production write",
            "Help, all keys",
        ];
        rows.sort_unstable();
        settled.sort_unstable();
        assert_eq!(rows, settled);
    }

    #[test]
    fn omarchy_binds_no_super_and_nothing_over_vims_insert_keys() {
        let keymap = Keymap::default();
        let ctrl = |key| Stroke::Key {
            mods: Mods {
                ctrl: true,
                ..Mods::default()
            },
            key,
        };
        let vims = [ctrl(Key::W), ctrl(Key::H), ctrl(Key::U), ctrl(Key::R)];
        for entry in entries(&keymap, Layout::Omarchy) {
            let presses = presses(Layout::Omarchy, &entry.spelled);
            for stroke in presses.iter().flatten() {
                // `cmd` is the Mac's. Omarchy has no key for it but Super.
                let cmd = matches!(stroke, Stroke::Key { mods, .. } if mods.cmd);
                assert!(!cmd, "{}: {:?}", entry.keys, entry.command);
                let insert = entry.mode.overlaps(Mode::Insert) && entry.scope != Scope::Prompt;
                assert!(
                    !(insert && vims.contains(stroke)),
                    "{} of {:?} would be taken from the text in insert mode",
                    entry.keys,
                    entry.command
                );
            }
        }
        // The Mac's layout names no Omarchy letters' modifier by mistake.
        for binding in BINDINGS {
            for chord in binding.mac.chords {
                assert!(!chord.keys.contains("super"), "{}", chord.keys);
            }
        }
    }

    #[test]
    fn a_key_written_beside_a_command_is_another_commands_own() {
        let keymap = Keymap::default();
        for layout in Layout::ALL {
            let own = entries(&keymap, layout);
            for binding in keymap.bindings() {
                for chord in binding.bound(layout).chords {
                    if chord.kind != Kind::Via {
                        continue;
                    }
                    let spelled = spell(chord.keys).expect("a spelling that reads");
                    let found = own.iter().any(|entry| {
                        entry.command != binding.command
                            && binding
                                .scopes
                                .iter()
                                .all(|scope| scope.alongside(entry.scope))
                            && clash(layout, &entry.spelled, &spelled)
                    });
                    assert!(found, "{layout:?}: {} of {:?}", chord.keys, binding.command);
                }
            }
        }
    }

    #[test]
    fn each_layout_writes_a_key_its_own_way() {
        let cases = [
            ("cmd+shift+f", "⇧⌘F", "Ctrl+Shift+F"),
            ("cmd+alt+backspace", "⌥⌘⌫", "Ctrl+Alt+Backspace"),
            ("cmd+enter", "⌘↩", "Ctrl+Enter"),
            ("cmd+shift+enter", "⇧⌘↩", "Ctrl+Shift+Enter"),
            ("ctrl+space", "⌃Space", "Ctrl+Space"),
            ("shift+tab", "⇧⇥", "Shift+Tab"),
            ("cmd+alt+down", "⌥⌘↓", "Ctrl+Alt+Down"),
            ("cmd+1..9", "⌘1…9", "Ctrl+1…9"),
            ("cmd+,", "⌘,", "Ctrl+,"),
            ("cmd+'", "⌘'", "Ctrl+'"),
            ("cmd+.", "⌘.", "Ctrl+."),
            ("enter", "↩", "Enter"),
            ("backspace", "⌫", "Backspace"),
            ("tab", "⇥", "Tab"),
            ("esc", "esc", "Esc"),
            ("f2", "F2", "F2"),
            ("?", "?", "?"),
        ];
        for (keys, mac, windows) in cases {
            assert_eq!(written(Layout::Mac, keys), mac, "{keys}");
            assert_eq!(written(Layout::Windows, keys), windows, "{keys}");
        }
    }

    #[test]
    fn omarchy_writes_a_key_as_it_is_spelled() {
        for binding in BINDINGS {
            for chord in binding.omarchy.chords {
                assert_eq!(written(Layout::Omarchy, chord.keys), chord.keys);
            }
        }
        // And never the way another layout or an older label had it.
        let keymap = Keymap::default();
        for command in Command::ALL {
            let label = keymap.label(Layout::Omarchy, *command);
            for wrong in ["Enter", "C-", "Ctrl", "⌘", "↩"] {
                assert!(!label.contains(wrong), "{command:?}: {label}");
            }
        }
    }

    #[test]
    fn a_command_is_labelled_by_its_keys_in_the_layout() {
        let keymap = Keymap::default();
        let label = |layout, command| keymap.label(layout, command);
        assert_eq!(label(Layout::Mac, Command::Connections), "⌘O");
        assert_eq!(label(Layout::Windows, Command::Connections), "Ctrl+O");
        assert_eq!(label(Layout::Omarchy, Command::Connections), "ctrl+shift+c");
        assert_eq!(label(Layout::Omarchy, Command::SaveChanges), ":w, ctrl+s");
        assert_eq!(
            label(Layout::Omarchy, Command::OpenInspector),
            "enter, ctrl+l"
        );
        assert_eq!(label(Layout::Omarchy, Command::MoveRow), "j/k");
        assert_eq!(label(Layout::Mac, Command::EditCell), "↩, F2");
        // A key that is only claimed is not offered: the editor has no
        // normal mode for `:wq` yet.
        assert_eq!(
            label(Layout::Omarchy, Command::ApplyLargeEditor),
            "ctrl+enter"
        );
        // A command that is not built is listed with the keys it will have.
        assert_eq!(label(Layout::Mac, Command::Rollback), "⌥⌘⌫");
        assert_eq!(label(Layout::Omarchy, Command::DuplicateRow), "yy p");
        // None on purpose.
        assert_eq!(label(Layout::Mac, Command::SwitchTab), "");
        assert_eq!(keymap.key(Layout::Omarchy, Command::SaveChanges), ":w");
    }

    #[test]
    fn a_command_that_is_not_built_answers_no_key() {
        let keymap = Keymap::default();
        for command in Command::ALL {
            if command.info().built {
                continue;
            }
            for layout in Layout::ALL {
                for scope in Scope::ALL {
                    let mut chords = keymap.chords(layout, *command, scope);
                    assert_eq!(chords.next(), None, "{command:?}");
                }
            }
        }
        // And one that is built does not answer a key it only claims.
        let mut large = keymap.chords(
            Layout::Omarchy,
            Command::ApplyLargeEditor,
            Scope::CellEditor,
        );
        assert_eq!(large.next(), Some("ctrl+enter"));
        assert_eq!(large.next(), None);
    }

    #[test]
    fn the_settled_keys_are_the_ones_bound() {
        let keymap = Keymap::default();
        let mac = |command| keymap.label(Layout::Mac, command);
        let omarchy = |command| keymap.label(Layout::Omarchy, command);
        // Throwing changes away is one chord on the Mac, and the query's
        // cancel is another.
        assert_eq!(mac(Command::DiscardChanges), "⌥⌘⌫");
        assert_eq!(mac(Command::Rollback), "⌥⌘⌫");
        assert_eq!(mac(Command::CancelQuery), "⌘.");
        assert_eq!(mac(Command::PasteCells), "⌘V");
        assert_eq!(mac(Command::PasteNewRows), "⇧⌘V");
        assert_eq!(mac(Command::SetDefault), "⌘'");
        assert_eq!(mac(Command::NextError), "⌥⌘↓");
        assert_eq!(mac(Command::PreviousError), "⌥⌘↑");
        assert_eq!(mac(Command::Format), "⇧⌘F");
        assert_eq!(mac(Command::Explain), "⌘E");
        assert_eq!(mac(Command::ComparePreviousRun), "⌥⌘E");
        assert_eq!(mac(Command::GoToConnectionWindow), "⌘1…9");
        assert_eq!(omarchy(Command::GoToConnectionWindow), "ctrl+shift+1..9");
        assert_eq!(omarchy(Command::EditCell), "i, enter");
        assert_eq!(omarchy(Command::ReplaceCell), "cc, s");
        assert_eq!(omarchy(Command::PasteNewRows), "\"+p, ctrl+shift+v");
        assert_eq!(omarchy(Command::DuplicateConnection), "yy p");
        assert_eq!(omarchy(Command::DuplicateRow), "yy p");
        assert_eq!(omarchy(Command::DiscardChanges), ":e!");
        assert_eq!(omarchy(Command::ReviewSql), ":diff");
        assert_eq!(omarchy(Command::History), ":history");
        assert_eq!(omarchy(Command::ReadWriteTab), ":rw");
        assert_eq!(omarchy(Command::ReadOnlyTab), ":ro");
        assert_eq!(omarchy(Command::Format), "=, ctrl+shift+f");
        assert_eq!(omarchy(Command::BooleanCycle), "space");
        assert_eq!(omarchy(Command::StepUp), "ctrl+a");
        assert_eq!(omarchy(Command::StepDown), "ctrl+x");
        assert_eq!(omarchy(Command::NextError), "]e");
        assert_eq!(omarchy(Command::Reload), "R");
        assert_eq!(omarchy(Command::Explain), "ctrl+e");
        assert_eq!(omarchy(Command::CancelQuery), "ctrl+c");
    }
    #[test]
    fn typed_characters_are_read_one_after_another() {
        let keymap = Keymap::default();
        let nothing = |_: When| false;
        let omarchy = |scope, typed| keymap.typed(Layout::Omarchy, scope, &nothing, typed);
        use Typed::{Claimed, Nothing, Waiting};
        let command = Typed::Command;
        // A row is duplicated with three letters, and that is not built.
        assert_eq!(omarchy(Scope::Grid, "y"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "yy"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "yyp"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "yyP"), Claimed);
        // A connection is, with the same three.
        assert_eq!(
            omarchy(Scope::Connections, "yyp"),
            command(Command::DuplicateConnection)
        );
        assert_eq!(
            omarchy(Scope::Connections, "dd"),
            command(Command::DeleteConnection)
        );
        assert_eq!(
            omarchy(Scope::Connections, "n"),
            command(Command::NewConnection)
        );
        assert_eq!(omarchy(Scope::Grid, "d"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "dd"), command(Command::DeleteRow));
        assert_eq!(omarchy(Scope::Grid, "i"), command(Command::EditCell));
        assert_eq!(omarchy(Scope::Grid, "s"), command(Command::ReplaceCell));
        assert_eq!(omarchy(Scope::Grid, "cc"), command(Command::ReplaceCell));
        assert_eq!(omarchy(Scope::Grid, "x"), command(Command::SetNull));
        assert_eq!(
            omarchy(Scope::Grid, "gd"),
            command(Command::OpenReferencedRow)
        );
        assert_eq!(omarchy(Scope::Grid, "gs"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "vy"), command(Command::CopyCells));
        assert_eq!(omarchy(Scope::Grid, "vp"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "\"+p"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "D"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "R"), command(Command::Reload));
        assert_eq!(omarchy(Scope::Sidebar, "R"), command(Command::Reload));
        // The slash is the sidebar's filter there and the WHERE line here.
        assert_eq!(omarchy(Scope::Sidebar, "/"), command(Command::FindObject));
        assert_eq!(omarchy(Scope::Grid, "/"), command(Command::WhereFilter));
        // A global letter is read in every scope but a prompt's.
        assert_eq!(omarchy(Scope::Grid, "?"), command(Command::Help));
        assert_eq!(omarchy(Scope::Prompt, "?"), Nothing);
        // The brackets step rows from the inspector, and begin other keys
        // on the grid.
        assert_eq!(omarchy(Scope::Inspector, "]"), command(Command::NextRow));
        assert_eq!(omarchy(Scope::Grid, "]"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "]e"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "q"), Nothing);
        // The letters of the booleans are nobody's.
        assert_eq!(omarchy(Scope::Grid, "f"), Nothing);
        assert_eq!(omarchy(Scope::Grid, "t"), Nothing);
        assert_eq!(omarchy(Scope::Sidebar, "t"), command(Command::ToggleTree));
        // A key that needs something on screen is nobody's without it.
        assert_eq!(omarchy(Scope::Grid, "Y"), Nothing);
        let review = |when: When| when == When::Review;
        assert_eq!(
            keymap.typed(Layout::Omarchy, Scope::Grid, &review, "Y"),
            command(Command::CopyReviewSql)
        );
        let leaving = |when: When| when == When::Leaving;
        assert_eq!(
            keymap.typed(Layout::Omarchy, Scope::Prompt, &leaving, "w"),
            command(Command::LeaveWrite)
        );
        // The other layouts have no letters but the list's.
        for layout in [Layout::Mac, Layout::Windows] {
            assert_eq!(
                keymap.typed(layout, Scope::Grid, &nothing, "?"),
                command(Command::Help)
            );
            assert_eq!(keymap.typed(layout, Scope::Grid, &nothing, "x"), Nothing);
        }
    }

    #[test]
    fn a_line_of_the_prompt_is_read_in_its_scope() {
        let keymap = Keymap::default();
        let nothing = |_: When| false;
        let ex = |scope, line| keymap.ex(Layout::Omarchy, scope, &nothing, line);
        use Typed::{Claimed, Nothing};
        let command = Typed::Command;
        assert_eq!(ex(Scope::Grid, "w"), command(Command::SaveChanges));
        assert_eq!(ex(Scope::Grid, "e!"), command(Command::DiscardChanges));
        assert_eq!(ex(Scope::Grid, "diff"), command(Command::ReviewSql));
        assert_eq!(ex(Scope::SqlEditor, "rw"), command(Command::ReadWriteTab));
        assert_eq!(ex(Scope::SqlEditor, "ro"), command(Command::ReadOnlyTab));
        // Not built yet: claimed.
        for line in ["commit", "rollback", "history", "42"] {
            assert_eq!(ex(Scope::SqlEditor, line), Claimed, "{line}");
        }
        assert_eq!(ex(Scope::Grid, "keys default"), Claimed);
        // Each in its own scope only, and `:w` never anything but a save.
        assert_eq!(ex(Scope::Grid, "rw"), Nothing);
        assert_eq!(ex(Scope::SqlEditor, "w"), Nothing);
        assert_eq!(ex(Scope::Grid, "wq"), Nothing);
        assert_eq!(ex(Scope::Grid, "q!"), Nothing);
        assert_eq!(ex(Scope::Grid, "W"), Nothing);
        assert_eq!(ex(Scope::Grid, ""), Nothing);
        // The floating editor's own, once it has a normal mode.
        let large = |when: When| when == When::LargeEditor;
        assert_eq!(
            keymap.ex(Layout::Omarchy, Scope::CellEditor, &large, "wq"),
            Claimed
        );
        assert_eq!(
            keymap.ex(Layout::Omarchy, Scope::CellEditor, &large, "q!"),
            Claimed
        );
        // The Mac has no prompt.
        assert_eq!(keymap.ex(Layout::Mac, Scope::Grid, &nothing, "w"), Nothing);
    }
    #[test]
    fn every_command_has_a_name_of_its_own() {
        let mut names: Vec<&str> = Vec::new();
        for command in Command::ALL {
            let name = command.info().name;
            assert!(!names.contains(&name), "{name} names two commands");
            names.push(name);
        }
    }

    #[test]
    fn a_key_types_the_character_its_spelling_names() {
        for char in ('a'..='z').chain('0'..='9') {
            let key = key_of(&char.to_string()).expect("a key");
            assert_eq!(char_of(key, false), Some(char));
        }
        for (mark, key) in MARKS {
            assert_eq!(char_of(*key, false), Some(*mark));
        }
        assert_eq!(char_of(Key::R, true), Some('R'));
        // What Shift makes of a digit or a mark depends on the keyboard.
        assert_eq!(char_of(Key::Num1, true), None);
        assert_eq!(char_of(Key::Slash, true), None);
        assert_eq!(char_of(Key::Enter, false), None);
        assert_eq!(char_of(Key::F2, false), None);
    }

    #[test]
    fn a_chord_answers_in_the_mode_the_keymap_gives_it() {
        let keymap = Keymap::default();
        let now = |layout, command, scope, normal| {
            keymap
                .chords_now(layout, command, scope, normal)
                .collect::<Vec<_>>()
        };
        use Layout::{Mac, Omarchy};
        // Omarchy closes a tab in normal mode only: in a text field the
        // chord deletes a word. The Mac's closes it from anywhere.
        assert_eq!(
            now(Omarchy, Command::CloseTab, Scope::Global, true),
            ["ctrl+w"]
        );
        assert!(now(Omarchy, Command::CloseTab, Scope::Global, false).is_empty());
        assert_eq!(now(Mac, Command::CloseTab, Scope::Global, false), ["cmd+w"]);
        // The Mac sets NULL on a cell, not in its editor.
        assert_eq!(
            now(Mac, Command::SetNull, Scope::Grid, true),
            ["cmd+backspace"]
        );
        assert!(now(Mac, Command::SetNull, Scope::Grid, false).is_empty());
        // A save is read while a cell is typed in. Its line of the prompt
        // is not: a colon is text there.
        assert_eq!(
            now(Omarchy, Command::SaveChanges, Scope::Grid, false),
            ["ctrl+s"]
        );
        assert_eq!(
            now(Omarchy, Command::SaveChanges, Scope::Grid, true),
            [":w", "ctrl+s"]
        );
        // Enter opens the inspector from the rows, and is the field's own.
        assert_eq!(
            now(Omarchy, Command::OpenInspector, Scope::Grid, true),
            ["enter"]
        );
        assert!(now(Omarchy, Command::OpenInspector, Scope::Grid, false).is_empty());
        // A run is asked for from the text and from outside it.
        for normal in [true, false] {
            assert_eq!(
                now(Omarchy, Command::RunStatement, Scope::SqlEditor, normal),
                ["ctrl+enter"]
            );
        }
        // What the Mac and Windows read from wherever the keyboard is, as
        // they did before the keymap: a reload, a page, the inspector, a
        // cancel. Omarchy's cancel on a table is normal mode's, because
        // its chord drops the edit of an open editor.
        for command in [
            Command::Reload,
            Command::NextPage,
            Command::ToggleInspector,
            Command::OpenInspector,
            Command::AddRow,
            Command::CancelQuery,
        ] {
            assert_eq!(
                now(Mac, command, Scope::Grid, false).len(),
                1,
                "{command:?}"
            );
        }
        assert!(now(Omarchy, Command::CancelQuery, Scope::Grid, false).is_empty());
        assert_eq!(
            now(Omarchy, Command::CancelQuery, Scope::SqlEditor, false),
            ["ctrl+c"]
        );
        // The panes are normal mode's: in a field Ctrl+H is a backspace.
        assert!(now(Omarchy, Command::PaneLeft, Scope::Global, false).is_empty());
    }

    #[test]
    fn keys_are_written_together_and_for_one_digit() {
        let keymap = Keymap::default();
        let together = |layout, commands: &[Command]| keymap.together(layout, commands);
        assert_eq!(
            together(Layout::Omarchy, &[Command::PreviousRow, Command::NextRow]),
            "[ ]"
        );
        assert_eq!(
            together(
                Layout::Omarchy,
                &[Command::NextCompletion, Command::PreviousCompletion]
            ),
            "ctrl+n/p"
        );
        assert_eq!(
            together(Layout::Omarchy, &[Command::MoveRow, Command::MoveColumn]),
            "j/k h/l"
        );
        assert_eq!(together(Layout::Mac, &[Command::SwitchTab]), "");
        let digit =
            |layout, digit| keymap.digit_label(layout, Command::GoToConnectionWindow, digit);
        assert_eq!(digit(Layout::Mac, 2), "⌘2");
        assert_eq!(digit(Layout::Windows, 2), "Ctrl+2");
        assert_eq!(digit(Layout::Omarchy, 2), "ctrl+shift+2");
        assert_eq!(digit(Layout::Omarchy, 10), "");
        assert_eq!(keymap.digit_label(Layout::Mac, Command::Settings, 2), "");
    }
}
