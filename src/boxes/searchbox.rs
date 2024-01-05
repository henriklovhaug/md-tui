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
}

impl SearchBox {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            height: 3,
            width: 20,
        }
    }

    pub fn insert(&mut self, c: char) {
        self.text.extend(c.to_string().chars());
        self.cursor += 1;
        self.height = self.cursor as u16 / (self.width - 1) + 3;
    }

    pub fn delete(&mut self) {
        if self.cursor > 0 {
            self.text.remove(self.cursor - 1);
            self.cursor -= 1;
        }
        self.height = self.cursor as u16 / (self.width - 1) + 3;
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
}

impl Default for SearchBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for SearchBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let paragraph = Paragraph::new(self.text)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        paragraph.render(area, buf);
    }
}
