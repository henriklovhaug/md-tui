use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Row, Table, Widget},
};

use crate::util::{
    Mode,
    colors::color_config,
    keys::{KEY_CONFIG, display_key},
};

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

    #[must_use]
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

fn collapsed_hint(area: Rect, buf: &mut Buffer) {
    let text = Text::styled("? - Help", Style::default().fg(Color::LightGreen).bold());
    text.render(area, buf);
}

fn render_file_tree_help(expanded: bool, area: Rect, buf: &mut Buffer) {
    if !expanded {
        collapsed_hint(area, buf);
        return;
    }

    let header = Row::new(vec!["Key", "Action", "Key", "Action"]);
    let pair = |k1: String, a1: &str, k2: String, a2: &str| {
        Row::new(vec![k1, a1.to_string(), k2, a2.to_string()])
    };

    let key_actions = [
        pair(
            format!("{} or \u{2193}", display_key(KEY_CONFIG.down)),
            "Move down",
            format!("{} or \u{2191}", display_key(KEY_CONFIG.up)),
            "Move up",
        ),
        pair(
            format!("{} or \u{2192}", display_key(KEY_CONFIG.page_down)),
            "Page down",
            format!("{} or \u{2190}", display_key(KEY_CONFIG.page_up)),
            "Page up",
        ),
        pair(
            display_key(KEY_CONFIG.top),
            "First file",
            display_key(KEY_CONFIG.bottom),
            "Last file",
        ),
        pair(
            format!("/ or {}", display_key(KEY_CONFIG.search)),
            "Search",
            "\u{21b5}".to_string(),
            "Open file",
        ),
        pair(
            display_key(KEY_CONFIG.sort),
            "Sort by name",
            display_key(KEY_CONFIG.back),
            "Back",
        ),
        pair(
            "Esc".to_string(),
            "Unselect / clear",
            display_key(KEY_CONFIG.quit),
            "Quit",
        ),
        pair(
            display_key(KEY_CONFIG.help),
            "Toggle this help",
            String::new(),
            "",
        ),
    ];

    let widths = [
        Constraint::Length(10),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(20),
    ];

    let table =
        Table::new(key_actions, widths).header(header.fg(color_config().table_header_fg_color));
    table.render(area, buf);
}

