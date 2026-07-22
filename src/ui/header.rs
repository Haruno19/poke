use chrono::{DateTime, Local, Timelike};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::timer::{Timer, TimerRuntime};
use super::{centered_area, panel};

const LOGO_HEIGHT: u16 = 5;
const LOGO_WIDTH: u16 = 18;
const CLOCK_HEIGHT: u16 = 3;
const RECAP_HEIGHT: u16 = 3;

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &App, now: DateTime<Local>) 
{
    let header_block = panel(" poke ");
    let inner = header_block.inner(area);
    frame.render_widget(header_block, area);

    let [logo, clock, recap] =
        Layout::horizontal([Constraint::Length(LOGO_WIDTH), Constraint::Fill(2), Constraint::Fill(1)]).areas(inner);
    let logo_area = centered_area(LOGO_HEIGHT, logo); 
    let clock_area = centered_area(CLOCK_HEIGHT, clock);
    let recap_area = centered_area(RECAP_HEIGHT, recap).inner(Margin::new(1, 0));
    frame.render_widget(draw_logo(now), logo_area);
    frame.render_widget(draw_clock(now), clock_area);
    frame.render_widget(draw_recap(&app.timers), recap_area);
}

fn draw_logo(now: DateTime<Local>) -> Paragraph<'static> 
{
    let art = match now.hour() 
    {
        5..=11 => SUNRISE,
        12..=17 => DAY,
        18..=21 => SUNSET,
        _ => NIGHT,
    };
    return Paragraph::new(art);
}

fn draw_clock(now: DateTime<Local>) -> Paragraph<'static> 
{
    // let (h, m) = (now.hour() as usize, now.minute() as usize);
    // let glyphs = [
    //     &DIGITS[h / 10], &DIGITS[h % 10],
    //     &COLON,
    //     &DIGITS[m / 10], &DIGITS[m % 10],
    // ];

    // let lines: Vec<Line> = (0..5)
    //     .map(|row| Line::from(glyphs.map(|g| g[row]).join(" ")))
    //     .collect();

    // return Paragraph::new(lines);

    return Paragraph::new(vec![
        Line::from(now.format("%H:%M").to_string()),
        Line::from(""),
        Line::from(now.format("%A, %B %-d, %Y").to_string()),
    ])
}

fn draw_recap(timers: &[(Timer, TimerRuntime)]) -> Paragraph<'static> 
{
    let active = timers.iter().filter(|(t, _)| t.enabled).count();
    let inactive = timers.len() - active;

    Paragraph::new(vec![
        Line::from(format!("{active} active timer{}", plural(active))),
        Line::from(""),
        Line::from(format!("{inactive} inactive timer{}", plural(inactive))),
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

// const DIGITS: [[&str; 5]; 10] = [
//     ["┏━━┓", "┃  ┃", "┃  ┃", "┃  ┃", "┗━━┛"], // 0
//     ["   ╻", "   ┃", "   ┃", "   ┃", "   ╹"], // 1
//     ["┏━━┓", "   ┃", "┏━━┛", "┃   ", "┗━━╸"], // 2
//     ["┏━━┓", "   ┃", "╺━━┫", "   ┃", "╺━━┛"], // 3
//     ["╻  ╻", "┃  ┃", "┗━━┫", "   ┃", "   ╹"], // 4
//     ["┏━━╸", "┃   ", "┗━━┓", "   ┃", "╺━━┛"], // 5
//     ["┏━━╸", "┃   ", "┣━━┓", "┃  ┃", "┗━━┛"], // 6
//     ["┏━━┓", "   ┃", "   ┃", "   ┃", "   ╹"], // 7
//     ["┏━━┓", "┃  ┃", "┣━━┫", "┃  ┃", "┗━━┛"], // 8
//     ["┏━━┓", "┃  ┃", "┗━━┫", "   ┃", "╺━━┛"], // 9
// ];

// const COLON: [&str; 5] = [" ", "●", " ", "●", " "];