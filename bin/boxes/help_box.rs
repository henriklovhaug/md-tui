use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Row, Table, Widget},
};

use md_tui::config::colors::COLOR_CONFIG;

use crate::util::keys::KEY_CONFIG;
use crate::util::Mode;

#[derive(Debug, Clone, Copy, Default)]
pub struct HelpBox {
    mode: Mode,
    expanded: bool,
}

impl HelpBox {
    pub fn close(&mut self) {
        self.expanded = false;
    }

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn expanded(&self) -> bool {
        self.expanded
    }
}

impl Widget for HelpBox {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self.mode {
            Mode::View => render_markdown_help(self.expanded, area, buf),
            Mode::FileTree => render_file_tree_help(self.expanded, area, buf),
        }
    }
}

fn render_file_tree_help(expanded: bool, area: Rect, buf: &mut Buffer) {
    if !expanded {
        let text = Text::styled("? - Help", Style::default().fg(Color::LightGreen).bold());
        text.render(area, buf);
        return;
    }

    let header = Row::new(vec!["Key", "Action"]);

    let key_actions = [
        Row::new(vec![
            format!("{} or \u{2193}", KEY_CONFIG.down),
            "Move down".to_string(),
        ]),
        Row::new(vec![
            format!("{} or \u{2191}", KEY_CONFIG.up),
            "Move up".to_string(),
        ]),
        Row::new(vec![
            format!("{} or \u{2190}", KEY_CONFIG.page_up),
            "Go to previous page".to_string(),
        ]),
        Row::new(vec![
            format!("{} or \u{2192}", KEY_CONFIG.page_down),
            "Go to next page".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.top),
            "Move to first file".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.bottom),
            "Move to last file".to_string(),
        ]),
        Row::new(vec![
            format!("/ or {}", KEY_CONFIG.search),
            "Search".to_string(),
        ]),
        Row::new(vec!["\u{21b5}", "Open file"]),
        Row::new(vec!["q", "Quit"]),
    ];

    let widths = [12, 20];

    let table =
        Table::new(key_actions, widths).header(header.fg(COLOR_CONFIG.table_header_fg_color));
    table.render(area, buf);
}

fn render_markdown_help(expandend: bool, area: Rect, buf: &mut Buffer) {
    if !expandend {
        let text = Text::styled("? - Help", Style::default().fg(Color::LightGreen).bold());
        text.render(area, buf);
        return;
    }

    let header = Row::new(vec!["Key", "Action"]);

    let key_actions = [
        Row::new(vec![
            format!("{} or \u{2193}", KEY_CONFIG.down),
            "Move down".to_string(),
        ]),
        Row::new(vec![
            format!("{} or \u{2191}", KEY_CONFIG.up),
            "Move up".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.half_page_down),
            "Move half page down".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.half_page_up),
            "Move half page up".to_string(),
        ]),
        Row::new(vec![
            format!("{} or \u{2192}", KEY_CONFIG.page_down),
            "Move full page down".to_string(),
        ]),
        Row::new(vec![
            format!("{} or \u{2190}", KEY_CONFIG.page_up),
            "Move full page up".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.bottom),
            "Move to bottom".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.top),
            "Move to top".to_string(),
        ]),
        Row::new(vec![
            format!("/ or {}", KEY_CONFIG.search),
            "Search".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.back),
            "Go back to previous file".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.file_tree),
            "To file tree".to_string(),
        ]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.select_link),
            "Enter select mode".to_string(),
        ]),
        Row::new(vec!["\u{21b5}", "Open link/file"]),
        Row::new(vec![
            format!("{}", KEY_CONFIG.edit),
            "Edit file".to_string(),
        ]),
        Row::new(vec!["q", "Quit"]),
    ];

    let widths = [12, 25];

    let table =
        Table::new(key_actions, widths).header(header.fg(COLOR_CONFIG.table_header_fg_color));

    table.render(area, buf);
}
