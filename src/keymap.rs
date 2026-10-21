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
//!
//! The person's changes from `settings.toml` are laid over these defaults
//! (`overrides.rs`): the keymap in force is [`Keymap::with`] them.

use egui::Key;

use crate::theme::{Look, Typeface};

mod overrides;
pub use overrides::{Bind, Line, Never, Overrides, Refused, Unbind, Why, intern, refuse_key};

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
        } else if look.faces.typeface == Typeface::Plex {
            Self::Mac
        } else {
            Self::Windows
        }
    }
}

/// Where a binding holds: what has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// The Agents tab.
    Agents,
    /// The start screen: a connection's window with no tab open.
    Start,
    /// The History tab.
    History,
}

impl Scope {
    pub const ALL: [Scope; 12] = [
        Scope::Global,
        Scope::Connections,
        Scope::Sidebar,
        Scope::Grid,
        Scope::CellEditor,
        Scope::Inspector,
        Scope::SqlEditor,
        Scope::Results,
        Scope::Prompt,
        Scope::Agents,
        Scope::Start,
        Scope::History,
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
            Self::Agents => "agents",
            Self::Start => "start",
            Self::History => "history",
        }
    }

    /// The scope's name as `layout` shows it, in the Scope column of the
    /// Keyboard settings: the brief's names where it gives one. English:
    /// the caller translates. A prompt's keys are not listed.
    pub fn name(self, layout: Layout) -> &'static str {
        match layout {
            Layout::Mac | Layout::Windows => match self {
                Self::Global => "Global",
                Self::Connections => "Connections",
                Self::Sidebar => "Sidebar",
                Self::Grid => "Table",
                Self::CellEditor => "Cell",
                Self::Inspector => "Inspector",
                Self::SqlEditor => "Editor",
                Self::Results => "Results",
                Self::Prompt => "Prompt",
                Self::Agents => "Agents",
                Self::Start => "Start screen",
                Self::History => "History",
            },
            // As the terminal look writes its scopes: lower case, and the
            // global one by its own word, as its board does.
            Layout::Omarchy => match self {
                Self::Global => "global",
                other => other.table(layout),
            },
        }
    }

    /// The table of `settings.toml` that holds the scope's keys
    /// (`[keys.<table>]`), as `layout` names it. Every look's names are
    /// read in every look ([`Scope::from_table`]); a table the app adds
    /// has the name of the look it runs in.
    pub fn table(self, layout: Layout) -> &'static str {
        let omarchy = layout == Layout::Omarchy;
        match self {
            Self::Global if omarchy => "window",
            Self::Global => "global",
            Self::Connections => "connections",
            Self::Sidebar if omarchy => "navigate",
            Self::Sidebar => "sidebar",
            Self::Grid if omarchy => "grid",
            Self::Grid => "table",
            Self::CellEditor if omarchy => "edit",
            Self::CellEditor => "cell",
            Self::Inspector => "inspector",
            Self::SqlEditor if omarchy => "sql",
            Self::SqlEditor => "editor",
            Self::Results => "results",
            Self::Prompt => "prompt",
            Self::Agents => "agents",
            Self::Start => "start",
            Self::History => "history",
        }
    }

    /// The scope a table of `settings.toml` names: by either look's name,
    /// or by the scope's id. None for a prompt's, whose keys are its own.
    pub fn from_table(name: &str) -> Option<Scope> {
        Self::ALL
            .into_iter()
            .filter(|scope| *scope != Self::Prompt)
            .find(|scope| {
                name == scope.id()
                    || Layout::ALL
                        .into_iter()
                        .any(|layout| name == scope.table(layout))
            })
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// The table on screen holds pending changes.
    Pending,
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
    /// The results offer Show all: a cut result shows.
    RowsCut,
    /// The results offer Run without timeout: a run the timeout stopped
    /// shows its cancel.
    TimedOut,
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
    /// The box of what agents may do on a connection, in the terminal look.
    AgentAccess,
    /// The question about leaving with pending changes.
    Leaving,
    /// The cursor's row is a new one, not saved yet: the row panel is its
    /// form.
    NewRow,
    /// The list of a foreign key cell's picker is open under its field.
    KeyPicker,
    /// A visual selection of rows is on, in the terminal look.
    Visual,
    /// The chooser of a table's label column is open: beside a picker's
    /// list, which gives it the keys.
    LabelChooser,
    /// The Structure view of a table shows, in place of its rows: the
    /// grid's keys that act on a row have nothing to act on, and the
    /// view's own act on the line of a column.
    Structure,
    /// The Agents tab shows its Log view: its keys come before the
    /// activity's.
    AgentsLog,
    /// The History tab asks whether to clear the connection's history.
    ClearingHistory,
    /// The start screen's search is on: its field has the keyboard. The
    /// keys that move among the matches, open one and clear the search are
    /// read then, and only then.
    StartSearch,
}

/// The mode a chord holds in: with the keyboard in a text field (insert),
/// or out of one (normal). Typed letters and `:` commands are normal mode's
/// whatever this says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    ($($name:ident = $id:literal),* $(,)?) => {
        /// Everything a key can ask for.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Command {
            $($name),*
        }

        impl Command {
            pub const ALL: &'static [Command] = &[$(Command::$name),*];

            /// The command's name in `settings.toml`: snake case, and never
            /// changed once a version has written it. A title is what a
            /// menu says, and can change; this cannot.
            pub fn id(self) -> &'static str {
                match self {
                    $(Self::$name => $id),*
                }
            }
        }
    };
}

impl Command {
    /// The command a file names by its id.
    pub fn from_id(id: &str) -> Option<Command> {
        Self::ALL.iter().copied().find(|command| command.id() == id)
    }
}

