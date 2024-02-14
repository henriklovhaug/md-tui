use std::usize;

use crate::parser::MdParseEnum;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

#[derive(Debug, Clone)]
pub struct RenderRoot {
    file_name: String,
    components: Vec<RenderComponent>,
    is_focused: bool,
}

impl RenderRoot {
    pub fn new(file_name: String, components: Vec<RenderComponent>) -> Self {
        Self {
            file_name,
            components,
            is_focused: false,
        }
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

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn select(&mut self, index: usize) -> Result<u16, String> {
        self.deselect();
        self.is_focused = true;
        let mut count = 0;
        for comp in self.components.iter_mut() {
            if index - count < comp.num_links() {
                if let Err(err) = comp.visually_select(index - count) {
                    return Err(err);
                }
                return Ok(comp.y_offset());
            }
            count += comp.num_links();
        }
        Err(format!("Index out of bounds: {} >= {}", index, count))
    }

    pub fn deselect(&mut self) {
        self.is_focused = false;
        for comp in self.components.iter_mut() {
            comp.deselect();
        }
    }

    /// Sets the y offset of the components
    pub fn set_scroll(&mut self, scroll: u16) {
        let mut y_offset = 0;
        for component in self.components.iter_mut() {
            component.set_y_offset(y_offset);
            component.set_scroll_offset(scroll);
            y_offset += component.height();
        }
    }

    pub fn heading_offset(&self, heading: &str) -> Result<u16, String> {
        let mut y_offset = 0;
        let heading = heading.split('-');
        for component in self.components.iter() {
            if component.kind() == RenderNode::Heading
                && component
                    .content()
                    .iter()
                    .flatten()
                    .filter(|c| c.content() != " ")
                    .map(|c| c.content().trim().to_lowercase().replace(['(', ')'], ""))
                    .eq(heading.clone())
            {
                return Ok(y_offset);
            }
            y_offset += component.height();
        }
        Err(format!(
            "Heading not found: {}",
            heading.collect::<Vec<_>>().join("-")
        ))
    }

    /// Return the content of the components, where each element a line
    pub fn content(&self) -> Vec<String> {
        self.components()
            .iter()
            .flat_map(|c| c.content_as_lines())
            .collect()
    }

    pub fn selected(&self) -> &str {
        let block = self.components.iter().find(|c| c.is_focused()).unwrap();
        block.highlight_link().unwrap()
    }

    fn get_component_from_height_mut(&mut self, height: u16) -> Option<&mut RenderComponent> {
        let mut y_offset = 0;
        for component in self.components.iter_mut() {
            if y_offset <= height && height < y_offset + component.height() {
                return Some(component);
            }
            y_offset += component.height();
        }
        None
    }

    pub fn mark_word(&mut self, height: usize, index: usize, length: usize) -> Result<(), String> {
        let component = self
            .get_component_from_height_mut(height as u16)
            .ok_or("index out of bounds")?;
        let height = height - component.y_offset() as usize;
        component.mark_word(height, index, length)
    }

    /// Transforms the content of the components to fit the given width
    pub fn transform(&mut self, width: u16) {
        for component in self.components_mut() {
            component.transform(width);
        }
    }

    pub fn height(&self) -> u16 {
        self.components.iter().map(|c| c.height()).sum()
    }

    pub fn num_links(&self) -> usize {
        self.components.iter().map(|c| c.num_links()).sum()
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
pub enum MetaData {
    UList,
    OList,
    PLanguage,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    MetaInfo(MetaData),
    Selected,
    LinkData,
    Normal,
    Code,
    Link,
    Italic,
    Bold,
    Strikethrough,
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
            MdParseEnum::Link => WordType::Link,

            MdParseEnum::Digit => WordType::ListMarker,

            MdParseEnum::Paragraph
            | MdParseEnum::TableRow
            | MdParseEnum::Sentence
            | MdParseEnum::Word => WordType::Normal,

            MdParseEnum::LinkData => WordType::LinkData,

            MdParseEnum::Heading
            | MdParseEnum::Task
            | MdParseEnum::UnorderedList
            | MdParseEnum::ListContainer
            | MdParseEnum::OrderedList
            | MdParseEnum::CodeBlock
            | MdParseEnum::CodeStr
            | MdParseEnum::Quote
            | MdParseEnum::Table
            | MdParseEnum::TableSeperator => {
                unreachable!("Edit this or pest file to fix for value: {:?}", value)
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderNode {
    Paragraph,
    LineBreak,
    Heading,
    Task,
    List,
    Table,
    CodeBlock,
    Quote,
}

#[derive(Debug, Clone)]
pub struct RenderComponent {
    kind: RenderNode,
    content: Vec<Vec<Word>>,
    meta_info: Vec<Word>,
    height: u16,
    offset: u16,
    scroll_offset: u16,
    focused: bool,
    focused_index: usize,
}

impl RenderComponent {
    pub fn new(kind: RenderNode, content: Vec<Word>) -> Self {
        let meta_info: Vec<Word> = content
            .iter()
            .filter(|c| !c.is_renderable())
            .cloned()
            .collect();

        let content = content.into_iter().filter(|c| c.is_renderable()).collect();

        Self {
            kind,
            content: vec![content],
            meta_info,
            height: 0,
            offset: 0,
            scroll_offset: 0,
            focused: false,
            focused_index: 0,
        }
    }

    pub fn new_formatted(kind: RenderNode, content: Vec<Vec<Word>>) -> Self {
        let meta_info: Vec<Word> = content
            .iter()
            .flatten()
            .filter(|c| !c.is_renderable())
            .cloned()
            .collect();

        let content = content
            .into_iter()
            .map(|c| c.into_iter().filter(|c| c.is_renderable()).collect())
            .collect::<Vec<Vec<Word>>>();

        Self {
            kind,
            height: content.len() as u16,
            meta_info,
            content,
            offset: 0,
            scroll_offset: 0,
            focused: false,
            focused_index: 0,
        }
    }

    pub fn kind(&self) -> RenderNode {
        self.kind
    }

    pub fn content(&self) -> &Vec<Vec<Word>> {
        &self.content
    }

    pub fn content_as_lines(&self) -> Vec<String> {
        self.content
            .iter()
            .map(|c| c.iter().map(|c| c.content()).collect::<Vec<_>>().join(""))
            .collect()
    }

    pub fn content_mut(&mut self) -> &mut Vec<Vec<Word>> {
        &mut self.content
    }

    pub fn meta_info(&self) -> &Vec<Word> {
        &self.meta_info
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

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn deselect(&mut self) {
        self.focused = false;
        self.focused_index = 0;
        self.content
            .iter_mut()
            .flatten()
            .filter(|c| c.kind() == WordType::Selected)
            .for_each(|c| {
                c.clear_kind();
            });
    }

    pub fn visually_select(&mut self, index: usize) -> Result<(), String> {
        self.focused = true;
        self.focused_index = index;

        if index >= self.num_links() {
            return Err(format!(
                "Index out of bounds: {} >= {}",
                index,
                self.num_links()
            ));
        }

        // Transform nth link to selected
        self.link_words_mut()
            .get_mut(index)
            .ok_or("index out of bounds")?
            .iter_mut()
            .for_each(|c| {
                c.set_kind(WordType::Selected);
            });
        Ok(())
    }

    pub fn mark_word(&mut self, height: usize, index: usize, length: usize) -> Result<(), String> {
        let line = self.content.get_mut(height).ok_or("index out of bounds")?;
        let mut skip_while = 0;
        let mut take_while = 0;
        line.iter_mut()
            .by_ref()
            .skip_while(|c| {
                skip_while += c.content().len();
                skip_while <= index + c.content().len() / 2
            })
            .take_while(|k| {
                take_while += k.content().len();
                take_while < length + k.content().len()
            })
            .for_each(|word| {
                // if word.content() != " " {
                //     word.set_kind(WordType::Selected);
                // }
                word.set_kind(WordType::Selected);
            });
        Ok(())
    }

    fn link_words_mut(&mut self) -> Vec<Vec<&mut Word>> {
        let mut selection: Vec<Vec<&mut Word>> = Vec::new();
        let mut iter = self.content.iter_mut().flatten().peekable();
        while let Some(e) = iter.peek() {
            if e.kind() == WordType::Link {
                selection.push(
                    iter.by_ref()
                        .take_while(|c| c.kind() == WordType::Link)
                        .collect(),
                );
            } else {
                iter.next();
            }
        }
        selection
    }

    pub fn highlight_link(&self) -> Result<&str, String> {
        Ok(self
            .meta_info()
            .iter()
            .filter(|c| c.kind() == WordType::LinkData)
            .nth(self.focused_index)
            .ok_or("index out of bounds")?
            .content())
    }

    pub fn num_links(&self) -> usize {
        self.meta_info
            .iter()
            .filter(|c| c.kind() == WordType::LinkData)
            .count()
    }

    pub fn transform(&mut self, width: u16) {
        match self.kind {
            RenderNode::Heading => self.height = 1,
            RenderNode::List => {
                let mut len = 0;
                let mut lines = Vec::new();
                let mut line = Vec::new();
                let indent_iter = self.meta_info.iter().filter(|c| c.content().trim() == "");
                let list_type_iter = self.meta_info.iter().filter(|c| {
                    matches!(
                        c.kind(),
                        WordType::MetaInfo(MetaData::OList) | WordType::MetaInfo(MetaData::UList)
                    )
                });

                let mut zip_iter = indent_iter.zip(list_type_iter);
                let mut indent = 0;
                let mut extra_indent = 0;
                for word in self.content.iter_mut().flatten() {
                    if word.content().len() + len < width as usize
                        && word.kind() != WordType::ListMarker
                    {
                        len += word.content().len() + 1;
                        line.push(word.clone());
                    } else {
                        let filler_content = if word.kind() == WordType::ListMarker {
                            indent = if let Some((meta, list_type)) = zip_iter.next() {
                                if list_type.kind() == WordType::MetaInfo(MetaData::OList) {
                                    extra_indent = 1;
                                } else {
                                    extra_indent = 0;
                                }
                                meta.content().len()
                            } else {
                                0
                            };
                            " ".repeat(indent)
                        } else {
                            " ".repeat(indent + 2 + extra_indent)
                        };
                        lines.push(line);
                        len = word.content.len() + 1;
                        let content = word.content.trim_start().to_owned();
                        word.set_content(content);
                        let filler = Word::new(filler_content, WordType::Normal);
                        line = vec![filler, word.to_owned()];
                    }
                }
                lines.push(line);
                // Remove empty lines
                lines.retain(|l| l.iter().any(|c| c.content() != ""));
                self.height = lines.len() as u16;
                self.content = lines;
            }
            RenderNode::CodeBlock => {
                let content = self
                    .content
                    .iter()
                    .filter(|c| c.iter().any(|x| x.is_renderable()))
                    .cloned()
                    .collect::<Vec<_>>();

                self.content = content;

                let height = self.content.len() as u16;
                self.height = height;
            }
            RenderNode::Paragraph | RenderNode::Task => {
                let width = match self.kind {
                    RenderNode::Paragraph => width as usize,
                    RenderNode::Task => width as usize - 4,
                    _ => unreachable!(),
                };
                let mut len = 0;
                let mut lines = Vec::new();
                let mut line = Vec::new();
                let iter = self.content.iter().flatten();
                for word in iter {
                    if word.content.len() + len < width {
                        len += word.content.len();
                        line.push(word.clone());
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
            RenderNode::Quote => self.height = 1,
        }
    }
}
