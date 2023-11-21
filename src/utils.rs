use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

#[derive(Debug)]
pub struct Components {
    content: Vec<MdComponent>,
}

impl Components {
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    pub fn push(&mut self, component: MdComponent) {
        self.content.push(component);
    }
}

#[derive(Debug)]
pub enum MdComponent {
    Heading(String),
    Task(String),
    UnorderedList(String),
    OrderedList(String),
    CodeBlock(String),
    Paragraph(String),
    Link(String),
    EmptyLine,
}

impl Widget for MdComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        todo!()
    }
}
