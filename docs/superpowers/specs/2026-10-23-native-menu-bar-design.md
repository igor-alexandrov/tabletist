# A native menu bar on macOS and Windows

## What happens now

winit builds the macOS app menu (About, Services, Hide, Hide Others, Show
All, Quit) before the window exists. `crates/tabletist-appkit` puts four
items into it through objc2: About Tabletist and Settings… in the app menu
(`about.rs`, `settings.rs`), and History and Quick Open in a View menu it
makes after the app menu (`view.rs`), all pointing at one `MenuTarget`
(`target.rs`). `src/macos.rs` wraps each as `AboutMenu`, `SettingsMenu`,
`HistoryMenu` and `QuickOpenMenu`, `src/entrypoint.rs` attaches them when
the window is created, and `Window::logic` turns a chosen item into an
`Action`. Only Settings has a key equivalent (`⌘,`): AppKit takes a key
equivalent before the window sees the key, and History's `⌘Y` must stay
egui's redo inside a text field. Windows and Linux have no menu bar.

The commands are reached through the keymap (`src/ui/keys.rs`) and Quick
Open. Quick Open runs the 29 commands of `RUNNABLE` (`src/app/commands.rs`)
through `App::command_actions`, which says what a command does in a window
now, or that it does not apply. A few commands' keys do more than Quick
Open's row (`RUNNABLE`'s doc comment lists them: `⌘R` on a table's rows
reloads the rows, `⌘F` in the filter bar closes it).

## Decision

A native menu bar on macOS (the global menu bar) and Windows (the menu
strip under the title bar), holding Quick Open's commands and the items
each platform's apps have. Linux gets none: muda attaches only to a GTK
window, not to winit's, and Omarchy has no menu bar.

It is built with muda, from one menu model in Rust, on both platforms. The
hand-written AppKit menu code goes. Rejected: extending the AppKit code and
writing Win32 menus by hand (two native layers to write and test, each with
its own unsafe code, and a model to keep in step between them), and muda on
Windows only (two ways of building the same menus).

**A key stays the keymap's.** The menu shows each command's key, but a key
press does what it does today, wherever it is pressed: on macOS, where
AppKit takes the key for a menu item, the app gives egui that same key
press back, and `keys.rs` reads it as it reads any key; on Windows the
menu only writes the key, and the press never leaves egui. A click on an
item runs the command as Quick Open runs it. So a menu adds a way to click
a command and changes no key.

## The menus

Names are fixed msgids in title case, held in `src/menu.rs` and translated
through `i18n` when the bar is built and again when the locale changes.
"(macOS)" and "(Windows)" after a group of items mark that whole group as
one platform's; `|` is a separator. On Windows each menu's title carries a
mnemonic (`&File`, `&Edit`, `&View`, `&Query`, `&Window`, `&Help`); muda
drops the `&` on macOS.

| Menu | Items |
|---|---|
| Tabletist (macOS) | About Tabletist \| Settings… \| Services \| Hide Tabletist, Hide Others, Show All \| Quit Tabletist |
| File | New Connection, New SQL Query, Connections \| Edit Connection, Close Connection \| Close Tab \| Settings… (Windows) \| Exit (Windows) |
| Edit | Undo, Redo \| Cut, Copy, Paste, Select All |
| View | Show Sidebar, Show Row Panel, Filter Rows \| History, Quick Open, Agents \| Larger Text, Smaller Text, Default Text Size |
| Query | Run All, Cancel Query, Format SQL, Make Tab Read-Only \| Review Pending SQL, Save Changes, Discard All Changes \| Reload Schema, Switch Database |
| Window | Minimize, Zoom (macOS) \| Next Tab, Previous Tab, Next Connection, Previous Connection \| Bring All to Front (macOS) |
| Help | Keyboard Shortcuts \| About Tabletist (Windows) |

