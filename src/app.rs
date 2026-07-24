use std::time::Duration;

use crate::config::Config;
use crate::timer::{Timer, parse_interval};
use crate::action::Action; 

use chrono::{NaiveTime};
use ratatui::widgets::TableState;

//——— Field Dimentions ——————————————————————————/

pub const NAME_LEN: u16 = 21;
pub const HOUR_LEN: u16 = 5; 
pub const INTERVAL_LEN: u16 = 6; 

//——— State Structures ——————————————————————————/

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus
{
    List,
    Form,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field
{
    Name,
    Interval,
    Start,
    End,
}

impl Field {
    pub fn max_len(self) -> usize {
        match self {
            Field::Name => NAME_LEN as usize,
            Field::Start | Field::End => HOUR_LEN as usize,
            Field::Interval => INTERVAL_LEN as usize,
        }
    }
}

//——— Form State ————————————————————————————————/

pub struct FormState {
    pub name: String,
    pub start: String,
    pub end: String,
    pub interval: String,
    pub focused: Field,
    pub errors: Vec<Field>,
}

impl FormState 
{
    pub fn new() -> Self 
    {
        Self 
        { 
            name: String::new(), 
            interval: String::new(), 
            start: String::new(), 
            end: String::new(), 
            focused: Field::Name, 
            errors: Vec::new(),
        }
    }

    pub fn to_timer(&self) -> Result<Timer, Vec<Field>> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push(Field::Name);
        }

        let start = NaiveTime::parse_from_str(&self.start, "%H:%M").ok();
        if start.is_none() { errors.push(Field::Start); }

        let end = NaiveTime::parse_from_str(&self.end, "%H:%M").ok();
        if end.is_none() { errors.push(Field::End); }

        let interval = parse_interval(&self.interval);
        if interval.is_none() { errors.push(Field::Interval); }

        // cross-field check: only meaningful if both parsed
        if let (Some(s), Some(e)) = (start, end) {
            if e <= s { errors.push(Field::End); }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Timer {
            name: self.name.trim().to_string(),
            interval: interval.unwrap(),
            start: start.unwrap(),
            end: end.unwrap(),
            enabled: true,
        })
    }

    pub fn focus_next(&mut self) {
        self.focused = match self.focused {
            Field::Name => Field::Start,
            Field::Start => Field::End,
            Field::End => Field::Interval,
            Field::Interval => Field::Name
        }
    }

    pub fn focus_prev(&mut self) {
        self.focused = match self.focused {
            Field::Name => Field::Interval,
            Field::Start => Field::Name,
            Field::End => Field::Start,
            Field::Interval => Field::End
        }
    }

    fn focused_mut(&mut self) -> &mut String 
    {
        match self.focused {
            Field::Name => &mut self.name,
            Field::Start => &mut self.start,
            Field::End => &mut self.end,
            Field::Interval => &mut self.interval,
        }
    }

    pub fn type_char(&mut self, c: char) {
        self.errors.retain(|f| *f != self.focused); 

        let max = self.focused.max_len();
        let buf = self.focused_mut();

        if buf.chars().count() < max 
        {
            buf.push(c);
        }
    }

    pub fn delete_char(&mut self)
    {
        self.errors.retain(|f| *f != self.focused); 

        self.focused_mut().pop();
    }
}

//——— App State —————————————————————————————————/

pub struct App 
{
    pub timers:Vec<Timer>,
    pub table_state: TableState,
    pub current_focus: Focus,
    pub form_state: FormState,
    pub should_quit: bool,
    pub config: Config,
}

impl App 
{
    pub fn new() -> Self 
    {
        Self 
        { 
            timers: Vec::new(),
            table_state: TableState::default().with_selected(0),
            current_focus: Focus::List,
            form_state: FormState::new(),
            should_quit: false,
            config: Config::load(),
        }
    }

    pub fn mock() -> Self 
    {
        let mk = |name: &str, mins: u64, s: (u32, u32), e: (u32, u32), on: bool| 
        {
            Timer 
            {
                name: name.to_string(),
                interval: Duration::from_secs(mins * 60),
                start: NaiveTime::from_hms_opt(s.0, s.1, 0).unwrap(),
                end: NaiveTime::from_hms_opt(e.0, e.1, 0).unwrap(),
                enabled: on,
            }
        };

        Self 
        {
            timers: vec![
                mk("tea", 30, (13, 0), (18, 0), true),
                mk("stretch", 60, (9, 0), (17, 0), false),
            ],
            table_state: TableState::default().with_selected(0),
            current_focus: Focus::List,
            form_state: FormState::new(),
            should_quit: false,
            config: Config::load(),
        }
    }

    pub fn update(&mut self, action: Action)
    {   
        match action
        {
            Action::Quit => 
            { 
                self.should_quit = true; 
            }
            Action::FocusForm => 
            {
                self.current_focus = Focus::Form;
                self.table_state.select(None);
            },
            Action::FocusList => 
            {
                self.current_focus = Focus::List;
                self.table_state.select_first();
                self.form_state.errors = Vec::new();
            },

            Action::TableMoveUp => 
            {
                self.table_state.select_previous();
            },
            Action::TableMoveDown => 
            {
                self.table_state.select_next();
            },
            Action::ToggleTimer =>
            {
                let Some(i) = self.table_state.selected() else { return };
                if let Some(timer) = self.timers.get_mut(i) 
                {
                    timer.enabled = !timer.enabled;
                }
            },
            Action::DeleteTimer => {
                let Some(i) = self.table_state.selected() else { return };
                if i >= self.timers.len() { return }
                self.timers.remove(i);

                if self.timers.is_empty() 
                {
                    self.table_state.select(None);
                } else if i >= self.timers.len() {
                    self.table_state.select(Some(self.timers.len() - 1));
                }
            }

            Action::FormMoveDown =>
            {
                self.form_state.focus_next();
            },
            Action::FormMoveUp =>
            {
                self.form_state.focus_prev();
            },
            Action::AddChar(c) =>
            {
                self.form_state.type_char(c);
            },
            Action::RemoveChar =>
            {
                self.form_state.delete_char();
            },
            Action::SubmitForm => 
            {
                match self.form_state.to_timer() 
                {
                    Ok(timer) => 
                    {
                        self.timers.insert(0, timer);
                        self.form_state = FormState::new();
                        self.current_focus = Focus::List;
                        self.table_state.select_first();
                    }
                    Err(errors) => self.form_state.errors = errors,
                }
            }
        }
    }
}