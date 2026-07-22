use std::time::Duration;

use crate::timer::{Timer, TimerRuntime};
use chrono::{NaiveTime, Local};
use ratatui::widgets::TableState;

pub struct App 
{
    pub timers:Vec<(Timer, TimerRuntime)>,
    pub table_state: TableState,
}

impl App 
{
    pub fn new() -> Self 
    {
        Self { 
            timers: Vec::new(),
            table_state: TableState::default().with_selected(0),
        }
    }

    pub fn mock() -> Self 
    {
        let now = Local::now();
        let mk = |name: &str, mins: u64, s: (u32, u32), e: (u32, u32), on: bool| {
            (
                Timer 
                {
                    name: name.to_string(),
                    interval: Duration::from_secs(mins * 60),
                    start: NaiveTime::from_hms_opt(s.0, s.1, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(e.0, e.1, 0).unwrap(),
                    enabled: on,
                },
                TimerRuntime { last_checked: now },
            )
        };

        Self 
        {
            timers: vec![
                mk("tea", 30, (13, 0), (18, 0), true),
                mk("stretch", 60, (9, 0), (17, 0), false),
            ],
            table_state: TableState::default().with_selected(0),
        }
    }
}