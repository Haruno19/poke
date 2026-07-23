use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Focus;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action
{
    //——— General ————/
    Quit,
    FocusList,
    FocusForm,
    //——— List ———————/
    TableMoveUp,
    TableMoveDown,
    ToggleTimer,
    DeleteTimer,
    //——— Form ———————/
    FormMoveUp,
    FormMoveDown,
    SubmitForm,
    RemoveChar,
    AddChar(char),
}

pub fn map_key(key: KeyEvent, focus: Focus) -> Option<Action> 
{
    use Focus::*;
    use KeyCode::*;

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) { return Some(Action::Quit) }
    match (key.code, focus) 
    {
        (Up,        List) => Some(Action::TableMoveUp),
        (Down,      List) => Some(Action::TableMoveDown),
        (Enter,     List) => Some(Action::ToggleTimer),
        (Char(' '), List) => Some(Action::ToggleTimer),
        (Backspace, List) | (Delete, List) => Some(Action::DeleteTimer),
        (Char('n'), List) => Some(Action::FocusForm),
        (Esc,       List) => Some(Action::Quit),

        (Up,        Form) => Some(Action::FormMoveUp),
        (Down,      Form) | (Tab, Form) => Some(Action::FormMoveDown),
        (Enter,     Form) => Some(Action::SubmitForm),
        (Backspace, Form) => Some(Action::RemoveChar),
        (Esc,       Form) => Some(Action::FocusList),
        (Char(c),   Form) => Some(Action::AddChar(c)),

        _ => None,
    }
}