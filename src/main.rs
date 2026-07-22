mod timer;
mod ui;
mod app;
mod action;

use std::io;
use std::time::Duration;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::app::App;

fn main() -> io::Result<()> 
{
    ratatui::run(|mut terminal| run_app(&mut terminal))
}

fn run_app(terminal: &mut DefaultTerminal) -> io::Result<()> 
{
    let mut app = App::mock();

    loop 
    {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        // Waits up to 250ms for input, then returns false and lets the
        // loop continue. This is what keeps the clock ticking later on.
        if event::poll(Duration::from_millis(250))? 
        {
            if let Event::Key(key) = event::read()? 
            {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    return Ok(());
                }
            }
        }
    }
}