Three items are toggles and say what a click does now, as Quick Open's
rows do: Show Sidebar / Hide Sidebar (`ToggleSidebar`), Show Row Panel /
Hide Row Panel (`ToggleInspector`), Make Tab Read-Only / Make Tab
Read-Write (`ToggleSqlMode`). Each pair is two msgids; the state below
says which one shows, and the bar sets the title when it changes. A greyed
toggle shows its first title (Show Sidebar, Show Row Panel, Make Tab
Read-Only).

The other `RUNNABLE` commands map one to one: New Connection
(`NewConnection`), New SQL Query (`NewSqlTab`), Connections
(`Connections`), Edit Connection, Close Connection, Close Tab, Settings…,
Filter Rows (`FilterBar`), History, Agents (`ShowAgents`), the three text
sizes, Run All, Cancel Query, Format SQL (`Format`), Review Pending SQL
(`ReviewSql`), Save Changes, Discard All Changes (`DiscardChanges`), Reload
Schema (`Reload`), Switch Database, Next and Previous Tab and Connection,
Keyboard Shortcuts (`Help`). Each `RUNNABLE` command is in exactly one
item on each platform. Agents is in the model wherever
`endpoint::AVAILABLE` is true, and left out elsewhere, as Quick Open
leaves it out.

Quick Open (`Action::OpenQuickOpen`), About (`Action::ShowAbout`), Exit and
the Edit items are not `RUNNABLE` commands and have item kinds of their
own (below).

muda's predefined items are used where they reach the app through a path
it already guards:

- macOS Services, Hide, Hide Others, Show All, Minimize, Zoom and Bring All
  to Front: AppKit's own. muda turns AppKit's automatic enabling off
  (`setAutoenablesItems(false)`), so these are always enabled, as winit's
  are today.
- macOS Quit: muda sends `terminate:`, which the winit fork's
  `macos-quit-as-close` turns into a close request, so `App::hold_close`
  still asks about pending changes.

And are not used where they do not:

- Windows Exit: muda's posts `PostQuitMessage`, which ends the loop
  without a close request. Exit is the app's own item and sends the
  viewport a close request (`ViewportCommand::Close`), as the window's
  close button does, so `hold_close` asks.
- About: muda's opens AppKit's panel (or its own dialog on Windows), which
  the app replaced with its own About dialog (`7cbbb49`). About is the
  app's own item, giving `Action::ShowAbout`.
- Edit: muda's are always enabled (automatic enabling is off), so on macOS
  they would take `⌘C` and the rest and send `copy:` to a responder winit
  does not have; on Windows they fake Ctrl+C with `SendInput`. The Edit
  items are the app's own (below).

On macOS the View menu gets no Enter Full Screen item of ours: AppKit adds
one to a menu titled View by itself, and Dictation and Emoji to Edit. The
Window menu is registered with `set_as_windows_menu_for_nsapp` (AppKit
lists the window there) and the Help menu with `set_as_help_menu_for_nsapp`
(AppKit adds its search field). **The plan checks** what AppKit inserts,
so nothing shows twice.

## The model (`src/menu.rs`)

Plain data, with no platform code:

```rust
pub enum Item {
    /// A `RUNNABLE` command, clicked through `App::command_actions`.
    Command(Command),
    /// The app's own items that are not `RUNNABLE` commands.
    About,
    QuickOpen,
    Exit,
    Edit(EditItem), // Undo, Redo, Cut, Copy, Paste, SelectAll
    /// A muda predefined item (Services, Hide, Quit, Minimize, ...).
    Native(NativeItem),
    Separator,
}

pub struct Menu {
    pub title: &'static str, // an msgid
    pub items: Vec<Item>,
}

pub fn bar(platform: Platform) -> Vec<Menu>;
```

`Platform` is `MacOs` or `Windows`, matched with every variant named, as
`Driver` is. `bar` is a function of the platform only, so the model is
tested on any host. An item's id for the crate is its position in `bar`
(menu index, item index), which `src/menu.rs` maps back to the `Item`.

