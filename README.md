# poke

Recurring, named timers with a TUI. Unix-first (Linux and macOS); Windows is explicitly out of scope for now.

Written in Rust as a learning project.

---

## Process model

Two processes, one binary.

- **The daemon** owns the timer logic. It is the only thing that evaluates boundaries and the only thing that fires notifications.
- **The TUI** owns editing. It reads and writes `timers.toml` and never fires anything.

They do not talk to each other. `timers.toml` is the entire interface between them.

```
   ┌───────────┐  writes on every edit   ┌──────────────┐
   │    TUI    │ ──────────────────────► │ timers.toml  │
   │  (poke)   │ ◄────────────────────── │              │
   └───────────┘  reads once at startup  └──────────────┘
                                                ▲
                                                │ re-reads when mtime changes
                                                │
                                         ┌──────────────┐
                                         │    daemon    │ ──► notifications
                                         │  (poke d)    │
                                         └──────────────┘
```

### Why this shape

Files as IPC means no sockets, no shared memory, no wire protocol, no serialization beyond the TOML that has to exist anyway. The cost is that changes propagate on a poll interval rather than instantly, which is irrelevant for a tool whose finest granularity is a minute.

Single-writer is what makes it safe: only the TUI writes `timers.toml`. The daemon is strictly a reader. There is no case where both processes write, so there is no case where two writes interleave.

### The TUI requires a running daemon

`poke` with no daemon running is an error, not an invitation to spawn one:

```
poke: no daemon running. start one with `poke d`.
```

This is deliberate. Auto-spawning means the child inherits a terminal that is in raw mode on an alternate screen — a single stray `eprintln!` from the daemon corrupts the display — and avoiding that means nulled stdio, process groups, and `setsid()`. Requiring an explicit `poke d` removes all of it.

---

## CLI

| Command                   | Effect                                          |
|---------------------------|-------------------------------------------------|
| `poke`                    | Open the TUI. Fails if no daemon is running.    |
| `poke d` / `poke daemon`  | Start the daemon in the foreground.             |
| `poke s` / `poke stop`    | Stop the running daemon.                        |
| `poke r` / `poke restart` | Stop then start.                                |

Four subcommands does not justify `clap`. A match on `std::env::args().nth(1).as_deref()` is about eight lines with no dependency:

```rust
match std::env::args().nth(1).as_deref() {
    None                        => run_tui(),
    Some("d") | Some("daemon")  => run_daemon(),
    Some("s") | Some("stop")    => stop_daemon(),
    Some("r") | Some("restart") => { stop_daemon()?; run_daemon() }
    Some(other)                 => { eprintln!("poke: unknown command `{other}`"); ... }
}
```

`main.rs` dispatches; it does not itself contain either mode. Both `run_tui` and `run_daemon` live in their own modules and `main` only chooses between them.

`daemon.rs` as a single file is the right start. It becomes `daemon/mod.rs` (or `daemon.rs` plus a `daemon/` directory, same thing) only if it grows enough to want splitting — which it probably will not, since the interesting logic lives in `timer.rs`.

---

## Liveness: the lock file

The daemon opens `~/.config/poke/daemon.lock` and takes an **exclusive non-blocking advisory lock** on it.

- Lock acquired → no daemon was running; this process is now the daemon.
- Lock fails with "would block" → a daemon is already running.

The TUI uses the same call inverted: if it *acquires* the lock, no daemon exists, so it prints the error and exits (releasing the lock on the way out).

**Why a lock and not a PID file.** The kernel releases the lock when the process dies — including on SIGKILL, including on a crash. A PID file left by a crashed daemon is stale, and staleness cannot be reliably detected, because PIDs are reused: after a reboot, PID 4242 is some other program. Every PID-file implementation eventually grows a "verify this process is really mine" hack. Locks have no stale state by construction.

Best of both: take the lock, then write the PID into the locked file as its contents. Liveness comes from the lock; `poke stop` reads the PID and sends `SIGTERM`.

