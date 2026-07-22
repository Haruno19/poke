use ratatui::Frame;
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Row, Table, TableState};

use crate::app::App;
use crate::timer::{Timer, TimerRuntime};
use super::{panel};

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &mut App) 
{
    let list_block = panel(" list ");
    let inner = list_block.inner(area).inner(Margin::new(1, 0));
    
    frame.render_widget(list_block, area);
    draw_list(frame, inner, &app.timers, &mut app.table_state);
}   


fn draw_list(frame: &mut Frame, area: Rect, timers: &[(Timer, TimerRuntime)], table_state: &mut TableState) 
{
    let rows: Vec<Row> = timers
        .iter()
        .map(|(timer, _runtime)| build_row(timer))
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(5),
        Constraint::Length(6),
    ];
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .row_highlight_style(Style::new().on_dark_gray().bold().black())
        .column_highlight_style(Color::White);

    frame.render_stateful_widget(table, area, table_state);
}

//——— Helpers —————————————————————————————————/

fn build_row(timer: &Timer) -> Row<'static> {
    Row::new([
        if timer.enabled {" ■"} else {" □"}.to_string(), 
        timer.name.to_string(), 
        timer.start.format("%H:%M").to_string(),
        timer.end.format("%H:%M ").to_string(),
    ])
}
