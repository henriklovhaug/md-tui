use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::boxes::textbox::TextBox;
use crate::comments::Comment;
use crate::parser::SourceSpan;
use crate::util::Caret;
use crate::util::colors::color_config;
use crate::util::keys::{KEY_CONFIG, display_key};

pub const SIDEBAR_WIDTH: u16 = 32;
const CARD_GAP: u16 = 1;

/// The fundamental states a comment card can be in.
#[derive(Debug, PartialEq, Eq)]
pub enum CommentBoxState<'a> {
    Inactive(&'a Comment),
    Active(&'a Comment),
    Editing(&'a EditingDraft<'a>),
}

/// A unified wrapper for anything that can appear as a card in the sidebar.
pub struct CommentBox<'a> {
    pub state: CommentBoxState<'a>,
    pub anchor: Caret,
    pub header: Option<&'a str>,
}

impl<'a> CommentBox<'a> {
    pub fn height(&self, width: u16) -> u16 {
        let header = if self.header.is_some() { 1 } else { 0 };
        let (text, has_cursor) = match self.state {
            CommentBoxState::Inactive(c) | CommentBoxState::Active(c) => (c.text.as_str(), false),
            CommentBoxState::Editing(d) => (d.draft, true),
        };
        header + TextBox::calculate_height(text, width, has_cursor) + 1
    }
}

impl<'a> Widget for &CommentBox<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let style = match self.state {
            CommentBoxState::Inactive(_) => Style::default().add_modifier(Modifier::DIM),
            CommentBoxState::Active(_) | CommentBoxState::Editing(_) => Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(color_config().link_selected_bg_color),
        };

        let mut constraints = vec![
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Bottom border
        ];
        if self.header.is_some() {
            constraints.insert(0, Constraint::Length(1));
        }
        let layout = Layout::vertical(constraints).split(area);

        let (body_area, border_area) = if let Some(text) = self.header {
            Paragraph::new(text)
                .style(style.add_modifier(Modifier::BOLD))
                .render(layout[0], buf);
            (layout[1], layout[2])
        } else {
            (layout[0], layout[1])
        };

        // Body
        let textbox = match self.state {
            CommentBoxState::Inactive(c) | CommentBoxState::Active(c) => {
                TextBox::new(&c.text).style(style)
            }
            CommentBoxState::Editing(d) => TextBox::new(d.draft).cursor(d.cursor).style(style),
        };
        textbox.render(body_area, buf);

        // Border
        for x in border_area.x..border_area.x + border_area.width {
            if let Some(cell) = buf.cell_mut((x, border_area.y)) {
                cell.set_char('─');
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EditingDraft<'a> {
    pub range: (Caret, Caret),
    pub source: Option<SourceSpan>,
    pub draft: &'a str,
    pub cursor: usize,
    pub replaces_saved_idx: Option<usize>,
}

pub struct CommentSideBar<'a> {
    pub boxes: Vec<CommentBox<'a>>,
    pub markdown_scroll: u16,
}

impl<'a> Widget for CommentSideBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.boxes.is_empty() {
            let hint = Paragraph::new(format!(
                "(no comments yet — press {} to add)",
                display_key(KEY_CONFIG.comment_select)
            ))
            .style(Style::default().add_modifier(Modifier::DIM))
            .wrap(Wrap { trim: true });
            hint.render(inner, buf);
            return;
        }

        let mut sorted_boxes = self.boxes;
        sorted_boxes.sort_by_key(|b| (b.anchor.line, b.anchor.col));

        let inner_top = inner.y as i32;
        let inner_bottom = (inner.y + inner.height) as i32;
        let mut next_y_min = inner_top;

        for cb in &sorted_boxes {
            let height = cb.height(inner.width);
            let preferred_y = inner_top + cb.anchor.line as i32 - self.markdown_scroll as i32;
            let y = preferred_y.max(next_y_min);

            if y >= inner_bottom {
                break;
            }

            if let Some(card_area) = clip_card_area(inner, y, height) {
                cb.render(card_area, buf);
            }
            next_y_min = y + height as i32 + CARD_GAP as i32;
        }
    }
}

