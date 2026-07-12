use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Widget},
};

use crate::boxes::textbox::TextBox;

#[derive(Debug, Clone)]
pub struct SearchBox {
    pub text: String,
    pub cursor: usize,
    height: u16,
    width: u16,
    x: u16,
    y: u16,
}

impl SearchBox {
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            height: 2,
            width: 20,
            x: 0,
            y: 0,
        }
    }

    pub fn insert(&mut self, c: char) {
        self.text.push(c);
        self.cursor += 1;
    }

    pub fn delete(&mut self) {
        // `cursor` tracks characters and the box only supports appending, so the
        // cursor is always at the end of `text`; pop the last char to stay on a
        // UTF-8 boundary (`String::remove` takes a byte index and would panic on
        // multi-byte input).
        if self.text.pop().is_some() {
            self.cursor -= 1;
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    #[must_use]
    pub fn dimensions(&self) -> (u16, u16) {
        (self.height, self.width)
    }

    pub fn consume(&mut self) -> String {
        let text = self.text.clone();
        self.clear();
        text
    }

    #[must_use]
    pub fn content_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn content(&self) -> Option<&str> {
        if self.text.is_empty() {
            None
        } else {
            Some(&self.text)
        }
    }

    pub fn set_position(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    #[must_use]
    pub fn x(&self) -> u16 {
        self.x
    }

    #[must_use]
    pub fn y(&self) -> u16 {
        self.y
    }
}

impl Default for SearchBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for SearchBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::BOTTOM);
        let inner = block.inner(area);
        block.render(area, buf);
        // `self.cursor` counts characters, but `TextBox` matches the cursor
        // against a byte index — convert so the caret renders in the right
        // place after multi-byte input (e.g. "é").
        let byte_cursor = self
            .text
            .char_indices()
            .nth(self.cursor)
            .map_or_else(|| self.text.len(), |(i, _)| i);
        TextBox::new(&self.text)
            .cursor(byte_cursor)
            .render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_handles_multibyte_chars_without_panicking() {
        let mut sb = SearchBox::new();
        sb.insert('a');
        sb.insert('é'); // 2 bytes — `cursor - 1` is not a byte boundary here.
        assert_eq!(sb.cursor, 2);
        sb.delete();
        assert_eq!(sb.text, "a");
        assert_eq!(sb.cursor, 1);
        sb.delete();
        assert_eq!(sb.text, "");
        assert_eq!(sb.cursor, 0);
        // Deleting past the start is a no-op.
        sb.delete();
        assert_eq!(sb.cursor, 0);
    }
}