The lock also prevents two daemons racing to fire the same timer, which is a real hazard the moment `poke d` can be typed twice.

Crates: `fs4` or `fd-lock` for `try_lock_exclusive()`; `nix` for `kill()`.

---

## Daemon startup: lock first, then detach

`poke d` backgrounds itself. The ordering is not arbitrary:

1. **Acquire the lock.** Still attached to the terminal, so a failure can actually be reported: `poke: daemon already running`. Taking the lock after detaching means a second `poke d` forks a child that dies invisibly and returns success to the user.
2. **Load config and timers.** Same reason — a malformed `config.toml` should print to the terminal the user is standing in front of, not to a log they do not know exists.
3. **Print `poke: daemon started`.** `poke d` returns immediately, so silence looks like a failure.
4. **Daemonize.** `setsid()`, redirect stdio, detach.
5. **Enter the loop.** From here on, all output goes to the log.

### Why detaching needs more than backgrounding

Backgrounding with `&` detaches from the *shell*, not from the *tty*. The process keeps fds 1 and 2 pointing at the terminal, which means:

- Any `eprintln!` from the daemon writes into the TUI's alternate screen the next time `poke` runs in that terminal — the same corruption that auto-spawning would have caused, just deferred.
- Closing the terminal sends `SIGHUP` and typically kills the daemon. Surviving that is the entire point of a daemon.
- Depending on how it was backgrounded, `Ctrl-C` aimed at the shell can reach it, since it is still in that session.

So there are two separate steps: **redirect stdio** (fixes the screen), and **`setsid()`** (fixes the signals).

Redirect to `~/.config/poke/daemon.log` rather than `/dev/null` — it is the only way to debug a process with no visible output.

The traditional recipe is fork → parent exits → `setsid()` → fork again (so the daemon cannot reacquire a tty) → redirect stdio → `chdir("/")`. The `daemonize` crate does all of it:

```rust
Daemonize::new()
    .stdout(log_file)
    .stderr(log_file_clone)
    .start()?;
```

Worth using rather than hand-rolling — the double-fork has subtleties and it is not the interesting part of this project.

### Hazard: does the lock survive the fork?

The lock is taken *before* `daemonize()` forks twice, so it must survive into the grandchild.

With BSD `flock()` this works: `fork` duplicates the fd, both refer to the same open file description, and the lock is released only when *all* fds referring to that description are closed — so the parent exiting does not drop it. With POSIX `fcntl()` record locks it does **not** work: those are not inherited across `fork`.

Which one `fs4` uses on Unix needs verifying rather than assuming. The test is two lines: start the daemon, then run `poke d` again and confirm it reports "already running" rather than starting a second one.

The `File` handle must also be kept alive for the daemon's whole lifetime. Dropping it closes the fd and releases the lock, and a `let _ = lock_file;` binding will drop it immediately — bind it to a real name that lives in the loop's scope.

---

## Reload: mtime polling

The daemon already wakes on an interval to evaluate boundaries, so noticing edits costs one extra line in a loop that exists regardless:

```
loop {
    if stat(timers.toml).mtime != last_seen {
        reload
        last_seen = mtime
    }
    check boundaries, fire if due
    sleep
}
```

**Polling here is not expensive.** `stat()` reads cached inode metadata — no disk I/O, sub-microsecond. Doing it once a second is free in any meaningful sense.

**Restart-on-write was considered and rejected.** Having the TUI restart the daemon on every edit is a kill-write-spawn sequence with ordering hazards, it discards the daemon's `last_checked` state, and toggling five timers quickly spawns five daemons. Real file watching (inotify/kqueue via the `notify` crate) is the technically correct answer but adds a dependency and a second event source to reason about, for a benefit measured in milliseconds.

**Atomic writes are mandatory now, not just nice.** Write `timers.toml.tmp`, then `fs::rename` over the real path. `rename()` is atomic on POSIX, so the daemon always sees either the complete old file or the complete new one, never a half-written one.

