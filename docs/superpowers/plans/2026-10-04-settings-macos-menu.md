# Settings: the macOS app menu item Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On macOS the app menu has "Settings…" under About, with `⌘,`, and choosing it opens the Settings window.

**Architecture:** winit builds the app menu and has no Settings item. `tabletist-appkit`, the one crate allowed `unsafe`, gains `SettingsItem::insert`, which puts the item and a separator after the menu's first separator and calls a closure when it is chosen; dropping it takes both out. It shares the Objective-C target class that `about.rs` defines today, moved to a file of its own and renamed for what it is. `src/macos.rs` wraps the item as `SettingsMenu`, as `AboutMenu` wraps `AboutItem`, and `Window::logic` turns a chosen item into `Action::ShowSettings`.

**Tech Stack:** Rust 2024, objc2 0.6, objc2-app-kit 0.3 and objc2-foundation 0.3 (macOS only), egui. No new dependency and no new feature of one.

**Spec:** `docs/superpowers/specs/2026-10-03-settings-general-design.md`, "Opening" and the last item of "Testing". This is what is left of step 4 of its "Delivery", and it replaces Task 7 of `docs/superpowers/plans/2026-10-03-settings-window.md`, which was written without compiling.

**What was checked before this plan was written:** every block of code below for `tabletist-appkit`, and the `SettingsMenu` wrapper, was compiled for `aarch64-apple-darwin` with `clippy --all-targets -- -D warnings` in a scratch copy, and formatted with `cargo fmt`. None of it was run: nothing here can run AppKit. CI's macOS job is the first run.

---

## Rules of the repository that bite here

- The workspace forbids `unsafe`. Only `crates/tabletist-appkit` lowers that to `deny`, and each file that needs it says `#![allow(unsafe_code)]` with a reason, and each use has a `SAFETY:` note.
- No em dashes anywhere, in code, comments, tests or docs.
- One topic per commit. Every commit passes all four on Linux:
  `~/.cargo/bin/cargo fmt --all --check`,
  `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `~/.cargo/bin/cargo test --locked --workspace --all-targets`,
  `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps`.
  On Linux the crate is empty and its two tests are empty `main`s, so those four say nothing of the macOS code. The check below does.
- The macOS type-check, after every change to the crate:
  `CARGO_TARGET_DIR=target/xcheck ~/.cargo/bin/cargo clippy --locked --offline -p tabletist-appkit --all-targets --target aarch64-apple-darwin -- -D warnings`
  Expected: `Finished`. Remove `target/xcheck` when the task is done. It type-checks; it does not run.
- The whole app does not cross-compile here (a dependency's build script needs Apple's C toolchain), so `src/macos.rs` and the `cfg(target_os = "macos")` lines of `src/entrypoint.rs` are checked in an extract (Task 3).
- `Cargo.toml` of the crate keeps its dependencies and their features as they are. `Cargo.lock` does not change.
- Subagents do not commit: they `git add` what the task changed and report. The controller commits, signed.
- Cargo builds use the worktree's own `target/`. Never a target directory under `/tmp`.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-appkit/src/target.rs` | new: `MenuTarget`, the class a menu item of ours points at, moved out of `about.rs` |
| `crates/tabletist-appkit/src/about.rs` | uses `MenuTarget` and its selector |
| `crates/tabletist-appkit/src/settings.rs` | new: `SettingsItem` |
| `crates/tabletist-appkit/src/lib.rs` | the two modules, and `pub use settings::SettingsItem` |
| `crates/tabletist-appkit/tests/about_menu.rs` | the selector's new name |
| `crates/tabletist-appkit/tests/settings_menu.rs` | new: the item's test, with its own `main` |
| `crates/tabletist-appkit/Cargo.toml` | the second `[[test]]`, and the comment that names where `unsafe` is allowed |
| `src/macos.rs` | `SettingsMenu` |
| `src/entrypoint.rs` | the item is attached at the start, and a chosen item becomes `Action::ShowSettings` |
| `docs/superpowers/specs/2026-10-03-settings-general-design.md` | "Opening" as it was built |

---

### Task 1: The menu target, in a file of its own

A refactoring: nothing behaves differently. The class `about.rs` defines is not About's alone once a second item uses it, and its selector should say what it is.

**Files:**
- Create: `crates/tabletist-appkit/src/target.rs`
- Modify: `crates/tabletist-appkit/src/about.rs`, `crates/tabletist-appkit/src/lib.rs`, `crates/tabletist-appkit/tests/about_menu.rs`, `crates/tabletist-appkit/Cargo.toml`

- [ ] **Step 1: Change the test to the new selector**