commands! {
    // Window and connections.
    Connections = "connections",
    GoToConnectionWindow = "go_to_connection_window",
    SwitchDatabase = "switch_database",
    Settings = "settings",
    LargerText = "larger_text",
    SmallerText = "smaller_text",
    DefaultTextSize = "default_text_size",
    CloseConnection = "close_connection",
    NextConnection = "next_connection",
    PreviousConnection = "previous_connection",
    CancelConnecting = "cancel_connecting",
    NewConnection = "new_connection",
    EditConnection = "edit_connection",
    DuplicateConnection = "duplicate_connection",
    DeleteConnection = "delete_connection",
    Connect = "connect",
    ConnectAgain = "connect_again",
    MoveInConnections = "move_in_connections",
    FilterConnections = "filter_connections",
    // Navigation.
    FindObject = "find_object",
    FilterObjects = "filter_objects",
    NewSqlTab = "new_sql_tab",
    ReopenClosedTab = "reopen_closed_tab",
    ShowAgents = "show_agents",
    CopyAgentCommand = "copy_agent_command",
    TurnAgentsOn = "turn_agents_on",
    MoveInAgentSessions = "move_in_agent_sessions",
    StopAgentSession = "stop_agent_session",
    StopAgentsHere = "stop_agents_here",
    ShowAgentsElsewhere = "show_agents_elsewhere",
    ShowAgentsLog = "show_agents_log",
    ToggleAgentsLogScope = "toggle_agents_log_scope",
    FilterAgentsLog = "filter_agents_log",
    CloseAgentsLogPane = "close_agents_log_pane",
    ShowAgentsLogDetails = "show_agents_log_details",
    OpenAgentsLogCall = "open_agents_log_call",
    ExportAgentsLog = "export_agents_log",
    AllowAgents = "allow_agents",
    ShowAgentSettings = "show_agent_settings",
    ChangeAgentAccess = "change_agent_access",
    MoveInAgentLevels = "move_in_agent_levels",
    ShowAgentsFromBox = "show_agents_from_box",
    OpenRecent = "open_recent",
    StartSearchFocus = "start_search_focus",
    StartSearchMove = "start_search_move",
    StartSearchOpen = "start_search_open",
    StartSearchOpenInEditor = "start_search_open_in_editor",
    StartSearchClear = "start_search_clear",
    ToggleSidebar = "toggle_sidebar",
    PaneLeft = "pane_left",
    PaneDown = "pane_down",
    PaneUp = "pane_up",
    PaneRight = "pane_right",
    NextPart = "next_part",
    PreviousPart = "previous_part",
    SwitchTab = "switch_tab",
    CloseTab = "close_tab",
    PreviousTab = "previous_tab",
    NextTab = "next_tab",
    MoveInTree = "move_in_tree",
    OpenObject = "open_object",
    ToggleTree = "toggle_tree",
    Reload = "reload",
    FilterBar = "filter_bar",
    WhereFilter = "where_filter",
    PreviousPage = "previous_page",
    NextPage = "next_page",
    Help = "help",
    KeysDefault = "keys_default",
    KeysCheatsheet = "keys_cheatsheet",
    // Grid and editing.
    MoveRow = "move_row",
    MoveColumn = "move_column",
    MovePage = "move_page",
    ExtendUp = "extend_up",
    ExtendDown = "extend_down",
    OpenInspector = "open_inspector",
    ToggleInspector = "toggle_inspector",
    CloseInspector = "close_inspector",
    BackToGrid = "back_to_grid",
    MoveField = "move_field",
    PreviousRow = "previous_row",
    NextRow = "next_row",
    FoldDocuments = "fold_documents",
    EditCell = "edit_cell",
    ReplaceCell = "replace_cell",
    CommitDown = "commit_down",
    CommitNext = "commit_next",
    CommitPrevious = "commit_previous",
    KeepEdit = "keep_edit",
    NextPick = "next_pick",
    PreviousPick = "previous_pick",
    PickPageDown = "pick_page_down",
    PickPageUp = "pick_page_up",
    ClosePicker = "close_picker",
    ChangeLabelColumn = "change_label_column",
    CancelEdit = "cancel_edit",
    OpenLargeEditor = "open_large_editor",
    ApplyLargeEditor = "apply_large_editor",
    KeepLargeEditor = "keep_large_editor",
    CancelLargeEditor = "cancel_large_editor",
    OpenInEditor = "open_in_editor",
    SetNull = "set_null",
    SetDefault = "set_default",
    UndoCell = "undo_cell",
    RedoCell = "redo_cell",
    CopyCells = "copy_cells",
    CopyRow = "copy_row",
    PasteCells = "paste_cells",
    PasteNewRows = "paste_new_rows",
    AddRow = "add_row",
    AddRowAbove = "add_row_above",
    AddRowAtEnd = "add_row_at_end",
    DuplicateRow = "duplicate_row",
    YankRow = "yank_row",
    PutRow = "put_row",
    PutRowAbove = "put_row_above",
    DeleteRow = "delete_row",
    SelectRows = "select_rows",
    DeleteSelection = "delete_selection",
    LeaveVisual = "leave_visual",
    OpenReferencedRow = "open_referenced_row",
    ShowSavedRow = "show_saved_row",
    MoveInStructure = "move_in_structure",
    OpenLabelChooser = "open_label_chooser",
    UseAsLabel = "use_as_label",
    LabelAutomatic = "label_automatic",
    LabelKeysOnly = "label_keys_only",
    LabelColumn = "label_column",
    ReviewSql = "review_sql",
    NextChange = "next_change",
    CopyReviewSql = "copy_review_sql",
    CloseReview = "close_review",
    DismissNote = "dismiss_note",
    SaveChanges = "save_changes",
    DiscardChanges = "discard_changes",
    AskDiscard = "ask_discard",
    NextError = "next_error",
    PreviousError = "previous_error",
    BooleanCycle = "boolean_cycle",
    StepUp = "step_up",
    StepDown = "step_down",
    Format = "format",
    // SQL editor.
    IndentLines = "indent_lines",
    OutdentLines = "outdent_lines",
    RunStatement = "run_statement",
    RunAll = "run_all",
    Explain = "explain",
    ExplainAnalyze = "explain_analyze",
    ComparePreviousRun = "compare_previous_run",
    CancelQuery = "cancel_query",
    FocusResults = "focus_results",
    LeaveEditor = "leave_editor",
    Complete = "complete",
    InsertCompletion = "insert_completion",
    NextCompletion = "next_completion",
    PreviousCompletion = "previous_completion",
    CloseCompletion = "close_completion",
    GoToLine = "go_to_line",
    History = "history",
    ClearHistory = "clear_history",
    MoveInHistory = "move_in_history",
    RunHistoryAgain = "run_history_again",
    OpenHistoryEntry = "open_history_entry",
    ShowHistoryCall = "show_history_call",
    ReapplyHistoryFilter = "reapply_history_filter",
    CopyHistorySql = "copy_history_sql",
    FilterHistory = "filter_history",
    CloseHistoryLine = "close_history_line",
    ConfirmClearHistory = "confirm_clear_history",
    KeepHistory = "keep_history",
    ReadOnlyTab = "read_only_tab",
    ReadWriteTab = "read_write_tab",
    ToggleSqlMode = "toggle_sql_mode",
    Commit = "commit",
    Rollback = "rollback",
    NextPlanTip = "next_plan_tip",
    EditRefusedConnection = "edit_refused_connection",
    AllowRefusedWrite = "allow_refused_write",
    ShowAllRows = "show_all_rows",
    RunWithoutTimeout = "run_without_timeout",
    // AI assistant.
    AskAi = "ask_ai",
    AcceptAi = "accept_ai",
    AcceptAiAndRun = "accept_ai_and_run",
    DiscardAi = "discard_ai",
    // Prompts and dialogs.
    ConfirmProductionWrite = "confirm_production_write",
    CancelProductionWrite = "cancel_production_write",
    ScrollStatements = "scroll_statements",
    LeaveWrite = "leave_write",
    LeaveDiscard = "leave_discard",
    LeaveStay = "leave_stay",
    ConflictOverwrite = "conflict_overwrite",
    ConflictUseServer = "conflict_use_server",
    ConflictKeepMine = "conflict_keep_mine",
    ConflictDiscard = "conflict_discard",
    FormPasteUrl = "form_paste_url",
    FormNextField = "form_next_field",
    FormTest = "form_test",
    FormSave = "form_save",
    FormSaveAndConnect = "form_save_and_connect",
    FormForgetPassword = "form_forget_password",
    FormCancel = "form_cancel",
    FormTableMove = "form_table_move",
    SettingsMove = "settings_move",
    SettingsChange = "settings_change",
    SettingsToggle = "settings_toggle",
    SettingsOpenFile = "settings_open_file",
    SettingsReset = "settings_reset",
    SettingsClose = "settings_close",
    SettingsPane = "settings_pane",
    SettingsOpenRow = "settings_open_row",
    SettingsCopyCommand = "settings_copy_command",
    SettingsDeleteHistory = "settings_delete_history",
    SettingsKeepHistory = "settings_keep_history",
    SettingsUnbind = "settings_unbind",
    SettingsFilter = "settings_filter",
    SettingsFindKey = "settings_find_key",
    SettingsSwap = "settings_swap",
    QuickOpenMove = "quick_open_move",
    QuickOpenPick = "quick_open_pick",
    QuickOpenPickInEditor = "quick_open_pick_in_editor",
    QuickOpenScope = "quick_open_scope",
    QuickOpenCommands = "quick_open_commands",
    QuickOpenNumber = "quick_open_number",
    QuickOpenClose = "quick_open_close",
    NextLabel = "next_label",
    PreviousLabel = "previous_label",
    ChooseLabel = "choose_label",
    CloseLabelChooser = "close_label_chooser",
    AddLabelPart = "add_label_part",
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

/// An agents' command's info: answered only where agents can reach the
/// app (`endpoint::AVAILABLE`).
const fn agents(info: Info) -> Info {
    match crate::endpoint::AVAILABLE {
        true => info,
        false => info.reserved(),
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
            C::SwitchDatabase => info("Switch database", "Switch database", Window),
            C::Settings => info("Settings", "Settings", Window),
            C::LargerText => info("Larger text", "", Window),
            C::SmallerText => info("Smaller text", "", Window),
            C::DefaultTextSize => info("Default text size", "", Window),
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
            C::FilterObjects => info("Filter the sidebar's objects", "", Navigation),
            C::NewSqlTab => info("New SQL editor tab", "New SQL editor tab", Navigation),
            // Nothing reopens a closed tab yet: the start screen opens a
            // recent by its number.
            C::ReopenClosedTab => info("Reopen closed tabs", "", Navigation).reserved(),
            // Nothing of agents answers where they cannot reach the app.
            C::ShowAgents => agents(info("Show the Agents tab", "", Navigation)),
            C::CopyAgentCommand => {
                agents(info("Copy the command that connects an agent", "", Window))
            }
            C::TurnAgentsOn => agents(info("Turn the agent server on", "", Window)),
            C::MoveInAgentSessions => agents(info("Move in the agents' sessions", "", Window)),
            C::StopAgentSession => agents(info("Stop the selected agent's session", "", Window)),
            C::StopAgentsHere => agents(info(
                "Stop the agents here, and turn their access off",
                "",
                Window,
            )),
            C::ShowAgentsElsewhere => {
                agents(info("Show where the selected agent works", "", Window))
            }
            C::ShowAgentsLog => agents(info("Show the log, or what agents do now", "", Window)),
            C::AllowAgents => agents(info("Allow agents on this connection…", "", Window)),
            C::ShowAgentSettings => agents(info("Show the agents' settings", "", Window)),
            C::ChangeAgentAccess => agents(info(
                "Change what agents may do on the connection",
                "",
                Window,
            )),
            C::MoveInAgentLevels => agents(info("Move among what agents may do", "", Prompts)),
            C::ShowAgentsFromBox => agents(info("Show the connection's Agents tab", "", Prompts)),
            C::OpenRecent => info("Open a recent object", "", Navigation),
            C::StartSearchFocus => info("Search the start screen", "", Navigation),
            C::StartSearchMove => info("Move in the start screen's search", "", Navigation),
            C::StartSearchOpen => info("Open the start screen's search match", "", Navigation),
            C::StartSearchOpenInEditor => info(
                "Open the start screen's search match in a SQL editor",
                "",
                Navigation,
            ),
            C::StartSearchClear => info("Clear the start screen's search", "", Navigation),
            C::ToggleAgentsLogScope => agents(info(
                "Show every connection's calls in the log, or this one's",
                "",
                Window,
            )),
            C::FilterAgentsLog => agents(info(
                "Filter the log by agent, result or request",
                "",
                Window,
            )),
            C::CloseAgentsLogPane => agents(info(
                "Close the log's filters, and keep them, or the call's details",
                "",
                Window,
            )),
            C::ShowAgentsLogDetails => agents(info(
                "Show the selected call's details in the log, or close them",
                "",
                Window,
            )),
            C::OpenAgentsLogCall => agents(info(
                "Open the selected call's statement in a SQL editor",
                "",
                Window,
            )),
            C::ExportAgentsLog => agents(info(
                "Export the log's calls that pass its filters",
                "",
                Window,
            )),
            C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
            C::PaneLeft => info("Pane to the left", PANES, Navigation),
            C::PaneDown => info("Pane below", PANES, Navigation),
            C::PaneUp => info("Pane above", PANES, Navigation),
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
            C::KeysDefault => info("Write the full list of keys", "", Navigation),
            C::KeysCheatsheet => info("Write a cheat sheet of the keys", "", Navigation),
            C::MoveRow => info("Move by row", "", Editing),
            C::MoveColumn => info("Move by column", "", Editing),
            C::MovePage => info("Move by page", "", Editing),
            C::ExtendUp => info("Extend the selection up", "", Editing),
            C::ExtendDown => info("Extend the selection down", "", Editing),
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
            C::NextPick => info("Next row of a foreign key's list", "", Editing),
            C::PreviousPick => info("Previous row of a foreign key's list", "", Editing),
            C::PickPageDown => info("A page down in a foreign key's list", "", Editing),
            C::PickPageUp => info("A page up in a foreign key's list", "", Editing),
            C::ClosePicker => info("Close a foreign key's list", "", Editing),
            C::ChangeLabelColumn => info("Change label column", "", Editing),
            C::CancelEdit => info("Cancel cell edit", "Cancel cell edit", Editing),
            C::OpenLargeEditor => info("Open the large editor", "", Editing),
            C::ApplyLargeEditor => info("Apply large editor", LARGE, Editing),
            C::KeepLargeEditor => info("Leave the large editor and keep the text", "", Editing),
            C::CancelLargeEditor => info("Cancel large editor", LARGE, Editing),
            C::OpenInEditor => {
                info("Open value in $EDITOR", "Open value in $EDITOR", Editing).reserved()
            }
            C::SetNull => info("Set NULL", "Set NULL", Editing),
            C::SetDefault => info("Set DEFAULT", "Set DEFAULT", Editing),
            C::UndoCell => info("Undo one cell", UNDO, Editing),
            C::RedoCell => info("Redo one cell", UNDO, Editing).reserved(),
            C::CopyCells => info("Copy cell values", COPY, Editing),
            C::CopyRow => info("Copy row", "", Editing),
            C::PasteCells => info("Paste cell values", COPY, Editing).reserved(),
            C::PasteNewRows => info("Paste as new rows", "Paste as new rows", Editing).reserved(),
            C::AddRow => info("Add row", "Add row", Editing),
            C::AddRowAbove => info("Add row above", "Add row", Editing),
            C::AddRowAtEnd => info("Add row after the last", "Add row", Editing),
            C::DuplicateRow => info("Duplicate row", "Duplicate row", Editing),
            C::YankRow => info("Yank the row", "Duplicate row", Editing),
            C::PutRow => info(
                "Put a copy of the yanked row below",
                "Duplicate row",
                Editing,
            ),
            C::PutRowAbove => info(
                "Put a copy of the yanked row above",
                "Duplicate row",
                Editing,
            ),
            C::DeleteRow => info("Delete row", "Delete row", Editing),
            C::SelectRows => info("Select rows", "", Editing),
            C::DeleteSelection => info("Delete the selected rows", "", Editing),
            C::LeaveVisual => info("Leave the selection of rows", "", Editing),
            C::OpenReferencedRow => info("Open referenced row", "Open referenced row", Editing),
            C::ShowSavedRow => info(
                "Show saved row in sorted position",
                "Show saved row in sorted position",
                Editing,
            )
            .reserved(),
            C::MoveInStructure => info("Move by column in the Structure view", "", Editing),
            C::OpenLabelChooser => info("Choose the label for foreign keys", "", Editing),
            C::UseAsLabel => info("Use the column as the label", "", Editing),
            C::LabelAutomatic => info("Label foreign keys automatically", "", Editing),
            C::LabelKeysOnly => info("Label foreign keys by their keys alone", "", Editing),
            C::LabelColumn => info("Label foreign keys by a column", "", Editing),
            C::ReviewSql => info("Review pending SQL", "Review pending SQL", Editing),
            C::NextChange => info("Next change", "Review pending SQL", Editing).reserved(),
            C::CopyReviewSql => info("Copy the pending SQL", "", Editing),
            C::CloseReview => info("Close the pending SQL", "", Editing),
            C::DismissNote => info("Dismiss what the last save came to", "", Editing),
            C::SaveChanges => info("Save changes", "Save changes", Editing),
            C::DiscardChanges => info("Discard all pending", "Discard all pending", Editing),
            C::AskDiscard => info(
                "Discard all pending, asking first",
                "Discard all pending",
                Editing,
            ),
            C::NextError => info("Next error", ERRORS, Editing),
            C::PreviousError => info("Previous error", ERRORS, Editing),
            C::BooleanCycle => info("Boolean cycle", "Boolean cycle", Editing),
            C::StepUp => info("Step up", STEP, Editing).reserved(),
            C::StepDown => info("Step down", STEP, Editing).reserved(),
            C::Format => info("Format SQL or JSON", "Format SQL or JSON", Sql),
            C::IndentLines => info("Indent", "", Sql),
            C::OutdentLines => info("Outdent", "", Sql),
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
            C::FocusResults => info("Focus results", "Focus results", Sql),
            C::LeaveEditor => info("Leave the editor", "", Sql),
            C::Complete => info("Completion", "Completion", Sql),
            C::InsertCompletion => info("Insert the completion", "Completion", Sql),
            C::NextCompletion => info("Next completion", "Completion", Sql),
            C::PreviousCompletion => info("Previous completion", "Completion", Sql),
            C::CloseCompletion => info("Close the completion list", "", Sql),
            C::GoToLine => info("Go to line", "Go to line", Sql).reserved(),
            C::History => info("History", "History", Sql),
            C::ClearHistory => info("Clear this connection's history", "", Sql),
            C::MoveInHistory => info("Move in the history", "", Sql),
            C::RunHistoryAgain => info("Run the selected read again", "", Sql),
            C::OpenHistoryEntry => info("Open the selected statement in a SQL editor", "", Sql),
            C::ShowHistoryCall => info("Show the agent's call in the Agents Log", "", Sql),
            C::ReapplyHistoryFilter => info("Reapply the selected filter", "", Sql),
            C::CopyHistorySql => info("Copy the selected statement", "", Sql),
            C::FilterHistory => info("Filter the history", "", Sql),
            C::CloseHistoryLine => info("Close the history's filter line, and keep it", "", Sql),
            C::ConfirmClearHistory => info("Clear the history, as asked", "", Prompts),
            C::KeepHistory => info("Keep the history, as asked", "", Prompts),
            C::ReadOnlyTab => info("Read-only tab", MODE, Sql),
            C::ReadWriteTab => info("Read-write tab", MODE, Sql),
            C::ToggleSqlMode => info("Read-only or read-write runs", "", Sql),
            C::Commit => info("Commit", "Commit", Sql).reserved(),
            C::Rollback => info("Rollback", "Rollback", Sql).reserved(),
            C::NextPlanTip => info("Next plan tip", "Next plan tip", Sql).reserved(),
            C::EditRefusedConnection => info("Edit the connection that refused", "", Prompts),
            C::AllowRefusedWrite => info("Allow writes, or run again", "", Prompts),
            C::ShowAllRows => info("Show all rows", "", Prompts),
            C::RunWithoutTimeout => info("Run without timeout", "", Prompts),
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
            C::FormForgetPassword => info("Forget the saved password", "", Prompts),
            C::FormCancel => info("Cancel", "", Prompts),
            C::FormTableMove => info("Move in Add table…'s or Add column…'s list", "", Prompts),
            C::SettingsMove => info("Move in the settings", "", Prompts),
            C::SettingsChange => info("Change the option", "", Prompts),
            C::SettingsToggle => info("Toggle the option", "", Prompts),
            C::SettingsOpenFile => info("Open the settings file in $EDITOR", "", Prompts),
            C::SettingsReset => info("Reset the option", "", Prompts),
            C::SettingsClose => info("Close the settings", "", Prompts),
            C::SettingsPane => info(
                "Show the next or the previous pane of the settings",
                "",
                Prompts,
            ),
            C::SettingsOpenRow => info("Open what the settings' row shows", "", Prompts),
            C::SettingsCopyCommand => agents(info(
                "Copy the command that connects an agent, in the settings",
                "",
                Prompts,
            )),
            C::SettingsDeleteHistory => info(
                "Delete the query history kept, as the settings ask",
                "",
                Prompts,
            ),
            C::SettingsKeepHistory => {
                info("Keep the query history, as the settings ask", "", Prompts)
            }
            C::SettingsUnbind => info("Unbind the keyboard row's keys", "", Prompts),
            C::SettingsFilter => info("Filter the keyboard rows by name", "", Prompts),
            C::SettingsFindKey => info("Find the keyboard rows by a key", "", Prompts),
            C::SettingsSwap => info("Swap the keys, as the keyboard row asks", "", Prompts),
            C::QuickOpenMove => info("Move in quick open", "", Prompts),
            C::QuickOpenPick => info("Open the one selected", "", Prompts),
            C::QuickOpenPickInEditor => info("Open the one selected in a SQL editor", "", Prompts),
            C::QuickOpenScope => info("Search this connection or every open one", "", Prompts),
            C::QuickOpenCommands => info("List the commands alone", "", Prompts),
            C::QuickOpenNumber => info("Open a recent by its number", "", Prompts),
            C::QuickOpenClose => info("Close quick open", "", Prompts),
            C::NextLabel => info("Next line of the label chooser", "", Prompts),
            C::PreviousLabel => info("Previous line of the label chooser", "", Prompts),
            C::ChooseLabel => info("Choose the label column", "", Prompts),
            C::CloseLabelChooser => info("Close the label chooser", "", Prompts),
            C::AddLabelPart => {
                info("Add a column as the label's second part", "", Prompts).reserved()
            }
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
                | Scope::Results
                | Scope::Agents
                | Scope::Start
                | Scope::History => Mode::Normal,
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
/// The global scope and a prompt's: keys that hold under a dialog too.
const ANYWHERE: &[Scope] = &[Scope::Global, Scope::Prompt];
const CONNECTIONS: &[Scope] = &[Scope::Connections];
const SIDEBAR: &[Scope] = &[Scope::Sidebar];
const GRID: &[Scope] = &[Scope::Grid];
const CELL_EDITOR: &[Scope] = &[Scope::CellEditor];
const INSPECTOR: &[Scope] = &[Scope::Inspector];
const SQL_EDITOR: &[Scope] = &[Scope::SqlEditor];
const RESULTS: &[Scope] = &[Scope::Results];
const PROMPT: &[Scope] = &[Scope::Prompt];
const ROWS: &[Scope] = &[Scope::Grid, Scope::Results];
const AGENTS: &[Scope] = &[Scope::Agents];
const HISTORY: &[Scope] = &[Scope::History];
const TREE_AND_GRID: &[Scope] = &[Scope::Sidebar, Scope::Grid];
const START: &[Scope] = &[Scope::Start];
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
            // In a text field the chord is the field's copy: the only one
            // a cell's editor has, where Ctrl+C drops the edit.
            keys(&[k("ctrl+shift+c")]).normal(),
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
        // The text's size, under a dialog too: the Settings window shows
        // what it changes. The terminal look resets it from that window.
        bind(
            C::LargerText,
            ANYWHERE,
            keys(&[k("cmd+=")]),
            keys(&[k("ctrl+=")]),
        ),
        bind(
            C::SmallerText,
            ANYWHERE,
            keys(&[k("cmd+-")]),
            keys(&[k("ctrl+-")]),
        ),
        bind(C::DefaultTextSize, ANYWHERE, keys(&[k("cmd+0")]), NONE),
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
            keys(&[k("up"), k("down"), k("cmd+up"), k("cmd+down")]),
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        ),
        bind(C::FilterConnections, CONNECTIONS, NONE, keys(&[k("/")])),
        // Navigation.
        // Quick open, from anywhere in a connection's window. The Omarchy
        // `/` filters the sidebar's tree where it is.
        bind(
            C::FindObject,
            GLOBAL,
            keys(&[k("cmd+p")]),
            keys(&[k("ctrl+p")]),
        ),
        bind(C::FilterObjects, SIDEBAR, NONE, keys(&[k("/")])),
        bind(
            C::NewSqlTab,
            GLOBAL,
            keys(&[k("cmd+t")]),
            keys(&[k("ctrl+t")]),
        ),
        bind(C::ReopenClosedTab, GLOBAL, keys(&[k("cmd+shift+t")]), NONE),
        bind(
            C::ShowAgents,
            GLOBAL,
            keys(&[k("cmd+alt+a")]),
            keys(&[k(":agents")]),
        ),
        bind(C::ShowAgents, TREE_AND_GRID, NONE, keys(&[k("ga")])),
        bind(C::CopyAgentCommand, AGENTS, NONE, keys(&[k("y")])),
        bind(C::TurnAgentsOn, AGENTS, NONE, keys(&[k("o")])),
        bind(
            C::MoveInAgentSessions,
            AGENTS,
            NONE,
            keys(&[k("j"), k("k")]).shown("j/k"),
        ),
        bind(C::StopAgentSession, AGENTS, NONE, keys(&[k("S")])),
        bind(C::StopAgentsHere, AGENTS, NONE, keys(&[k("X")])),
        bind(C::ShowAgentsElsewhere, AGENTS, NONE, keys(&[k("enter")])),
        bind(C::ShowAgentsLog, GLOBAL, NONE, keys(&[k(":agents log")])),
        bind(C::ShowAgentsLog, AGENTS, NONE, keys(&[k("l")])),
        bind(C::ToggleAgentsLogScope, AGENTS, NONE, keys(&[k("L")])),
        bind(C::FilterAgentsLog, AGENTS, NONE, keys(&[k("/")])),
        bind(C::CloseAgentsLogPane, AGENTS, NONE, keys(&[k("esc")])),
        bind(C::ShowAgentsLogDetails, AGENTS, NONE, keys(&[k("enter")])).when(When::AgentsLog),
        bind(C::OpenAgentsLogCall, AGENTS, NONE, keys(&[k("o")])).when(When::AgentsLog),
        bind(C::ExportAgentsLog, AGENTS, NONE, keys(&[k("E")])).when(When::AgentsLog),
        bind(C::AllowAgents, AGENTS, NONE, keys(&[k("e")])),
        bind(C::ShowAgentSettings, AGENTS, NONE, keys(&[k("s")])),
        bind(C::ChangeAgentAccess, CONNECTIONS, NONE, keys(&[k("a")])),
        bind(
            C::MoveInAgentLevels,
            PROMPT,
            NONE,
            keys(&[k("j"), k("k")]).shown("j/k"),
        )
        .when(When::AgentAccess),
        bind(C::ShowAgentsFromBox, PROMPT, NONE, keys(&[k("t")])).when(When::AgentAccess),
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
        // The start screen's own keys, while it has the keyboard: a
        // numbered recent, and the letters of its actions. On the Mac the
        // screen's buttons carry the global keys.
        bind(C::OpenRecent, START, NONE, keys(&[k("1..9")])),
        bind(C::ReopenClosedTab, START, NONE, keys(&[k("u")])),
        bind(C::NewSqlTab, START, NONE, keys(&[k("t")])),
        bind(C::SwitchDatabase, START, NONE, keys(&[k("d")])),
        bind(C::EditConnection, START, NONE, keys(&[k("e")])),
        bind(C::ShowAgents, START, NONE, keys(&[k("a")])),
        // The start screen's search. On macOS the field takes the keyboard
        // when a character is typed on the screen, so it has no key of its
        // own; Omarchy's `/` gives it the keys. The keys that move among
        // the matches, open one and clear the search hold while its field
        // has the keyboard.
        bind(C::StartSearchFocus, START, NONE, keys(&[k("/")])),
        bind(
            C::StartSearchMove,
            START,
            keys(&[k("up"), k("down")]).shown("↑↓").any(),
            keys(&[k("ctrl+p"), k("ctrl+n")]).shown("ctrl+n/p").any(),
        )
        .when(When::StartSearch),
        bind(
            C::StartSearchOpen,
            START,
            keys(&[k("enter")]).any(),
            keys(&[k("enter")]).any(),
        )
        .when(When::StartSearch),
        bind(
            C::StartSearchOpenInEditor,
            START,
            keys(&[k("alt+enter")]).any(),
            NONE,
        )
        .when(When::StartSearch),
        bind(
            C::StartSearchClear,
            START,
            keys(&[k("esc")]).any(),
            keys(&[k("esc")]).any(),
        )
        .when(When::StartSearch),
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
                k("cmd+up"),
                k("cmd+down"),
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
        bind(
            C::KeysCheatsheet,
            GLOBAL,
            NONE,
            keys(&[k(":keys cheatsheet")]),
        ),
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
            keys(&[k("left"), k("right"), k("cmd+left"), k("cmd+right")]),
            keys(&[k("h"), k("l"), k("left"), k("right")]).shown("h/l"),
        ),
        // Mod with an arrow goes to the end it points at, as in a list
        // and a spreadsheet on the Mac. The terminal look has its own.
        bind(
            C::MovePage,
            ROWS,
            keys(&[
                k("pgup"),
                k("pgdn"),
                k("home"),
                k("end"),
                k("cmd+up"),
                k("cmd+down"),
            ]),
            keys(&[k("pgup"), k("pgdn"), k("home"), k("end")]),
        ),
        // A table's rows only: a result has one selected row.
        bind(C::ExtendUp, GRID, keys(&[k("shift+up")]), NONE),
        bind(C::ExtendDown, GRID, keys(&[k("shift+down")]), NONE),
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
        // From the rows too, while the inspector shows: its header offers
        // the brackets, and `j` and `k` are the rows' there. A bracket
        // also begins the keys of the errors and the changes (`]e`, `]c`).
        // With the inspector open the bracket is its own, at once, except
        // where there is an error to go to: the letters (`ui::keys`) then
        // wait for the key after it.
        bind(C::PreviousRow, ROWS, NONE, keys(&[k("[")])).when(When::InspectorOpen),
        bind(C::NextRow, ROWS, NONE, keys(&[k("]")])).when(When::InspectorOpen),
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
        // The list under a foreign key cell's field, while it is open. The
        // keys that commit the edit take its highlighted row. The one that
        // closes it leaves the cell as it was: the text is a search there,
        // not a value to keep.
        bind(
            C::NextPick,
            CELL_EDITOR,
            keys(&[k("down")]),
            keys(&[k("ctrl+n"), k("down")]),
        )
        .when(When::KeyPicker),
        bind(
            C::PreviousPick,
            CELL_EDITOR,
            keys(&[k("up")]),
            keys(&[k("ctrl+p"), k("up")]),
        )
        .when(When::KeyPicker),
        bind(
            C::PickPageDown,
            CELL_EDITOR,
            keys(&[k("pgdn")]),
            keys(&[k("pgdn")]),
        )
        .when(When::KeyPicker),
        bind(
            C::PickPageUp,
            CELL_EDITOR,
            keys(&[k("pgup")]),
            keys(&[k("pgup")]),
        )
        .when(When::KeyPicker),
        bind(
            C::ClosePicker,
            CELL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::KeyPicker),
        // What labels the list's rows is chosen from the list: the chord
        // is the field's while the list is open, and insert mode's, where
        // Omarchy's `ctrl+l` is no move to the pane on the right.
        bind(
            C::ChangeLabelColumn,
            CELL_EDITOR,
            keys(&[k("cmd+l")]),
            keys(&[k("ctrl+l")]),
        )
        .when(When::KeyPicker),
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
        // Wherever the key that adds a row is read: it does not ask what
        // has the keyboard. In the terminal look a copy is two keys, each
        // a command of its own: the row is yanked, and put as often as it
        // is asked for.
        bind(
            C::DuplicateRow,
            GRID,
            keys(&[k("cmd+d")]).any(),
            keys(&[via("yy"), via("p")]).shown("yy p"),
        ),
        // From a field of the row panel too, as the letters that edit are.
        bind(C::YankRow, CELLS, NONE, keys(&[k("yy")])),
        bind(C::PutRow, CELLS, NONE, keys(&[k("p")])),
        bind(C::PutRowAbove, CELLS, NONE, keys(&[k("P")])),
        bind(
            C::DeleteRow,
            GRID,
            keys(&[k("backspace"), k("delete")]),
            keys(&[k("dd")]),
        ),
        // From a field of a new row's form too, whose footer names the
        // key: the row under the cursor is the one the form is of. The
        // other looks have a button there.
        bind(C::DeleteRow, INSPECTOR, NONE, keys(&[k("dd")])).when(When::NewRow),
        bind(C::AddRow, INSPECTOR, NONE, keys(&[k("o")])).when(When::NewRow),
        // The terminal look's visual selection of rows: `V` starts and
        // leaves it, and while it is on `d` alone deletes its rows.
        bind(C::SelectRows, GRID, NONE, keys(&[k("V")])),
        bind(C::DeleteSelection, GRID, NONE, keys(&[k("d")])).when(When::Visual),
        bind(C::LeaveVisual, GRID, NONE, keys(&[k("esc")])).when(When::Visual),
        bind(C::OpenReferencedRow, GRID, NONE, keys(&[k("gd")])),
        bind(C::ShowSavedRow, GRID, NONE, keys(&[k("gs")])),
        // The Structure view of a table, which is the grid's tab with its
        // columns in the rows' place. Its cursor is on the line of a
        // column, and what labels the table's rows for the foreign keys
        // that point at it is chosen here: with the chooser, by the column
        // under the cursor, or by a line of the prompt.
        bind(
            C::MoveInStructure,
            GRID,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        )
        .when(When::Structure),
        bind(
            C::OpenLabelChooser,
            GRID,
            keys(&[k("cmd+l")]),
            keys(&[k(":label")]),
        )
        .when(When::Structure),
        bind(
            C::UseAsLabel,
            GRID,
            keys(&[k("alt+cmd+l")]),
            keys(&[k("gl")]),
        )
        .when(When::Structure),
        bind(
            C::LabelAutomatic,
            GRID,
            control("Use Automatic button"),
            keys(&[k(":label auto")]),
        )
        .when(When::Structure),
        bind(
            C::LabelKeysOnly,
            GRID,
            control("label chooser"),
            keys(&[k(":label none")]),
        )
        .when(When::Structure),
        bind(
            C::LabelColumn,
            GRID,
            control("label chooser"),
            keys(&[k(":label {column}")]),
        )
        .when(When::Structure),
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
        // Esc on the rows asks before it drops them, as closing the tab
        // does. The terminal look's Esc stays harmless: `:e!` drops them.
        bind(C::AskDiscard, GRID, keys(&[k("esc")]), NONE).when(When::Pending),
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
        // In the editor's text: an indent at the cursor, or one on each
        // line the selection touches; one taken off each line. Settings ›
        // Editor says what an indent is.
        bind(
            C::IndentLines,
            SQL_EDITOR,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        ),
        bind(
            C::OutdentLines,
            SQL_EDITOR,
            keys(&[k("shift+tab")]),
            keys(&[k("shift+tab")]),
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
        bind(
            C::ExplainAnalyze,
            SQL_EDITOR,
            keys(&[k("cmd+alt+e")]).any(),
            NONE,
        ),
        bind(C::ComparePreviousRun, SQL_EDITOR, NONE, NONE),
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
        // Out of a text field only: in one, egui's text editing takes the
        // chord as redo (Ctrl+Y on Windows), the SQL editor's included.
        bind(
            C::History,
            GLOBAL,
            keys(&[k("cmd+y")]).normal(),
            keys(&[k(":history")]),
        ),
        bind(C::ClearHistory, GLOBAL, NONE, keys(&[k(":history clear")])),
        bind(
            C::MoveInHistory,
            HISTORY,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        ),
        bind(
            C::RunHistoryAgain,
            HISTORY,
            keys(&[k("cmd+enter")]),
            keys(&[k("r")]),
        ),
        bind(C::OpenHistoryEntry, HISTORY, NONE, keys(&[k("enter")])),
        bind(C::ShowHistoryCall, HISTORY, NONE, keys(&[k("a")])),
        bind(C::ReapplyHistoryFilter, HISTORY, NONE, keys(&[k("f")])),
        bind(
            C::CopyHistorySql,
            HISTORY,
            keys(&[k("cmd+c")]),
            keys(&[k("y")]),
        ),
        bind(
            C::FilterHistory,
            HISTORY,
            keys(&[k("cmd+f")]).any(),
            keys(&[k("/")]),
        ),
        bind(C::CloseHistoryLine, HISTORY, NONE, keys(&[k("esc")])),
        bind(C::ConfirmClearHistory, HISTORY, NONE, keys(&[k("y")])).when(When::ClearingHistory),
        bind(
            C::KeepHistory,
            HISTORY,
            keys(&[k("esc")]),
            keys(&[k("n"), k("esc")]).shown("n"),
        )
        .when(When::ClearingHistory),
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
        bind(C::ShowAllRows, SQL_EDITOR, NONE, keys(&[k("a")])).when(When::RowsCut),
        bind(C::RunWithoutTimeout, SQL_EDITOR, NONE, keys(&[k("t")])).when(When::TimedOut),
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
        // In the Password field while it is empty and says the password
        // is saved: the keys that would delete the saved one were it shown.
        bind(
            C::FormForgetPassword,
            PROMPT,
            keys(&[k("backspace"), k("delete")]),
            keys(&[k("backspace"), k("delete")]),
        )
        .when(When::ConnectionForm),
        bind(C::FormCancel, PROMPT, keys(&[k("esc")]), keys(&[k("esc")]))
            .when(When::ConnectionForm),
        // In Add table…'s field, while its list of the window's tables
        // shows: up moves up it, and down down.
        bind(
            C::FormTableMove,
            PROMPT,
            keys(&[k("up"), k("down")]),
            keys(&[k("up"), k("down")]),
        )
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
            C::SettingsPane,
            PROMPT,
            NONE,
            keys(&[k("ctrl+h"), k("ctrl+l")]).shown("ctrl+h/l"),
        )
        .when(When::SettingsScreen),
        bind(C::SettingsOpenRow, PROMPT, NONE, keys(&[k("enter")])).when(When::SettingsScreen),
        bind(C::SettingsCopyCommand, PROMPT, NONE, keys(&[k("y")])).when(When::SettingsScreen),
        bind(C::SettingsDeleteHistory, PROMPT, NONE, keys(&[k("D")])).when(When::SettingsScreen),
        bind(C::SettingsKeepHistory, PROMPT, NONE, keys(&[k("n")])).when(When::SettingsScreen),
        // The Keyboard pane's own: Enter records the row (the screen's
        // Enter), `R` resets it, and the arrows' letters move as on any
        // pane.
        bind(C::SettingsUnbind, PROMPT, NONE, keys(&[k("dd")])).when(When::SettingsScreen),
        bind(C::SettingsFilter, PROMPT, NONE, keys(&[k("/")])).when(When::SettingsScreen),
        bind(C::SettingsFindKey, PROMPT, NONE, keys(&[k("?")])).when(When::SettingsScreen),
        bind(C::SettingsSwap, PROMPT, NONE, keys(&[k("s")])).when(When::SettingsScreen),
        bind(
            C::SettingsClose,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::SettingsScreen),
        // Quick open: its field has the keyboard. Omarchy's digits pick a
        // recent while nothing is typed; once anything is, they are typed
        // (names hold digits), and quick open reads them as text. Its
        // moves list the keys that go up first, then those that go down.
        bind(
            C::QuickOpenMove,
            PROMPT,
            keys(&[k("up"), k("down")]).shown("↑↓"),
            keys(&[k("ctrl+p"), k("up"), k("ctrl+n"), k("down")]).shown("ctrl+n/p"),
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
            C::QuickOpenPickInEditor,
            PROMPT,
            keys(&[k("alt+enter")]),
            NONE,
        )
        .when(When::QuickOpen),
        bind(
            C::QuickOpenScope,
            PROMPT,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        )
        .when(When::QuickOpen),
        // Typed first, it lists the commands alone: what quick open reads
        // from its field's text (macOS, Windows). Omarchy's commands stay
        // on `:`.
        bind(C::QuickOpenCommands, PROMPT, keys(&[k(">")]), NONE).when(When::QuickOpen),
        bind(C::QuickOpenNumber, PROMPT, NONE, keys(&[k("1..9")])).when(When::QuickOpen),
        bind(
            C::QuickOpenClose,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::QuickOpen),
        // The chooser of a table's label column. It has no field: a letter
        // is a key of its own here. Shift with the key that chooses is
        // kept for a label of two columns.
        bind(
            C::NextLabel,
            PROMPT,
            keys(&[k("down")]),
            keys(&[k("j"), k("ctrl+n"), k("down")]),
        )
        .when(When::LabelChooser),
        bind(
            C::PreviousLabel,
            PROMPT,
            keys(&[k("up")]),
            keys(&[k("k"), k("ctrl+p"), k("up")]),
        )
        .when(When::LabelChooser),
        bind(
            C::ChooseLabel,
            PROMPT,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        )
        .when(When::LabelChooser),
        bind(
            C::CloseLabelChooser,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::LabelChooser),
        bind(
            C::AddLabelPart,
            PROMPT,
            keys(&[claimed("shift+enter")]),
            keys(&[claimed("shift+enter")]),
        )
        .when(When::LabelChooser),
    ]
};

