use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Row, Table, Widget},
};

use crate::util::{Mode, CONFIG};

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
        Row::new(vec!["j", "Move down"]),
        Row::new(vec!["k", "Move up"]),
        Row::new(vec!["h", "Go to previous page"]),
        Row::new(vec!["l", "Go to next page"]),
        Row::new(vec!["g", "Move to top"]),
        Row::new(vec!["G", "Move to bottom"]),
        Row::new(vec!["/ or f", "Search"]),
        Row::new(vec!["Enter", "Open file"]),
        Row::new(vec!["q", "Quit"]),
    ];

    let widths = [10, 20];

    let table = Table::new(key_actions, widths).header(header.fg(CONFIG.table_header_fg_color));
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
        Row::new(vec!["j", "Move down"]),
        Row::new(vec!["k", "Move up"]),
        Row::new(vec!["h", "Move full page up"]),
        Row::new(vec!["l", "Move full page down"]),
        Row::new(vec!["g", "Move to top"]),
        Row::new(vec!["G", "Move to bottom"]),
        Row::new(vec!["d", "Move half page down"]),
        Row::new(vec!["u", "Move half page up"]),
        Row::new(vec!["r", "Reload file"]),
        Row::new(vec!["/ or f", "Search"]),
        Row::new(vec!["t", "Toggle file tree"]),
        Row::new(vec!["s", "Enter select mode"]),
        Row::new(vec!["Enter", "Open link/file"]),
        Row::new(vec!["q", "Quit"]),
    ];

    let widths = [10, 20];

    let table = Table::new(key_actions, widths).header(header.fg(CONFIG.table_header_fg_color));

    table.render(area, buf);
}