Consequence: the daemon must `stat` the **path**, not hold an open file descriptor. `rename` replaces the directory entry; a held fd keeps pointing at the old, now-unlinked inode and would never see another update.

---

## Timer semantics

A timer is a **daily-recurring window**:

- `start` — a time of day
- `end` — a time of day (required)
- `interval` — a duration

It fires at `start`, then every `interval` after, while the result is `<= end`. The window resets daily; nothing is synchronised to when either process started.

```
boundary = start + n * interval,  n >= 0,  while boundary <= end
```

`09:00 → 10:00 every 25m` fires at 09:00, 09:25, 09:50. Not 10:15.

`start` itself **is** a boundary: a 30m timer set for 15:00 while it is 09:41 fires at 15:00, 15:30, 16:00.

Missed boundaries are skipped. Closed the daemon, or slept the machine, across four boundaries → one fire on the next check, not four.

### The `last_checked` interval

"Does `now` equal a boundary?" has no well-defined answer — `now` is an instant, the boundary is an instant, and the loop never observes the exact instant. Polling and asking that question either misses boundaries entirely or, if truncated to the minute, fires repeatedly for the same one.

The predicate is over an **interval**:

> Did at least one boundary fall within `(last_checked, now]`?

Half-open on the left, so a boundary exactly at `last_checked` (already fired) does not re-fire. Closed on the right, so a boundary exactly at `now` does fire.

One expression handles both failure modes:

| Situation          | Interval width | Boundaries inside | Fires |
|--------------------|----------------|-------------------|-------|
| Normal running     | one tick       | 0 or 1            | 0 or 1 |
| Woke from 3h sleep | ~3h            | many              | 1 — the predicate is boolean |

`last_checked` is:

- **daemon-only** — it does not exist in the TUI at all, and is never serialized
- **per-timer**, not global — so enabling a timer sets *its* `last_checked = now` at that moment, and a timer enabled at 15:00:00.100 cannot fire for the 15:00:00.000 boundary it was not enabled for
- initialized to `Local::now()` when the daemon starts, so launching mid-window never fires retroactively

Because `last_checked` lives only in the daemon, `TimerRuntime` disappears from the TUI entirely — `Vec<(Timer, TimerRuntime)>` collapses back to `Vec<Timer>`, and all the `.0` accessors go with it. The TUI can still display "rings in 12m": that is a pure function of `(start, interval, end, now)` and needs no runtime state.

### Design rule: no clock reads inside the logic

```rust
fn should_fire(timer: &Timer, last_checked: DateTime<Local>, now: DateTime<Local>) -> bool
```

`now` is a parameter. `should_fire` never calls `Local::now()`. It is a pure function of three inputs, so the entire firing logic is unit-testable in microseconds with no process, no UI, and no waiting.

The same rule applies to rendering: `Local::now()` is read **once** at the top of `ui::draw` and threaded down. Two independent `now()` calls in one frame can straddle a minute boundary and disagree.

Tests worth writing:

- boundary exactly at `now` → fires
- boundary exactly at `last_checked` → does not fire
- gap spanning several boundaries → fires once
- gap spanning the whole window → fires once
- `now` outside the window → no fire
- gap crossing midnight into the next day's window
- `end` not landing on a boundary (09:00–10:00 every 25m)
- `end <= start` → rejected at validation; v1 does not support windows crossing midnight

### DST

`Local.from_local_datetime()` returns `LocalResult` with three variants, because a local wall-clock time can be ambiguous (occurs twice) or nonexistent (skipped) on transition days. Pick a branch and move on.

### Notifications

The daemon fires an OS desktop notification, falling back to the terminal bell (`\x07`).

`notify-rust` on macOS goes through `mac-notification-sys`, which wants an application bundle identifier — a bare `cargo run` binary is not a bundle, so notifications can silently do nothing until `set_application()` is called with a valid id. Build the bell fallback first so this is never blocking.

