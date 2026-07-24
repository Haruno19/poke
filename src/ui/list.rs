use ratatui::Frame;
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::style::{Style};
use ratatui::widgets::{Row, Table, TableState};

use crate::app::{App, Focus};
use crate::config::Config;
use crate::timer::{Timer, TimerRuntime};
use super::{panel};

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &mut App) 
{
    let list_block = panel(" timers ", app.current_focus == Focus::List, app.config.accent);
    let inner = list_block.inner(area).inner(Margin::new(1, 1));
    
    frame.render_widget(list_block, area);
    draw_list(frame, inner, &app.timers, &mut app.table_state, &app.config);
}   


fn draw_list(frame: &mut Frame, area: Rect, timers: &[(Timer, TimerRuntime)], table_state: &mut TableState, config: &Config) 
{
    let rows: Vec<Row> = timers
        .iter()
        .map(|(timer, _runtime)| build_row(timer))
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(7),
        Constraint::Length(5),
        Constraint::Length(6),
    ];
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .row_highlight_style(Style::new().bg(config.selected_bg).bold().fg(config.selected_text));

    frame.render_stateful_widget(table, area, table_state);
}

//——— Helpers —————————————————————————————————/

fn build_row(timer: &Timer) -> Row<'static> {
    Row::new([
        if timer.enabled {" ■"} else {" □"}.to_string(), 
        timer.name.to_string(), 
        format!("{hour}h{min:0>2}m", hour=(timer.interval.as_secs() / 60) / 60, min=(timer.interval.as_secs() / 60) % 60).to_string(),
        timer.start.format("%H:%M").to_string(),
        timer.end.format("%H:%M ").to_string(),
    ])
}
