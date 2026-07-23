use std::time::Duration;
use chrono::{NaiveTime, DateTime, Local};

#[derive(Debug, Clone)]
pub struct Timer 
{
    pub name: String,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub interval: Duration,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct TimerRuntime 
{
    pub last_checked: DateTime<Local>,
}

pub fn parse_interval(s: &str) -> Option<Duration> 
{
    let s = s.trim();
    let (hours, rest) = match s.split_once('h') 
    {
        Some((h, rest)) => (h.parse::<u64>().ok()?, rest),
        None => (0, s),
    };
    let mins = match rest.strip_suffix('m') 
    {
        Some(m) => m.parse::<u64>().ok()?,
        None if rest.is_empty() => 0,
        None => return None,
    };

    let total = hours * 3600 + mins * 60;
    (total > 0).then(|| Duration::from_secs(total))
}