/// What `line` of the `:` prompt gives the command spelled `name`, where
/// the spelling ends in a word in braces (`label {column}`): what follows
/// the command's own words and a space, without the white space round it.
/// None for a spelling that takes nothing, for another command's line and
/// for a line with nothing after the name.
fn argument<'a>(name: &str, line: &'a str) -> Option<&'a str> {
    let (head, word) = name.rsplit_once(' ')?;
    if !(word.starts_with('{') && word.ends_with('}')) {
        return None;
    }
    let given = line.strip_prefix(head)?.strip_prefix(' ')?.trim();
    (!given.is_empty()).then_some(given)
}

/// The modifiers of a chord, as a spelling names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stroke {
    Key { mods: Mods, key: Key },
    Text(char),
}

/// A spelling, read.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    ("f1", Key::F1),
    ("f2", Key::F2),
    ("f3", Key::F3),
    ("f4", Key::F4),
    ("f5", Key::F5),
    ("f6", Key::F6),
    ("f7", Key::F7),
    ("f8", Key::F8),
    ("f9", Key::F9),
    ("f10", Key::F10),
    ("f11", Key::F11),
    ("f12", Key::F12),
    ("f13", Key::F13),
    ("f14", Key::F14),
    ("f15", Key::F15),
    ("f16", Key::F16),
    ("f17", Key::F17),
    ("f18", Key::F18),
    ("f19", Key::F19),
];

