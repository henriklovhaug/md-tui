use std::cmp;

use itertools::Itertools;

use ratatui::style::Color;
use tree_sitter_highlight::HighlightEvent;

use crate::{
    highlight::{COLOR_MAP, HighlightInfo, highlight_code},
    nodes::word::MetaData,
};

use super::word::{Word, WordType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextNode {
    Image,
    Paragraph,
    LineBreak,
    Heading,
    Task,
    List,
    Footnote,
    /// (`widths_by_column`, `heights_by_row`)
    Table(Vec<u16>, Vec<u16>),
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
    #[must_use]
    pub fn new(kind: TextNode, content: Vec<Word>) -> Self {
        let meta_info: Vec<Word> = content
            .iter()
            .filter(|c| !c.is_renderable() || c.kind() == WordType::FootnoteInline)
            .cloned()
            .collect();

        let content = content
            .into_iter()
            .filter(Word::is_renderable)
            .collect();

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

    #[must_use]
    pub fn new_formatted(kind: TextNode, content: Vec<Vec<Word>>) -> Self {
        let meta_info: Vec<Word> = content
            .iter()
            .flatten()
            .filter(|c| !c.is_renderable())
            .cloned()
            .collect();

        let content = content
            .into_iter()
            .map(|c| {
                c.into_iter()
                    .filter(Word::is_renderable)
                    .collect()
            })
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

    #[must_use]
    pub fn kind(&self) -> TextNode {
        self.kind.clone()
    }

    #[must_use]
    pub fn content(&self) -> &Vec<Vec<Word>> {
        &self.content
    }

    #[must_use]
    pub fn content_as_lines(&self) -> Vec<String> {
        if let TextNode::Table(widths, _) = self.kind() {
            let column_count = widths.len();

            let moved_content = self.content.chunks(column_count).collect::<Vec<_>>();

            let mut lines = Vec::new();

            moved_content.iter().for_each(|line| {
                let temp = line
                    .iter()
                    .map(|c| c.iter().map(Word::content).join(""))
                    .join(" ");
                lines.push(temp);
            });

            lines
        } else {
            self.content
                .iter()
                .map(|c| {
                    c.iter()
                        .map(Word::content)
                        .collect::<Vec<_>>()
                        .join("")
                })
                .collect()
        }
    }

    #[must_use]
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

    #[must_use]
    pub fn content_owned(self) -> Vec<Vec<Word>> {
        self.content
    }

    #[must_use]
    pub fn meta_info(&self) -> &Vec<Word> {
        &self.meta_info
    }

    #[must_use]
    pub fn height(&self) -> u16 {
        self.height
    }

    #[must_use]
    pub fn y_offset(&self) -> u16 {
        self.offset
    }

    #[must_use]
    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub fn set_y_offset(&mut self, y_offset: u16) {
        self.offset = y_offset;
    }

    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.scroll_offset = offset;
    }

    #[must_use]
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
            if matches!(e.kind(), WordType::Link | WordType::FootnoteInline) {
                selection.push(
                    iter.by_ref()
                        .take_while(|c| {
                            matches!(c.kind(), WordType::Link | WordType::FootnoteInline)
                        })
                        .collect(),
                );
            } else {
                iter.next();
            }
        }
        selection
    }

    #[must_use]
    pub fn get_footnote(&self, search: &str) -> String {
        self.content()
            .iter()
            .flatten()
            .skip_while(|c| c.kind() != WordType::FootnoteData && c.content() != search)
            .take_while(|c| c.kind() == WordType::Footnote)
            .map(Word::content)
            .collect()
    }

    pub fn highlight_link(&self) -> Result<&str, String> {
        Ok(self
            .meta_info()
            .iter()
            .filter(|c| matches!(c.kind(), WordType::LinkData | WordType::FootnoteInline))
            .nth(self.focused_index)
            .ok_or("index out of bounds")?
            .content())
    }

    #[must_use]
    pub fn num_links(&self) -> usize {
        self.meta_info
            .iter()
            .filter(|c| matches!(c.kind(), WordType::LinkData | WordType::FootnoteInline))
            .count()
    }

    #[must_use]
    pub fn selected_heights(&self) -> Vec<usize> {
        let mut heights = Vec::new();

        if let TextNode::Table(widths, _) = self.kind() {
            let column_count = widths.len();
            let iter = self.content.chunks(column_count).enumerate();

            for (i, line) in iter {
                if line
                    .iter()
                    .flatten()
                    .any(|c| c.kind() == WordType::Selected)
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
            TextNode::List => {
                transform_list(self, width);
            }
            TextNode::CodeBlock => {
                transform_codeblock(self);
            }
            TextNode::Paragraph | TextNode::Task | TextNode::Quote => {
                transform_paragraph(self, width);
            }
            TextNode::LineBreak | TextNode::Heading => {
                self.height = 1;
            }
            TextNode::Table(_, _) => {
                transform_table(self, width);
            }
            TextNode::HorizontalSeperator => self.height = 1,
            TextNode::Image => unreachable!("Image should not be transformed"),
            TextNode::Footnote => self.height = 0,
        }
    }
}

fn word_wrapping<'a>(
    words: impl IntoIterator<Item = &'a Word>,
    width: usize,
    allow_hyphen: bool,
) -> Vec<Vec<Word>> {
    let enable_hyphen = allow_hyphen && width > 4;

    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut line_len = 0;
    for word in words {
        let word_len = word.content().len();
        if line_len + word_len <= width {
            line_len += word_len;
            line.push(word.clone());
        } else if word_len <= width {
            lines.push(line);
            let mut word = word.clone();
            let content = word.content().trim_start().to_owned();
            word.set_content(content);

            line_len = word.content().len();
            line = vec![word];
        } else {
            let mut content = word.content().to_owned();

            if width - line_len < 4 {
                line_len = 0;
                lines.push(line);
                line = Vec::new();
            }

            let mut newline_content: String = content
                .char_indices()
                .skip(width - line_len - 1)
                .map(|(_index, character)| character)
                .collect();

            if enable_hyphen && !content.ends_with('-') {
                newline_content.insert(0, content.pop().unwrap());
                content.push('-');
            }

            line.push(Word::new(content, word.kind()));
            lines.push(line);

            while newline_content.len() > width {
                let mut next_newline_content: String = newline_content
                    .char_indices()
                    .skip(width - 1)
                    .map(|(_index, character)| character)
                    .collect();
                if enable_hyphen && !newline_content.ends_with('-') {
                    next_newline_content.insert(0, newline_content.pop().unwrap());
                    newline_content.push('-');
                }

                line = vec![Word::new(newline_content, word.kind())];
                lines.push(line);

                newline_content = next_newline_content;
            }

            if newline_content.is_empty() {
                line_len = 0;
                line = Vec::new();
            } else {
                line_len = newline_content.len();
                line = vec![Word::new(newline_content, word.kind())];
            }
        }
    }

    if !line.is_empty() {
        lines.push(line);
    }

    lines
}

