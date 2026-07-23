# poke

A small TUI app for recurring, named timers. Runs in the foreground — no daemon, no background process. When the app is closed, no timers fire.

Written in Rust as a learning project.

---

## Functionality

### Interface

```
╭─ poke ─────────────────────────────────────────────────╮
│  <ascii art>       09:41              1 active timer    │
│                    Monday, January 1 2026               │
╰─────────────────────────────────────────────────────────╯
╭─ timers ──────────────────────╮╭─ new ─────────────────╮
│ ■ tea         13:00  18:00    ││ Name                  │
│ □ stretch     09:00  17:00    ││ Interval              │
│                               ││ Start                 │
│                               ││ End                   │
╰───────────────────────────────╯╰───────────────────────╯
```

- **Header** — ASCII-art logo that changes with the hour, the clock, the date below it, and an active/inactive count on the right.
- **Timer list** — a `Table`, one row per timer: enabled marker, name, start, end. Shows a "no timers" message when empty.
- **Form** — fields for creating a new timer: Name, Interval, Start, End.

The focused panel is indicated by its title style — accent colour, bold, italic. See [Styling rules](#styling-rules).

### Keybindings

| Key             | Action                                              |
|-----------------|-----------------------------------------------------|
| `1`             | Focus the timer list                                |
| `2`             | Focus the form                                      |
| `↑` / `↓`       | Move through list rows, or between form fields       |
| `Space`         | Toggle the selected timer on/off (list focus)        |
| `Backspace`/`Del` | Delete the selected timer (list focus)             |
| any character   | Type into the focused field (form focus)             |
| `Enter`         | Validate and submit the form                         |
| `Esc`           | Quit                                                 |

**Note on single-letter shortcuts.** Any bare `Char(c)` binding that is not gated on focus will be impossible to type into the form. Either restrict such bindings to `(Char('q'), Focus::List)`, or use modifiers. `Esc` and the digits are safe as globals only until the form accepts digits — which Interval, Start and End all do. Gate them.

Backspace does different things depending on focus (delete timer vs. delete character). This is not a conflict — `map_key` dispatches on `(key, focus)`, so it falls out of a single match arm.

Invalid Interval / Start / End input is rejected on submit, with the offending field visually flagged.

### Firing

When a timer's boundary is reached it triggers an OS desktop notification. If notifications are unavailable, it falls back to the terminal bell (`\x07`).

---

## Timer semantics

A timer is a **daily-recurring window**. It is defined by:

- `start` — a time of day
- `end` — a time of day (required, not optional)
- `interval` — a duration

It fires at `start`, then every `interval` after that, for as long as the result is `<= end`. The window resets every day; there is no synchronisation with when the program was opened or closed.

```
next boundary = start + n * interval,  for integer n >= 0,  while <= end
```

Example: `09:00 → 10:00, every 25m` fires at 09:00, 09:25, 09:50. Not at 10:15.

`start` itself **is** a firing boundary. A timer set with a 30m interval for 15:00 while it is 09:41 fires at 15:00, then 15:30, 16:00, and so on.

Missed boundaries are skipped: if the program was closed or the machine asleep across several boundaries, the timer fires **once** on the next check.

### The `last_checked` interval

The naive check "does `now` equal a boundary?" has no well-defined answer: `now` is an instant, the boundary is an instant, and the event loop never observes the exact instant. Polling every 250ms and asking that question either misses the boundary entirely or, if the comparison is truncated to the minute, fires hundreds of times for the same boundary.

The correct predicate is over an **interval**, not an instant:

> Did at least one boundary fall within `(last_checked, now]`?

- Half-open on the left, so a boundary landing exactly on `last_checked` (already fired) does not fire again.
- Closed on the right, so a boundary landing exactly on `now` does fire.

This handles both problems with the same expression:

| Situation           | Interval width | Boundaries inside | Fires |
|---------------------|----------------|-------------------|-------|
| Normal running      | ~250ms         | 0 or 1            | 0 or 1 |
| Woke from 3h sleep  | ~3h            | many              | 1 (predicate is boolean) |

`last_checked` is:

- **runtime-only state** — never serialized to disk
- **per-timer**, not global — so that enabling a timer with Space, or creating one via the form, can set *its* `last_checked = now` at that moment. Otherwise a timer enabled at 15:00:00.100 could immediately fire for the 15:00:00.000 boundary it was not enabled for.
- initialized to `Local::now()` at startup, so launching mid-window never fires retroactively.

### Design rule: no clock reads inside the logic

`should_fire` takes `now` as a parameter. It never calls `Local::now()` internally. This keeps it a pure function of its three inputs, which means the entire firing logic is unit-testable in microseconds with no UI and no waiting:

```rust
fn should_fire(timer: &Timer, last_checked: DateTime<Local>, now: DateTime<Local>) -> bool
```

The same rule applies to rendering: `Local::now()` is read **once** at the top of `ui::draw` and threaded down to the header, list and form. Two independent `now()` calls in one frame can straddle a minute boundary and disagree.

Test cases worth covering:

- boundary exactly at `now` → fires
- boundary exactly at `last_checked` → does not fire (no double-fire)
- gap spanning several boundaries → fires once
- gap spanning the entire window → fires once
- `now` outside the window → does not fire
- gap crossing midnight into the next day's window
- `end` not landing on a boundary (09:00–10:00 every 25m)
- `end` before `start` → rejected at validation (v1 does not support windows crossing midnight)

### DST

`chrono`'s `Local.from_local_datetime()` returns a `LocalResult` with three variants, because a local wall-clock time can be ambiguous (occurs twice) or nonexistent (skipped) on DST transition days. Pick a branch and move on.

---

## Data on disk

Both files are TOML, under `~/.config/poke/`:

| File          | Contents                                                  |
|---------------|-----------------------------------------------------------|
| `config.toml` | Time format, date format, theme colours                   |
| `timers.toml` | Persisted timers                                          |

The config struct is roughly:

```rust
pub struct Config {
    pub time_format: String,     // strftime, e.g. "%H:%M"
    pub date_format: String,     // strftime, e.g. "%A, %B %-d %Y"
    pub accent: Color,           // default Color::Yellow
    pub accent_soft: Color,      // default Color::LightYellow
    pub selected_text: Color,    // default Color::Black
}
```

`ratatui::style::Color` implements `Serialize`/`Deserialize` when ratatui's `serde` feature is enabled, so `accent = "yellow"` or `accent = "#e0af68"` in TOML deserializes directly — no wrapper enum needed. `Config` should implement `Default` and be loaded with a load-or-default that never fails hard on a missing file.

Once `Config` exists, `panel()` needs the colours, so it takes them as parameters rather than reading a global. Same principle as `now`.

### Styling rules

Two shades of one accent, plus a text colour for anything drawn on top of a filled background.

| Element                     | Style                                          |
|-----------------------------|------------------------------------------------|
| Focused panel title         | `.fg(accent)` + bold + italic                  |
| Unfocused panel title       | `Style::default()`                             |
| Selected table row          | `.bg(accent_soft)` + `.fg(selected_text)`      |
| Focused form field          | `.bg(accent_soft)` + `.fg(selected_text)` + italic |
| Logo art                    | `.fg(accent_soft)`                             |

Notes on why it is shaped this way:

- **Named `accent` / `accent_soft`, not `fg` / `bg`.** The soft shade is used as a foreground on the logo art and as a background on selected rows, so naming them after their CSS-ish role would be actively misleading. The names describe the colour, not its position.
- **`selected_text` is a config field, not a hardcoded black.** Black on LightYellow is fine; black on a user's dark blue accent is unreadable, and hardcoding it makes that unfixable without patching source. `.reversed()` is the alternative — it swaps whatever fg/bg are in effect and adapts for free — but an explicit field gives finer control over the two shades.
- **Focused titles get no background.** Tried and rejected: a filled title on a rounded border reads as noise. Bold + accent + italic is enough, and it keeps one visual language for "focused panel" and another for "selected item within a panel", so a focused panel containing a selected row does not shout twice.
- **Palette colours by default, hex allowed.** `Color::Yellow` and `Color::LightYellow` are palette indices 3 and 11, so they resolve to whatever the user's terminal theme defines and harmonise with any colourscheme out of the box. The trade-off is that the two shades are not guaranteed to differ much in every theme — hence hex overrides being available.
- **Italic support is patchy.** Some terminals ignore SGR 3, and a few older ones render it as inverse, which would collide with the inverted rows. Accepted risk for now.

Other notes:

- **Path resolution.** The `dirs`/`directories` crates return `~/Library/Application Support/...` on macOS, not `~/.config`. To get XDG paths on every platform, use `etcetera` with its base strategy, or resolve `$XDG_CONFIG_HOME` manually with a `$HOME/.config` fallback.
- **Atomic writes.** Write to `timers.toml.tmp`, then `fs::rename` over the real file. A crash mid-write otherwise leaves a truncated file and no timers.
- **Derived state is never persisted.** `last_checked` lives only in memory.
- **`Duration` serializes badly by default** — serde's built-in impl writes a `{ secs, nanos }` struct. Not a problem for a machine-written file, but if the TOML should stay hand-editable, a `#[serde(with = "...")]` module wrapping `humantime` keeps it as `"30m"`.

---

## Dependencies

| Crate          | Status  | Purpose                                                    |
|----------------|---------|------------------------------------------------------------|
| `ratatui`      | added   | TUI rendering. Use the main crate, not `ratatui-core`.     |
| `chrono`       | added   | Date/time types, strftime-style formatting from config.    |
| `serde`        | pending | Derive for the persisted structs.                          |
| `toml`         | pending | Config and timer serialization.                            |
| `humantime`    | pending | Parses `"30m"`, `"1h"`, `"2h30m"` into a `Duration`.       |
| `notify-rust`  | pending | Desktop notifications.                                     |
| `anyhow`       | pending | Error handling (`Result` + `?` + `.context(...)`).         |
| `etcetera`     | pending | XDG config paths on all platforms.                         |

Notes:

- **Do not add `crossterm` as a separate dependency.** `ratatui` re-exports it as `ratatui::crossterm`. Adding it separately risks two incompatible crossterm versions in the graph, producing confusing errors where `KeyCode` is not `KeyCode`.
- **Do not use tokio.** A single-threaded poll loop covers everything here.
- **No panic hook needed.** `ratatui::run()` (added in 0.30) handles terminal init, restore, *and* panic hooks. Older tutorials showing manual `enable_raw_mode()` / `EnterAlternateScreen` are out of date.
- **macOS notification caveat.** `notify-rust` on macOS goes through `mac-notification-sys`, which wants an application bundle identifier. A bare `cargo run` binary is not a bundle, so notifications may silently do nothing until `set_application()` is called with a valid bundle id. Implement the terminal-bell fallback first so this is never blocking.

---

## Project structure

```
src/
  main.rs        // the event loop
  app.rs         // App + FormState, Focus/Field enums, update(Action)
  action.rs      // Action enum + map_key(key, focus)
  config.rs      // Config struct, load-or-default            [pending]
  timer.rs       // Timer + TimerRuntime + pure firing logic + tests
  storage.rs     // load/save timers, atomic write            [pending]
  notify.rs      // fire() with graceful fallback to the bell [pending]
  ui/
    mod.rs       // top-level layout + shared helpers, dispatches to the three below
    header.rs    // logo + clock + recap
    list.rs      // timer table
    form.rs      // new-timer form                            [pending]
```

Note `ui/mod.rs` **is** the `ui` module — not a sibling of `header.rs` but its parent. Private items in `mod.rs` are therefore visible to `header.rs`, `list.rs` and `form.rs` without any `pub`, because privacy in Rust means "this module and its descendants".

### Module responsibilities

- **`main.rs`** — `ratatui::run()`, then: draw, `event::poll(250ms)`, map the key, `app.update(action)`, check timers, repeat, `while !app.should_quit`. No per-key logic lives here.
- **`app.rs`** — the single source of truth. Holds the timers, the table state, the form buffer, validation errors, and current focus. The **only** place state mutates, via `update(Action)`.
- **`action.rs`** — the `Action` enum plus `map_key(key, focus) -> Option<Action>`. Knows about keys; knows nothing about `App`. Pure and testable. Lives at the crate root rather than in `ui/` so that the import arrows stay one-way: `app` consumes `Action`, and `ui` must never be imported by `app`.
- **`config.rs`** — loads `config.toml`, falls back to defaults if absent.
- **`timer.rs`** — `Timer`, `TimerRuntime`, interval/time parsing, and `should_fire`. Knows nothing about the terminal, the filesystem, or the system clock. This is the only module with real logic, so it is the only one that really needs tests.
- **`storage.rs`** — reads and writes `timers.toml`. Atomic writes.
- **`notify.rs`** — sends the notification; falls back to the bell on failure.
- **`ui/*`** — rendering only. Each submodule exposes a single `draw(frame, area, ...)`; helpers below it are private.

### The three lines that matter

1. `timer.rs` knows nothing about the terminal.
2. `ui/` knows nothing about time or files.
3. `app.rs` is the only place state changes.

**One deliberate exception to (2) and (3).** `render_stateful_widget` needs `&mut TableState`, and `TableState` owns both the selection and the scroll offset, so it must persist across frames — which means it lives in `App` and `ui::draw` takes `&mut App`. The exception is narrow: the UI may mutate `table_state` and nothing else. The moment a render function mutates a `Timer`, the invariant is genuinely broken.

Note that this works because the borrow checker tracks individual fields: `draw_list(frame, inner, &app.timers, &mut app.table_state)` is legal — one shared and one mutable borrow of *disjoint* fields. Prefer passing the specific fields a function needs over passing `&mut App` wholesale; it is both more flexible and easier on the borrow checker.

### State shapes

```rust
pub struct App {
    pub timers: Vec<(Timer, TimerRuntime)>,
    pub table_state: TableState,
    pub current_focus: Focus,
    pub form_state: FormState,
    pub should_quit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus { List, Form }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field { Name, Interval, Start, End }
```

`Focus` and `Field` need `Copy` so they can be passed by value while `App` is otherwise borrowed, and `PartialEq` for comparison. `Focus` is flat rather than carrying the selected index, because `TableState` already owns that.

```rust
// serde, written to timers.toml
pub struct Timer {
    pub name: String,
    pub interval: Duration,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub enabled: bool,
}

// runtime only, never serialized
pub struct TimerRuntime {
    pub last_checked: DateTime<Local>,
}
```

Paired as `Vec<(Timer, TimerRuntime)>` rather than two parallel `Vec`s — it cannot desynchronize when a row is deleted. If the `.0` accessors start grating, the alternative is a single struct with `#[serde(skip)]` on the runtime field.

### Form validation

Parse, don't validate. `FormState` holds `String`s while typing; Enter attempts `parse()` into the typed values. On failure, store the error in `App` and render the offending field's border in red — which requires `Option<(Field, String)>` rather than a bare `Option<String>`, since a plain message cannot say *which* field was wrong.

A `focused_mut(&mut self) -> &mut String` helper on `FormState` collapses the four-arm match that would otherwise be repeated for every keystroke:

```rust
Action::TypeChar(c) => self.form_state.focused_mut().push(c),
Action::DeleteChar  => { self.form_state.focused_mut().pop(); }
```

Ratatui ships no text-input widget by design — a text field at this level *is* a `String` plus `frame.set_cursor_position()`. `tui-textarea` exists for multiline editing with selection and undo, but four single-line fields do not need it.

### Bounds safety

`TableState::selected()` can return a stale index after a delete, and both `Vec::remove` and `[]` panic out of range. Guard with `let ... else` plus `get_mut`:

```rust
let Some(i) = self.table_state.selected() else { return };
if let Some((timer, _)) = self.timers.get_mut(i) { /* ... */ }
```

After deleting the last row, `select_previous()` so the selection does not visually vanish.

---

## Build order

Deliberately sequenced so nothing blocks on anything else:

1. ~~Static header + hardcoded timer list. No time, no files.~~ **done**
2. ~~Live clock, date, logo, recap in the header.~~ **done**
3. ~~Timer table with persistent `TableState` selection.~~ **done**
4. ~~`Action` enum, `map_key`, `update` — list keybindings wired end to end.~~ **done**
5. Focus styling on panel titles. *(in progress)*
6. `timer.rs` with full tests — no UI involved at all.
7. Wire firing to the terminal bell.
8. `config.rs` + `storage.rs`.
9. The form, with validation.
10. Real OS notifications last, since this is the piece most likely to eat an afternoon on macOS.