### Which items are enabled

`App::menu_state(&self) -> MenuState` says, for every item, whether it is
enabled and, for the three toggles, which title shows. It needs only
`App` (no egui context) and is computed after each frame's `App::apply`.

- A `Command` item is enabled when `command_actions(active tab, command)`
  is `Some` (Quick Open would offer it now) and, while a dialog is open
  (any `Dialog`, Quick Open and the shortcuts dialog included), only when
  one of the command's bindings holds in `Scope::Prompt`, the scope a
  dialog's keys are read in (`ANYWHERE` binds `Global` and `Prompt`
  together): the text sizes
  stay enabled under a dialog, as their keys work there; New SQL Query
  does not. Whether a command is read there is a fact of `BINDINGS` (an
  override cannot add a `Prompt` binding), so this needs no focus. One
  exception keeps a key working: Settings… stays enabled under the
  shortcuts dialog, whose button and the `ShowSettings` reducer open
  Settings from there (`None | Help`); today `⌘,` reaches it there through
  the always-enabled AppKit item, and `keys::handle` does not run under a
  dialog, so a greyed item would lose the key.
- About, Quick Open and Exit are always enabled. Their actions decide, as
  today: the `ShowAbout` reducer does nothing while a dialog is worked in,
  and Exit's close request goes through `hold_close`.
- Edit items are always enabled. What a chosen one does depends on where
  the keyboard is, and egui decides that (below).
- Native items are always enabled (above).

The platform side keeps the last state it applied and calls `set_enabled`
or `set_title` only for items that changed, so a still window costs a
comparison of a few dozen booleans per frame.

Enabling is for clicks. For keys it does not matter: a disabled item's key
goes on to the window as it does today, and an enabled item's key comes
back to egui as the same press (below). **The plan proves first** that
AppKit skips a disabled item's key equivalent.

## Shortcuts

`keymap.rs` gains a structured key, in the layout the look uses, with the
person's overrides from `settings.toml` applied:

```rust
pub struct MenuKey {
    pub key: egui::Key, // logical, as the keymap reads it
    pub command: bool,  // ⌘ on macOS, Ctrl on Windows
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,     // the Control key on macOS; unused on Windows
}

impl Keymap {
    /// The single chord a menu can show for `command`, if it has one.
    pub fn menu_key(&self, command: Command, layout: Layout) -> Option<MenuKey>;
    /// The chord of an Edit item, from the keys the keymap reserves for
    /// the text fields (`reserved()`), so no key is spelled outside it.
    pub fn edit_key(&self, item: EditItem, layout: Layout) -> Option<MenuKey>;
}
```

