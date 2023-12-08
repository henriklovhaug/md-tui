use std::str::FromStr;

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

#[derive(Debug, Clone)]
pub struct ParseRoot {
    children: Vec<ParseNode>,
}

impl ParseRoot {
    pub fn new(children: Vec<ParseNode>) -> Self {
        Self { children }
    }

    pub fn children(&self) -> &Vec<ParseNode> {
        &self.children
    }

    pub fn children_owned(self) -> Vec<ParseNode> {
        self.children
    }
}

#[derive(Debug, Clone)]
pub struct ParseNode {
    kind: MdEnum,
    content: String,
    children: Vec<ParseNode>,
}

impl ParseNode {
    pub fn new(kind: MdEnum, content: String) -> Self {
        Self {
            kind,
            content,
            children: Vec::new(),
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
    Bold,
    Italic,
    Strikethrough,
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
            "normal" | "digit" => Ok(Self::Sentence),
            "table_word" | "o_list_counter" => Ok(Self::Word),
            "list_container" => Ok(Self::ListContainer),
            "word" => Ok(Self::Word),
            "bold" | "bold_word" => Ok(Self::Bold),
            "italic" | "italic_word" => Ok(Self::Italic),
            "strikethrough" | "strikethrough_word" => Ok(Self::Strikethrough),
            _e => {
                // println!("Parseerror on: {_e}");
                Ok(Self::Paragraph)
            }
        }
    }
}

#[derive(Debug, Clone)]
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

    pub fn set_scroll(&mut self, scroll: u16) {
        let mut y_offset = 0;
        for component in self.components.iter_mut() {
            component.set_y_offset(y_offset);
            component.set_scroll_offset(scroll);
            y_offset += component.height();
        }
    }

    pub fn transform(&mut self, width: u16) {
        for component in self.components_mut() {
            component.transform(width);
        }
    }
}

impl Widget for RenderRoot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for component in self.components_owned() {
            component.render(area, buf);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    MetaInfo,
    Normal,
    Code,
    Link,
    Italic,
    Bold,
    Strikethrough,
}

impl From<MdEnum> for WordType {
    fn from(value: MdEnum) -> Self {
        match value {
            MdEnum::Heading => todo!(),
            MdEnum::Word => WordType::Normal,
            MdEnum::Task => todo!(),
            MdEnum::UnorderedList => todo!(),
            MdEnum::ListContainer => todo!(),
            MdEnum::OrderedList => todo!(),
            MdEnum::CodeBlock => todo!(),
            MdEnum::PLanguage => WordType::MetaInfo,
            MdEnum::CodeStr => todo!(),
            MdEnum::Code => WordType::Code,
            MdEnum::Paragraph => WordType::Normal,
            MdEnum::Link => todo!(),
            MdEnum::Quote => todo!(),
            MdEnum::Table => todo!(),
            MdEnum::TableSeperator => todo!(),
            MdEnum::TableRow => WordType::Normal,
            MdEnum::Digit => WordType::Normal,
            MdEnum::VerticalSeperator => todo!(),
            MdEnum::BlockSeperator => WordType::MetaInfo,
            MdEnum::Sentence => WordType::Normal,
            MdEnum::Bold => WordType::Bold,
            MdEnum::Italic => WordType::Italic,
            MdEnum::Strikethrough => WordType::Strikethrough,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    content: String,
    word_type: WordType,
}

impl Word {
    pub fn new(content: String, word_type: WordType) -> Self {
        Self { content, word_type }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn content_mut(&mut self) -> &mut String {
        &mut self.content
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    pub fn kind(&self) -> WordType {
        self.word_type
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RenderNode {
    Paragraph,
    LineBreak,
    Heading,
    Task,
    List,
    Table,
    CodeBlock,
}

#[derive(Debug, Clone)]
pub struct RenderComponent {
    kind: RenderNode,
    content: Vec<Vec<Word>>,
    height: u16,
    offset: u16,
    scroll_offset: u16,
}

impl RenderComponent {
    pub fn new(kind: RenderNode, content: Vec<Word>) -> Self {
        Self {
            kind,
            content: vec![content],
            height: 0,
            offset: 0,
            scroll_offset: 0,
        }
    }

    pub fn new_formatted(kind: RenderNode, content: Vec<Vec<Word>>) -> Self {
        Self {
            kind,
            height: content.len() as u16,
            content,
            offset: 0,
            scroll_offset: 0,
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

    pub fn content_owned(self) -> Vec<Vec<Word>> {
        self.content
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn y_offset(&self) -> u16 {
        self.offset
    }

    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub fn set_y_offset(&mut self, y_offset: u16) {
        self.offset = y_offset;
    }

    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.scroll_offset = offset;
    }

    pub fn transform(&mut self, width: u16) {
        match self.kind {
            RenderNode::Heading => self.height = 1,
            RenderNode::Task => self.height = 1,
            RenderNode::List => {
                self.content.iter().len();
            }
            RenderNode::CodeBlock => {
                let height = self
                    .content
                    .iter()
                    .filter(|c| c.iter().any(|x| x.kind() != WordType::MetaInfo))
                    .count() as u16;
                self.height = height;
            }
            RenderNode::Paragraph => {
                let mut len = 0;
                let mut lines = Vec::new();
                let mut line = Vec::new();
                for word in self.content.iter().flatten() {
                    if word.content.len() + len < width as usize {
                        if line.is_empty() {
                            let mut word = word.clone();
                            let content = word.content.trim_start().to_owned();
                            word.set_content(content);
                            line.push(word);
                        } else {
                            line.push(word.clone());
                        }
                        len += word.content.len() + 1;
                    } else {
                        lines.push(line);
                        len = word.content.len() + 1;
                        let mut word = word.clone();
                        let content = word.content.trim_start().to_owned();
                        word.set_content(content);
                        line = vec![word];
                    }
                }
                if !line.is_empty() {
                    lines.push(line);
                }
                self.height = lines.len() as u16;
                self.content = lines;
            }
            RenderNode::LineBreak => {
                self.height = 1;
            }
            RenderNode::Table => {
                let height = self.content.len() as u16;
                self.height = height;
            }
        }
    }
}
