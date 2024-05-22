use std::cmp;

use itertools::Itertools;

use ratatui::style::Color;
use tree_sitter_highlight::HighlightEvent;

use crate::{
    highlight::{highlight_code, HighlightInfo, COLOR_MAP},
    nodes::word::MetaData,
};

use super::word::{Word, WordType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextNode {
    Image,
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
pub struct TextComponent {
    kind: TextNode,
    content: Vec<Vec<Word>>,
    meta_info: Vec<Word>,
    height: u16,
    offset: u16,
    scroll_offset: u16,
    focused: bool,
    focused_index: usize,
}

impl TextComponent {
    pub fn new(kind: TextNode, content: Vec<Word>) -> Self {
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

    pub fn new_formatted(kind: TextNode, content: Vec<Vec<Word>>) -> Self {
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

    pub fn kind(&self) -> TextNode {
        self.kind
    }

    pub fn content(&self) -> &Vec<Vec<Word>> {
        &self.content
    }

    pub fn content_as_lines(&self) -> Vec<String> {
        if self.kind == TextNode::Table {
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
            TextNode::CodeBlock => self.content_as_lines().join("").as_bytes().to_vec(),

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

        if self.kind() == TextNode::Table {
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

    pub fn words_mut(&mut self) -> Vec<&mut Word> {
        self.content.iter_mut().flatten().collect()
    }

    pub fn transform(&mut self, width: u16) {
        match self.kind {
            TextNode::Heading => self.height = 1,
            TextNode::List => {
                transform_list(self, width);
            }
            TextNode::CodeBlock => {
                transform_codeblock(self);
            }
            TextNode::Paragraph | TextNode::Task | TextNode::Quote => {
                transform_paragraph(self, width);
            }
            TextNode::LineBreak => {
                self.height = 1;
            }
            TextNode::Table => {
                self.content.retain(|c| !c.is_empty());
                let height = (self.content.len() / self.meta_info().len()) as u16;
                self.height = height;
            }
            TextNode::HorizontalSeperator => self.height = 1,
            TextNode::Image => unreachable!("Image should not be transformed"),
        }
    }
}

fn transform_paragraph(component: &mut TextComponent, width: u16) {
    let width = match component.kind {
        TextNode::Paragraph => width as usize,
        TextNode::Task => width as usize - 4,
        TextNode::Quote => width as usize - 2,
        _ => unreachable!(),
    };
    let mut len = 0;
    let mut lines = Vec::new();
    let mut line = Vec::new();
    if component.kind() == TextNode::Quote && component.meta_info().is_empty() {
        let filler = Word::new(" ".to_string(), WordType::Normal);
        line.push(filler);
    }
    let iter = component.content.iter().flatten();
    for word in iter {
        if word.content().len() + len < width {
            len += word.content().len();
            line.push(word.clone());
        } else {
            lines.push(line);
            len = word.content().len() + 1;
            let mut word = word.clone();
            let content = word.content().trim_start().to_owned();
            word.set_content(content);
            if component.kind() == TextNode::Quote {
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

fn transform_codeblock(component: &mut TextComponent) {
    let language = if let Some(word) = component.meta_info().first() {
        word.content()
    } else {
        ""
    };

    let highlight = highlight_code(language, &component.content_as_bytes());

    let content = component.content_as_lines().join("");

    let mut new_content = Vec::new();

    if language.is_empty() {
        component.content.insert(
            0,
            vec![Word::new("".to_string(), WordType::CodeBlock(Color::Reset))],
        );
    }
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
                    for (i, c) in word.content().char_indices() {
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

fn transform_list(component: &mut TextComponent, width: u16) {
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

    let mut o_list_counter_stack = vec![0];
    let mut max_stack_len = 1;
    let mut indent = 0;
    let mut extra_indent = 0;
    let mut tmp = indent;
    for word in component.content.iter_mut().flatten() {
        if word.content().len() + len < width as usize && word.kind() != WordType::ListMarker {
            len += word.content().len();
            line.push(word.clone());
        } else {
            let filler_content = if word.kind() == WordType::ListMarker {
                indent = if let Some((meta, list_type)) = zip_iter.next() {
                    match tmp.cmp(&meta.content().len()) {
                        cmp::Ordering::Less => {
                            o_list_counter_stack.push(0);
                            max_stack_len += 1;
                        }
                        cmp::Ordering::Greater => {
                            o_list_counter_stack.pop();
                        }
                        cmp::Ordering::Equal => (),
                    }
                    if list_type.kind() == WordType::MetaInfo(MetaData::OList) {
                        let counter = o_list_counter_stack
                            .last_mut()
                            .expect("List parse error. Stack is empty");

                        *counter += 1;

                        word.set_content(format!("{counter}. "));

                        extra_indent = 1; // Ordered list is longer than unordered and needs extra space
                    } else {
                        extra_indent = 0;
                    }
                    tmp = meta.content().len();
                    tmp
                } else {
                    0
                };

                " ".repeat(indent)
            } else {
                " ".repeat(indent + 2 + extra_indent)
            };

            let filler = Word::new(filler_content, WordType::Normal);

            lines.push(line);
            let content = word.content().trim_start().to_owned();
            word.set_content(content);
            len = word.content().len() + filler.content().len();
            line = vec![filler, word.to_owned()];
        }
    }
    lines.push(line);
    // Remove empty lines
    lines.retain(|l| l.iter().any(|c| c.content() != ""));

    // Find out if there are ordered indexes longer than 3 chars. F.ex. `1. ` is three chars, but `10. ` is four chars.
    // To align the list on the same column, we need to find the longest index and add the difference to the shorter indexes.
    let mut indent_correction = vec![0; max_stack_len];
    let mut indent_index: u32 = 0;
    let mut indent_len = 0;

    for line in lines.iter() {
        if !line[1]
            .content()
            .starts_with(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
        {
            continue;
        }

        match indent_len.cmp(&line[0].content().len()) {
            cmp::Ordering::Less => {
                indent_index += 1;
                indent_len = line[0].content().len();
            }
            cmp::Ordering::Greater => {
                indent_index = indent_index.saturating_sub(1);
                indent_len = line[0].content().len();
            }
            cmp::Ordering::Equal => (),
        }

        indent_correction[indent_index as usize] = cmp::max(
            indent_correction[indent_index as usize],
            line[1].content().len(),
        );
    }

    // Finally, apply the indent correction to the list for each ordered index which is shorter
    // than the longest index.

    indent_index = 0;
    indent_len = 0;
    let mut unordered_list_skip = false; // Skip unordered list items. They are already aligned.

    for line in lines.iter_mut() {
        if line[1]
            .content()
            .starts_with(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
            && !line[1].content().ends_with(". ")
        {
            unordered_list_skip = false;
        }

        if line[1].content() == "• " || unordered_list_skip {
            unordered_list_skip = true;
            continue;
        }

        let amount = if line[1]
            .content()
            .starts_with(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
            && line[1].content().ends_with(". ")
        {
            match indent_len.cmp(&line[0].content().len()) {
                cmp::Ordering::Less => {
                    indent_index += 1;
                    indent_len = line[0].content().len();
                }
                cmp::Ordering::Greater => {
                    indent_index = indent_index.saturating_sub(1);
                    indent_len = line[0].content().len();
                }
                cmp::Ordering::Equal => (),
            }
            indent_correction[indent_index as usize].saturating_sub(line[1].content().len())
                + line[0].content().len()
        } else {
            // -3 because that is the length of the shortest ordered index (1. )
            (indent_correction[indent_index as usize] + line[0].content().len()).saturating_sub(3)
        };

        line[0].set_content(" ".repeat(amount));
    }

    component.height = lines.len() as u16;
    component.content = lines;
}
