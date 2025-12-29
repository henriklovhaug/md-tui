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
    Bold,
    BoldItalic,
    Code,
    CodeBlock(Color),
    Footnote,
    FootnoteData,
    FootnoteInline,
    Italic,
    Link,
    LinkData,
    ListMarker,
    MetaInfo(MetaData),
    Normal,
    Selected,
    Strikethrough,
    White,
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
            MdParseEnum::FootnoteRef => WordType::FootnoteInline,
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
            | MdParseEnum::Footnote
            | MdParseEnum::Table
            | MdParseEnum::TableCell
            | MdParseEnum::Task
            | MdParseEnum::UnorderedList
            | MdParseEnum::TableSeperator => {
                unreachable!("Edit this or pest file to fix for value: {:?}", value)
            }
            MdParseEnum::CodeBlockStr | MdParseEnum::CodeBlockStrSpaceIndented => {
                WordType::CodeBlock(Color::Reset)
            } // MdParseEnum::FootnoteRef => todo!(),
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
    #[must_use]
    pub fn new(content: String, word_type: WordType) -> Self {
        Self {
            word_type,
            previous_type: None,
            content,
        }
    }

    #[must_use]
    pub fn previous_type(&self) -> WordType {
        self.previous_type.unwrap_or(self.word_type)
    }

    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn content_mut(&mut self) -> &mut String {
        &mut self.content
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    #[must_use]
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

    #[must_use]
    pub fn is_renderable(&self) -> bool {
        !matches!(
            self.kind(),
            WordType::MetaInfo(_) | WordType::LinkData | WordType::FootnoteData
        )
    }

    pub fn split_off(&mut self, at: usize) -> Word {
        Word {
            content: self.content.split_off(at),
            word_type: self.word_type,
            previous_type: self.previous_type,
        }
    }
}
