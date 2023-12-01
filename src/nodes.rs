use std::str::FromStr;

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

#[derive(Debug, Clone)]
pub struct ParseRoot {
    children: Vec<ParseNode>,
}

impl ParseRoot {
    pub fn new(children: ParseNode) -> Self {
        let children = vec![children];
        Self { children }
    }

    pub fn children(&self) -> &Vec<ParseNode> {
        &self.children
    }

    pub fn children_owned(self) -> Vec<ParseNode> {
        self.children
    }

    pub fn set_y_offset(&mut self, scroll: u16) {
        let mut y_offset = 0;
        for child in self.children.iter_mut() {
            child.set_y_offset(y_offset);
            child.set_scroll_offset(scroll);
            y_offset += child.height();
        }
    }
}

// impl Widget for ParseRoot {
//     fn render(self, area: Rect, buf: &mut Buffer) {
//         for child in self..children_owned() {
//             child.render(area, buf);
//         }
//     }
// }

#[derive(Debug, Clone)]
pub struct ParseNode {
    kind: MdEnum,
    _parent_kind: Option<MdEnum>,
    height: u16,
    width: u16,
    y_offset: u16,
    scroll_offset: u16,
    content: String,
    children: Vec<ParseNode>,
}

impl ParseNode {
    pub fn new(kind: MdEnum, width: u16, content: String, parent: Option<MdEnum>) -> Self {
        Self {
            kind,
            height: 0,
            width,
            content,
            y_offset: 0,
            scroll_offset: 0,
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
            height += child.height();
        }
    }

    pub fn transform(&mut self, width: u16) {
        match self.kind {
            MdEnum::Heading => self.height = 1,
            // MdEnum::Task => todo!(),
            // MdEnum::UnorderedList => todo!(),
            MdEnum::ListContainer => self.height = self.children().len() as u16,
            // MdEnum::OrderedList => todo!(),
            MdEnum::CodeBlock => {
                self.height = self
                    .children()
                    .iter()
                    .filter(|c| c.kind() == MdEnum::Sentence)
                    .count() as u16;
            }
            // MdEnum::Code => todo!(),
            MdEnum::Paragraph => {
                let mut height = 1;
                let mut prev_offset = 0;
                let mut offset_index = 0;
                let mut offsets = Vec::new();
                self.content()
                    .replace('\n', " ")
                    .chars()
                    .enumerate()
                    .for_each(|(i, c)| {
                        if c == ' ' {
                            offset_index = i - offsets.iter().sum::<usize>();
                            if offset_index > width as usize {
                                height += 1;
                                offsets.push(prev_offset);
                            }
                            prev_offset = offset_index;
                        }
                    });
                self.height = height as u16;
                // let mut children = Vec::new();
                // let mut line_components = Vec::new();
                // for child in self.children_owned() {
                //     let current_line_length = line_components
                //         .iter()
                //         .map(|c: &MdComponent| c.content().len() as u16)
                //         .sum::<u16>();
                //     if (current_line_length + child.content().len() as u16) < width
                //     {
                //         line_components.push(child);
                //     } else {
                //         let split_index = width as usize - current_line_length as usize;
                //         let (left, right) = self.split(split_index);
                //         line_components.push(left);
                //         // children.pus
                //     }
                // }
            }
            MdEnum::Table => self.height = self.children().len() as u16 - 1,
            _ => self.height = 1,
            // MdEnum::Link => todo!(),
            // MdEnum::Quote => todo!(),
            // MdEnum::TableRow => todo!(),
            // MdEnum::Digit => todo!(),
            // MdEnum::VerticalSeperator => todo!(),
            // MdEnum::BlockSeperator => todo!(),
            // MdEnum::Sentence => todo!(),
        }
    }

    pub fn count_seperators(&self) -> u16 {
        self.children
            .iter()
            .filter(|c| c.kind() == MdEnum::VerticalSeperator)
            .count() as u16
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

    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.scroll_offset = offset;
    }

    pub fn add_children(&mut self, children: Vec<ParseNode>) {
        self.children.extend(children);
    }

    pub fn children(&self) -> &Vec<ParseNode> {
        &self.children
    }

