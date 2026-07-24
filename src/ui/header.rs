use chrono::{DateTime, Local, Timelike};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::timer::{Timer};
use super::{centered_area, panel};

//——— Dimensions —————————————————————————————————/

const LOGO_HEIGHT: u16 = 5;
const LOGO_WIDTH: u16 = 18;
const CLOCK_HEIGHT: u16 = 3;
const RECAP_HEIGHT: u16 = 3;

//——— Render ————————————————————————————————————/

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &App, now: DateTime<Local>) 
{
    let header_block = panel(" poke ", false, app.config.accent).title_style(Style::new().bold());
    let inner = header_block.inner(area);
    frame.render_widget(header_block, area);

    let [logo, clock, recap] =
        Layout::horizontal([Constraint::Length(LOGO_WIDTH), Constraint::Fill(2), Constraint::Fill(1)]).areas(inner);
    let logo_area = centered_area(LOGO_HEIGHT, logo);
    let clock_area = centered_area(CLOCK_HEIGHT, clock);
    let recap_area = centered_area(RECAP_HEIGHT, recap);
    frame.render_widget(draw_logo(now, app.config.accent), logo_area);
    frame.render_widget(draw_clock(now, &app.config.time_format, &app.config.date_format), clock_area);
    frame.render_widget(draw_recap(&app.timers, app.config.accent), recap_area);
}

fn draw_logo(now: DateTime<Local>, color: Color) -> Paragraph<'static> 
{
    let art = match now.hour() 
    {
        5..=11 => SUNRISE,
        12..=17 => DAY,
        18..=21 => SUNSET,
        _ => NIGHT,
    };
    return Paragraph::new(art).style(Style::new().fg(color));
}

fn draw_clock(now: DateTime<Local>, time_format: &String, date_format: &String) -> Paragraph<'static> 
{
    return Paragraph::new(vec![
        Line::from(now.format(time_format).to_string()).style(Style::new().bold()),
        Line::from(""),
        Line::from(now.format(date_format).to_string()).style(Style::new().italic()),
    ])
}

fn draw_recap(timers: &[Timer], accent: Color) -> Paragraph<'static> 
{
    let active = timers.iter().filter(|t| t.enabled).count();
    let inactive = timers.len() - active;

    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(format!("{active}"), Style::new().bold().fg(accent)),
            Span::from(format!(" active timer{}", plural(active)))
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{inactive}"), Style::new().bold().fg(accent)),
            Span::from(format!(" inactive timer{}", plural(inactive)))
        ]),
    ])
}

//——— Helpers —————————————————————————————————/

fn plural(n: usize) -> &'static str 
{
    if n == 1 { "" } else { "s" }
}

//——— Drawings ———————————————————————————————/

const SUNRISE: &str = r#"   \   |   /   
  ─  ▄███▄  ─     
  ▁▁███████▁▁  
    ░▒▒▒▒▒░        
     ░░▒▒░     "#;

const DAY: &str = r#"   \   |   /   
     ▄███▄     
  ─ ███████ ─  
     ▀███▀     
   /   |   \   "#;

const NIGHT: &str = r#" ✦    ▄▄▄▄     
    ▄██████▒   
   ███████▒▒  ·
    ▀█████▒▒   
 ·    ▀▀▀▀   ✦ "#;

const SUNSET: &str = r#"       |       
     ▄███▄     
  ─ ███████ ─  
  ▁▁▁▀█▀▁▁▁▁▁  
      ░▒░      "#;