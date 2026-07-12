use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};

use crate::util::{App, PendingInput, general::GENERAL_CONFIG};

pub fn render_footer(f: &mut Frame, app: &App, area: Rect, sidebar_hidden: bool) {
    if area.height == 0 {
        return;
    }

    let mode = if app.caret_mode { "caret" } else { "scroll" };

    let pending = match app.pending_input {
        Some(PendingInput::BookmarkSet) => " | Set mark: _",
        Some(PendingInput::BookmarkJump) => " | Jump to mark: _",
        None => "",
    };

    let marks: String = app.bookmarks.keys().collect();

    let warn =
        if app.bookmark_origin_width != 0 && app.bookmark_origin_width != GENERAL_CONFIG.width {
            " | width changed since last save"
        } else {
            ""
        };

    let sidebar = if sidebar_hidden {
        " | comments hidden (widen terminal)"
    } else {
        ""
    };

    let line = format!(
        " {}:{}   mode: {}   marks: [{}]{}{}{}",
        app.caret.line + 1,
        app.caret.col + 1,
        mode,
        marks,
        pending,
        warn,
        sidebar,
    );

    let style = Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    f.render_widget(Paragraph::new(line).style(style), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn footer_line(sidebar_hidden: bool) -> String {
        let app = App::default();
        let mut terminal = Terminal::new(TestBackend::new(100, 1)).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                render_footer(f, &app, area, sidebar_hidden);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area.width)
            .map(|x| buf[(x, 0)].symbol().to_string())
            .collect()
    }

    #[test]
    fn sidebar_hidden_shows_widen_terminal_notice() {
        assert!(footer_line(true).contains("comments hidden (widen terminal)"));
    }

    #[test]
    fn sidebar_visible_omits_the_notice() {
        assert!(!footer_line(false).contains("comments hidden"));
    }
}
