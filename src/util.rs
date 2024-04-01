use std::{io, str::FromStr};

use config::{Config, File};
use crossterm::{
    cursor,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use lazy_static::lazy_static;
use ratatui::style::Color;

use crate::boxes::{errorbox::ErrorBox, help_box::HelpBox, linkbox::LinkBox, searchbox::SearchBox};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    View,
    FileTree,
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
        Self::FileTree
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

    pub fn last(&self) -> Option<&Jump> {
        self.history.last()
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

#[derive(Debug)]
pub struct MdConfig {
    // General settings
    pub width: u16,

    // Inline styles
    pub italic_color: Color,
    pub bold_color: Color,
    pub striketrough_color: Color,
    pub bold_italic_color: Color,
    pub code_fg_color: Color,
    pub code_bg_color: Color,
    pub link_color: Color,
    pub link_selected_fg_color: Color,
    pub link_selected_bg_color: Color,

    // Block styles
    pub code_block_fg_color: Color,
    pub code_block_bg_color: Color,
    pub heading_fg_color: Color,
    pub heading_bg_color: Color,
    pub table_header_fg_color: Color,
    pub table_header_bg_color: Color,
    pub quote_bg_color: Color,

    // File tree
    pub file_tree_selected_fg_color: Color,
    pub file_tree_page_count_color: Color,
    pub file_tree_name_color: Color,
    pub file_tree_path_color: Color,
    pub gitignore: bool,

    // Quote markings
    pub quote_important: Color,
    pub quote_warning: Color,
    pub quote_tip: Color,
    pub quote_note: Color,
    pub quote_caution: Color,
    pub quote_default: Color,
}

lazy_static! {
    pub static ref CONFIG: MdConfig = {
        let config_dir = dirs::home_dir().unwrap();
        let config_file = config_dir.join(".config").join("mdt").join("config.toml");
        let settings = Config::builder()
            .add_source(File::with_name(config_file.to_str().unwrap()).required(false))
            .build()
            .unwrap();

        MdConfig {
            width: settings.get::<u16>("width").unwrap_or(80),
            heading_bg_color: Color::from_str(
                &settings.get::<String>("h_bg_color").unwrap_or_default(),
            )
            .unwrap_or(Color::Blue),
            heading_fg_color: Color::from_str(
                &settings.get::<String>("h_fg_color").unwrap_or_default(),
            )
            .unwrap_or(Color::Black),
            italic_color: Color::from_str(
                &settings.get::<String>("italic_color").unwrap_or_default(),
            )
            .unwrap_or(Color::Reset),
            bold_color: Color::from_str(&settings.get::<String>("bold_color").unwrap_or_default())
                .unwrap_or(Color::Reset),
            striketrough_color: Color::from_str(
                &settings
                    .get_string("striketrough_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Reset),
            quote_bg_color: Color::from_str(
                &settings.get::<String>("quote_bg_color").unwrap_or_default(),
            )
            .unwrap_or(Color::Reset),
            code_fg_color: Color::from_str(
                &settings.get::<String>("code_fg_color").unwrap_or_default(),
            )
            .unwrap_or(Color::Red),
            code_bg_color: Color::from_str(
                &settings.get::<String>("code_bg_color").unwrap_or_default(),
            )
            .unwrap_or(Color::Rgb(48, 48, 48)),
            code_block_fg_color: Color::from_str(
                &settings
                    .get::<String>("code_block_fg_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Red),
            code_block_bg_color: Color::from_str(
                &settings
                    .get::<String>("code_block_bg_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Rgb(48, 48, 48)),
            link_color: Color::from_str(&settings.get::<String>("link_color").unwrap_or_default())
                .unwrap_or(Color::Blue),
            link_selected_fg_color: Color::from_str(
                &settings
                    .get::<String>("link_selected_fg_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Green),
            link_selected_bg_color: Color::from_str(
                &settings
                    .get::<String>("link_selected_bg_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::DarkGray),
            table_header_fg_color: Color::from_str(
                &settings
                    .get::<String>("table_header_fg_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Yellow),
            table_header_bg_color: Color::from_str(
                &settings
                    .get::<String>("table_header_bg_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Reset),
            file_tree_selected_fg_color: Color::from_str(
                &settings
                    .get::<String>("file_tree_selected_fg_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::LightGreen),
            file_tree_page_count_color: Color::from_str(
                &settings
                    .get::<String>("file_tree_page_count_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::LightGreen),
            file_tree_name_color: Color::from_str(
                &settings
                    .get::<String>("file_tree_name_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Blue),
            file_tree_path_color: Color::from_str(
                &settings
                    .get::<String>("file_tree_path_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::DarkGray),
            bold_italic_color: Color::from_str(
                &settings
                    .get::<String>("bold_italic_color")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::Reset),
            gitignore: settings.get::<bool>("gitignore").unwrap_or_default(),
            quote_important: Color::from_str(
                &settings
                    .get::<String>("quote_important")
                    .unwrap_or_default(),
            )
            .unwrap_or(Color::LightRed),
            quote_warning: Color::from_str(
                &settings.get::<String>("quote_warning").unwrap_or_default(),
            )
            .unwrap_or(Color::LightYellow),

            quote_tip: Color::from_str(&settings.get::<String>("quote_tip").unwrap_or_default())
                .unwrap_or(Color::LightGreen),

            quote_note: Color::from_str(&settings.get::<String>("quote_note").unwrap_or_default())
                .unwrap_or(Color::LightBlue),

            quote_caution: Color::from_str(
                &settings.get::<String>("quote_caution").unwrap_or_default(),
            )
            .unwrap_or(Color::LightMagenta),

            quote_default: Color::from_str(
                &settings.get::<String>("quote_default").unwrap_or_default(),
            )
            .unwrap_or(Color::White),
        }
    };
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
