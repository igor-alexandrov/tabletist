# Esc asks to discard pending changes

Issue: igor-alexandrov/tabletist#187. User feedback: "after editing, Discard
should be on Esc."

## What happens now

With pending changes in a table's grid and no cell being edited, Esc does
nothing in the macOS and Windows looks. Esc only cancels the open cell's
edit (`CancelEdit`, Cell editor scope). Dropping every pending change takes
⌥⌘⌫ (`DiscardChanges`), which asks nothing, and the pending bar's
**Discard all** does not write that key, so it is hard to find.

## Decision

Esc on the grid, with pending changes and no open editor, **asks first**:
the Leave prompt comes up titled "Discard 4 changes?". A stray second Esc
(after one that cancelled a cell's edit) never loses anything, since in the
prompt Esc is Cancel and Enter never discards.

Rejected: discarding at once with Undo in the toast (there is no undo for a
whole discarded set: new rows, delete marks and the counts of what deletes
take would all need restoring), and reverting only the cursor's cell (safe,
but not what was asked; ⌘Z does that already).

## Keys (`src/keymap.rs`)

- A new command, `AskDiscard`, "Discard pending, asking first", group
  Editing, docs heading "Discard all pending".
- Bound in the Grid scope: **Esc** for macOS (and so Windows), nothing for
  Omarchy, whose `:e!` stays and whose normal-mode Esc stays harmless.
  `.when(When::Pending)`, a new `When`: the table on screen holds pending
  changes. That keeps it clear of any later Esc in the grid with nothing
  pending.
- `DiscardChanges` keeps ⌥⌘⌫ and keeps dropping at once: one command per
  behaviour, so the generated no-shared-key tests stay meaningful.
- Not in quick open's `RUNNABLE`: quick open already has "Discard all
  pending", and a run from quick open is deliberate.

## Where Esc is read (`src/ui/keys.rs`, `editing_keys`)

After the early return for an open editor, beside `DiscardChanges`, read
only:

- not in the terminal look (the keymap gives it no key there anyway),
- with the keys on the grid itself (`keyboard`, no row panel `field`),
- with `object.edits.pending()`, not under a save (`saving.is_none()`),
- a fresh press, not a repeat: an Esc held since cancelling an edit does
  not go on to ask,
- not when that Esc left a text field (`was_editing_id`) or an open popup
  or menu takes it (`egui::Popup::is_any_open`), as the terminal look's
  Esc chain already guards.

It pushes `Action::AskDiscard { tab, id }`.

## The prompt (reusing the Leave prompt)

- `Held` gets a variant `Nothing`: nothing waits on the answer.
  `App::apply(AskDiscard)` calls `hold(Held::Nothing, vec![(tab, id)])`, so
  everything `hold` already does applies: with
  `safety.confirm_close_edits` off the changes are dropped at once, under a
  save nothing happens, and with another dialog open the notice says so.
- `perform(Held::Nothing)` does nothing.
- The sheet's title with `Held::Nothing`: "Discard 4 changes?" (whatever
  `can_save`), its text unchanged ("They have not been written."). Buttons
  unchanged: Cancel, Discard, and Save when the save is not blocked. Esc
  cancels, Enter follows the focused button or saves, never discards.
- The terminal look's box names what is left by `doing`: `Held::Nothing`
  gets words there too ("discarding"), though no key of that look asks.
- `doing` and every other `match` on `Held` name the new variant.

## Pending bar (`src/ui/pending_bar.rs`)

**Discard all** writes `DiscardChanges`'s key (⌥⌘⌫) as Save writes its
own, through `keys::written`. `Buttons::Whole` keys both; `Plain` and
`Folded` write neither. The room measured for `Whole` counts both keys. A
look with no chord for it (Omarchy) writes none.

## Docs

- `docs/_reference/keyboard-shortcuts.md` and `docs/_guide/macos.md`: an
  Esc row, "Discard all pending changes, asking first".
- `docs/_guide/editing-data.md`: Esc on the grid asks to discard, and
  Discard all shows its key.

## Tests

Headless, through `src/testing.rs`:

1. Esc on the grid with a pending change opens the Leave prompt titled
   "Discard 1 change?"; Discard drops it.
2. Esc in that prompt keeps the change.
3. Esc in an open cell editor cancels that edit only: no prompt that frame,
   and a held Esc's repeats ask nothing.
4. Esc with nothing pending opens nothing.
5. With `confirm_close_edits` off, Esc drops the changes at once.
6. The terminal look's Esc on the grid with pending changes asks nothing.
7. Esc that closes a popup over the grid asks nothing.
8. The pending bar writes ⌥⌘⌫ beside Discard all in the macOS look.

The keymap's generated tests cover the new binding against every other Esc.
