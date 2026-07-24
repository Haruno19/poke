use std::fs;

use ratatui::style::Color;
use serde::Deserialize;

use crate::paths::config_path;

//——— Structs ——————————————————————————————————/

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config 
{
    pub time_format: String,
    pub date_format: String,
    pub accent: Color,
    pub selected_bg: Color,
    pub selected_text: Color,
}

impl Default for Config 
{
    fn default() -> Self 
    {
        Self 
        {
            time_format: "%H:%M".into(),
            date_format: "%A, %B %-d %Y".into(),
            accent: Color::Yellow,
            selected_bg: Color::LightYellow,
            selected_text: Color::Black,
        }
    }
}

impl Config
{
    pub fn load() -> Self 
    {
        let Some(path) = config_path() else { return Self::default() };
        let Ok(text) = fs::read_to_string(&path) else { return Self::default() };

        match toml::from_str(&text) 
        {
            Ok(config) => config,
            Err(e) => 
            {
                eprintln!("poke: bad config, using defaults: {e}");
                Self::default()
            }
        }
    }
}