/// The function keys a key alone can be on macOS and Windows, where every
/// other key a command is given needs Cmd, Ctrl or Alt with it.
fn function_key(key: Key) -> bool {
    matches!(
        key,
        Key::F1
            | Key::F2
            | Key::F3
            | Key::F4
            | Key::F5
            | Key::F6
            | Key::F7
            | Key::F8
            | Key::F9
            | Key::F10
            | Key::F11
            | Key::F12
            | Key::F13
            | Key::F14
            | Key::F15
            | Key::F16
            | Key::F17
            | Key::F18
            | Key::F19
    )
}

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
    if let Some(line) = keys.strip_prefix(':') {
        if line.is_empty() {
            return Err(Unread(format!("{keys}: a command without a name")));
        }
        return Ok(Spelled::Ex(line));
    }
    spell_keys(keys)
}

/// Whether `keys` reads as a spelling, without keeping it: for text read
/// from a file, which is kept ([`intern`]) only once it reads.
pub fn spells(keys: &str) -> bool {
    match keys.strip_prefix(':') {
        Some(line) => !line.is_empty(),
        None => spell_keys(keys).is_ok(),
    }
}

/// Reads a spelling that is no line of the prompt: keys, or a range of
/// digits, which hold nothing of the text.
fn spell_keys(keys: &str) -> Result<Spelled, Unread> {
    let unread = |why: &str| Unread(format!("{keys}: {why}"));
    if let Some(key) = named(keys) {
        let mods = Mods::default();
        return Ok(Spelled::Strokes(vec![Stroke::Key { mods, key }]));
    }
    // The digits with nothing held: the start screen's rows.
    if keys == "1..9" {
        return Ok(Spelled::Digits(Mods::default()));
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
            // The word: no face the Mac look is drawn with has `⇥`.
            Key::Tab => "Tab".to_owned(),
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

    /// The key in the brackets a prompt draws round the letter that
    /// answers it: `[w]`.
    pub fn bracketed(&self) -> Written {
        Written(format!("[{}]", self.0))
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

impl Command {
    /// Whether a count before the command's keys says of how many rows it
    /// is. Before any other command's keys a count is dropped.
    pub fn takes_count(self) -> bool {
        matches!(self, Self::DeleteRow)
    }
}

/// The count typed before a key, and what follows it: `12dd` is 12 and
/// `dd`. A count begins with a digit that is not nought, and more than six
/// digits read as 999,999.
pub fn count_of(typed: &str) -> (Option<u32>, &str) {
    let digits = typed.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || typed.starts_with('0') {
        return (None, typed);
    }
    let (count, rest) = typed.split_at(digits);
    let count = if digits > 6 {
        999_999
    } else {
        count.parse().unwrap_or(999_999)
    };
    (Some(count), rest)
}

/// Whether `typed` is a count with no key after it yet.
pub fn is_count(typed: &str) -> bool {
    matches!(count_of(typed), (Some(_), ""))
}

/// What typed characters, or a line of the `:` prompt, come to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typed {
    /// They are this command's keys.
    Command(Command),
    /// They are this command's keys with a count before them (`3dd`), and
    /// the command takes one.
    Counted(Command, u32),
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
        // A count before the key, where one is read: on a table's rows, in
        // the terminal look.
        let (count, typed) = if layout == Layout::Omarchy && scope == Scope::Grid {
            count_of(typed)
        } else {
            (None, typed)
        };
        if count.is_some() && typed.is_empty() {
            return Typed::Waiting;
        }
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
        let read = self.read(
            layout,
            scope,
            holds,
            &|spelled| letters(spelled).is_some_and(|letters| letters == typed),
            &|spelled| letters(spelled).is_some_and(|letters| letters.starts_with(typed)),
        );
        match (read, count) {
            (Typed::Command(command), Some(count)) if command.takes_count() => {
                Typed::Counted(command, count)
            }
            _ => read,
        }
    }

    /// What a line of the `:` prompt (`line`, without its colon) comes to
    /// in `scope`. A line of digits is the one spelled `:N`. A line that
    /// is no command as it stands is read as one that takes a word after
    /// its name, where a command is spelled so (`:label {column}`): the
    /// lines spelled whole come first, so `:label auto` is never a column
    /// called `auto`.
    pub fn ex(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        line: &str,
    ) -> Typed {
        let number = !line.is_empty() && line.chars().all(|char| char.is_ascii_digit());
        let whole = self.read(
            layout,
            scope,
            holds,
            &|spelled| match spelled {
                Spelled::Ex(name) => *name == line || (number && *name == "N"),
                Spelled::Strokes(_) | Spelled::Digits(_) => false,
            },
            &|_| false,
        );
        if whole != Typed::Nothing {
            return whole;
        }
        self.read(
            layout,
            scope,
            holds,
            &|spelled| match spelled {
                Spelled::Ex(name) => argument(name, line).is_some(),
                Spelled::Strokes(_) | Spelled::Digits(_) => false,
            },
            &|_| false,
        )
    }

    /// The word a line of the `:` prompt gives `command`, where the
    /// command is spelled as taking one in `scope` (`:label {column}`) and
    /// `line` is that command with something after its name.
    pub fn ex_argument<'a>(
        &self,
        layout: Layout,
        command: Command,
        scope: Scope,
        line: &'a str,
    ) -> Option<&'a str> {
        self.chords(layout, command, scope)
            .filter_map(|keys| keys.strip_prefix(':'))
            .find_map(|name| argument(name, line))
    }

    /// The first key of `command` as `layout` writes it, for a hint beside
    /// a control. Empty where it has none.
    pub fn key(&self, layout: Layout, command: Command) -> Written {
        let label = self.label(layout, command);
        let first = label.split(", ").next().unwrap_or_default();
        Written(first.to_owned())
    }

    /// The first key `command` has in `scope`, as `layout` writes it: for a
    /// hint drawn where that scope has the keys, of a command whose keys
    /// differ by where it is read (a format is `=` in a SQL tab and a
    /// chord in the large editor). A key that is only claimed is not
    /// offered. Empty where it has none there.
    pub fn key_in(&self, layout: Layout, command: Command, scope: Scope) -> Written {
        let built = command.info().built;
        self.of(command)
            .filter(|binding| binding.scopes.contains(&scope))
            .flat_map(|binding| binding.bound(layout).chords.iter())
            .find(|chord| chord.kind != Kind::Claimed || !built)
            .map_or_else(Written::default, |chord| {
                Written(written(layout, chord.keys))
            })
    }

    /// The `nth` keys that `first` and `second` have in `scope`, written
    /// as one hint of two keys that do opposite things: either side of a
    /// stroke (`j/k`), and what they hold written once where they hold
    /// alike (`ctrl+n/p`). None where either has no such key.
    pub fn pair(
        &self,
        layout: Layout,
        (first, second): (Command, Command),
        scope: Scope,
        nth: usize,
    ) -> Option<Written> {
        let key = |command| {
            let keys = self.chords(layout, command, scope).nth(nth)?;
            Some(written(layout, keys))
        };
        let (first, second) = (key(first)?, key(second)?);
        let held = first.rfind('+').map_or("", |plus| &first[..=plus]);
        let pair = match second.strip_prefix(held) {
            Some(rest) if !held.is_empty() => format!("{first}/{rest}"),
            _ => format!("{first}/{second}"),
        };
        Some(Written(pair))
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

/// The presses of a spelling, each digit of a range apart.
pub(crate) fn presses(layout: Layout, spelled: &Spelled) -> Vec<Vec<Stroke>> {
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
pub(crate) fn clash(layout: Layout, a: &Spelled, b: &Spelled) -> bool {
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

/// One key of the keymap where it is read: a line's key in one of its
/// scopes, in one layout.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    /// The line it is a key of, by its place in the keymap.
    pub line: usize,
    pub command: Command,
    pub scope: Scope,
    pub when: When,
    pub mode: Mode,
    pub keys: &'static str,
    pub spelled: Spelled,
}

/// Every key of the keymap that is bound in `layout`, claimed ones and
/// those of commands not built yet too. Not the ones written beside a
/// command that are another's.
pub(crate) fn entries(keymap: &Keymap, layout: Layout) -> Vec<Entry> {
    let mut entries = Vec::new();
    for (line, binding) in keymap.bindings().iter().enumerate() {
        for chord in binding.bound(layout).chords {
            if chord.kind == Kind::Via {
                continue;
            }
            let spelled = spell(chord.keys).expect("a spelling that reads");
            for scope in binding.scopes {
                entries.push(Entry {
                    line,
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

/// Whether two spellings are the same keys in `layout`: the same line of
/// the prompt, or the same presses (on Windows `cmd+s` is `ctrl+s`).
pub fn same(layout: Layout, a: &Spelled, b: &Spelled) -> bool {
    match (a, b) {
        (Spelled::Ex(a), Spelled::Ex(b)) => a == b,
        (Spelled::Ex(_), _) | (_, Spelled::Ex(_)) => false,
        _ => presses(layout, a) == presses(layout, b),
    }
}

/// How the key handler reads a key. A key recorded for a command must be
/// of a kind the command is read by, or nothing would answer it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Press {
    /// One press of a key with what is held (`ctrl+s`, `enter`, `f5`).
    Chord,
    /// Characters typed one after another (`gd`, `x`).
    Typed,
    /// A line of the `:` prompt (`:w`).
    Line,
}

impl Press {
    /// How `spelled` is read. None for a range of digits, which stands for
    /// nine keys at once.
    pub fn of(spelled: &Spelled) -> Option<Press> {
        match spelled {
            Spelled::Ex(_) => Some(Self::Line),
            Spelled::Digits(_) => None,
            Spelled::Strokes(strokes) => match strokes.as_slice() {
                [Stroke::Key { .. }] => Some(Self::Chord),
                typed if typed.iter().all(|stroke| matches!(stroke, Stroke::Text(_))) => {
                    Some(Self::Typed)
                }
                _ => None,
            },
        }
    }
}

/// A row of the Keyboard settings: a command where its keys hold. A
/// command with keys in two places has a row for each (`Edit cell` on the
/// rows and in the inspector).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Row {
    pub command: Command,
    /// The first scope of its line: the table of `settings.toml` its keys
    /// are written in.
    pub scope: Scope,
    /// What must also show for its keys to hold.
    pub when: When,
}

impl Row {
    /// The row a line of the keymap is.
    pub fn of(binding: &Binding) -> Row {
        Row {
            command: binding.command,
            scope: binding.scopes.first().copied().unwrap_or(Scope::Global),
            when: binding.when,
        }
    }
}

/// Why a row's keys cannot be recorded again. Its keys are Tabletist's: a
/// row that would take one is told so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fixed {
    /// They hold only while something else shows (the completion list,
    /// the large editor), where the same key is another's the rest of the
    /// time.
    While,
    /// The key says which way to go: the handler reads the way from it.
    Direction,
    /// A range of digits, one key for each of nine.
    Digits,
    /// The system has the key: the app menu's, or one every text field
    /// reads ([`reserved`]).
    System,
    /// Read where nothing asks the keymap for another key: Help's `?` is a
    /// character in every look, and Esc leaves the editor by egui's rule.
    Own,
    /// Its keys are another command's (`yy`, then `p`).
    Others,
}

/// The commands whose key says which way to go.
const DIRECTIONS: [Command; 9] = [
    Command::MoveRow,
    Command::MoveColumn,
    Command::MovePage,
    Command::MoveField,
    Command::MoveInTree,
    Command::MoveInConnections,
    Command::MoveInAgentSessions,
    Command::MoveInHistory,
    Command::MoveInStructure,
];

/// The commands read where nothing asks the keymap for another key.
const OWN: [Command; 2] = [Command::Help, Command::LeaveEditor];

/// The rows with no key in one look whose chords the key handler reads in
/// every look: there a chord can be given them. Any other row with no key
/// in a look is read by that look's letters or prompt, or by the other
/// look's handler alone, and nothing would answer a key given it.
const ANY_LOOK: [(Command, Scope); 14] = [
    (Command::SwitchDatabase, Scope::Global),
    (Command::PaneLeft, Scope::Global),
    (Command::PaneRight, Scope::Global),
    (Command::PaneUp, Scope::Global),
    (Command::PaneDown, Scope::Global),
    (Command::DefaultTextSize, Scope::Global),
    (Command::ExtendUp, Scope::Grid),
    (Command::ExtendDown, Scope::Grid),
    (Command::CopyRow, Scope::Grid),
    (Command::ToggleSqlMode, Scope::SqlEditor),
    (Command::KeepEdit, Scope::CellEditor),
    (Command::OpenHistoryEntry, Scope::History),
    (Command::ShowHistoryCall, Scope::History),
    (Command::ReapplyHistoryFilter, Scope::History),
];

/// The commands every look reads by one typed character, before anything
/// else takes a key: their typed keys are one letter each.
pub const ONE_LETTER: [Command; 4] = [
    Command::Reload,
    Command::FilterObjects,
    Command::ToggleTree,
    Command::Help,
];

/// Vim's own chords in insert mode, which text typed in an editor keeps:
/// a word back, a character back, the line back, and a register.
pub const VIM_INSERT: [&str; 4] = ["ctrl+w", "ctrl+h", "ctrl+u", "ctrl+r"];

/// Who keeps a key the app never takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    MacOs,
    Windows,
}

impl Owner {
    pub fn name(self) -> &'static str {
        match self {
            Self::MacOs => "macOS",
            Self::Windows => "Windows",
        }
    }
}

/// The keys `layout`'s system keeps: the app menu's and the window's, and
/// the text editing every field reads. No command is given one; a command
/// that has one by default keeps it, and its row is fixed. Omarchy has
/// none here: Super is Hyprland's, and the window never sees it.
pub fn reserved(layout: Layout) -> &'static [&'static str] {
    match layout {
        Layout::Mac => &[
            "cmd+q",
            "cmd+w",
            "cmd+h",
            "alt+cmd+h",
            "cmd+m",
            "cmd+tab",
            "cmd+space",
            "ctrl+cmd+f",
            "cmd+,",
            "cmd+c",
            "cmd+v",
            "cmd+x",
            "cmd+z",
            "cmd+shift+z",
            "cmd+a",
        ],
        Layout::Windows => &[
            "alt+f4",
            "cmd+c",
            "cmd+v",
            "cmd+x",
            "cmd+z",
            "cmd+shift+z",
            "cmd+a",
        ],
        Layout::Omarchy => &[],
    }
}

/// Who keeps `spelled` in `layout`, if the system does.
pub fn reserved_by(layout: Layout, spelled: &Spelled) -> Option<Owner> {
    let owner = match layout {
        Layout::Mac => Owner::MacOs,
        Layout::Windows => Owner::Windows,
        Layout::Omarchy => return None,
    };
    reserved(layout)
        .iter()
        .filter_map(|keys| spell(keys).ok())
        .any(|kept| same(layout, &kept, spelled))
        .then_some(owner)
}

/// The keymap's lines a row is, as the defaults have them.
fn default_lines(row: Row) -> impl Iterator<Item = &'static Binding> {
    BINDINGS
        .iter()
        .filter(move |binding| Row::of(binding) == row)
}

/// A row's keys by default in `layout`: its own and those it claims, not
/// another command's written beside it.
pub fn defaults(layout: Layout, row: Row) -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = Vec::new();
    for binding in default_lines(row) {
        for chord in binding.bound(layout).chords {
            if chord.kind != Kind::Via && !keys.contains(&chord.keys) {
                keys.push(chord.keys);
            }
        }
    }
    keys
}

