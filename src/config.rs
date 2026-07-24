use std::{fs, path::{Path, PathBuf}};

use ratatui::style::Color;
use serde::Deserialize;
use etcetera::{choose_base_strategy, BaseStrategy};

//——— Paths ————————————————————————————————————/

const POKE_DIR: &str = "poke";
const CONFIG_FILE: &str = "config.toml";
const TIMERS_FILE: &str = "timers.toml";

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

//——— Helpers ————————————————————————————————————/

fn config_path() -> Option<PathBuf> {
    let mut path= poke_dir()?;
    path.push(CONFIG_FILE);
    Some(path)
}

fn timers_path() -> Option<PathBuf> {
    let mut path= poke_dir()?;
    path.push(TIMERS_FILE);
    Some(path)
}

pub fn poke_dir() -> Option<PathBuf> {
    let mut path = choose_base_strategy().ok()?.config_dir();
    path.push(POKE_DIR);
    Some(path)
}