fn transform_paragraph(component: &mut TextComponent, width: u16) {
    let width = match component.kind {
        TextNode::Paragraph => width as usize - 1,
        TextNode::Task => width as usize - 4,
        TextNode::Quote => width as usize - 2,
        _ => unreachable!(),
    };

    let mut lines = word_wrapping(component.content.iter().flatten(), width, true);

    if component.kind() == TextNode::Quote {
        let is_special_quote = !component.meta_info.is_empty();

        for line in lines.iter_mut().skip(usize::from(is_special_quote)) {
            line.insert(0, Word::new(" ".to_string(), WordType::Normal));
        }
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
            vec![Word::new(String::new(), WordType::CodeBlock(Color::Reset))],
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
                if word.content().contains('\n') {
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
                } else {
                    inner_content.push(word);
                }
            }

            final_content.push(vec![Word::new(String::new(), WordType::CodeBlock(color))]);

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
            WordType::MetaInfo(MetaData::OList | MetaData::UList)
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

    for line in &lines {
        if !line[1]
            .content()
            .strip_prefix(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
            .is_some_and(|c| c.ends_with(". "))
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
    let mut unordered_list_skip = true; // Skip unordered list items. They are already aligned.

    for line in &mut lines {
        if line[1]
            .content()
            .strip_prefix(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
            .is_some_and(|c| c.ends_with(". "))
        {
            unordered_list_skip = false;
        }

        if line[1].content() == "• " || unordered_list_skip {
            unordered_list_skip = true;
            continue;
        }

        let amount = if line[1]
            .content()
            .strip_prefix(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
            .is_some_and(|c| c.ends_with(". "))
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

fn transform_table(component: &mut TextComponent, width: u16) {
    let content = &mut component.content;

    let column_count = component
        .meta_info
        .iter()
        .filter(|w| w.kind() == WordType::MetaInfo(MetaData::ColumnsCount))
        .count();

    if !content.len().is_multiple_of(column_count) || column_count == 0 {
        component.height = 1;
        component.kind = TextNode::Table(vec![], vec![]);
        return;
    }

    assert!(
        content.len().is_multiple_of(column_count),
        "Invalid table cell distribution: content.len() = {}, column_count = {}",
        content.len(),
        column_count
    );

    let row_count = content.len() / column_count;

    ///////////////////////////
    // Find unbalanced width //
    ///////////////////////////
    let widths = {
        let mut widths = vec![0; column_count];
        content.chunks(column_count).for_each(|row| {
            row.iter().enumerate().for_each(|(col_i, entry)| {
                let len = content_entry_len(entry);
                if len > widths[col_i] as usize {
                    widths[col_i] = len as u16;
                }
            });
        });

        widths
    };

    let styling_width = column_count as u16;
    let unbalanced_cells_width = widths.iter().sum::<u16>();

    /////////////////////////////////////
    // Return if unbalanced width fits //
    /////////////////////////////////////
    if width >= unbalanced_cells_width + styling_width {
        component.height = (content.len() / column_count) as u16;
        component.kind = TextNode::Table(widths, vec![1; component.height as usize]);
        return;
    }

    //////////////////////////////
    // Find overflowing columns //
    //////////////////////////////
    let overflow_threshold = (width - styling_width) / column_count as u16;
    let mut overflowing_columns = vec![];

    let (overflowing_width, non_overflowing_width) = {
        let mut overflowing_width = 0;
        let mut non_overflowing_width = 0;

        for (column_i, column_width) in widths.iter().enumerate() {
            if *column_width > overflow_threshold {
                overflowing_columns.push((column_i, column_width));

                overflowing_width += column_width;
            } else {
                non_overflowing_width += column_width;
            }
        }

        (overflowing_width, non_overflowing_width)
    };

    assert!(
        !overflowing_columns.is_empty(),
        "table overflow should not be handled when there are no overflowing columns"
    );

    /////////////////////////////////////////////
    // Assign new width to overflowing columns //
    /////////////////////////////////////////////
    let mut available_balanced_width = width - non_overflowing_width - styling_width;
    let mut available_overflowing_width = overflowing_width;

    let overflowing_column_min_width =
        (available_balanced_width / (2 * overflowing_columns.len() as u16)).max(1);

    let mut widths_balanced: Vec<u16> = widths.clone();
    for (column_i, old_column_width) in overflowing_columns
        .iter()
        // Sorting ensures the smallest overflowing cells receive minimum area without the
        // need for recalculating the larger cells
        .sorted_by(|a, b| Ord::cmp(a.1, b.1))
    {
        // Ensure the longest cell gets the most amount of area
        let ratio = f32::from(**old_column_width) / f32::from(available_overflowing_width);
        let mut balanced_column_width =
            (ratio * f32::from(available_balanced_width)).floor() as u16;

        if balanced_column_width < overflowing_column_min_width {
            balanced_column_width = overflowing_column_min_width;
            available_overflowing_width -= **old_column_width;
            available_balanced_width -= balanced_column_width;
        }

        widths_balanced[*column_i] = balanced_column_width;
    }

    ////////////////////////////////////////
    // Wrap words based on balanced width //
    ////////////////////////////////////////
    let mut heights = vec![1; row_count];
    for (row_i, row) in content
        .iter_mut()
        .chunks(column_count)
        .into_iter()
        .enumerate()
    {
        for (column_i, entry) in row.into_iter().enumerate() {
            let lines = word_wrapping(
                entry.drain(..).as_ref(),
                widths_balanced[column_i] as usize,
                true,
            );

            if heights[row_i] < lines.len() as u16 {
                heights[row_i] = lines.len() as u16;
            }

            let _drop = std::mem::replace(entry, lines.into_iter().flatten().collect());
        }
    }

    component.height = heights.iter().copied().sum::<u16>();

    component.kind = TextNode::Table(widths_balanced, heights);
}

#[must_use]
pub fn content_entry_len(words: &[Word]) -> usize {
    words.iter().map(|word| word.content().len()).sum()
}
