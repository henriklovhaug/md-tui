use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

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
        self.text.push_str(&c.to_string());
        self.cursor += 1;
    }

    pub fn delete(&mut self) {
        if self.cursor > 0 {
            self.text.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn dimensions(&self) -> (u16, u16) {
        (self.height, self.width)
    }

    pub fn consume(&mut self) -> String {
        let text = self.text.clone();
        self.clear();
        text
    }

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

    pub fn x(&self) -> u16 {
        self.x
    }

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
        let paragraph = Paragraph::new(self.text)
            .block(Block::default().borders(Borders::BOTTOM))
            .wrap(Wrap { trim: true });
        paragraph.render(area, buf);
    }
}
