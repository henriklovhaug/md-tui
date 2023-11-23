use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Paragraph, Widget, Wrap},
};

#[derive(Debug, Clone)]
pub struct MdFile {
    content: Vec<MdComponent>,
}

impl MdFile {
    pub fn compact(&mut self) {
        let mut new_content = Vec::new();

        for component in self.content.iter() {
            if let Some(MdComponent {
                kind: MdEnum::CodeBlock,
                content,
            }) = new_content.last_mut()
            {
                if component.kind != MdEnum::CodeBlock {
                    content.push_str(&component.content);
                    continue;
                }
            }
            match component.kind {
                MdEnum::Table => {
                    if let Some(MdComponent {
                        kind: MdEnum::Table,
                        content,
                    }) = new_content.last_mut()
                    {
                        content.push_str(&component.content);
                    } else {
                        new_content.push(component.clone());
                    }
                }
                MdEnum::CodeBlock => {
                    if let Some(MdComponent { content, .. }) = new_content.last_mut() {
                        content.push_str(&component.content);
                    } else {
                        new_content.push(component.clone());
                    }
                }

                MdEnum::OrderedList => {
                    if let Some(MdComponent {
                        kind: MdEnum::OrderedList,
                        content,
                    }) = new_content.last_mut()
                    {
                        content.push_str(&component.content);
                    } else {
                        new_content.push(component.clone());
                    }
                }

                MdEnum::UnorderedList => {
                    if let Some(MdComponent {
                        kind: MdEnum::UnorderedList,
                        content,
                    }) = new_content.last_mut()
                    {
                        content.push_str(&component.content);
                    } else {
                        new_content.push(component.clone());
                    }
                }

                _ => new_content.push(component.clone()),
            }
        }

        self.content = new_content;
    }

    pub fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    pub fn push(&mut self, component: MdComponent) {
        self.content.push(component);
    }

    pub fn content(&self) -> &Vec<MdComponent> {
        &self.content
    }

    pub fn height(&self) -> u16 {
        self.content.iter().map(|c| c.height()).sum()
    }
}

impl Default for MdFile {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for MdFile {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.top();
        let mut height = area.height;
        self.content.get(0).unwrap().clone().render(area, buf);
        for component in self.content {
            let comp_height = component.height();
            component.render(
                Rect::new(area.left(), y, area.width, height),
                buf,
            );
            y += comp_height;
            height -= comp_height;
        }
    }
}

#[derive(Debug, Clone)]
pub struct MdComponent {
    kind: MdEnum,
    content: String,
}

impl MdComponent {
    pub fn new(kind: MdEnum, content: String) -> Self {
        Self { kind, content }
    }

    pub fn kind(&self) -> MdEnum {
        self.kind
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn content_mut(&mut self) -> &mut String {
        &mut self.content
    }

    pub fn height(&self) -> u16 {
        self.content.lines().count() as u16
    }
}

impl Widget for MdComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.height() + area.y > area.height {
            return;
        }

        let paragraph = Paragraph::new(self.content).wrap(Wrap { trim: true });

        paragraph.render(area, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdEnum {
    Heading,
    Task,
    UnorderedList,
    OrderedList,
    CodeBlock,
    Paragraph,
    Link,
    Quote,
    Table,
    EmptyLine,
}