    pub fn children_owned(self) -> Vec<ParseNode> {
        self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<ParseNode> {
        &mut self.children
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdEnum {
    Heading,
    Word,
    Task,
    UnorderedList,
    ListContainer,
    OrderedList,
    CodeBlock,
    PLanguage,
    CodeStr,
    Code,
    Paragraph,
    Link,
    Quote,
    Table,
    TableSeperator,
    TableRow,
    Digit,
    VerticalSeperator,
    BlockSeperator,
    Sentence,
}

impl FromStr for MdEnum {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "h1" | "h2" | "h3" | "h4" => Ok(Self::Heading),
            "heading" => Ok(Self::Heading),
            "task" => Ok(Self::Task),
            "u_list" => Ok(Self::UnorderedList),
            "o_list" => Ok(Self::OrderedList),
            "code_block" => Ok(Self::CodeBlock),
            "programming_language" => Ok(Self::PLanguage),
            "inline_code_word" => Ok(Self::Code),
            "code_str" => Ok(Self::CodeStr),
            "paragraph" => Ok(Self::Paragraph),
            "link" => Ok(Self::Link),
            "quote" => Ok(Self::Quote),
            "table" => Ok(Self::Table),
            "table_seperator" => Ok(Self::TableSeperator),
            "table_row" => Ok(Self::TableRow),
            "v_seperator" => Ok(Self::VerticalSeperator),
            "block_sep" => Ok(Self::BlockSeperator),
            "sentence" | "code_line" => Ok(Self::Sentence),
            "normal" => Ok(Self::Sentence),
            "table_sentence" => Ok(Self::Sentence),
            "list_container" => Ok(Self::ListContainer),
            "word" => Ok(Self::Word),
            _e => {
                // println!("Parseerror on: {_e}");
                Ok(Self::Paragraph)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RenderNode {
    Paragraph,
    Heading,
    Task,
    List,
    CodeBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    MetaInfo,
    Normal,
    Code,
    Link,
    Italic,
    Bold,
}

#[derive(Debug, Clone)]
pub struct Word {
    content: String,
    word_type: WordType,
}

pub struct RenderComponent {
    kind: RenderNode,
    content: Vec<Vec<Word>>,
    height: u16,
}

pub struct RenderRoot {
    components: Vec<RenderComponent>,
}

impl RenderRoot {
    pub fn new(components: Vec<RenderComponent>) -> Self {
        Self { components }
    }

    pub fn components(&self) -> &Vec<RenderComponent> {
        &self.components
    }

    pub fn components_owned(self) -> Vec<RenderComponent> {
        self.components
    }

    pub fn components_mut(&mut self) -> &mut Vec<RenderComponent> {
        &mut self.components
    }
}

impl RenderComponent {
    pub fn new(kind: RenderNode, content: Vec<Word>) -> Self {
        Self {
            kind,
            content: vec![content],
            height: 0,
        }
    }

    pub fn kind(&self) -> RenderNode {
        self.kind
    }

    pub fn content(&self) -> &Vec<Vec<Word>> {
        &self.content
    }

    pub fn content_mut(&mut self) -> &mut Vec<Vec<Word>> {
        &mut self.content
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn transform(&mut self, width: u16) {
        match self.kind {
            RenderNode::Heading => self.height = 1,
            RenderNode::Task => self.height = 1,
            RenderNode::List => {
                self.content.iter().len() as u16;
            }
            RenderNode::CodeBlock => {
                let height = self
                    .content
                    .iter()
                    .filter(|c| c.iter().all(|w| w.word_type == WordType::Normal))
                    .count() as u16;
                self.height = height;
            }
            RenderNode::Paragraph => {
                let mut len = 0;
                let mut lines = Vec::new();
                let mut line = Vec::new();
                for word in self.content.iter().flatten() {
                    if word.content.len() + len < width as usize {
                        len += word.content.len();
                        line.push(word.clone());
                    } else {
                        lines.push(line);
                        line = Vec::new();
                        len = 0;
                    }
                }
                if !line.is_empty() {
                    lines.push(line);
                }
                self.height = lines.len() as u16;
                self.content = lines;
            }
        }
    }
}