/// Why a row's keys cannot be recorded in `layout`, if they cannot.
pub fn fixed(layout: Layout, row: Row) -> Option<Fixed> {
    if row.when != When::Always {
        return Some(Fixed::While);
    }
    if DIRECTIONS.contains(&row.command) {
        return Some(Fixed::Direction);
    }
    if OWN.contains(&row.command) {
        return Some(Fixed::Own);
    }
    let chords: Vec<&Chord> = default_lines(row)
        .flat_map(|binding| binding.bound(layout).chords.iter())
        .collect();
    let spelled = |chord: &Chord| spell(chord.keys).ok();
    if chords
        .iter()
        .any(|chord| matches!(spelled(chord), Some(Spelled::Digits(_))))
    {
        return Some(Fixed::Digits);
    }
    // ⌘, is the app menu's item, which AppKit takes before the window.
    let menu = row.command == Command::Settings && layout == Layout::Mac;
    let kept = chords.iter().any(|chord| {
        chord.kind != Kind::Via
            && spelled(chord).is_some_and(|keys| reserved_by(layout, &keys).is_some())
    });
    if menu || kept {
        return Some(Fixed::System);
    }
    if !chords.is_empty() && chords.iter().all(|chord| chord.kind == Kind::Via) {
        return Some(Fixed::Others);
    }
    None
}

