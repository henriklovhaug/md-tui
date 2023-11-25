use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget, Wrap},
};

#[derive(Debug, Clone)]
pub struct MdComponentTree {
    root: MdComponent,
}

impl MdComponentTree {
    pub fn new(root: MdComponent) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &MdComponent {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut MdComponent {
        &mut self.root
    }

    pub fn set_y_offset(&mut self) {
        let mut y_offset = 0;
        for child in self.root.children_mut() {
            child.set_y_offset(y_offset);
            if child.kind() != MdEnum::CodeBlock {
                y_offset += 1;
            } else {
                y_offset += child.height();
            }
        }
    }
}

impl Widget for MdComponentTree {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for child in self.root.children_owned() {
            child.render(area, buf);
        }
    }
}

#[derive(Debug, Clone)]
pub struct MdComponent {
    kind: MdEnum,
    _parent_kind: Option<MdEnum>,
    height: u16,
    width: u16,
    y_offset: u16,
    content: String,
    children: Vec<MdComponent>,
}

fn count_newlines(s: &str) -> usize {
    s.as_bytes().iter().filter(|&&c| c == b'\n').count()
}

impl MdComponent {
    pub fn new(kind: MdEnum, width: u16, content: String, parent: Option<MdEnum>) -> Self {
        let height = count_newlines(&content) as u16 + 1;
        Self {
            kind,
            height,
            width,
            content,
            y_offset: 0,
            _parent_kind: parent,
            children: Vec::new(),
        }
    }

    pub fn set_y_offset(&mut self, y_offset: u16) {
        self.y_offset = y_offset;
        let mut height = self.height();
        if !self.is_leaf() {
            height = 0;
        }
        for child in self.children_mut() {
            child.set_y_offset(y_offset + height);
        }
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
        self.height
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn y_offset(&self) -> u16 {
        self.y_offset
    }

    pub fn add_child(&mut self, child: MdComponent) {
        self.children.push(child);
    }

    pub fn add_children(&mut self, children: Vec<MdComponent>) {
        self.children.extend(children);
    }

    pub fn children(&self) -> &Vec<MdComponent> {
        &self.children
    }

    pub fn children_owned(self) -> Vec<MdComponent> {
        self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<MdComponent> {
        &mut self.children
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

impl Widget for MdComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.height() + self.y_offset() > area.height {
            return;
        }

        if self.kind == MdEnum::VerticalSeperator || self.kind == MdEnum::EmptyLine {
            return;
        }

        if self.has_children() {
            for child in self.children {
                child.render(area, buf);
            }
            return;
        }

        let area = Rect {
            height: self.height(),
            width: area.width,
            y: self.y_offset(),
            ..area
        };

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
    Code,
    Paragraph,
    Link,
    Quote,
    Table,
    EmptyLine,
    Digit,
    VerticalSeperator,
    Sentence,
}

impl MdEnum {
    pub fn from_str(s: &str) -> Self {
        match s {
            "h1" | "h2" | "h3" | "h4" => Self::Heading,
            "task" => Self::Task,
            "u_list" => Self::UnorderedList,
            "o_list" => Self::OrderedList,
            "code_block" => Self::CodeBlock,
            "code_str" => Self::CodeBlock,
            "paragraph" => Self::Paragraph,
            "link" => Self::Link,
            "quote" => Self::Quote,
            "table" => Self::Table,
            "empty_line" => Self::EmptyLine,
            "v_seperator" => Self::VerticalSeperator,
            "sentence" => Self::Sentence,
            _e => {
                // println!("Parseerror on: {_e}");
                Self::Paragraph
            }
        }
    }
}
