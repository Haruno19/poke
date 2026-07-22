mod header;
mod list;

use chrono::{DateTime, Local};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;

const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 20;
const HEADER_HEIGHT: u16 = 9;

pub fn draw(frame: &mut Frame, app: &mut App) 
{
    let now: DateTime<Local> = Local::now();

    let area = frame.area();

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT 
    {
        let msg = Paragraph::new(format!(
            "terminal too small\nneed {MIN_WIDTH}x{MIN_HEIGHT}, have {}x{}",
            area.width, area.height
        ))
        .centered();
        frame.render_widget(msg, area);
        return;
    }

    let [header, body] =
        Layout::vertical([Constraint::Length(HEADER_HEIGHT), Constraint::Min(0)]).areas(area);
    let [list, form] =
        Layout::horizontal([Constraint::Fill(2), Constraint::Fill(1)]).areas(body);

    header::draw(frame, header, app, now);
    list::draw(frame, list, app);
    frame.render_widget(panel(" new "), form);
}

//——— Helpers —————————————————————————————————/

fn panel(title: &str) -> Block<'_> 
{
    Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
}

fn centered_area(len: u16, area: Rect) -> Rect 
{
    let [centered] = Layout::vertical([Constraint::Length(len)])
        .flex(Flex::Center)
        .areas(area);
    return centered;
}