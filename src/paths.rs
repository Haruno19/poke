use std::path::PathBuf;

use etcetera::{BaseStrategy, choose_base_strategy};

//——— Paths ——————————————————————————————————/

const POKE_DIR: &str = "poke";
const CONFIG_FILE: &str = "config.toml";
const TIMERS_FILE: &str = "timers.toml";
const TIMERS_TEMP_FILE: &str = "timers.toml.tmp";
const DAEMON_LOCK: &str = "daemon.lock";
const DAEMON_LOg: &str = "daemon.log";


//——— Helper —————————————————————————————————/

pub fn poke_dir() -> Option<PathBuf> {
    let mut path = choose_base_strategy().ok()?.config_dir();
    path.push(POKE_DIR);
    Some(path)
}

pub fn config_path() -> Option<PathBuf> {
    let mut path= poke_dir()?;
    path.push(CONFIG_FILE);
    Some(path)
}

pub fn timers_path() -> Option<PathBuf> {
    let mut path= poke_dir()?;
    path.push(TIMERS_FILE);
    Some(path)
}

pub fn lock_path() -> Option<PathBuf> {
    let mut path= poke_dir()?;
    path.push(DAEMON_LOCK);
    Some(path)
}