/// The kinds of key a row can be given in `layout`: those its keys have
/// by default (`Save changes` on Omarchy, a line and a chord). A row with
/// no key in the look takes a chord where the handler reads its chords in
/// every look, and where nothing answers it yet. Empty: nothing would read
/// a key given it in this look.
pub fn takes(layout: Layout, row: Row) -> Vec<Press> {
    let keys = defaults(layout, row);
    let mut kinds: Vec<Press> = Vec::new();
    for press in keys
        .iter()
        .filter_map(|keys| spell(keys).ok())
        .filter_map(|spelled| Press::of(&spelled))
    {
        if !kinds.contains(&press) {
            kinds.push(press);
        }
    }
    let none = keys.is_empty()
        && default_lines(row).all(|binding| binding.bound(layout).chords.is_empty());
    if none && (!row.command.info().built || ANY_LOOK.contains(&(row.command, row.scope))) {
        kinds.push(Press::Chord);
    }
    kinds
}

impl Group {
    /// The groups the Keyboard settings list, in order. A prompt's keys
    /// are its own, and are not listed.
    pub const LISTED: [Group; 5] = [
        Group::Window,
        Group::Navigation,
        Group::Editing,
        Group::Sql,
        Group::Ai,
    ];

    /// The group's heading, as `layout` writes it. English: the caller
    /// translates.
    pub fn name(self, layout: Layout) -> &'static str {
        let omarchy = layout == Layout::Omarchy;
        match self {
            Self::Window if omarchy => "window",
            Self::Window => "Window and connections",
            Self::Navigation if omarchy => "navigate",
            Self::Navigation => "Navigation",
            Self::Editing if omarchy => "edit",
            Self::Editing => "Grid and editing",
            Self::Sql if omarchy => "sql",
            Self::Sql => "SQL editor",
            Self::Ai if omarchy => "ai",
            Self::Ai => "AI assistant",
            Self::Prompts if omarchy => "prompts",
            Self::Prompts => "Prompts and dialogs",
        }
    }
}

impl Keymap {
    /// The rows of the Keyboard settings in `layout`, by their groups in
    /// [`Group::LISTED`] order, each in the keymap's order. A row is
    /// listed where it has a key in the look, or can be given one.
    pub fn rows(&self, layout: Layout) -> Vec<Row> {
        let mut rows: Vec<Row> = Vec::new();
        for group in Group::LISTED {
            for binding in BINDINGS {
                let row = Row::of(binding);
                if row.command.info().group != group || rows.contains(&row) {
                    continue;
                }
                let has = default_lines(row).any(|line| !line.bound(layout).chords.is_empty());
                if has || !takes(layout, row).is_empty() {
                    rows.push(row);
                }
            }
        }
        rows
    }

    /// The lines of the keymap a row is.
    pub fn lines_of(&self, row: Row) -> impl Iterator<Item = &Binding> {
        self.bindings
            .iter()
            .filter(move |binding| Row::of(binding) == row)
    }

    /// A row's keys as they are now in `layout`: its own and those it
    /// claims, not another command's written beside it.
    pub fn keys_of(&self, layout: Layout, row: Row) -> Vec<&'static str> {
        let mut keys: Vec<&'static str> = Vec::new();
        for binding in self.lines_of(row) {
            for chord in binding.bound(layout).chords {
                if chord.kind != Kind::Via && !keys.contains(&chord.keys) {
                    keys.push(chord.keys);
                }
            }
        }
        keys
    }

    /// A row's keys as `layout` writes them, one for each key cap: what
    /// its lines write together where they do (`j/k`), and another
    /// command's key written beside its own as [`Keymap::label`] does.
    pub fn row_keys(&self, layout: Layout, row: Row) -> Vec<Written> {
        let built = row.command.info().built;
        let mut all: Vec<Written> = Vec::new();
        for binding in self.lines_of(row) {
            let bound = binding.bound(layout);
            let mut line: Vec<String> = Vec::new();
            if bound.shown.is_empty() {
                for chord in bound.chords {
                    if chord.kind != Kind::Claimed || !built {
                        line.push(written(layout, chord.keys));
                    }
                }
            } else {
                line.push(bound.shown.to_owned());
            }
            for keys in line {
                if !all.iter().any(|written| written.as_str() == keys) {
                    all.push(Written(keys));
                }
            }
        }
        all
    }

    /// Whether a row's keys differ from its defaults in `layout`.
    pub fn changed(&self, layout: Layout, row: Row) -> bool {
        let now = self.keys_of(layout, row);
        let then = defaults(layout, row);
        let read = |keys: &[&'static str]| -> Vec<Spelled> {
            keys.iter().filter_map(|keys| spell(keys).ok()).collect()
        };
        let (now, then) = (read(&now), read(&then));
        now.len() != then.len()
            || now
                .iter()
                .any(|key| !then.iter().any(|old| same(layout, key, old)))
    }
}

/// A key as `layout` writes it, for a painter of keys: a key cap, a hint.
pub fn key_written(layout: Layout, keys: &'static str) -> Written {
    Written(written(layout, keys))
}

/// A press as `settings.toml` spells it: what is held (`cmd`, `ctrl`,
/// `alt`, `shift`, in that order) joined with `+`, then the key. None for
/// a key no spelling names.
pub fn spelling(mods: Mods, key: Key) -> Option<String> {
    let named = NAMED
        .iter()
        .find(|(_, known)| *known == key)
        .map(|(name, _)| (*name).to_owned());
    let name = named
        .or_else(|| key_mark(key).map(|mark| mark.to_string()))
        .or_else(|| {
            let name = key.name();
            let one = name.len() == 1 && name.chars().all(|char| char.is_ascii_alphanumeric());
            one.then(|| name.to_ascii_lowercase())
        })?;
    let mut spelled = String::new();
    for (held, word) in [
        (mods.cmd, "cmd+"),
        (mods.ctrl, "ctrl+"),
        (mods.alt, "alt+"),
        (mods.shift, "shift+"),
    ] {
        if held {
            spelled.push_str(word);
        }
    }
    spelled.push_str(&name);
    Some(spelled)
}

/// Whether a chord needs nothing held with it to be given to a command on
/// macOS and Windows: a function key.
pub fn alone_will_do(spelled: &Spelled) -> bool {
    match spelled {
        Spelled::Strokes(strokes) => match strokes.as_slice() {
            [Stroke::Key { mods, key }] => mods.cmd || mods.ctrl || mods.alt || function_key(*key),
            _ => false,
        },
        Spelled::Digits(mods) => mods.cmd || mods.ctrl || mods.alt,
        Spelled::Ex(_) => false,
    }
}