---

## The config directory

`~/.config/poke/` on both Linux and macOS, via `etcetera`'s `choose_base_strategy()`, which uses XDG on both.

| File               | Written by | Read by      | Purpose                                  |
|--------------------|------------|--------------|------------------------------------------|
| `config.toml`      | the user   | both         | Formats and colours                      |
| `timers.toml`      | TUI        | both         | The timers. Single writer.               |
| `timers.toml.tmp`  | TUI        | —            | Transient; renamed over `timers.toml`    |
| `daemon.lock`      | daemon     | both         | Advisory lock + daemon PID               |
| `daemon.log`       | daemon     | —            | stdout/stderr after detaching            |

`poke_dir()` resolves the directory; `config_path()`, `timers_path()` and `lock_path()` build on it. One place to `create_dir_all` before the first write.

### `config.toml`

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub time_format: String,     // "%H:%M"
    pub date_format: String,     // "%A, %B %-d %Y"
    pub accent: Color,           // Color::Yellow
    pub selected_bg: Color,      // Color::LightYellow
    pub selected_text: Color,    // Color::Black
}
```

`#[serde(default)]` at struct level means every missing field falls back to `Default` — a file containing only `accent = "blue"` is valid. `deny_unknown_fields` turns `acccent = "blue"` into an error rather than silence. `ratatui::style::Color` already implements `Deserialize` behind ratatui's `serde` feature and accepts both names and `#rrggbb`, so no wrapper enum is needed.

Load-or-default never fails hard: missing file → defaults silently; malformed file → defaults with a message on stderr, which is safe only because this runs *before* `ratatui::run()` takes the screen.

**Unhandled:** `time_format` and `date_format` are user strings handed to chrono's `format()`, which **panics** on an invalid specifier at display time. `time_format = "%Q"` would crash inside `terminal.draw` — exactly the case that leaves a wrecked terminal. Validate at load by formatting a known date and falling back on failure.

### `timers.toml`

Serialized `Timer` only. `last_checked` is never written.

`Duration`'s default serde impl writes a `{ secs, nanos }` struct. Acceptable for a machine-written file; if it should stay hand-editable, a `#[serde(with = "...")]` module wrapping `humantime` keeps it as `"30m"`.

---

## Styling

Two shades of one accent, plus a text colour for anything drawn on a filled background.

| Element               | Style                                                   |
|-----------------------|---------------------------------------------------------|
| Focused panel title   | `.fg(accent)` + bold + italic                           |
| Unfocused panel title | `Style::default()`                                      |
| Selected table row    | `.bg(selected_bg)` + `.fg(selected_text)`               |
| Focused form field    | `.bg(selected_bg)` + `.fg(selected_text)` + italic      |
| Invalid form field    | `.bg(#E88A8A)` + `.fg(selected_text)` + italic          |
| Logo art              | `.fg(selected_bg)`                                      |

- **Named for the colour, not the position.** The soft shade is a foreground on the logo and a background on rows; naming them `fg`/`bg` would be actively misleading.
- **`selected_text` is configurable, not a hardcoded black.** Black on light yellow is fine; black on a dark blue accent is unreadable, and hardcoding makes that unfixable.
- **Focused titles get no background.** Tried and rejected — a filled title on a rounded border reads as noise. This also keeps one visual language for "focused panel" and another for "selected item within a panel", so a focused panel containing a selected row does not shout twice.
- **Palette colours by default, hex allowed.** `Color::Yellow` is palette index 3, so it inherits the user's terminal theme and harmonises out of the box. The trade-off is that the two shades may not differ much in every theme — hence hex overrides.
- **Italic support is patchy.** Some terminals ignore SGR 3 and a few render it as inverse. Accepted risk.

---

## Layout

