use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};

#[derive(Debug, Clone)]
pub struct ErrorBox {
    message: String,
}

impl ErrorBox {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn dimensions(&self) -> (u16, u16) {
        ((self.message.len() / 28) as u16 + 3, 30)
    }

    pub fn consume(&mut self) -> String {
        let message = self.message.clone();
        self.message.clear();
        message
    }

    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }
}

impl Default for ErrorBox {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Widget for ErrorBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let paragraph = Paragraph::new(self.message).block(Block::default().borders(Borders::ALL));
        paragraph.render(area, buf);
    }
}
