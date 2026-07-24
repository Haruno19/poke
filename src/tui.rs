use std::io;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::action::map_key;
use crate::app::App;
use crate::ui;

pub fn run() -> io::Result<()> {
    ratatui::run(|mut terminal| event_loop(&mut terminal))
}

fn event_loop(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::mock();

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(action) = map_key(key, app.current_focus) {
                        app.update(action);
                    }
                }
            }
        }
    }

    Ok(())
}