```
src/
  main.rs        // arg dispatch only — chooses TUI or daemon
  paths.rs       // poke_dir, config_path, timers_path, lock_path
  config.rs      // Config + load-or-default
  storage.rs     // load/save timers, atomic write
  timer.rs       // Timer + parsing + should_fire + tests
  notify.rs      // fire() with bell fallback
  lock.rs        // acquire/check the daemon lock

  daemon.rs      // the daemon loop: reload on mtime, evaluate, fire

  tui.rs         // run_tui: terminal setup + event loop
  app.rs         // App + FormState, Focus/Field, update(Action)
  action.rs      // Action enum + map_key(key, focus)
  ui/
    mod.rs       // layout + shared helpers
    header.rs    // logo + clock + recap
    list.rs      // timer table
    form.rs      // new-timer form
```

Note `ui/mod.rs` **is** the `ui` module, not a sibling of `header.rs` but its parent. Private items in `mod.rs` are therefore visible to `header.rs`, `list.rs` and `form.rs` with no `pub`, because privacy in Rust means "this module and its descendants".

### Responsibilities

- **`main.rs`** — parses `argv[1]`, calls one of four entry points. No logic.
- **`daemon.rs`** — lock, load, then loop: reload on mtime change, `should_fire` each timer, fire, sleep. Never writes `timers.toml`.
- **`tui.rs`** — checks the lock, refuses to start without a daemon, then `ratatui::run()` and the event loop: draw, `poll(250ms)`, `map_key`, `update`, repeat while `!should_quit`. No per-key logic.
- **`app.rs`** — the single source of truth for the TUI. Timers, table state, form buffer, validation errors, focus. The **only** place TUI state mutates, via `update(Action)`. Persists to disk on every change.
- **`action.rs`** — `Action` enum plus `map_key(key, focus) -> Option<Action>`. Knows keys; knows nothing about `App`. Pure and testable. Lives at the crate root, not in `ui/`, so the import arrows stay one-way — `app` consumes `Action`, and `ui` is never imported by `app`.
- **`timer.rs`** — `Timer`, interval/time parsing, `should_fire`. Knows nothing about terminals, files, or the system clock. The only module with real logic, and the only one that genuinely needs tests.
- **`storage.rs`** — reads and writes `timers.toml`, atomically.
- **`ui/*`** — rendering only. Each submodule exposes one `draw(frame, area, ...)`; helpers below it stay private.

### The lines that matter

1. `timer.rs` knows nothing about the terminal.
2. `ui/` knows nothing about time or files.
3. `app.rs` is the only place TUI state changes.
4. Only the daemon fires. If the TUI also ran `should_fire`, every notification would arrive twice while it was open.

**One deliberate exception to (2) and (3).** `render_stateful_widget` needs `&mut TableState`, and `TableState` owns both selection and scroll offset, so it must persist across frames — it lives in `App`, and `ui::draw` takes `&mut App`. The exception is narrow: the UI may mutate `table_state` and nothing else.

This works because the borrow checker tracks individual fields: `draw_list(frame, area, &app.timers, &mut app.table_state)` is legal — one shared and one mutable borrow of *disjoint* fields. Prefer passing the specific fields a function needs over `&mut App` wholesale.

`Config` living in `App` and being read by `ui/` is a second, smaller exception. Reading a colour is not file access — the loading happened elsewhere — and threading a fourth parameter through every draw function would be worse.

---

## State shapes

```rust
pub struct App {
    pub timers: Vec<Timer>,
    pub table_state: TableState,
    pub current_focus: Focus,
    pub form_state: FormState,
    pub config: Config,
    pub should_quit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus { List, Form }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field { Name, Start, End, Interval }
```

`Focus` and `Field` need `Copy` so they can be passed by value while `App` is otherwise borrowed, and `PartialEq` for comparison. `Field`'s declaration order matches the on-screen order, so `focus_next`/`focus_prev` follow the visual layout. `Focus` is flat rather than carrying a selected index, because `TableState` already owns that.

