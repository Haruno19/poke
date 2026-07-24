mod action;
mod app;
mod config;
mod timer;
mod tui;
mod ui;
mod paths;
mod storage;

use std::io;

fn main() -> io::Result<()> {
    tui::run()
}