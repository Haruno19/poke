use std::time::Duration;
use chrono::{NaiveTime, DateTime, Local};

#[derive(Debug, Clone)]
pub struct Timer 
{
    pub name: String,
    pub interval: Duration,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct TimerRuntime 
{
    pub last_checked: DateTime<Local>,
}