# poke

A small TUI app for recurring, named timers. Runs in the foreground — no daemon, no background process. When the app is closed, no timers fire.

Written in Rust as a learning project.

---

## Functionality

### Interface

The screen is split into three regions:

```
┌─ header ───────────────────────────────────────────────┐
│  <ascii art>    09:41            [1] active timer      │
│                 Monday, January 1st 2026               │
└────────────────────────────────────────────────────────┘
┌─ timer list ──────────────────┐┌─ form ────────────────┐
│ [x] Simple timer   30m  13:00 ││ Name                  │
│ [ ] Simple timer 2 1h   17:00 ││ Interval              │
│                               ││ Start                 │
│                               ││ End                   │
└───────────────────────────────┘└───────────────────────┘
```

- **Header** — ASCII-art logo, a clock rendered in block characters, the date
  below it, and the count of currently active timers on the right.
- **Timer list** — one row per timer, each with a checkbox for enabled/disabled.
  Shows a "no timers" message when empty.
- **Form** — fields for creating a new timer: Name, Interval, Start, End.

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
| `q` / `Esc`     | Quit                                                 |

Backspace does different things depending on focus (delete timer vs. delete character). This is not a conflict — key handling dispatches on the current focus, so it falls out of a single `match`.

Invalid Interval / Start / End input is rejected on submit, with the offending field visually flagged.

### Firing

When a timer's boundary is reached it triggers an OS desktop notification. If notifications are unavailable, it falls back to the terminal bell (`\x07`).

---

## Timer semantics

A timer is a **daily-recurring window**. It is defined by:

- `start` — a time of day
- `end` — a time of day 
- `interval` — a duration

It fires at `start`, then every `interval` after that, for as long as the result is `<= end`. 

```
next boundary = start + n * interval,  for integer n >= 0,  while <= end
```

Example: `09:00 → 10:00, every 25m` fires at 09:00, 09:25, 09:50. Not at 10:15.

`start` itself **is** a firing boundary. A timer set with a 30m interval for 15:00 while it is 09:41 fires at 15:00, then 15:30, 16:00, and so on.


### The `last_checked` interval

The naive check "does `now` equal a boundary?" has no well-defined answer: `now` is an instant, the boundary is an instant, and the event loop never observes the exact instant. Polling every 250ms and asking that question either misses the boundary entirely or, if the comparison is truncated to the minute, fires hundreds of times for the same boundary.

The correct predicate is over an **interval**, not an instant:

> Did at least one boundary fall within `(last_checked, now]`?

- Half-open on the left, so a boundary landing exactly on `last_checked` (already
  fired) does not fire again.
- Closed on the right, so a boundary landing exactly on `now` does fire.

This handles both problems with the same expression:

| Situation           | Interval width | Boundaries inside | Fires |
|---------------------|----------------|-------------------|-------|
| Normal running      | ~250ms         | 0 or 1            | 0 or 1 |
| Woke from 3h sleep  | ~3h            | many              | 1 (predicate is boolean) |

`last_checked` is:

- **runtime-only state** — never serialized to disk
- **per-timer**, not global — so that enabling a timer with Space, or creating
  one via the form, can set *its* `last_checked = now` at that moment. Otherwise
  a timer enabled at 15:00:00.100 could immediately fire for the 15:00:00.000
  boundary it was not enabled for.
- initialized to `Local::now()` at startup, so launching mid-window never fires
  retroactively.

### Design rule: no clock reads inside the logic

`should_fire` takes `now` as a parameter. It never calls `Local::now()`
internally. This keeps it a pure function of its three inputs, which means the
entire firing logic is unit-testable in microseconds with no UI and no waiting:

```rust
fn should_fire(timer: &Timer, last_checked: DateTime<Local>, now: DateTime<Local>) -> bool
```

Test cases worth covering:

- boundary exactly at `now` → fires
- boundary exactly at `last_checked` → does not fire (no double-fire)
- gap spanning several boundaries → fires once
- gap spanning the entire window → fires once
- `now` outside the window → does not fire
- gap crossing midnight into the next day's window
- `end` not landing on a boundary (09:00–10:00 every 25m)
- `end` before `start` → rejected at validation (v1 does not support windows
  crossing midnight)

### DST

`chrono`'s `Local.from_local_datetime()` returns a `LocalResult` with three
variants, because a local wall-clock time can be ambiguous (occurs twice) or
nonexistent (skipped) on DST transition days. Pick a branch and move on.

