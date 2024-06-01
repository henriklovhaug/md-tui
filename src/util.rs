use std::{cmp, io};

use crossterm::{
    cursor,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};

use crate::boxes::{errorbox::ErrorBox, help_box::HelpBox, linkbox::LinkBox, searchbox::SearchBox};

use self::colors::COLOR_CONFIG;

pub mod colors;
pub mod keybinds;
pub mod keys;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    View,
    FileTree,
    Loading,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Boxes {
    Error,
    Search,
    LinkPreview,
    None,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Loading
    }
}

impl From<JumpHistory> for Mode {
    fn from(jump_history: JumpHistory) -> Self {
        match jump_history.history.last() {
            Some(jump) => match jump {
                Jump::File(_) => Mode::View,
                Jump::FileTree => Mode::FileTree,
            },
            None => Mode::FileTree,
        }
    }
}

impl Default for Boxes {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Default, Clone)]
pub struct App {
    pub vertical_scroll: u16,
    width: u16,
    pub selected: bool,
    pub select_index: usize,
    pub mode: Mode,
    pub boxes: Boxes,
    pub history: JumpHistory,
    pub search_box: SearchBox,
    pub error_box: ErrorBox,
    pub help_box: HelpBox,
    pub link_box: LinkBox,
}

impl App {
    pub fn reset(&mut self) {
        self.vertical_scroll = 0;
        self.selected = false;
        self.select_index = 0;
        self.boxes = Boxes::None;
        self.help_box.close();
    }

    pub fn set_width(&mut self, width: u16) -> bool {
        let temp_width = self.width;
        self.width = cmp::min(width, COLOR_CONFIG.width);
        temp_width != self.width
    }

    pub fn width(&self) -> u16 {
        self.width
    }
}

pub enum LinkType<'a> {
    Internal(&'a str),
    External(&'a str),
    MarkdownFile(&'a str),
}

impl<'a> From<&'a str> for LinkType<'a> {
    fn from(s: &'a str) -> Self {
        if s.starts_with('#') {
            return Self::Internal(s);
        }
        if s.ends_with("md") {
            return Self::MarkdownFile(s);
        }
        return Self::External(s);
    }
}

pub fn destruct_terminal() {
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();
}

#[derive(Debug, Clone)]
pub struct JumpHistory {
    history: Vec<Jump>,
}

impl JumpHistory {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn push(&mut self, jump: Jump) {
        self.history.push(jump);
    }

    pub fn pop(&mut self) -> Jump {
        if let Some(jump) = self.history.pop() {
            jump
        } else {
            Jump::FileTree
        }
    }
}

impl Default for JumpHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Jump {
    File(String),
    FileTree,
}

#[cfg(test)]
#[test]
fn test_jump_history() {
    let mut jump_history = JumpHistory::default();
    jump_history.push(Jump::File("file".to_string()));
    jump_history.push(Jump::File("file2".to_string()));
    jump_history.push(Jump::FileTree);
    assert_eq!(jump_history.pop(), Jump::FileTree);
    assert_eq!(jump_history.pop(), Jump::File("file2".to_string()));
    assert_eq!(jump_history.pop(), Jump::File("file".to_string()));
    assert_eq!(jump_history.pop(), Jump::FileTree);
    assert_eq!(jump_history.pop(), Jump::FileTree);
    assert_eq!(jump_history.pop(), Jump::FileTree);
}