In `crates/tabletist-appkit/tests/about_menu.rs`, the one assertion on the action the item is given:

```rust
    assert_eq!(about.action(), Some(sel!(menuItemChosen:)));
```

This cannot be seen to fail from Linux. It is what would fail on macOS if `about.rs` kept `showAbout:`.

- [ ] **Step 2: Create `target.rs`**

```rust
//! An object that calls a closure when a menu item is chosen: what a menu
//! item of ours points at.

// Defining an Objective-C class is a message the compiler cannot check.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};

pub(crate) struct Ivars {
    chosen: Box<dyn Fn()>,
}

define_class!(
    // SAFETY: NSObject has no rules for its subclasses, and `MenuTarget`
    // does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    pub(crate) struct MenuTarget;

    impl MenuTarget {
        // SAFETY: an action takes its sender (an object or nil) and returns
        // nothing.
        #[unsafe(method(menuItemChosen:))]
        fn menu_item_chosen(&self, _sender: Option<&AnyObject>) {
            (self.ivars().chosen)();
        }
    }
);

impl MenuTarget {
    pub(crate) fn new(mtm: MainThreadMarker, chosen: Box<dyn Fn()>) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Ivars { chosen });
        // SAFETY: NSObject's `init` takes nothing and returns the object.
        unsafe { msg_send![super(this), init] }
    }
}
```

- [ ] **Step 3: Have `about.rs` use it**

Remove `Ivars`, the `define_class!` block and `impl Target` from `about.rs`. Its imports become:

```rust
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSMenuItem};
use objc2_foundation::NSString;

use crate::target::MenuTarget;
```

