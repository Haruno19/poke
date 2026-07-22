mod timer;
mod ui;

use std::io;
use std::time::Duration;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

fn main() -> io::Result<()> {
    ratatui::run(|mut terminal| run_app(&mut terminal))
}

fn run_app(terminal: &mut DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(ui::draw)?;

        // Waits up to 250ms for input, then returns false and lets the
        // loop continue. This is what keeps the clock ticking later on.
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    return Ok(());
                }
            }
        }
    }
}



