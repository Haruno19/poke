use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use crate::app::{App, Field, Focus, FormState, HOUR_LEN};
use crate::config::Config;
use super::{panel};

//——— Dimensions —————————————————————————————————/

const HOUR_SPACE: u16 = 10;

//——— Render —————————————————————————————————————/

pub(super) fn draw(frame: &mut Frame, area: Rect, app: & App) 
{
    let form_block = panel(" new ", app.current_focus == Focus::Form, app.config.accent);
    let inner = form_block.inner(area).inner(Margin::new(2, 1));
    
    frame.render_widget(form_block, area);
    draw_form(frame, inner, &app, app.current_focus == Focus::Form);
}   


fn draw_form(frame: &mut Frame, area: Rect, app: &App, is_focused: bool) 
{
    let [name_area, _, times_area, _, interval_area, _] = Layout::vertical([
        Constraint::Length(2), Constraint::Length(1),
        Constraint::Length(2), Constraint::Length(1),
        Constraint::Length(2), Constraint::Min(0),
    ]).areas(area);

    let [start_area, _, end_area] = Layout::horizontal([
        Constraint::Length(HOUR_LEN), Constraint::Length(HOUR_SPACE), Constraint::Length(HOUR_LEN),
    ]).areas(times_area);

    field(frame, name_area, "Name", Field::Name, &app.form_state, is_focused, &app.config);
    field(frame, start_area, "Start", Field::Start, &app.form_state, is_focused, &app.config);
    field(frame, end_area, "End", Field::End, &app.form_state, is_focused, &app.config);
    field(frame, interval_area, "Interval", Field::Interval, &app.form_state, is_focused, &app.config);
}

//——— Helpers —————————————————————————————————————/

fn field(frame: &mut Frame, area: Rect, label: &str, which: Field, form: &FormState, panel_focused: bool, config: &Config)
{
    let [label_area, value_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    let [value_area_sized] =
        Layout::horizontal([Constraint::Length(which.max_len() as  u16)]).areas(value_area);
    
    let value = match which {
        Field::Name => &form.name,
        Field::Start => &form.start,
        Field::End => &form.end,
        Field::Interval => &form.interval,
    };
    let field_focused = form.focused == which;

    frame.render_widget(Paragraph::new(label).style(Style::new().bold()), label_area);
    frame.render_widget(Paragraph::new(value.as_str()).style(field_style(field_focused, form.errors.contains(&which), panel_focused, config)), value_area_sized);
    if field_focused && panel_focused
    {
        frame.set_cursor_position((value_area_sized.x + value.chars().count() as u16, value_area_sized.y));
    }
}

fn field_style(field_focused: bool, has_error:bool, panel_focused: bool, config: &Config) -> Style {
    let base = Style::new().italic();

    if !panel_focused {
        base
    } else if has_error {
        base.bg(Color::Rgb(0xE8, 0x8A, 0x8A)).fg(Color::Rgb(0x00, 0x00, 0x00))
    } else if field_focused {
        base.bg(config.selected_bg).fg(config.selected_text)
    } else {
        base.underlined()
    }
}