/// A key as a screen reader says it on macOS: the names of what is held,
/// in Apple's order, then the key's ("Shift Command F"). The other looks
/// say a key as they write it.
pub fn spoken(layout: Layout, keys: &'static str) -> String {
    let Layout::Mac = layout else {
        return written(layout, keys);
    };
    let Ok(Spelled::Strokes(strokes)) = spell(keys) else {
        return written(layout, keys);
    };
    let [Stroke::Key { mods, key }] = strokes.as_slice() else {
        return written(layout, keys);
    };
    let mut words: Vec<String> = Vec::new();
    for (held, word) in [
        (mods.ctrl, "Control"),
        (mods.alt, "Option"),
        (mods.shift, "Shift"),
        (mods.cmd, "Command"),
    ] {
        if held {
            words.push(word.to_owned());
        }
    }
    let name = match key {
        Key::Enter => "Return",
        Key::Backspace => "Delete",
        Key::Delete => "Forward Delete",
        Key::Escape => "Escape",
        Key::ArrowUp => "Up Arrow",
        Key::ArrowDown => "Down Arrow",
        Key::ArrowLeft => "Left Arrow",
        Key::ArrowRight => "Right Arrow",
        Key::PageUp => "Page Up",
        Key::PageDown => "Page Down",
        Key::Comma => "Comma",
        Key::Period => "Period",
        Key::Quote => "Apostrophe",
        Key::OpenBracket => "Left Bracket",
        Key::CloseBracket => "Right Bracket",
        Key::Slash => "Slash",
        Key::Equals => "Equals",
        Key::Minus => "Minus",
        Key::Semicolon => "Semicolon",
        Key::Backtick => "Grave Accent",
        Key::Backslash => "Backslash",
        other => other.name(),
    };
    words.push(name.to_owned());
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_is_named_in_the_file_by_a_stable_snake_case_id() {
        let mut ids: Vec<&str> = Vec::new();
        for command in Command::ALL {
            let id = command.id();
            assert!(
                !id.is_empty()
                    && id.chars().all(|char| char.is_ascii_lowercase()
                        || char.is_ascii_digit()
                        || char == '_'),
                "{id}"
            );
            assert!(!ids.contains(&id), "{id} names two commands");
            ids.push(id);
            assert_eq!(Command::from_id(id), Some(*command));
        }
        // The brief's own.
        assert_eq!(Command::DeleteRow.id(), "delete_row");
        assert_eq!(Command::SetDefault.id(), "set_default");
        assert_eq!(Command::RunAll.id(), "run_all");
        assert_eq!(Command::from_id("Delete row"), None);
    }

    #[test]
    fn a_scope_is_named_and_filed_as_each_look_has_it() {
        assert_eq!(Scope::Grid.name(Layout::Mac), "Table");
        assert_eq!(Scope::SqlEditor.name(Layout::Mac), "Editor");
        assert_eq!(Scope::CellEditor.name(Layout::Mac), "Cell");
        assert_eq!(Scope::Global.name(Layout::Omarchy), "global");
        assert_eq!(Scope::Global.table(Layout::Omarchy), "window");
        assert_eq!(Scope::Sidebar.table(Layout::Omarchy), "navigate");
        assert_eq!(Scope::CellEditor.table(Layout::Omarchy), "edit");
        assert_eq!(Scope::SqlEditor.table(Layout::Omarchy), "sql");
        for layout in Layout::ALL {
            let mut tables: Vec<&str> = Vec::new();
            for scope in Scope::ALL {
                let table = scope.table(layout);
                assert!(!tables.contains(&table), "{table}");
                tables.push(table);
                if scope == Scope::Prompt {
                    assert_eq!(Scope::from_table(table), None);
                } else {
                    assert_eq!(Scope::from_table(table), Some(scope), "{table}");
                    assert_eq!(Scope::from_table(scope.id()), Some(scope));
                }
            }
        }
        // Every look's name is read in every look.
        assert_eq!(Scope::from_table("table"), Some(Scope::Grid));
        assert_eq!(Scope::from_table("grid"), Some(Scope::Grid));
        assert_eq!(Scope::from_table("editor"), Some(Scope::SqlEditor));
        assert_eq!(Scope::from_table("unbind"), None);
    }

    #[test]
    fn a_press_is_spelled_as_the_file_writes_it_and_reads_back() {
        let mods = |cmd, ctrl, alt, shift| Mods {
            cmd,
            ctrl,
            alt,
            shift,
        };
        assert_eq!(
            spelling(mods(true, false, false, true), Key::F).as_deref(),
            Some("cmd+shift+f")
        );
        assert_eq!(
            spelling(mods(true, false, true, false), Key::E).as_deref(),
            Some("cmd+alt+e")
        );
        assert_eq!(
            spelling(mods(false, true, false, false), Key::Enter).as_deref(),
            Some("ctrl+enter")
        );
        assert_eq!(spelling(Mods::default(), Key::F5).as_deref(), Some("f5"));
        assert_eq!(
            spelling(mods(true, false, false, false), Key::Comma).as_deref(),
            Some("cmd+,")
        );
        let keys = NAMED
            .iter()
            .map(|(_, key)| *key)
            .chain(MARKS.iter().map(|(_, key)| *key));
        for key in keys.chain([Key::A, Key::Num7]) {
            for held in [
                mods(true, false, false, false),
                mods(false, true, true, true),
            ] {
                let spelled = spelling(held, key).expect("a key the keymap names");
                let read = spell(Box::leak(spelled.into_boxed_str())).expect("reads");
                assert_eq!(
                    read,
                    Spelled::Strokes(vec![Stroke::Key { mods: held, key }])
                );
            }
        }
    }

    #[test]
    fn a_key_is_spoken_by_its_names_on_macos() {
        assert_eq!(spoken(Layout::Mac, "cmd+shift+f"), "Shift Command F");
        assert_eq!(spoken(Layout::Mac, "cmd+alt+e"), "Option Command E");
        assert_eq!(spoken(Layout::Mac, "ctrl+space"), "Control Space");
        assert_eq!(spoken(Layout::Mac, "cmd+enter"), "Command Return");
        assert_eq!(spoken(Layout::Mac, "cmd+backspace"), "Command Delete");
        assert_eq!(spoken(Layout::Windows, "cmd+shift+f"), "Ctrl+Shift+F");
        assert_eq!(spoken(Layout::Omarchy, "ctrl+shift+f"), "ctrl+shift+f");
        assert_eq!(spoken(Layout::Omarchy, "gd"), "gd");
    }

    #[test]
    fn the_keyboard_settings_list_the_recent_commands_with_their_defaults() {
        let keymap = Keymap::default();
        let row = |command, scope| Row {
            command,
            scope,
            when: When::Always,
        };
        let listed = |layout, row: Row| keymap.rows(layout).contains(&row);
        let written = |layout, row| -> Vec<String> {
            keymap
                .row_keys(layout, row)
                .iter()
                .map(|key| key.as_str().to_owned())
                .collect()
        };
        let global = Scope::Global;
        for (command, mac, omarchy) in [
            (Command::FindObject, "⌘P", "ctrl+p"),
            (Command::History, "⌘Y", ":history"),
            (Command::NewSqlTab, "⌘T", "ctrl+t"),
            (Command::ReopenClosedTab, "⇧⌘T", ""),
        ] {
            let row = row(command, global);
            assert!(listed(Layout::Mac, row), "{command:?}");
            assert_eq!(written(Layout::Mac, row), [mac]);
            if !omarchy.is_empty() {
                assert!(listed(Layout::Omarchy, row), "{command:?}");
                assert_eq!(written(Layout::Omarchy, row), [omarchy]);
            }
        }
        if crate::endpoint::AVAILABLE {
            assert_eq!(
                written(Layout::Mac, row(Command::ShowAgents, global)),
                ["⌥⌘A"]
            );
        }
        let start = row(Command::ReopenClosedTab, Scope::Start);
        assert!(listed(Layout::Omarchy, start));
        assert_eq!(written(Layout::Omarchy, start), ["u"]);
        // No prompt's keys, and every row once.
        for layout in Layout::ALL {
            let rows = keymap.rows(layout);
            for (index, row) in rows.iter().enumerate() {
                assert_ne!(row.command.info().group, Group::Prompts, "{row:?}");
                assert!(!rows[index + 1..].contains(row), "{row:?}");
            }
        }
        // A command with no key in the look is listed where it can be
        // given one, and not where nothing would read it.
        assert!(listed(Layout::Mac, row(Command::SwitchDatabase, global)));
        assert!(written(Layout::Mac, row(Command::SwitchDatabase, global)).is_empty());
        assert!(listed(
            Layout::Mac,
            row(Command::ComparePreviousRun, Scope::SqlEditor)
        ));
        assert!(!listed(
            Layout::Mac,
            row(Command::CopyAgentCommand, Scope::Agents)
        ));
        assert!(!listed(
            Layout::Mac,
            row(Command::OpenReferencedRow, Scope::Grid)
        ));
    }

    #[test]
    fn a_row_takes_the_kinds_of_key_its_command_is_read_by() {
        let row = |command, scope| Row {
            command,
            scope,
            when: When::Always,
        };
        use Press::{Chord, Line, Typed};
        let save = row(Command::SaveChanges, Scope::Grid);
        assert_eq!(takes(Layout::Omarchy, save), [Line, Chord]);
        assert_eq!(takes(Layout::Mac, save), [Chord]);
        assert_eq!(
            takes(Layout::Omarchy, row(Command::DeleteRow, Scope::Grid)),
            [Typed]
        );
        assert_eq!(
            takes(Layout::Mac, row(Command::DeleteRow, Scope::Grid)),
            [Chord]
        );
        assert_eq!(
            takes(Layout::Mac, row(Command::SwitchDatabase, Scope::Global)),
            [Chord]
        );
        assert!(takes(Layout::Mac, row(Command::SwitchDatabase, Scope::Start)).is_empty());
        assert_eq!(
            takes(Layout::Omarchy, row(Command::CopyRow, Scope::Grid)),
            [Chord]
        );
        // Nothing answers it yet: it can be given a chord in either look.
        for layout in Layout::ALL {
            let compare = row(Command::ComparePreviousRun, Scope::SqlEditor);
            assert_eq!(takes(layout, compare), [Chord]);
        }
        assert!(takes(Layout::Mac, row(Command::CopyAgentCommand, Scope::Agents)).is_empty());
    }

    #[test]
    fn some_rows_cannot_be_recorded_and_say_why() {
        let row = |command, scope| Row {
            command,
            scope,
            when: When::Always,
        };
        let (mac, omarchy) = (Layout::Mac, Layout::Omarchy);
        assert_eq!(
            fixed(mac, row(Command::MoveRow, Scope::Grid)),
            Some(Fixed::Direction)
        );
        assert_eq!(
            fixed(omarchy, row(Command::MoveInTree, Scope::Sidebar)),
            Some(Fixed::Direction)
        );
        assert_eq!(
            fixed(mac, row(Command::GoToConnectionWindow, Scope::Global)),
            Some(Fixed::Digits)
        );
        assert_eq!(
            fixed(omarchy, row(Command::SwitchTab, Scope::Global)),
            Some(Fixed::Digits)
        );
        assert_eq!(
            fixed(mac, row(Command::CloseTab, Scope::Global)),
            Some(Fixed::System)
        );
        assert_eq!(
            fixed(mac, row(Command::CopyCells, Scope::Grid)),
            Some(Fixed::System)
        );
        assert_eq!(
            fixed(mac, row(Command::UndoCell, Scope::Grid)),
            Some(Fixed::System)
        );
        assert_eq!(
            fixed(mac, row(Command::Settings, Scope::Global)),
            Some(Fixed::System)
        );
        assert_eq!(fixed(omarchy, row(Command::Settings, Scope::Global)), None);
        assert_eq!(
            fixed(Layout::Windows, row(Command::Settings, Scope::Global)),
            None
        );
        assert_eq!(
            fixed(omarchy, row(Command::DuplicateRow, Scope::Grid)),
            Some(Fixed::Others)
        );
        assert_eq!(fixed(mac, row(Command::DuplicateRow, Scope::Grid)), None);
        assert_eq!(
            fixed(mac, row(Command::Help, Scope::Global)),
            Some(Fixed::Own)
        );
        let completing = Row {
            command: Command::InsertCompletion,
            scope: Scope::SqlEditor,
            when: When::Completion,
        };
        assert_eq!(fixed(mac, completing), Some(Fixed::While));
        assert_eq!(fixed(mac, row(Command::Format, Scope::SqlEditor)), None);
        // Every row that can be recorded takes some kind of key.
        let keymap = Keymap::default();
        for layout in Layout::ALL {
            for row in keymap.rows(layout) {
                if fixed(layout, row).is_none() {
                    assert!(!takes(layout, row).is_empty(), "{layout:?}: {row:?}");
                }
            }
        }
    }

    #[test]
    fn the_systems_keys_are_its_own_in_each_look() {
        let read = |keys| spell(keys).unwrap();
        assert_eq!(reserved_by(Layout::Mac, &read("cmd+q")), Some(Owner::MacOs));
        assert_eq!(
            reserved_by(Layout::Mac, &read("alt+cmd+h")),
            Some(Owner::MacOs)
        );
        assert_eq!(
            reserved_by(Layout::Mac, &read("cmd+alt+h")),
            Some(Owner::MacOs)
        );
        assert_eq!(reserved_by(Layout::Mac, &read("ctrl+c")), None);
        assert_eq!(reserved_by(Layout::Mac, &read("cmd+j")), None);
        // Windows reads Ctrl for Cmd.
        assert_eq!(
            reserved_by(Layout::Windows, &read("ctrl+c")),
            Some(Owner::Windows)
        );
        assert_eq!(
            reserved_by(Layout::Windows, &read("alt+f4")),
            Some(Owner::Windows)
        );
        assert_eq!(reserved_by(Layout::Omarchy, &read("ctrl+c")), None);
        for layout in Layout::ALL {
            for keys in reserved(layout) {
                assert!(spell(keys).is_ok(), "{keys}");
            }
        }
    }

    #[test]
    fn a_key_alone_will_do_only_as_a_function_key() {
        let read = |keys| spell(keys).unwrap();
        assert!(alone_will_do(&read("f5")));
        assert!(alone_will_do(&read("f19")));
        assert!(alone_will_do(&read("cmd+j")));
        assert!(alone_will_do(&read("ctrl+alt+j")));
        assert!(!alone_will_do(&read("shift+j")));
        assert!(!alone_will_do(&read("enter")));
        assert!(!alone_will_do(&read("j")));
        assert!(!alone_will_do(&read(":w")));
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
        let listed: Vec<Spelled> = VIM_INSERT.iter().map(|keys| spell(keys).unwrap()).collect();
        let strokes: Vec<Stroke> = listed
            .iter()
            .flat_map(|spelled| presses(Layout::Omarchy, spelled).concat())
            .collect();
        assert_eq!(strokes, vims, "VIM_INSERT is vim's four");
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
            ("shift+tab", "⇧Tab", "Shift+Tab"),
            ("cmd+alt+down", "⌥⌘↓", "Ctrl+Alt+Down"),
            ("cmd+1..9", "⌘1…9", "Ctrl+1…9"),
            ("cmd+,", "⌘,", "Ctrl+,"),
            ("cmd+'", "⌘'", "Ctrl+'"),
            ("cmd+.", "⌘.", "Ctrl+."),
            ("enter", "↩", "Enter"),
            ("backspace", "⌫", "Backspace"),
            ("tab", "Tab", "Tab"),
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
        // A format has its key by where it is read: the large editor has
        // no normal mode for Omarchy's `=`.
        let format = |layout, scope| keymap.key_in(layout, Command::Format, scope);
        assert_eq!(format(Layout::Omarchy, Scope::SqlEditor), "=");
        assert_eq!(format(Layout::Omarchy, Scope::CellEditor), "ctrl+shift+f");
        assert_eq!(format(Layout::Mac, Scope::CellEditor), "⇧⌘F");
        assert_eq!(format(Layout::Mac, Scope::Grid), "");
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
        // The brief of the Keyboard settings (open question 1): Explain
        // analyze has the chord, and Compare with previous run none.
        assert_eq!(mac(Command::ExplainAnalyze), "⌥⌘E");
        assert_eq!(mac(Command::ComparePreviousRun), "");
        assert_eq!(mac(Command::GoToConnectionWindow), "⌘1…9");
        assert_eq!(omarchy(Command::GoToConnectionWindow), "ctrl+shift+1..9");
        assert_eq!(omarchy(Command::EditCell), "i, enter");
        assert_eq!(omarchy(Command::ReplaceCell), "cc, s");
        assert_eq!(omarchy(Command::PasteNewRows), "\"+p, ctrl+shift+v");
        assert_eq!(omarchy(Command::DuplicateConnection), "yy p");
        assert_eq!(omarchy(Command::DuplicateRow), "yy p");
        assert_eq!(mac(Command::ExtendUp), "⇧↑");
        assert_eq!(mac(Command::ExtendDown), "⇧↓");
        assert_eq!(omarchy(Command::ExtendDown), "");
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
        // A row is yanked with two letters, and a copy of it put with a
        // third: each a key of its own, so the copy can be put again.
        assert_eq!(omarchy(Scope::Grid, "y"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "yy"), command(Command::YankRow));
        assert_eq!(omarchy(Scope::Grid, "p"), command(Command::PutRow));
        assert_eq!(omarchy(Scope::Grid, "P"), command(Command::PutRowAbove));
        assert_eq!(omarchy(Scope::Inspector, "yy"), command(Command::YankRow));
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
        assert_eq!(omarchy(Scope::Grid, "D"), command(Command::SetDefault));
        assert_eq!(omarchy(Scope::Grid, "R"), command(Command::Reload));
        assert_eq!(omarchy(Scope::Sidebar, "R"), command(Command::Reload));
        // The slash is the sidebar's filter there and the WHERE line here.
        assert_eq!(
            omarchy(Scope::Sidebar, "/"),
            command(Command::FilterObjects)
        );
        assert_eq!(omarchy(Scope::Grid, "/"), command(Command::WhereFilter));
        // A global letter is read in every scope but a prompt's.
        assert_eq!(omarchy(Scope::Grid, "?"), command(Command::Help));
        assert_eq!(omarchy(Scope::Prompt, "?"), Nothing);
        // The brackets step rows from the inspector, and from the rows
        // while it shows. Without it they begin other keys on the grid.
        assert_eq!(omarchy(Scope::Inspector, "]"), command(Command::NextRow));
        assert_eq!(omarchy(Scope::Grid, "]"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "]e"), command(Command::NextError));
        assert_eq!(omarchy(Scope::Grid, "]c"), Claimed);
        let inspector = |when: When| when == When::InspectorOpen;
        for scope in [Scope::Grid, Scope::Results] {
            assert_eq!(
                keymap.typed(Layout::Omarchy, scope, &inspector, "]"),
                command(Command::NextRow)
            );
            assert_eq!(
                keymap.typed(Layout::Omarchy, scope, &inspector, "["),
                command(Command::PreviousRow)
            );
        }
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
    fn a_visual_selection_has_its_own_keys() {
        let keymap = Keymap::default();
        let visual = |when: When| when == When::Visual;
        let nothing = |_: When| false;
        let grid = |holds: &dyn Fn(When) -> bool, typed: &str| {
            keymap.typed(Layout::Omarchy, Scope::Grid, holds, typed)
        };
        assert_eq!(grid(&nothing, "V"), Typed::Command(Command::SelectRows));
        // `d` alone is the whole key while a selection is on, and the
        // first of two without one.
        assert_eq!(grid(&visual, "d"), Typed::Command(Command::DeleteSelection));
        assert_eq!(grid(&nothing, "d"), Typed::Waiting);
        assert_eq!(grid(&nothing, "dd"), Typed::Command(Command::DeleteRow));
        assert_eq!(keymap.label(Layout::Omarchy, Command::LeaveVisual), "esc");
        assert_eq!(keymap.label(Layout::Mac, Command::SelectRows), "");
    }

    #[test]
    fn a_count_is_read_before_the_key_that_takes_one() {
        let keymap = Keymap::default();
        let nothing = |_: When| false;
        let read = |layout, scope, typed: &str| keymap.typed(layout, scope, &nothing, typed);
        let grid = |typed: &str| read(Layout::Omarchy, Scope::Grid, typed);
        assert_eq!(grid("3"), Typed::Waiting);
        assert_eq!(grid("3d"), Typed::Waiting);
        assert_eq!(grid("3dd"), Typed::Counted(Command::DeleteRow, 3));
        assert_eq!(grid("12dd"), Typed::Counted(Command::DeleteRow, 12));
        assert_eq!(
            grid("1234567dd"),
            Typed::Counted(Command::DeleteRow, 999_999)
        );
        // Before any other key the count is dropped, and the key acts once.
        assert_eq!(grid("3j"), Typed::Command(Command::MoveRow));
        // Nought begins no count.
        assert_eq!(grid("0"), Typed::Nothing);
        assert_eq!(grid("0dd"), Typed::Nothing);
        // Only the terminal look's grid of a table reads one.
        assert_eq!(read(Layout::Omarchy, Scope::Results, "3"), Typed::Nothing);
        assert_eq!(read(Layout::Omarchy, Scope::Inspector, "3"), Typed::Nothing);
        assert_eq!(read(Layout::Mac, Scope::Grid, "3"), Typed::Nothing);
        assert_eq!(count_of("12dd"), (Some(12), "dd"));
        assert_eq!(count_of("dd"), (None, "dd"));
        assert!(is_count("12") && !is_count("12d") && !is_count("") && !is_count("0"));
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
        for line in ["commit", "rollback", "42"] {
            assert_eq!(ex(Scope::SqlEditor, line), Claimed, "{line}");
        }
        // Built, from any tab of the window.
        assert_eq!(ex(Scope::SqlEditor, "history"), command(Command::History));
        assert_eq!(
            ex(Scope::Grid, "history clear"),
            command(Command::ClearHistory)
        );
        assert_eq!(
            ex(Scope::Grid, "keys default"),
            command(Command::KeysDefault)
        );
        assert_eq!(
            ex(Scope::SqlEditor, "keys cheatsheet"),
            command(Command::KeysCheatsheet)
        );
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
    /// The keys of a foreign key's label, as its brief has them
    /// (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`,
    /// test 15): the chord that opens the chooser from the picker, and the
    /// chooser's own.
    #[test]
    fn the_label_choosers_keys_are_the_briefs() {
        let keymap = Keymap::default();
        let chords = |layout, command, scope| -> Vec<&str> {
            keymap.chords(layout, command, scope).collect()
        };
        let (editor, prompt) = (Scope::CellEditor, Scope::Prompt);
        use Command::{
            ChangeLabelColumn, ChooseLabel, CloseLabelChooser, NextLabel, PreviousLabel,
        };
        for layout in [Layout::Mac, Layout::Windows] {
            assert_eq!(chords(layout, ChangeLabelColumn, editor), ["cmd+l"]);
            assert_eq!(chords(layout, NextLabel, prompt), ["down"]);
            assert_eq!(chords(layout, PreviousLabel, prompt), ["up"]);
        }
        let omarchy = Layout::Omarchy;
        assert_eq!(chords(omarchy, ChangeLabelColumn, editor), ["ctrl+l"]);
        assert_eq!(chords(omarchy, NextLabel, prompt), ["j", "ctrl+n", "down"]);
        assert_eq!(
            chords(omarchy, PreviousLabel, prompt),
            ["k", "ctrl+p", "up"]
        );
        for layout in Layout::ALL {
            assert_eq!(chords(layout, ChooseLabel, prompt), ["enter"]);
            assert_eq!(chords(layout, CloseLabelChooser, prompt), ["esc"]);
            // Kept for a label of two columns: claimed, and no key yet.
            assert!(chords(layout, Command::AddLabelPart, prompt).is_empty());
        }
        // The chord is insert mode's: with the keyboard out of a field
        // Omarchy's `ctrl+l` is the pane on the right.
        let now = |normal| -> Vec<&str> {
            let command = ChangeLabelColumn;
            keymap
                .chords_now(omarchy, command, editor, normal)
                .collect()
        };
        assert_eq!(now(false), ["ctrl+l"]);
        assert!(now(true).is_empty());
        // In the chooser `j` and `k` are keys. Under the picker alone
        // they are letters of the search, and nothing here reads them.
        let choosing = |when: When| when == When::LabelChooser;
        let picking = |when: When| when == When::KeyPicker;
        let command = Typed::Command;
        assert_eq!(
            keymap.typed(omarchy, prompt, &choosing, "j"),
            command(NextLabel)
        );
        assert_eq!(
            keymap.typed(omarchy, prompt, &choosing, "k"),
            command(PreviousLabel)
        );
        assert_eq!(keymap.typed(omarchy, editor, &picking, "j"), Typed::Nothing);
        assert_eq!(keymap.typed(omarchy, editor, &picking, "k"), Typed::Nothing);
        assert_eq!(
            keymap.typed(Layout::Mac, prompt, &choosing, "j"),
            Typed::Nothing
        );
    }

    /// The keys of the Structure view's door to a foreign key's label, as
    /// its brief has them
    /// (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`,
    /// test 15). They are the grid's keys while the Structure view shows.
    #[test]
    fn the_structure_views_label_keys_are_the_briefs() {
        let keymap = Keymap::default();
        let grid = Scope::Grid;
        let chords =
            |layout, command| -> Vec<&str> { keymap.chords(layout, command, grid).collect() };
        use Command::{
            LabelAutomatic, LabelColumn, LabelKeysOnly, MoveInStructure, OpenLabelChooser,
            UseAsLabel,
        };
        for layout in [Layout::Mac, Layout::Windows] {
            assert_eq!(chords(layout, OpenLabelChooser), ["cmd+l"]);
            assert_eq!(chords(layout, UseAsLabel), ["alt+cmd+l"]);
            assert_eq!(chords(layout, MoveInStructure), ["up", "down"]);
            // A button and the chooser do these: no key.
            for command in [LabelAutomatic, LabelKeysOnly, LabelColumn] {
                assert!(chords(layout, command).is_empty(), "{command:?}");
            }
        }
        let omarchy = Layout::Omarchy;
        assert_eq!(chords(omarchy, OpenLabelChooser), [":label"]);
        assert_eq!(chords(omarchy, UseAsLabel), ["gl"]);
        assert_eq!(chords(omarchy, LabelAutomatic), [":label auto"]);
        assert_eq!(chords(omarchy, LabelKeysOnly), [":label none"]);
        assert_eq!(chords(omarchy, LabelColumn), [":label {column}"]);
        // Every one of them holds only while the Structure view shows.
        let structure = |when: When| when == When::Structure;
        let rows = |_: When| false;
        let command = Typed::Command;
        let typed = |holds: &dyn Fn(When) -> bool, keys| keymap.typed(omarchy, grid, holds, keys);
        assert_eq!(typed(&structure, "gl"), command(UseAsLabel));
        assert_eq!(typed(&rows, "gl"), Typed::Nothing);
        // `j` and `k` are the view's there, and the rows' elsewhere.
        assert_eq!(typed(&structure, "j"), command(MoveInStructure));
        assert_eq!(typed(&rows, "j"), command(Command::MoveRow));
        // The grid's own `g` keys stay.
        assert_eq!(typed(&structure, "gd"), command(Command::OpenReferencedRow));
        let ex = |holds: &dyn Fn(When) -> bool, line| keymap.ex(omarchy, grid, holds, line);
        assert_eq!(ex(&structure, "label"), command(OpenLabelChooser));
        assert_eq!(ex(&structure, "label auto"), command(LabelAutomatic));
        assert_eq!(ex(&structure, "label none"), command(LabelKeysOnly));
        assert_eq!(ex(&structure, "label legal_name"), command(LabelColumn));
        assert_eq!(ex(&rows, "label"), Typed::Nothing);
        assert_eq!(ex(&rows, "label legal_name"), Typed::Nothing);
        // The grid's lines are still its own there.
        assert_eq!(ex(&structure, "w"), command(Command::SaveChanges));
        // The word a line gives the command that takes one.
        let given = |line| keymap.ex_argument(omarchy, LabelColumn, grid, line);
        assert_eq!(given("label legal_name"), Some("legal_name"));
        assert_eq!(given("label  first name "), Some("first name"));
        assert_eq!(given("label"), None);
        assert_eq!(given("label "), None);
        assert_eq!(given("labels x"), None);
        assert_eq!(
            keymap.ex_argument(omarchy, OpenLabelChooser, grid, "label x"),
            None
        );
    }

    #[test]
    fn two_keys_that_do_opposite_things_are_written_as_one_hint() {
        let keymap = Keymap::default();
        let moves = (Command::NextLabel, Command::PreviousLabel);
        let pair = |layout, nth| keymap.pair(layout, moves, Scope::Prompt, nth);
        // Letters either side of a stroke; what is held, once.
        assert_eq!(pair(Layout::Omarchy, 0).as_deref(), Some("j/k"));
        assert_eq!(pair(Layout::Omarchy, 1).as_deref(), Some("ctrl+n/p"));
        assert_eq!(pair(Layout::Omarchy, 2).as_deref(), Some("down/up"));
        // No such key: no hint.
        assert_eq!(pair(Layout::Omarchy, 3), None);
        assert_eq!(pair(Layout::Mac, 1), None);
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