The comment over `#![allow(unsafe_code)]` loses what moved: "Wiring a target and an action are messages the compiler cannot check." In the rest of the file, `Target` becomes `MenuTarget` (the field's type and `MenuTarget::new(mtm, Box::new(chosen))`), and `showAbout:` becomes `menuItemChosen:` in the `SAFETY` note and in `sel!(..)`. `Drop` is unchanged.

`lib.rs`, under the same `cfg` as `mod about`:

```rust
#[cfg(target_os = "macos")]
mod target;
```

`Cargo.toml`: the comment that says `unsafe_code` is allowed "only in `about.rs` and its test" now names the files that have it: `about.rs`, `target.rs` and the test.

- [ ] **Step 4: Type-check for macOS, and run the four checks**

Run the macOS type-check from "Rules".
Expected: `Finished`, no warnings.
Run the four checks.
Expected: PASS.

- [ ] **Step 5: Stage for the controller**

`git add crates/tabletist-appkit`. Commit message: "Give the menu items' target a file and a name of its own".

---

### Task 2: `SettingsItem`

**Files:**
- Create: `crates/tabletist-appkit/src/settings.rs`, `crates/tabletist-appkit/tests/settings_menu.rs`
- Modify: `crates/tabletist-appkit/src/lib.rs`, `crates/tabletist-appkit/Cargo.toml`

- [ ] **Step 1: Write the test**

`crates/tabletist-appkit/tests/settings_menu.rs`, beside `about_menu.rs` and shaped as it is. It builds an app menu in winit's order (About, a separator, Hide, a separator, Quit), and checks that nothing is inserted before there is a menu or where the menu has no separator; that the item lands after About's separator with a separator of its own; that it carries `,` as its key and calls back when chosen; and that dropping it leaves the menu as it was and the item with no target.

```rust
//! The Settings item in an app menu built the way winit builds it, chosen
//! the way a click chooses it. No test harness: AppKit's menus belong to the
//! main thread, so this runs as a plain `main` (on macOS; nothing elsewhere).

// Building winit's menu takes the same unchecked calls winit makes.
#![allow(unsafe_code)]

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::cell::Cell;
    use std::rc::Rc;

    use objc2::{MainThreadMarker, sel};
    use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
    use objc2_foundation::ns_string;
    use tabletist_appkit::SettingsItem;

    let mtm = MainThreadMarker::new().expect("a test without a harness runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    assert!(
        SettingsItem::insert("Settings…", || {}).is_none(),
        "there is no menu yet"
    );

    // The menu bar and the app menu in it, in winit's order and shorter:
    // About, a separator, Hide, a separator, Quit. (winit has Services,
    // Hide Others and Show All beside Hide.)
    let bar = NSMenu::new(mtm);
    let holder = NSMenuItem::new(mtm);
    bar.addItem(&holder);
    let menu = NSMenu::new(mtm);
    let about = NSMenuItem::new(mtm);
    about.setTitle(ns_string!("About tabletist"));
    let hide = NSMenuItem::new(mtm);
    hide.setTitle(ns_string!("Hide tabletist"));
    let quit = NSMenuItem::new(mtm);
    quit.setTitle(ns_string!("Quit tabletist"));
    // SAFETY: NSApplication implements the three actions.
    unsafe {
        about.setAction(Some(sel!(orderFrontStandardAboutPanel:)));
        hide.setAction(Some(sel!(hide:)));
        quit.setAction(Some(sel!(terminate:)));
    }
    menu.addItem(&about);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&hide);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&quit);
    holder.setSubmenu(Some(&menu));

    // A menu with no separator has no place for the item.
    let bare = NSMenu::new(mtm);
    let bare_holder = NSMenuItem::new(mtm);
    bare_holder.setSubmenu(Some(&bare));
    let bare_bar = NSMenu::new(mtm);
    bare_bar.addItem(&bare_holder);
    app.setMainMenu(Some(&bare_bar));
    assert!(
        SettingsItem::insert("Settings…", || {}).is_none(),
        "no separator to put it after"
    );

    app.setMainMenu(Some(&bar));
    let before = menu.numberOfItems();
    let chosen = Rc::new(Cell::new(0));
    let item = SettingsItem::insert("Settings…", {
        let chosen = chosen.clone();
        move || chosen.set(chosen.get() + 1)
    })
    .expect("the Settings item");

    // After About's separator, with a separator of its own after it.
    assert_eq!(menu.numberOfItems(), before + 2);
    // The menu as it reads, a separator as `-`: what a separator's title
    // is, AppKit does not say.
    let titles = |menu: &NSMenu| -> Vec<String> {
        let items = menu.itemArray().to_vec();
        let title = |item: &NSMenuItem| match item.isSeparatorItem() {
            true => "-".to_owned(),
            false => item.title().to_string(),
        };
        items.iter().map(|item| title(item)).collect()
    };
    assert_eq!(
        titles(&menu),
        [
            "About tabletist",
            "-",
            "Settings…",
            "-",
            "Hide tabletist",
            "-",
            "Quit tabletist"
        ]
    );
    let settings = menu.itemAtIndex(2).expect("the item");
    assert_eq!(settings.keyEquivalent().to_string(), ",");
    assert_eq!(settings.action(), Some(sel!(menuItemChosen:)));
    assert!(settings.target().is_some(), "Settings has a target");
    assert!(
        menu.itemAtIndex(3)
            .expect("its separator")
            .isSeparatorItem()
    );
    assert_eq!(about.action(), Some(sel!(orderFrontStandardAboutPanel:)));
    assert_eq!(quit.action(), Some(sel!(terminate:)), "Quit is left alone");
    assert_eq!(chosen.get(), 0, "nothing is chosen yet");

    // What a click on the item does.
    menu.performActionForItemAtIndex(2);
    assert_eq!(chosen.get(), 1, "choosing Settings calls back");
    menu.performActionForItemAtIndex(2);
    assert_eq!(chosen.get(), 2, "and again");

    // Dropped, the menu is as winit made it, and the item, which this
    // test still holds, calls nothing.
    drop(item);
    assert!(settings.target().is_none(), "no dangling target");
    assert_eq!(menu.numberOfItems(), before);
    assert_eq!(
        titles(&menu),
        [
            "About tabletist",
            "-",
            "Hide tabletist",
            "-",
            "Quit tabletist"
        ]
    );

    println!("settings_menu: ok");
}
```

The key's modifier is not asserted: `keyEquivalentModifierMask` needs objc2-app-kit's `NSEvent` feature, which the crate does not turn on, and AppKit's default for a new item is Command.

`Cargo.toml`, after the first `[[test]]`:

```toml
[[test]]
name = "settings_menu"
harness = false
```

and the comment over the tests says "these tests have their own `main`".

- [ ] **Step 2: See it fail to compile for macOS**

Run the macOS type-check from "Rules".
Expected: an error that `tabletist_appkit` has no `SettingsItem`.

- [ ] **Step 3: Write `settings.rs`**

```rust
//! The app menu's Settings item, which winit's menu does not have.

// Giving a menu item a target and an action are messages the compiler
// cannot check.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::{NSString, ns_string};

use crate::target::MenuTarget;

/// "Settings…" in the app menu, with `⌘,`, and the separator after it.
/// Dropping it takes both out again.
pub struct SettingsItem {
    menu: Retained<NSMenu>,
    item: Retained<NSMenuItem>,
    separator: Retained<NSMenuItem>,
    /// A menu item does not keep its target alive.
    _target: Retained<MenuTarget>,
}

impl SettingsItem {
    /// Puts the item, titled `title`, after the separator that follows
    /// About: the place the platform gives it. It calls `chosen` (on the
    /// main thread) when it is chosen, by a click or by `⌘,`.
    ///
    /// `None` off the main thread, before the menu exists, and when the app
    /// menu has no separator to put it after.
    pub fn insert(title: &str, chosen: impl Fn() + 'static) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let bar = NSApplication::sharedApplication(mtm).mainMenu()?;
        // The app menu is the first one in the menu bar.
        let menu = bar.itemArray().firstObject()?.submenu()?;
        let after = menu
            .itemArray()
            .to_vec()
            .iter()
            .position(|item| item.isSeparatorItem())?;
        let target = MenuTarget::new(mtm, Box::new(chosen));
        // SAFETY: `target` implements `menuItemChosen:` as an action, and
        // it lives for as long as the item is in the menu: `Drop` takes
        // the item out and its target away before the target goes.
        let item = unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(sel!(menuItemChosen:)),
                ns_string!(","),
            );
            item.setTarget(Some(&target));
            item
        };
        let separator = NSMenuItem::separatorItem(mtm);
        let at = isize::try_from(after + 1).ok()?;
        menu.insertItem_atIndex(&item, at);
        menu.insertItem_atIndex(&separator, at + 1);
        Some(Self {
            menu,
            item,
            separator,
            _target: target,
        })
    }
}

impl Drop for SettingsItem {
    fn drop(&mut self) {
        // Still where they were put: someone else may have rebuilt the menu.
        for item in [&self.separator, &self.item] {
            if self.menu.indexOfItem(item) >= 0 {
                self.menu.removeItem(item);
            }
        }
        // Whoever still holds the item holds one that calls nothing.
        // SAFETY: an item may have no target.
        unsafe { self.item.setTarget(None) };
    }
}
```

`lib.rs`, with the others:

```rust
#[cfg(target_os = "macos")]
mod settings;
```

```rust
#[cfg(target_os = "macos")]
pub use settings::SettingsItem;
```

The `Cargo.toml` comment on where `unsafe` is allowed names `settings.rs` and its test too.

- [ ] **Step 4: Type-check for macOS, and run the four checks**

Run the macOS type-check from "Rules".
Expected: `Finished`, no warnings.
Run the four checks.
Expected: PASS. On Linux `settings_menu` runs as an empty `main`.

- [ ] **Step 5: Stage for the controller**

`git add crates/tabletist-appkit`. Commit message: "Add a Settings item to the app menu on macOS".

---

### Task 3: Open the Settings window from the menu

**Files:**
- Modify: `src/macos.rs`, `src/entrypoint.rs`
- Modify: `docs/superpowers/specs/2026-10-03-settings-general-design.md`

- [ ] **Step 1: `SettingsMenu` in `src/macos.rs`**

After `AboutMenu`:

```rust
/// The app menu's Settings item, asking for the Settings window.
///
/// winit's app menu has none, so one is added where the platform puts it.
/// AppKit takes `⌘,` for the item before egui sees the key.
pub struct SettingsMenu {
    chosen: Rc<Cell<bool>>,
    /// The item is in the menu for as long as this lives.
    _item: tabletist_appkit::SettingsItem,
}

impl SettingsMenu {
    /// Adds the item, titled `title`. Choosing it draws a frame, which
    /// finds it with [`SettingsMenu::take`]. `None` when there is no app
    /// menu or it has no separator to put the item after (or off the main
    /// thread).
    pub fn attach(ctx: &egui::Context, title: &str) -> Option<Self> {
        let chosen = Rc::new(Cell::new(false));
        let item = tabletist_appkit::SettingsItem::insert(title, {
            let chosen = chosen.clone();
            let ctx = ctx.clone();
            move || {
                chosen.set(true);
                ctx.request_repaint();
            }
        })?;
        Some(Self {
            chosen,
            _item: item,
        })
    }

    /// Whether the item was chosen since the last call.
    pub fn take(&self) -> bool {
        self.chosen.replace(false)
    }
}
```

The module's doc comment, which ends with a paragraph on the About item, gains a sentence: the app menu's Settings item is added the same way (`SettingsMenu`).

- [ ] **Step 2: Attach it, and turn a chosen item into an action**

`src/entrypoint.rs`. Where `about_menu` is attached, after it:

```rust
            #[cfg(target_os = "macos")]
            let settings_menu = crate::macos::SettingsMenu::attach(
                &cc.egui_ctx,
                &crate::i18n::gettext(app.locale, "Settings…"),
            );
            #[cfg(target_os = "macos")]
            if settings_menu.is_none() {
                log::warn!("the app menu has no place for a Settings item");
            }
```

In the `Window { .. }` that is built there, after `about_menu`:

```rust
                #[cfg(target_os = "macos")]
                settings_menu,
```

In `struct Window`, after the `about_menu` field:

```rust
    /// macOS: the app menu's Settings item, which opens the Settings
    /// window.
    #[cfg(target_os = "macos")]
    settings_menu: Option<crate::macos::SettingsMenu>,
```

In `Window::logic`, after the About block and before `self.app.logic(ctx)`:

```rust
        // The same window `Mod+,` opens. AppKit takes that key for the
        // item before egui sees it, so on macOS the key arrives here; where
        // the item could not be added, the key handler still has it. Asked
        // for while it is open, the window stays as it is, so a press that
        // reached both would still open it once.
        #[cfg(target_os = "macos")]
        if self
            .settings_menu
            .as_ref()
            .is_some_and(crate::macos::SettingsMenu::take)
        {
            self.app.actions.push(crate::model::Action::ShowSettings);
        }
```

- [ ] **Step 3: Type-check the macOS lines in an extract**

The workspace does not cross-compile, so check `SettingsMenu` on its own. In the session's scratchpad directory, a crate `macosx` with:

```toml
[package]
name = "macosx"
version = "0.0.0"
edition = "2024"

[dependencies]
egui = "0.36"
tabletist-appkit = { path = "<the worktree>/crates/tabletist-appkit" }

[workspace]
```

and a `src/lib.rs` that starts with `#![deny(warnings)]`, `use std::cell::Cell;` and `use std::rc::Rc;` and then holds the `SettingsMenu` block copied from `src/macos.rs` as it is.

Run: `CARGO_TARGET_DIR=<the worktree>/target/xcheck ~/.cargo/bin/cargo clippy --offline --manifest-path <scratchpad>/macosx/Cargo.toml --target aarch64-apple-darwin -- -D warnings`
Expected: `Finished`. Remove `target/xcheck` and the scratch crate afterwards.

The lines in `src/entrypoint.rs` are not checked by this. They mirror the `about_menu` lines beside them one for one; read the two side by side. Say in the report that CI's macOS job is their first compile.

- [ ] **Step 4: The spec**

In "Opening" of `docs/superpowers/specs/2026-10-03-settings-general-design.md`:

- `SettingsItem::insert(title, chosen)` puts the item and a separator after the first separator of the app menu, which in winit's menu is the one after About; where the menu has no separator, nothing is inserted and the key handler opens the window.
- The shared class is `MenuTarget` in `target.rs`, and its action is `menuItemChosen:`.

In "Testing", the macOS menu item: the test is `settings_menu.rs`; it checks the place, the key `,`, the callback and the removal on drop, and not the key's modifier, which is AppKit's default (Command) and needs a feature the crate does not turn on.

- [ ] **Step 5: Run the four checks**

Expected: PASS. They compile none of this task's Rust on Linux; they guard the spec's formatting and that nothing else moved.

- [ ] **Step 6: Stage for the controller**

Two commits: `src/macos.rs` and `src/entrypoint.rs` ("Open the Settings window from the macOS app menu"); the spec ("Say in the spec how the macOS menu item was built").

---

## By hand, on a Mac

No agent's session can run AppKit. These go in the pull request's test plan unchecked:

- The app menu reads: About Tabletist, a separator, Settings…, a separator, Services, and the rest as before.
- Settings… shows `⌘,` at its right.
- Choosing it opens the Settings window. `⌘,` opens it too, once, with the keyboard anywhere (the grid, a text field, the SQL editor).
- With the window open, `⌘,` and the item do nothing more.
- With another dialog open (the connection dialog), the item does nothing, as the key does.
- About Tabletist still opens the app's own About dialog.

## Self-review

- Spec coverage: "Opening" asks for the item with `⌘,` after About's separator, a shared target class renamed for what it is, removal on drop, `SettingsMenu` wrapping it as `AboutMenu` wraps `AboutItem`, and `Window::logic` turning a chosen item into `Action::ShowSettings` (Tasks 1 to 3). "Testing" asks for a test with its own `main` beside `about_menu.rs` (Task 2).
- One departure from the spec, written back in Task 3: the test does not assert the modifier of the key.
- Names across tasks: `MenuTarget`, `menuItemChosen:`, `SettingsItem::insert`, `SettingsMenu::{attach, take}`, the `settings_menu` field.
- What no step here can show: that AppKit runs any of it. The crate's code is type-checked for macOS, the wrapper in an extract, the entrypoint's lines not at all.
