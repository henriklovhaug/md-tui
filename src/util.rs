use std::io;

use crossterm::{
    cursor,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    View,
    FileTree,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Boxes {
    Error,
    Search,
    None,
}

impl Default for Mode {
    fn default() -> Self {
        Self::FileTree
    }
}

impl Default for Boxes {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Default, Clone, Copy)]
pub struct App {
    pub vertical_scroll: u16,
    pub selected: bool,
    pub select_index: usize,
    pub mode: Mode,
    pub boxes: Boxes,
}

impl App {
    fn reset(&mut self) {
        self.vertical_scroll = 0;
        self.selected = false;
        self.select_index = 0;
        self.boxes = Boxes::None;
    }
}

pub enum LinkType<'a> {
    Internal(&'a str),
    External(&'a str),
    MarkdownFile(&'a str),
}

impl<'a> From<&'a str> for LinkType<'a> {
    fn from(s: &'a str) -> Self {
        if s.starts_with("http") {
            return Self::External(s);
        }
        if s.starts_with('/') {
            return Self::MarkdownFile(s);
        }
        Self::Internal(s)
    }
}

pub fn destruct_terminal() {
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();
}