fn clip_card_area(inner: Rect, top: i32, height: u16) -> Option<Rect> {
    let bottom = top + height as i32;
    let inner_top = inner.y as i32;
    let inner_bottom = (inner.y + inner.height) as i32;
    if bottom <= inner_top || top >= inner_bottom {
        return None;
    }
    let visible_top = top.max(inner_top);
    let visible_bottom = bottom.min(inner_bottom);
    let h = (visible_bottom - visible_top) as u16;
    Some(Rect {
        x: inner.x,
        y: visible_top as u16,
        width: inner.width,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{SourcePos, SourceSpan};

    fn comment(line: u16, text: &str) -> Comment {
        Comment {
            source: SourceSpan {
                start: SourcePos {
                    byte: 0,
                    line: line as u32,
                    column: 1,
                },
                end: SourcePos {
                    byte: 1,
                    line: line as u32,
                    column: 2,
                },
            },
            selected_text: None,
            text: text.to_string(),
        }
    }

    #[test]
    fn boxes_handle_scrolling_and_anchoring() {
        let c = comment(10, "hello");
        let cb = CommentBox {
            state: CommentBoxState::Inactive(&c),
            anchor: Caret { line: 10, col: 0 },
            header: None,
        };
        let area = Rect::new(0, 0, 32, 20);
        let sidebar = CommentSideBar {
            boxes: vec![cb],
            markdown_scroll: 0,
        };

        // No scroll -> y = 10
        let mut buf = Buffer::empty(area);
        sidebar.render(area, &mut buf);
        assert!(buf[(1, 10)].symbol() != " ");
        assert!(buf[(1, 9)].symbol() == " ");

        // Scroll 5 -> y = 5
        let sidebar_scrolled = CommentSideBar {
            markdown_scroll: 5,
            boxes: vec![CommentBox {
                state: CommentBoxState::Inactive(&c),
                anchor: Caret { line: 10, col: 0 },
                header: None,
            }],
        };
        let mut buf2 = Buffer::empty(area);
        sidebar_scrolled.render(area, &mut buf2);
        assert!(buf2[(1, 5)].symbol() != " ");
    }

    #[test]
    fn sidebar_sorts_and_stacks_boxes() {
        let c1 = comment(1, "one");
        let c2 = comment(1, "two");
        let boxes = vec![
            CommentBox {
                state: CommentBoxState::Inactive(&c1),
                anchor: Caret { line: 1, col: 0 },
                header: None,
            },
            CommentBox {
                state: CommentBoxState::Inactive(&c2),
                anchor: Caret { line: 1, col: 0 },
                header: None,
            },
        ];
        let area = Rect::new(0, 0, 32, 20);
        let sidebar = CommentSideBar {
            boxes,
            markdown_scroll: 0,
        };
        let mut buf = Buffer::empty(area);
        sidebar.render(area, &mut buf);

        // Box 1 (y=1): Height=2 (Body + Border)
        assert_eq!(buf[(1, 1)].symbol(), "o");
        assert_eq!(buf[(1, 2)].symbol(), "─");

        // Box 2 (y=1+2+1=4): Gap of 1
        assert_eq!(buf[(1, 4)].symbol(), "t");
        assert_eq!(buf[(1, 5)].symbol(), "─");
    }

    /// Collect the whole buffer into a single string for substring assertions.
    fn buf_text(buf: &Buffer) -> String {
        let area = buf.area;
        let mut s = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    fn editing_draft(draft: &str, cursor: usize) -> EditingDraft<'_> {
        EditingDraft {
            range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 1 }),
            source: None,
            draft,
            cursor,
            replaces_saved_idx: None,
        }
    }

    #[test]
    fn empty_sidebar_shows_hint() {
        let sidebar = CommentSideBar {
            boxes: vec![],
            markdown_scroll: 0,
        };
        let area = Rect::new(0, 0, 32, 10);
        let mut buf = Buffer::empty(area);
        sidebar.render(area, &mut buf);
        assert!(
            buf_text(&buf).contains("no comments yet"),
            "empty sidebar must show the add-comment hint"
        );
    }

    #[test]
    fn header_renders_above_body() {
        let c = comment(1, "body");
        let cb = CommentBox {
            state: CommentBoxState::Inactive(&c),
            anchor: Caret { line: 0, col: 0 },
            header: Some("alice"),
        };
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        (&cb).render(area, &mut buf);
        let text = buf_text(&buf);
        assert!(text.contains("alice"), "header label must render");
        assert!(text.contains("body"), "comment body must render");
        // Header occupies row 0; body starts on row 1.
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(0, 1)].symbol(), "b");
    }

    #[test]
    fn editing_card_renders_draft_and_reversed_cursor() {
        let draft = editing_draft("hi", 2);
        let cb = CommentBox {
            state: CommentBoxState::Editing(&draft),
            anchor: Caret { line: 0, col: 0 },
            header: None,
        };
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        (&cb).render(area, &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(1, 0)].symbol(), "i");
        // Cursor sits one past "hi" (byte 2) and renders as a reversed cell.
        assert!(
            buf[(2, 0)]
                .style()
                .add_modifier
                .contains(Modifier::REVERSED),
            "draft cursor must be a reversed cell"
        );
    }

    #[test]
    fn active_card_uses_selected_bg_inactive_does_not() {
        let c = comment(1, "x");
        let area = Rect::new(0, 0, 20, 5);

        let active = CommentBox {
            state: CommentBoxState::Active(&c),
            anchor: Caret { line: 0, col: 0 },
            header: None,
        };
        let mut active_buf = Buffer::empty(area);
        (&active).render(area, &mut active_buf);
        assert_eq!(
            active_buf[(0, 0)].style().bg,
            Some(color_config().link_selected_bg_color),
            "active card body must use the selected background"
        );

        let inactive = CommentBox {
            state: CommentBoxState::Inactive(&c),
            anchor: Caret { line: 0, col: 0 },
            header: None,
        };
        let mut inactive_buf = Buffer::empty(area);
        (&inactive).render(area, &mut inactive_buf);
        assert_ne!(
            inactive_buf[(0, 0)].style().bg,
            Some(color_config().link_selected_bg_color),
            "inactive card must not use the selected background"
        );
        assert!(
            inactive_buf[(0, 0)]
                .style()
                .add_modifier
                .contains(Modifier::DIM),
            "inactive card body is dimmed"
        );
    }

    #[test]
    fn clip_card_area_handles_top_bottom_and_outside() {
        let inner = Rect::new(0, 2, 32, 10); // rows 2..12

        // Fully above and fully below the viewport -> None.
        assert!(clip_card_area(inner, -5, 3).is_none());
        assert!(clip_card_area(inner, 100, 3).is_none());

        // Straddling the top edge: top clamped to inner.y, height reduced.
        let top = clip_card_area(inner, 0, 5).expect("partially visible at top");
        assert_eq!(top.y, 2);
        assert_eq!(top.height, 3); // bottom at 5, top clamped to 2 -> 3 rows

        // Straddling the bottom edge: height clipped to the inner bottom.
        let bottom = clip_card_area(inner, 10, 5).expect("partially visible at bottom");
        assert_eq!(bottom.y, 10);
        assert_eq!(bottom.height, 2); // inner bottom is 12 -> rows 10,11
    }
}