```rust
// serde, written to timers.toml
pub struct Timer {
    pub name: String,
    pub interval: Duration,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub enabled: bool,
}

// daemon only, never serialized, never in App
pub struct TimerRuntime {
    pub last_checked: DateTime<Local>,
}
```

### Form validation

Parse, don't validate — and parse **once**. A single `to_timer(&self) -> Result<Timer, Vec<Field>>` either produces a valid `Timer` or reports every field that failed. Splitting this into a `validate()` and a `convert()` means parsing twice, and the two can disagree — the classic failure being a `convert()` that silently falls back to midnight on input `validate()` should have caught.

`Vec<Field>` and not `Vec<(Field, bool)>`: presence in the vec *is* the error, so the bool could only ever be `true`.

No error messages are stored. The user learns the expected shape from the labels (`Start (HH:MM)`, `Interval (30m, 1h30m)`), which is better than a message because it is visible *before* they get it wrong.

Errors clear per-field on edit: `errors.retain(|f| *f != edited_field)` in `type_char`, so the red disappears as soon as the offending field is touched.

Ratatui ships no text-input widget by design — a field at this level *is* a `String` plus `frame.set_cursor_position()`. Cursor offsets use `chars().count()`, not `len()`: `len()` is bytes, and an accented character would put the cursor past the text.

### Bounds safety

`TableState::selected()` can return a stale index after a delete, and both `Vec::remove` and `[]` panic out of range. Guard with `let ... else` plus `get_mut`. After a delete, the surviving elements shift *down*, so the index already points at the following timer — `select_next()` would skip one. Only three cases need handling: list emptied → `select(None)`; deleted the last row → select `len() - 1`; anything else → leave the index alone.

---

## Dependencies

| Crate         | Status  | Purpose                                              |
|---------------|---------|------------------------------------------------------|
| `ratatui`     | added   | TUI rendering, with the `serde` feature for `Color`  |
| `chrono`      | added   | Date/time types, strftime formatting                 |
| `etcetera`    | added   | XDG config paths                                     |
| `serde`       | added   | Derive for the persisted structs                     |
| `toml`        | added   | Config and timer serialization                       |
| `fs4`         | pending | Advisory file locking for the daemon                 |
| `daemonize`   | pending | Double-fork, `setsid`, stdio redirection             |
| `nix`         | pending | `kill()` for `poke stop`                             |
| `notify-rust` | pending | Desktop notifications                                |
| `anyhow`      | pending | `Result` + `?` + `.context(...)`                     |

- **Do not add `crossterm` separately.** `ratatui` re-exports it as `ratatui::crossterm`. Adding it separately risks two incompatible versions in the graph and errors where `KeyCode` is not `KeyCode`.
- **Do not use tokio.** Single-threaded poll loops cover both processes.
- **No panic hook needed.** `ratatui::run()` handles init, restore, and panic hooks. Tutorials showing manual `enable_raw_mode()` / `EnterAlternateScreen` are out of date.
- **No `regex`.** Interval parsing is `split_once('h')` plus `strip_suffix('m')` and a couple of `?`s.
- **No `clap`.** Four subcommands is a `match` on `args().nth(1)`.

---

## Next steps

1. **Drop `last_checked` from `App`.** `TimerRuntime` leaves the TUI; `Vec<(Timer, TimerRuntime)>` becomes `Vec<Timer>` and the `.0`s go away.
2. **`storage.rs`.** Load at startup, atomic save on every edit. `App::mock()` can be deleted once this works.
3. **`timer.rs` with its tests.** `should_fire` proven correct as a pure function, with no process management in the picture. Wire it into the existing TUI loop firing a bell, purely to watch it work.
4. **Split out the daemon.** Move that logic to `daemon.rs`, add the lock, add arg dispatch, remove firing from the TUI. Mechanical — the logic does not change, only which process runs it.
5. **Real notifications.** Last, because it is the piece most likely to eat an afternoon on macOS.

Building the daemon before step 3 would mean debugging boundary math and IPC simultaneously, in a process with no visible output.