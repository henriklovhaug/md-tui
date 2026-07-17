use ratatui::style::Color;

use crate::parser::{MdParseEnum, SourceSpan};

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
    LineLength(u16),
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
            | MdParseEnum::BlockSeparator
            | MdParseEnum::TaskOpen
            | MdParseEnum::TaskClosed
            | MdParseEnum::Indent
            | MdParseEnum::HorizontalSeparator => WordType::MetaInfo(MetaData::Other),
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
            MdParseEnum::CodeBlockStr | MdParseEnum::CodeBlockStrSpaceIndented => {
                WordType::CodeBlock(Color::Reset)
            }
            // Container variants that the higher-level parser unpacks before
            // word conversion. If one ever flows through here (grammar drift
            // or a malformed parse), fall back to Normal instead of
            // crashing the viewer.
            MdParseEnum::Heading
            | MdParseEnum::BoldItalicStr
            | MdParseEnum::BoldStr
            | MdParseEnum::CodeBlock
            | MdParseEnum::CodeStr
            | MdParseEnum::Details
            | MdParseEnum::DetailsBody
            | MdParseEnum::DetailsOpenAttr
            | MdParseEnum::DetailsSummary
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
            | MdParseEnum::TableSeparator => WordType::Normal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    content: String,
    word_type: WordType,
    previous_type: Option<WordType>,
    source_span: Option<SourceSpan>,
}

impl Word {
    #[must_use]
    pub fn new(content: String, word_type: WordType) -> Self {
        Self::new_with_source_span(content, word_type, None)
    }

    #[must_use]
    pub fn new_with_source_span(
        content: String,
        word_type: WordType,
        source_span: Option<SourceSpan>,
    ) -> Self {
        Self {
            word_type,
            previous_type: None,
            content,
            source_span,
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
    pub fn source_span(&self) -> Option<SourceSpan> {
        self.source_span
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
        let (head_content, tail_content) = self.content.split_at(at);
        let head_content = head_content.to_owned();
        let tail_content = tail_content.to_owned();

        let (head_span, tail_span) = if let Some(span) = self.source_span {
            (
                span.subspan(&self.content, 0, at),
                span.subspan(&self.content, at, self.content.len()),
            )
        } else {
            (None, None)
        };

        self.content = head_content;
        self.source_span = head_span;

        Word {
            content: tail_content,
            word_type: self.word_type,
            previous_type: self.previous_type,
            source_span: tail_span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_span() -> SourceSpan {
        SourceSpan {
            start: crate::parser::SourcePos {
                byte: 0,
                line: 1,
                column: 1,
            },
            end: crate::parser::SourcePos {
                byte: 5,
                line: 1,
                column: 6,
            },
        }
    }

    #[test]
    fn synthetic_words_default_to_no_source_span() {
        let word = Word::new("synthetic".to_string(), WordType::Normal);

        assert_eq!(word.source_span(), None);
    }

    #[test]
    fn source_backed_words_store_source_span() {
        let span = source_span();
        let word = Word::new_with_source_span("hello".to_string(), WordType::Bold, Some(span));

        assert_eq!(word.source_span(), Some(span));
    }
}