A command shows the chord its bindings share: `menu_key` collects the
distinct chords of all its lines (Cancel Query and Format SQL each have two
lines with the same chord, which is one chord) and gives `None` when there
is no chord, more than one, a sequence (vim's `g g`), or a count. Help's
`?` is a typed character, not a chord, so Keyboard Shortcuts shows none.
Quick Open shows `FindObject`'s chord, the key that opens it. About and
Exit show none.

`EditItem` lives in `keymap.rs` beside `Command` (the Edit items' keys are
the keymap's), and `src/menu.rs` uses it.

`tabletist-menubar` translates a `Shortcut` (a `MenuKey`, field for field)
to muda's logical `KeyAccelerator` (`keyboard_types::Key`), not to a
physical `Code`: AppKit matches a key equivalent by its character, so ⇧⌘[
and ⇧⌘] (Previous Tab, Next Tab) and layouts other than US match as the
keymap reads them. **The plan tests** those two, ⌃⇥ (Next Connection),
and a non-US layout.

When the keymap or the look changes (a settings reload, the Keyboard page),
every item's accelerator is set again.

## Running an item

muda's `MenuEvent::set_event_handler` runs on the main thread, inside the
menu's action. It records the item's id and how it was chosen, then calls
`egui::Context::request_repaint` so an idle window wakes:

```rust
pub enum Via {
    Click,
    /// Its key equivalent.
    Key,
}
```

muda keeps one event handler for the process (a `OnceLock`: only the first
`set_event_handler` takes effect), so the crate installs it once and routes
each `MenuId` to the bar that made it; a second `attach` (a test) replaces
the route, not the handler.

On macOS the crate reads `NSApplication::currentEvent` in the handler. It
is `Via::Key` only when that event is a key-down whose
`charactersIgnoringModifiers` and modifier flags match the item's own key
equivalent. Anything else is a click: choosing from an open menu with
Return, the keyboard's way into the menu bar (Ctrl+F2), the Help menu's
search, VoiceOver. These calls are in objc2-app-kit's safe API
(`NSApplication::currentEvent`, `NSEvent::type`, `modifierFlags`,
`charactersIgnoringModifiers`, behind the `NSEvent` feature muda already
turns on). On Windows every choice is a
click (the menu takes no key).

eframe calls `raw_input_hook` before the frame's `logic`, so the two take
their own: `raw_input_hook` (it has `&mut self`) drains every choice that
replays (those made by a key, and clicked Undo, Redo, Select All, Cut and
Copy) into that frame's input, and `Window::logic` takes the other clicks.
Whatever a choice queues for a later frame (a `ViewportCommand`) is
followed by `request_repaint`.

A **replay** gives egui the events a real press of the chord gives it
through egui-winit, with the modifiers egui-winit reports for that chord
on the platform held for that frame (`command` and `mac_cmd` on macOS,
`command` and `ctrl` on Windows; `keys.rs`'s `held_is` matches these):
for most chords an `Event::Key` (`physical_key: None`, `pressed: true`);
for Cut and Copy the `Event::Cut` or `Event::Copy` egui-winit makes of
`⌘X` and `⌘C`.

egui decides what is a repeat itself, from the keys it holds down: a press
of a key it holds is a repeat, and a key stops being held only on a
release. So the release matters, and the two paths differ:

- **By a key**: the real key is down, and winit sends its release (it
  forwards a Cmd key-up to the window even after a key equivalent). The
  replayed press goes into the frame's input **before** any release of
  the same key already there, so a quick tap is press then release, not
  the other way round. A held chord's AppKit repeats replay presses of a
  key egui still holds, which egui marks as repeats: Run All runs once.
- **By a click**: no key is down and no release will come, so the replay
  adds a release right after its press, in the same frame. Readers of that
  frame see the press; no key is left held.

**The plan checks** the release during IME composition, when winit drops
key-ups.

- **Chosen by its key** (macOS only): the item's `Shortcut` is replayed,
  so `keys.rs` and the text fields read it as they read the key today. A
  held chord that runs once (Run All, Format SQL, Make Tab Read-Only)
  still runs once, `⌘R` on a table's rows still reloads the rows, and `⌘Y`
  in a text field is still egui's redo. Paste is the one exception
  (below).
- **Clicked**, by kind:
  - `Command(c)`: the actions of `command_actions(active tab, c)`, pushed
    on `app.actions`. A command that stopped applying between the frame
    that enabled it and the click does nothing.
  - `About`: `Action::ShowAbout`. `QuickOpen`: `Action::OpenQuickOpen`.
  - `Exit`: `ViewportCommand::Close`.
  - `Edit(e)`: as by its key.
- **Edit items**, by key or click: Undo, Redo, Select All, Cut and Copy
  replay their `edit_key` chord, as above. So a clicked Copy reaches the
  grid too: the grid's copy (`press_of` in `keys.rs`) takes a clipboard
  event as `cmd+c` only with Cmd held in its frame, and a replay holds it.
  `press_of`'s rule that a copy with nothing held (a keyboard's Copy key)
  is no chord stays as it is. Paste sends `ViewportCommand::RequestPaste`,
  which eframe answers in the next frame with `Event::Paste` holding the
  clipboard's text, as egui-winit does for `⌘V`: the app has no clipboard
  reader of its own. That `Event::Paste` comes with no modifiers held,
  which text fields do not need; the grid's paste (`PasteCells`) is not
  built yet, and when it is, it reads a menu's paste the same way. Text
  fields and the grid answer each as they answer the key; where nothing
  would, nothing happens.

Settings is the `Command::Settings` item, so `⌘,` keeps opening the window
once whether AppKit or the keymap takes it.

## The crate (`crates/tabletist-menubar`)

A workspace member that turns a description of the bar into a muda menu,
behind a safe API. It cannot name `Command` or `App` (they are the main
crate's), so it is generic over the app's item id, and `src/menu.rs`
describes the bar to it from the model above:

```rust
pub enum Entry<T> {
    Item { id: T, title: String, key: Option<Shortcut> },
    Native(Native), // Services, Hide, HideOthers, ShowAll, Quit, Minimize, ...
    Separator,
}

pub struct Shortcut {
    pub key: egui::Key,
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

pub struct MenuBar<T> { /* muda::Menu, each id's muda item */ }

impl<T: Copy + Eq + Hash + Send + 'static> MenuBar<T> {
    /// Builds the bar and attaches it: to NSApp on macOS, to the window
    /// on Windows. `None` off those platforms or if attaching fails.
    pub fn attach(window: &impl HasWindowHandle, menus: &[(String, Vec<Entry<T>>)],
                  wake: impl Fn() + Send + Sync + 'static) -> Option<Self>;
    pub fn set_enabled(&mut self, id: T, enabled: bool);
    pub fn set_shortcut(&mut self, id: T, key: Option<Shortcut>);
    pub fn set_title(&mut self, id: T, title: &str);
    /// The items chosen since the last call, oldest first.
    pub fn chosen(&self) -> Vec<(T, Via)>;
}
```

It depends on `egui` for `egui::Key` alone (egui is already built; the
crate draws nothing) and on `raw-window-handle`, which eframe already
builds.

- macOS: `Menu::init_for_nsapp`, a safe call. winit's default menu is
  turned off with `EventLoopBuilderExtMacOS::with_default_menu(false)`
  through eframe's `NativeOptions::event_loop_builder`, so nothing builds a
  menu muda then replaces. **The plan checks** that the app menu's title
  and Services behave as before with winit's menu off. Reading
  `currentEvent` goes through objc2-app-kit's safe API; if a call needs
  `unsafe`, it joins the module below.
- Windows: `Menu::init_for_hwnd` is `unsafe` (it subclasses the window).
  It sits in one module where the crate lowers `unsafe_code` to `deny` and
  allows it, with a SAFETY note: the handle is the live window eframe
  created, on its thread, and eframe drops the app (and the bar) before
  the window. The app installs no accelerator table and no message hook.
- `muda` is a dependency only for `cfg(any(target_os = "macos",
  windows))`, with `default-features = false`: its GTK crates are optional
  and Linux-only, so Linux builds neither muda nor GTK. Off those
  platforms `attach` returns `None`.

`Cargo.toml` explains muda: the menu bar on macOS and Windows, built from
one model. On both platforms it adds `keyboard-types`,
`crossbeam-channel`, `dpi` and `thiserror` where they are not in the lock
already; on macOS its objc2 crates are ones winit builds, and `png`; on
Windows it adds `windows-sys` features. The plan lists what `Cargo.lock`
gains.

### Windows: what attaching a menu changes

- `SetMenu` after the window exists makes the client area shorter by the
  menu's height. **The plan checks** that the size eframe persists does not
  shrink by that much on every launch, and corrects it if it does.
- With a menu, winit passes a lone Alt (and F10) on to `DefWindowProc`,
  which opens the menu bar, as in any Windows app. No Windows binding is
  Alt alone; **the plan checks** that none is F10.

### What goes

`about.rs`, `settings.rs`, `view.rs` and `target.rs` in `tabletist-appkit`
and their tests; `AboutMenu`, `SettingsMenu`, `HistoryMenu` and
`QuickOpenMenu` in `src/macos.rs` and their fields and polling in
`src/entrypoint.rs`. `tabletist-appkit` keeps `fonts.rs`, and its
dependency line in `Cargo.toml` says so. The `NSMenu` and `NSMenuItem`
features it no longer needs come out of its `Cargo.toml`.

## Order of work

macOS first, as one change that replaces today's four items: the model,
the keymap's `MenuKey`, the crate on macOS, key replay, Edit items, and
the removals. Windows second: `init_for_hwnd`, Exit, About in Help,
mnemonics, and the two checks above. Each lands passing the checks.

## Docs

- `AGENTS.md`: the unsafe paragraph names `tabletist-menubar` beside
  `tabletist-appkit` and `tabletist-sqlite-ffi`, and says a key the menu
  takes goes back to the keymap.
- `_guide/macos.md`: the menu bar, its menus, and that a greyed item is a
  command that does not apply now.
- `_reference/keyboard-shortcuts.md`: a line that the macOS and Windows
  menus show the keys of this page, and your own.
- No Windows page exists. If `_guide/what-is-tabletist.md` or
  `getting-started.md` lists what each platform has, it gains the Windows
  menu bar.
- No screenshot: the site's scenes draw the window's content, not the
  system's menu bar.

## Tests

- `src/menu.rs`, headless: every `RUNNABLE` command is in exactly one item
  on each platform; every item has a title; no menu starts or ends with a
  separator or has two in a row; Agents is there exactly where
  `endpoint::AVAILABLE` is.
- Enabling, through `src/testing.rs`: a `Command` item is greyed where
  Quick Open hides its command (Switch Database with one database) and
  under a dialog unless its key is read anywhere (Larger Text stays
  enabled, New SQL Query does not); enabled otherwise. The toggles' titles
  follow the sidebar, the row panel and the tab's mode.
- Shortcuts, generated from `BINDINGS`: for every item, the key it shows
  is `menu_key`'s (or `edit_key`'s), `menu_key` agrees with the bound
  chord in both layouts, a command with two distinct chords shows none,
  and an override in `settings.toml` changes the item's key.
- Key replay: a `Via::Key` choice of Run All gives `keys.rs` one press and
  runs once, a held chord's repeats do not run it again, two quick taps
  (each release already in the frame's input) run it twice, and a clicked
  Undo leaves no key held; `⌘R` replayed on a
  table's rows reloads the rows; `⌘Y` replayed in a text field is egui's
  redo, not History.
- Clicks: a clicked `Command` item pushes exactly the actions Quick Open
  would; Exit with pending changes asks, as the close button does; a
  choice made from an open menu with Return is a click, and Switch
  Database (no key) chosen that way opens its menu.
- Edit items: a clicked Copy copies a text field's selection and the
  grid's cells; a clicked Paste pastes into a text field; `press_of`'s
  test that a copy with nothing held is no chord still passes.
- Settings… under the shortcuts dialog is enabled, and `⌘,` opens Settings
  from there.
- `tabletist-menubar` on macOS: a test without a harness (AppKit's menus
  belong to the main thread) builds the bar, finds each menu's title,
  chooses an item, and checks that a disabled item's key equivalent is
  not performed. On Windows a test builds the bar on a hidden window and
  chooses an item. CI's test matrix runs both (`macos-latest`,
  `windows-latest`). What needs eyes (the menus as drawn, `⌘Q` with
  pending changes, a non-US layout) is reported as not run until someone
  runs it on a Mac and on Windows.
