use crate::{
    highlight::{highlight_code, HighlightInfo, COLOR_MAP},
    parser::MdParseEnum,
    search::{compare_heading, find_and_mark},
};
use itertools::Itertools;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};
use tree_sitter_highlight::HighlightEvent;

#[derive(Debug, Clone)]
pub struct RenderRoot {
    file_name: Option<String>,
    components: Vec<RenderComponent>,
    is_focused: bool,
}

impl RenderRoot {
    pub fn new(file_name: Option<String>, components: Vec<RenderComponent>) -> Self {
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

    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    pub fn words(&self) -> Vec<&Word> {
        self.components
            .iter()
            .flat_map(|c| c.content().iter().flatten())
            .collect()
    }

    pub fn find_and_mark(&mut self, search: &str) {
        let mut words = self
            .components
            .iter_mut()
            .flat_map(|c| c.words_mut())
            .collect::<Vec<_>>();
        find_and_mark(search, &mut words)
    }

    pub fn search_results_heights(&self) -> Vec<usize> {
        self.components
            .iter()
            .flat_map(|c| {
                let mut heights = c.selected_heights();
                heights.iter_mut().for_each(|h| *h += c.y_offset() as usize);
                heights
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.file_name = None;
        self.components.clear();
    }

    pub fn select(&mut self, index: usize) -> Result<u16, String> {
        self.deselect();
        self.is_focused = true;
        let mut count = 0;
        for comp in self.components.iter_mut() {
            if index - count < comp.num_links() {
                comp.visually_select(index - count)?;
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

    pub fn link_index_and_height(&self) -> Vec<(usize, u16)> {
        let mut indexes = Vec::new();
        let mut count = 0;
        self.components.iter().for_each(|comp| {
            let height = comp.y_offset();
            comp.content().iter().enumerate().for_each(|(index, row)| {
                row.iter().for_each(|c| {
                    if c.kind() == WordType::Link || c.kind() == WordType::Selected {
                        indexes.push((count, height + index as u16));
                        count += 1;
                    }
                })
            });
        });

        indexes
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
        for component in self.components.iter() {
            if component.kind() == RenderNode::Heading
                && compare_heading(&heading[1..], component.content())
            {
                return Ok(y_offset);
            }
            y_offset += component.height();
        }
        Err(format!("Heading not found: {}", heading))
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

    /// Transforms the content of the components to fit the given width
    pub fn transform(&mut self, width: u16) {
        for component in self.components_mut() {
            component.transform(width);
        }
    }

    /// Because of the parsing, every table has a missing newline at the end
    pub fn add_missing_components(self) -> Self {
        let mut components = Vec::new();
        let mut iter = self.components.iter().peekable();
        while let Some(component) = iter.next() {
            components.push(component.to_owned());
            if let Some(next) = iter.peek() {
                if component.kind() != RenderNode::LineBreak && next.kind() != RenderNode::LineBreak
                {
                    components.push(RenderComponent::new(RenderNode::LineBreak, Vec::new()));
                }
            }
        }
        Self {
            file_name: self.file_name,
            components,
            is_focused: self.is_focused,
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
    ColumnsCount,
    Important,
    Note,
    Tip,
    Warning,
    Caution,
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
            MdParseEnum::Link | MdParseEnum::WikiLink => WordType::Link,
            MdParseEnum::BoldItalic => WordType::BoldItalic,

            MdParseEnum::Digit => WordType::ListMarker,

            MdParseEnum::Paragraph
            | MdParseEnum::TableRow
            | MdParseEnum::Sentence
            | MdParseEnum::Word => WordType::Normal,

            MdParseEnum::LinkData => WordType::LinkData,
            MdParseEnum::Imortant => WordType::MetaInfo(MetaData::Important),
            MdParseEnum::Note => WordType::MetaInfo(MetaData::Note),
            MdParseEnum::Tip => WordType::MetaInfo(MetaData::Tip),
            MdParseEnum::Warning => WordType::MetaInfo(MetaData::Warning),
            MdParseEnum::Caution => WordType::MetaInfo(MetaData::Caution),

            MdParseEnum::Heading
            | MdParseEnum::Task
            | MdParseEnum::UnorderedList
            | MdParseEnum::ListContainer
            | MdParseEnum::OrderedList
            | MdParseEnum::CodeBlock
            | MdParseEnum::CodeStr
            | MdParseEnum::ItalicStr
            | MdParseEnum::Quote
            | MdParseEnum::Table
            | MdParseEnum::TableCell
            | MdParseEnum::BoldStr
            | MdParseEnum::BoldItalicStr
            | MdParseEnum::StrikethroughStr
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
    HorizontalSeperator,
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
        if self.kind == RenderNode::Table {
            let column_count = self.meta_info.len();

            let moved_content = self.content.chunks(column_count).collect::<Vec<_>>();

            let mut lines = Vec::new();

            moved_content.iter().for_each(|line| {
                let noe = line
                    .iter()
                    .map(|c| c.iter().map(|word| word.content()).join(""))
                    .join(" ");
                lines.push(noe);
            });

            lines
        } else {
            self.content
                .iter()
                .map(|c| c.iter().map(|c| c.content()).collect::<Vec<_>>().join(""))
                .collect()
        }
    }

    pub fn content_as_bytes(&self) -> Vec<u8> {
        match self.kind() {
            RenderNode::CodeBlock => self.content_as_lines().join("").as_bytes().to_vec(),

            _ => {
                let strings = self.content_as_lines();
                let string = strings.join("\n");
                string.as_bytes().to_vec()
            }
        }
    }

    pub fn content_owned(self) -> Vec<Vec<Word>> {
        self.content
    }

    pub fn meta_info(&self) -> &Vec<Word> {
        &self.meta_info
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

    pub fn selected_heights(&self) -> Vec<usize> {
        let mut heights = Vec::new();

        if self.kind() == RenderNode::Table {
            let column_count = self.meta_info.len();
            let iter = self.content.chunks(column_count).enumerate();

            for (i, line) in iter {
                if line
                    .iter()
                    .any(|c| c.iter().any(|x| x.kind() == WordType::Selected))
                {
                    heights.push(i);
                }
            }
            return heights;
        }

        for (i, line) in self.content.iter().enumerate() {
            if line.iter().any(|c| c.kind() == WordType::Selected) {
                heights.push(i);
            }
        }
        heights
    }

    fn words_mut(&mut self) -> Vec<&mut Word> {
        self.content.iter_mut().flatten().collect()
    }

    pub fn transform(&mut self, width: u16) {
        match self.kind {
            RenderNode::Heading => self.height = 1,
            RenderNode::List => {
                transform_list(self, width);
            }
            RenderNode::CodeBlock => {
                transform_codeblock(self);
            }
            RenderNode::Paragraph | RenderNode::Task | RenderNode::Quote => {
                transform_paragraph(self, width);
            }
            RenderNode::LineBreak => {
                self.height = 1;
            }
            RenderNode::Table => {
                self.content.retain(|c| !c.is_empty());
                let height = (self.content.len() / self.meta_info().len()) as u16;
                self.height = height;
            }
            RenderNode::HorizontalSeperator => self.height = 1,
        }
    }
}

fn transform_paragraph(component: &mut RenderComponent, width: u16) {
    let width = match component.kind {
        RenderNode::Paragraph => width as usize,
        RenderNode::Task => width as usize - 4,
        RenderNode::Quote => width as usize - 2,
        _ => unreachable!(),
    };
    let mut len = 0;
    let mut lines = Vec::new();
    let mut line = Vec::new();
    if component.kind() == RenderNode::Quote && component.meta_info().is_empty() {
        let filler = Word::new(" ".to_string(), WordType::Normal);
        line.push(filler);
    }
    let iter = component.content.iter().flatten();
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
            if component.kind() == RenderNode::Quote {
                let filler = Word::new(" ".to_string(), WordType::Normal);
                line = vec![filler, word];
            } else {
                line = vec![word];
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    component.height = lines.len() as u16;
    component.content = lines;
}

fn transform_codeblock(component: &mut RenderComponent) {
    let language = if let Some(word) = component.meta_info().first() {
        word.content()
    } else {
        ""
    };

    let highlight = highlight_code(language, &component.content_as_bytes());

    let content = component.content_as_lines().join("");

    let mut new_content = Vec::new();

    match highlight {
        HighlightInfo::Highlighted(e) => {
            let mut color = Color::Reset;
            for event in e {
                match event {
                    HighlightEvent::Source { start, end } => {
                        let word =
                            Word::new(content[start..end].to_string(), WordType::CodeBlock(color));
                        new_content.push(word);
                    }
                    HighlightEvent::HighlightStart(index) => {
                        color = COLOR_MAP[index.0];
                    }
                    HighlightEvent::HighlightEnd => color = Color::Reset,
                }
            }

            // Find all the new lines to split the content correctly
            let mut final_content = Vec::new();
            let mut inner_content = Vec::new();
            for word in new_content {
                if !word.content().contains('\n') {
                    inner_content.push(word);
                } else {
                    let mut start = 0;
                    let mut end;
                    for (i, c) in word.content().chars().enumerate() {
                        if c == '\n' {
                            end = i;
                            let new_word =
                                Word::new(word.content()[start..end].to_string(), word.kind());
                            inner_content.push(new_word);
                            start = i + 1;
                            final_content.push(inner_content);
                            inner_content = Vec::new();
                        } else if i == word.content().len() - 1 {
                            let new_word =
                                Word::new(word.content()[start..].to_string(), word.kind());
                            inner_content.push(new_word);
                        }
                    }
                }
            }

            final_content.push(vec![Word::new("".to_string(), WordType::CodeBlock(color))]);

            component.content = final_content;
        }
        HighlightInfo::Unhighlighted => (),
    }

    let height = component.content.len() as u16;
    component.height = height;
}

fn transform_list(component: &mut RenderComponent, width: u16) {
    let mut len = 0;
    let mut lines = Vec::new();
    let mut line = Vec::new();
    let indent_iter = component
        .meta_info
        .iter()
        .filter(|c| c.content().trim() == "");
    let list_type_iter = component.meta_info.iter().filter(|c| {
        matches!(
            c.kind(),
            WordType::MetaInfo(MetaData::OList) | WordType::MetaInfo(MetaData::UList)
        )
    });

    let mut zip_iter = indent_iter.zip(list_type_iter);
    let mut indent = 0;
    let mut extra_indent = 0;
    for word in component.content.iter_mut().flatten() {
        if word.content().len() + len < width as usize && word.kind() != WordType::ListMarker {
            len += word.content().len();
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
            len = word.content.len();
            let content = word.content.trim_start().to_owned();
            word.set_content(content);
            let filler = Word::new(filler_content, WordType::Normal);
            len += filler.content().len();
            line = vec![filler, word.to_owned()];
        }
    }
    lines.push(line);
    // Remove empty lines
    lines.retain(|l| l.iter().any(|c| c.content() != ""));
    component.height = lines.len() as u16;
    component.content = lines;
}
