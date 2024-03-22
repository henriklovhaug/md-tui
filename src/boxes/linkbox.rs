use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

#[derive(Debug, Clone)]
pub struct LinkBox {
    message: String,
}

impl LinkBox {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn dimensions(&self) -> (u16, u16) {
        ((self.message.len() / 30) as u16 + 4, 50)
    }

    pub fn consume(&mut self) -> String {
        let message = self.message.to_owned();
        self.message.clear();
        message
    }

    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }
}

impl Default for LinkBox {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Widget for LinkBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let paragraph = Paragraph::new(self.message)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        paragraph.render(area, buf);
    }
}
