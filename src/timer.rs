use std::time::Duration;
use chrono::NaiveTime;

#[derive(Debug, Clone)]
pub struct Timer {
    pub name: String,
    pub interval: Duration,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub enabled: bool,
}