---

## Data on disk

Both files are TOML, under `~/.config/poke/`:

| File          | Contents                                    |
|---------------|---------------------------------------------|
| `config.toml` | User settings: time format, date format     |
| `timers.toml` | Persisted timers                            |

Notes:

- **Path resolution.** The `dirs`/`directories` crates return
  `~/Library/Application Support/...` on macOS, not `~/.config`. To get XDG paths
  on every platform, use `etcetera` with its base strategy, or resolve
  `$XDG_CONFIG_HOME` manually with a `$HOME/.config` fallback.
- **Atomic writes.** Write to `timers.toml.tmp`, then `fs::rename` over the real
  file. A crash mid-write otherwise leaves a truncated file and no timers.
- **Derived state is never persisted.** `last_checked` lives only in memory.

---

## Dependencies

| Crate          | Purpose                                                    |
|----------------|------------------------------------------------------------|
| `ratatui`      | TUI rendering. Use the main crate, not `ratatui-core`.     |
| `chrono`       | Date/time types, strftime-style formatting from config.    |
| `serde`        | Derive for the persisted structs.                          |
| `toml`         | Config and timer serialization.                            |
| `humantime`    | Parses `"30m"`, `"1h"`, `"2h30m"` into a `Duration`.       |
| `notify-rust`  | Desktop notifications.                                     |
| `anyhow`       | Error handling (`Result` + `?` + `.context(...)`).         |
| `etcetera`     | XDG config paths on all platforms.                         |

---

## Project structure

```
src/
  main.rs        // terminal setup/teardown, the event loop
  app.rs         // App state, Focus enum, update(Action)
  action.rs      // Action enum + key -> Action mapping
  config.rs      // Config struct, load-or-default
  timer.rs       // Timer struct + pure firing/parsing logic + tests
  storage.rs     // load/save timers, atomic write
  notify.rs      // fire() with graceful fallback to the bell
  digits.rs      // block-character glyphs for the clock
  ui/
    mod.rs       // top-level layout, dispatches to the three below
    header.rs    // logo + clock + active count
    list.rs      // timer list
    form.rs      // new-timer form
```

### Module responsibilities

- **`main.rs`** — `ratatui::run()`, then the loop: draw, `event::poll(250ms)`,
  handle any key, check timers, repeat. Contains the "terminal too small"
  guard threshold.
- **`app.rs`** — the single source of truth. Holds the timer list, the form
  buffer, validation errors, and current focus. The **only** place state
  mutates, via `update(Action)`.
- **`action.rs`** — translates raw key events into semantic `Action`s. Keeps
  keybinding decisions in one file and out of the update logic.
- **`config.rs`** — loads `config.toml`, falls back to defaults if absent.
- **`timer.rs`** — the `Timer` struct, interval/time parsing, and `should_fire`.
  Knows nothing about the terminal, the filesystem, or the system clock. This is
  the only module with real logic, so it is the only one that really needs tests.
- **`storage.rs`** — reads and writes `timers.toml`. Atomic writes.
- **`notify.rs`** — sends the notification; falls back to the bell on failure.
- **`ui/*`** — pure rendering. Takes `&App`, produces widgets. Knows nothing
  about time, files, or state mutation.

### The three lines that matter

1. `timer.rs` knows nothing about the terminal.
2. `ui/` knows nothing about time or files — it only reads `&App`.
3. `app.rs` is the only place state changes.

Keep these clean and the project stays pleasant at 1500 lines.

### State shapes

```rust
enum Focus {
    Header,
    List(usize),
    Form(Field),   // Field: Name | Interval | Start | End
}
```

A single focus enum is what makes the keybindings work without special-casing.

```rust
// serde, written to timers.toml
struct Timer {
    name: String,
    interval: Duration,
    start: NaiveTime,
    end: NaiveTime,
    enabled: bool,
}

// runtime only, never serialized
struct TimerRuntime {
    last_checked: DateTime<Local>,
}
```

Pair these as `Vec<(Timer, TimerRuntime)>` rather than two parallel `Vec`s — it
cannot desynchronize when a row is deleted with Backspace.

### Form validation

Parse, don't validate. The form holds `String`s while typing; Enter attempts
`parse()` into the typed values. On failure, store an error message in `App` and
render the offending field's border in red.