fn render_markdown_help(expanded: bool, area: Rect, buf: &mut Buffer) {
    if !expanded {
        collapsed_hint(area, buf);
        return;
    }

    let header = Row::new(vec!["Key", "Action", "Key", "Action"]);
    let pair = |k1: String, a1: &str, k2: String, a2: &str| {
        Row::new(vec![k1, a1.to_string(), k2, a2.to_string()])
    };
    // Section labels live in the wide Action column (col 1, 26 chars) so
    // longer titles like "Editing a comment" don't overflow the 10-char Key
    // column.
    let section = |label: &str| {
        Row::new(vec![
            String::new(),
            label.to_string(),
            String::new(),
            String::new(),
        ])
        .fg(color_config().table_header_fg_color)
    };

    // Sections track the actual state machine: each group lists only the
    // bindings that fire in that state (see `handle_keyboard_input` /
    // `handle_comment_mode_key`). Keep this in sync with those handlers when
    // adding new keys.
    let key_actions = [
        section("Reading"),
        pair(
            format!("{} or \u{2193}", display_key(KEY_CONFIG.down)),
            "Scroll down",
            format!("{} or \u{2191}", display_key(KEY_CONFIG.up)),
            "Scroll up",
        ),
        pair(
            format!("{} or \u{2192}", display_key(KEY_CONFIG.page_down)),
            "Page down",
            format!("{} or \u{2190}", display_key(KEY_CONFIG.page_up)),
            "Page up",
        ),
        pair(
            display_key(KEY_CONFIG.half_page_down),
            "Half page down",
            display_key(KEY_CONFIG.half_page_up),
            "Half page up",
        ),
        pair(
            display_key(KEY_CONFIG.bottom),
            "Bottom",
            display_key(KEY_CONFIG.top),
            "Top",
        ),
        pair(
            format!("/ or {}", display_key(KEY_CONFIG.search)),
            "Search",
            format!(
                "{} / {}",
                display_key(KEY_CONFIG.search_next),
                display_key(KEY_CONFIG.search_previous)
            ),
            "Next / prev match",
        ),
        pair(
            display_key(KEY_CONFIG.select_link),
            "Next link",
            display_key(KEY_CONFIG.select_link_alt),
            "Nearest link",
        ),
        pair(
            "\u{21b5}".to_string(),
            "Open link",
            display_key(KEY_CONFIG.hover),
            "Preview link / footnote",
        ),
        pair(
            display_key(KEY_CONFIG.edit),
            "Edit in $EDITOR",
            display_key(KEY_CONFIG.back),
            "Back",
        ),
        pair(
            display_key(KEY_CONFIG.file_tree),
            "File tree",
            display_key(KEY_CONFIG.toggle_caret),
            "Toggle caret mode",
        ),
        pair(
            display_key(KEY_CONFIG.comment),
            "Toggle comment mode",
            display_key(KEY_CONFIG.outline),
            "Outline jump / mark",
        ),
        pair(
            format!("{}<a-z>", display_key(KEY_CONFIG.bookmark_set)),
            "Set bookmark",
            format!("{}<a-z>", display_key(KEY_CONFIG.bookmark_jump)),
            "Jump to bookmark",
        ),
        pair(
            format!("{0}{0}", display_key(KEY_CONFIG.bookmark_jump)),
            "Jump to outline mark",
            display_key(KEY_CONFIG.help),
            "Toggle this help",
        ),
        pair(display_key(KEY_CONFIG.quit), "Quit", String::new(), ""),
        section("Caret mode"),
        pair(
            format!(
                "{} / {}",
                display_key(KEY_CONFIG.down),
                display_key(KEY_CONFIG.up)
            ),
            "Caret down / up",
            format!(
                "{} / {}",
                display_key(KEY_CONFIG.half_page_up),
                display_key(KEY_CONFIG.half_page_down)
            ),
            "Caret left / right",
        ),
        pair(
            format!(
                "{} / {}",
                display_key(KEY_CONFIG.caret_line_start),
                display_key(KEY_CONFIG.caret_line_end)
            ),
            "Line start / end",
            format!(
                "{} / {}",
                display_key(KEY_CONFIG.top),
                display_key(KEY_CONFIG.bottom)
            ),
            "To top / bottom",
        ),
        pair(
            display_key(KEY_CONFIG.comment_select),
            "Start comment selection",
            "\u{21b5}".to_string(),
            "Open comment under caret",
        ),
        pair("Esc".to_string(), "Exit caret mode", String::new(), ""),
        section("Comment mode"),
        pair(
            format!(
                "{} / {}",
                display_key(KEY_CONFIG.search_next),
                display_key(KEY_CONFIG.search_previous)
            ),
            "Cycle next / prev comment",
            display_key(KEY_CONFIG.comment_select),
            "Start selection",
        ),
        pair(
            "\u{21b5}".to_string(),
            "Edit focused / commit",
            format!("{} or Esc", display_key(KEY_CONFIG.comment)),
            "Exit comment mode",
        ),
        section("Editing a comment"),
        pair("\u{21b5}".to_string(), "Save", "Esc".to_string(), "Cancel"),
        pair(
            "Backspace".to_string(),
            "Delete char",
            "type".to_string(),
            "Insert text",
        ),
        section("Mouse"),
        pair(
            "Scroll".to_string(),
            "Scroll viewport",
            "Click TOC".to_string(),
            "Jump to outline anchor",
        ),
        pair(
            "Click".to_string(),
            "Place caret / open comment",
            "Drag".to_string(),
            "Select range for comment",
        ),
    ];

    let widths = [
        Constraint::Length(10),
        Constraint::Length(26),
        Constraint::Length(10),
        Constraint::Length(26),
    ];

    let table =
        Table::new(key_actions, widths).header(header.fg(color_config().table_header_fg_color));

    table.render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_help(mode: Mode, width: u16, height: u16) -> Vec<String> {
        let area = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let mut buf = Buffer::empty(area);
        let mut hb = HelpBox::default();
        hb.set_mode(mode);
        hb.toggle();
        hb.render(area, &mut buf);
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn markdown_help_groups_commands_by_mode() {
        let lines = render_help(Mode::View, 80, 30);
        let dump = lines.join("\n");

        // Section headers.
        for section in [
            "Reading",
            "Caret mode",
            "Comment mode",
            "Editing a comment",
            "Mouse",
        ] {
            assert!(
                dump.contains(section),
                "expected section `{section}` in help; got:\n{dump}"
            );
        }

        // Order check: each section must precede the next.
        let pos = |needle: &str| {
            dump.find(needle)
                .unwrap_or_else(|| panic!("missing `{needle}`"))
        };
        let reading = pos("Reading");
        let caret = pos("Caret mode");
        let comment = pos("Comment mode");
        let editing = pos("Editing a comment");
        let mouse = pos("Mouse");
        assert!(reading < caret, "Reading must come before Caret mode");
        assert!(caret < comment, "Caret mode must come before Comment mode");
        assert!(comment < editing, "Comment mode must come before Editing");
        assert!(editing < mouse, "Editing must come before Mouse");

        // Spot-check that representative bindings live in each section.
        for needle in [
            // Reading-only
            "Scroll down",
            "Scroll up",
            "Page down",
            "Page up",
            "Half page down",
            "Search",
            "Next / prev match",
            "Edit in $EDITOR",
            "File tree",
            "Toggle caret mode",
            "Toggle comment mode",
            "Set bookmark",
            "Jump to bookmark",
            "Outline jump / mark",
            "Jump to outline mark",
            "Toggle this help",
            "Quit",
            // Caret mode
            "Caret down / up",
            "Caret left / right",
            "Line start / end",
            "To top / bottom",
            "Start comment selection",
            "Open comment under caret",
            "Exit caret mode",
            // Comment mode
            "Cycle next / prev comment",
            "Start selection",
            "Edit focused / commit",
            "Exit comment mode",
            // Editing
            "Save",
            "Cancel",
            "Delete char",
            "Insert text",
            // Mouse
            "Scroll viewport",
            "Click TOC",
            "Jump to outline anchor",
            "Place caret / open comment",
            "Select range for comment",
        ] {
            assert!(
                dump.contains(needle),
                "expected `{needle}` in help; got:\n{dump}"
            );
        }

        // Configurable keys render via display_key — space surfaces as <Space>.
        assert!(
            dump.contains("<Space>"),
            "expected `<Space>` label for comment-select; got:\n{dump}"
        );
    }

    #[test]
    #[ignore = "visual-only inspection helper; run with --ignored to print the rendered help"]
    fn dump_help_for_visual_inspection() {
        let lines = render_help(Mode::View, 80, 30);
        eprintln!("--- markdown view help (80x30) ---");
        for line in &lines {
            eprintln!("|{line}|");
        }
        eprintln!("--- file tree help (80x12) ---");
        let lines = render_help(Mode::FileTree, 80, 12);
        for line in &lines {
            eprintln!("|{line}|");
        }
    }

    #[test]
    fn file_tree_help_lists_core_commands() {
        let lines = render_help(Mode::FileTree, 80, 12);
        let dump = lines.join("\n");
        for needle in [
            "Move down",
            "Move up",
            "Page down",
            "Page up",
            "First file",
            "Last file",
            "Search",
            "Open file",
            "Sort by name",
            "Back",
            "Quit",
            "Toggle this help",
        ] {
            assert!(
                dump.contains(needle),
                "expected `{needle}` in file-tree help; got:\n{dump}"
            );
        }
    }
}
