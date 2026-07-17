use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::Widget,
};

/// A multi-line text widget that supports character-level wrapping and an
/// optional interactive cursor.
///
/// Unlike Ratatui's standard `Paragraph`, this widget uses strict character
/// wrapping (no word-wrap) to ensure the cursor position always stays in sync
/// with the rendered text.
pub struct TextBox<'a> {
    pub text: &'a str,
    pub cursor: Option<usize>,
    pub style: Style,
}

impl<'a> TextBox<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            cursor: None,
            style: Style::default(),
        }
    }

    pub fn cursor(mut self, cursor: usize) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Calculate the height required to render the text within a given width.
    ///
    /// This mirrors the character-wrapping rule in `render` (including the
    /// extra row a trailing cursor wraps onto) and must be kept in sync with it.
    pub fn calculate_height(text: &str, width: u16, has_cursor: bool) -> u16 {
        if width == 0 {
            return 1;
        }
        let w = width as usize;
        let mut lines: u16 = 0;
        let mut paragraphs = text.split('\n').peekable();

        while let Some(paragraph) = paragraphs.next() {
            let is_last = paragraphs.peek().is_none();
            // If it's the last paragraph and we have a cursor, we need an extra slot
            // for the insertion point at the end.
            let n = if is_last && has_cursor {
                paragraph.chars().count() + 1
            } else {
                paragraph.chars().count()
            };

            lines += (n.div_ceil(w)).max(1) as u16;
        }
        lines.max(1)
    }
}

impl<'a> Widget for TextBox<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        fill_area(buf, area, self.style);
        draw_text(buf, area, self.text, self.style);

        if let Some((cx, cy)) = self
            .cursor
            .and_then(|c| wrapped_cursor_xy(self.text, area, c))
            && let Some(cell) = buf.cell_mut((cx, cy))
        {
            cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
        }
    }
}

/// Fill `area` with spaces styled as `style` (the box background).
fn fill_area(buf: &mut Buffer, area: Rect, style: Style) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ");
                cell.set_style(style);
            }
        }
    }
}

/// Advance the wrapping pen `(line, col)` past one char. Returns the cell to
/// draw a printable char at (`None` for a newline), and `stop = true` once the
/// area is exhausted (the caller should stop). `(line, col)` end up *after* the
/// char. Shared by `draw_text` and `wrapped_cursor_xy` so they wrap identically.
fn advance(line: &mut u16, col: &mut u16, ch: char, area: Rect) -> (Option<(u16, u16)>, bool) {
    if ch == '\n' {
        *line += 1;
        *col = 0;
        return (None, *line >= area.height);
    }
    if *col >= area.width {
        *line += 1;
        *col = 0;
    }
    if *line >= area.height {
        return (None, true);
    }
    let cell = (area.x + *col, area.y + *line);
    *col += 1;
    (Some(cell), false)
}

/// Draw `text` into `area` with character-level wrapping (no word wrap).
fn draw_text(buf: &mut Buffer, area: Rect, text: &str, style: Style) {
    let (mut line, mut col) = (0u16, 0u16);
    let mut tmp = [0u8; 4];
    for ch in text.chars() {
        let (cell, stop) = advance(&mut line, &mut col, ch, area);
        if let Some((x, y)) = cell
            && let Some(c) = buf.cell_mut((x, y))
        {
            c.set_symbol(ch.encode_utf8(&mut tmp));
            c.set_style(style);
        }
        if stop {
            break;
        }
    }
}

/// Cell where the cursor at byte offset `cursor` lands under the same wrapping
/// as `draw_text`, or `None` if it falls outside `area`. A cursor at the very
/// end of the text wraps onto the next row when the last line is full (matching
/// `calculate_height`).
fn wrapped_cursor_xy(text: &str, area: Rect, cursor: usize) -> Option<(u16, u16)> {
    let (mut line, mut col) = (0u16, 0u16);
    // Checked before the char at that offset is placed, so it lands on the cell.
    for (byte_idx, ch) in text.char_indices() {
        if byte_idx == cursor {
            return Some((area.x + col, area.y + line));
        }
        if advance(&mut line, &mut col, ch, area).1 {
            break;
        }
    }
    end_cursor_xy(area, line, col, cursor == text.len())
}

/// Cursor cell for an offset at the very end of the text: wraps onto the next
/// row if the last line is full. `None` if not at the end or off-area.
fn end_cursor_xy(area: Rect, mut line: u16, mut col: u16, at_end: bool) -> Option<(u16, u16)> {
    if !at_end {
        return None;
    }
    if col >= area.width {
        line += 1;
        col = 0;
    }
    (line < area.height).then_some((area.x + col, area.y + line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_height_basics() {
        // Empty text always needs at least one row.
        assert_eq!(TextBox::calculate_height("", 10, false), 1);
        // A cursor on empty text still fits in one row.
        assert_eq!(TextBox::calculate_height("", 10, true), 1);
        // Short single line.
        assert_eq!(TextBox::calculate_height("hello", 10, false), 1);
        // Wrapping: 5 chars at width 3 -> 2 rows.
        assert_eq!(TextBox::calculate_height("hello", 3, false), 2);
        // Explicit newlines split into paragraphs.
        assert_eq!(TextBox::calculate_height("abc\ndef", 10, false), 2);
        // Width 0 degrades to a single row.
        assert_eq!(TextBox::calculate_height("hello", 0, false), 1);
    }

    #[test]
    fn calculate_height_reserves_a_row_for_a_trailing_cursor() {
        // A full-width last line plus the insertion cursor needs an extra row,
        // matching where the renderer places the wrapped cursor.
        assert_eq!(TextBox::calculate_height("0123456789", 10, false), 1);
        assert_eq!(TextBox::calculate_height("0123456789", 10, true), 2);
    }

    /// Position of the reverse-video cursor cell after rendering, if any.
    fn cursor_pos(text: &str, cursor: usize, w: u16, h: u16) -> Option<(u16, u16)> {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        TextBox::new(text).cursor(cursor).render(area, &mut buf);
        for y in 0..h {
            for x in 0..w {
                if buf[(x, y)]
                    .style()
                    .add_modifier
                    .contains(Modifier::REVERSED)
                {
                    return Some((x, y));
                }
            }
        }
        None
    }

    #[test]
    fn cursor_renders_at_end_and_mid_string() {
        // Cursor at the end sits just past the last char.
        assert_eq!(cursor_pos("ab", 2, 10, 2), Some((2, 0)));
        // Cursor in the middle lands on that char's cell.
        assert_eq!(cursor_pos("abc", 1, 10, 2), Some((1, 0)));
    }

    #[test]
    fn cursor_wraps_to_next_line_past_a_full_width_line() {
        // 10 chars exactly fill row 0; the trailing cursor wraps to (0, 1).
        assert_eq!(cursor_pos("0123456789", 10, 10, 2), Some((0, 1)));
    }
}
