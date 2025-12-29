use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

#[derive(Debug, Clone)]
pub struct ErrorBox {
    message: String,
}

impl ErrorBox {
    #[must_use]
    pub fn new(message: String) -> Self {
        Self { message }
    }

    #[must_use]
    pub fn dimensions(&self) -> (u16, u16) {
        ((self.message.len() / 30) as u16 + 4, 30)
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
        let paragraph = Paragraph::new(self.message)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        paragraph.render(area, buf);
    }
}
