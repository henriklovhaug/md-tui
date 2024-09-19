use ratatui::style::Color;

use crate::parser::MdParseEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaData {
    UList,
    OList,
    PLanguage,
    Other,
    ColumnsCount,
    Important,
    Note,
    Tip,
    Warning,
    Caution,
    HeadingLevel(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    MetaInfo(MetaData),
    Selected,
    LinkData,
    Normal,
    Code,
    CodeBlock(Color),
    Link,
    Italic,
    Bold,
    Strikethrough,
    BoldItalic,
    White,
    ListMarker,
}

impl From<MdParseEnum> for WordType {
    fn from(value: MdParseEnum) -> Self {
        match value {
            MdParseEnum::PLanguage
            | MdParseEnum::BlockSeperator
            | MdParseEnum::TaskOpen
            | MdParseEnum::TaskClosed
            | MdParseEnum::Indent
            | MdParseEnum::HorizontalSeperator => WordType::MetaInfo(MetaData::Other),

            MdParseEnum::Code => WordType::Code,
            MdParseEnum::Bold => WordType::Bold,
            MdParseEnum::Italic => WordType::Italic,
            MdParseEnum::Strikethrough => WordType::Strikethrough,
            MdParseEnum::Link | MdParseEnum::WikiLink | MdParseEnum::InlineLink => WordType::Link,
            MdParseEnum::BoldItalic => WordType::BoldItalic,

            MdParseEnum::Digit => WordType::ListMarker,

            MdParseEnum::Paragraph
            | MdParseEnum::AltText
            | MdParseEnum::Quote
            | MdParseEnum::Sentence
            | MdParseEnum::TableRow
            | MdParseEnum::Word => WordType::Normal,

            MdParseEnum::LinkData => WordType::LinkData,
            MdParseEnum::Imortant => WordType::MetaInfo(MetaData::Important),
            MdParseEnum::Note => WordType::MetaInfo(MetaData::Note),
            MdParseEnum::Tip => WordType::MetaInfo(MetaData::Tip),
            MdParseEnum::Warning => WordType::MetaInfo(MetaData::Warning),
            MdParseEnum::Caution => WordType::MetaInfo(MetaData::Caution),

            MdParseEnum::Heading
            | MdParseEnum::BoldItalicStr
            | MdParseEnum::BoldStr
            | MdParseEnum::CodeBlock
            | MdParseEnum::CodeStr
            | MdParseEnum::Image
            | MdParseEnum::ItalicStr
            | MdParseEnum::ListContainer
            | MdParseEnum::OrderedList
            | MdParseEnum::StrikethroughStr
            | MdParseEnum::Table
            | MdParseEnum::TableCell
            | MdParseEnum::Task
            | MdParseEnum::UnorderedList
            | MdParseEnum::TableSeperator => {
                unreachable!("Edit this or pest file to fix for value: {:?}", value)
            }
            MdParseEnum::CodeBlockStr => WordType::CodeBlock(Color::Reset),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    content: String,
    word_type: WordType,
    previous_type: Option<WordType>,
}

impl Word {
    pub fn new(content: String, word_type: WordType) -> Self {
        Self {
            word_type,
            previous_type: None,
            content,
        }
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

    pub fn set_kind(&mut self, kind: WordType) {
        self.previous_type = Some(self.word_type);
        self.word_type = kind;
    }

    pub fn clear_kind(&mut self) {
        self.word_type = self.previous_type.unwrap_or(self.word_type);
        self.previous_type = None;
    }

    pub fn is_renderable(&self) -> bool {
        !matches!(self.kind(), WordType::MetaInfo(_) | WordType::LinkData)
    }
}
