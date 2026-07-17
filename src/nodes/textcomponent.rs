use std::cmp;

use itertools::Itertools;
use mermaid_text::render_with_width;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::style::Color;
use tree_sitter_highlight::HighlightEvent;

use crate::{
    highlight::{COLOR_MAP, HighlightInfo, highlight_code},
    nodes::word::MetaData,
    util::general::GENERAL_CONFIG,
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
    HorizontalSeparator,
    DetailsSummary {
        id: u32,
        folded: bool,
        body_len: usize,
    },
}

pub(crate) const TABLE_CELL_PADDING: u16 = 1;

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
    owning_details_ids: Vec<u32>,
    hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clipping {
    Both,
    Upper,
    Lower,
    None,
}

impl TextComponent {
    #[must_use]
    pub fn clip_content(&self, area_height: u16) -> (Vec<Vec<Word>>, Clipping) {
        let y_offset = self.y_offset();
        let scroll_offset = self.scroll_offset();
        let height = self.height();

        let clip = if y_offset < scroll_offset && y_offset + height > scroll_offset + area_height {
            Clipping::Both
        } else if y_offset < scroll_offset {
            Clipping::Upper
        } else if y_offset + height > scroll_offset + area_height {
            Clipping::Lower
        } else {
            Clipping::None
        };

        let mut content = self.content().to_owned();
        let top = scroll_offset.saturating_sub(y_offset) as usize;

        let clipped_content = match clip {
            Clipping::Both => {
                let len = content.len();
                let end = top.saturating_add(area_height as usize).min(len);
                content.drain(end..);
                let start = top.min(content.len());
                content.drain(0..start);
                content
            }
            Clipping::Upper => {
                let offset = content.len().saturating_sub(area_height as usize);
                content.drain(0..offset);
                content
            }
            Clipping::Lower => {
                let end = (area_height as usize).min(content.len());
                content.drain(end..);
                content
            }
            Clipping::None => content,
        };

        (clipped_content, clip)
    }
    #[must_use]
    pub fn new(kind: TextNode, content: Vec<Word>) -> Self {
        let meta_info: Vec<Word> = content
            .iter()
            .filter(|c| !c.is_renderable() || c.kind() == WordType::FootnoteInline)
            .cloned()
            .collect();

        let content = content.into_iter().filter(Word::is_renderable).collect();

        Self {
            kind,
            content: vec![content],
            meta_info,
            height: 0,
            offset: 0,
            scroll_offset: 0,
            focused: false,
            focused_index: 0,
            owning_details_ids: Vec::new(),
            hidden: false,
        }
    }

    #[must_use]
    pub fn new_formatted(kind: TextNode, content: Vec<Vec<Word>>) -> Self {
        Self::new_formatted_with_meta(kind, content, Vec::new())
    }

    #[must_use]
    pub fn new_formatted_with_meta(
        kind: TextNode,
        content: Vec<Vec<Word>>,
        mut meta_info: Vec<Word>,
    ) -> Self {
        meta_info.extend(
            content
                .iter()
                .flatten()
                .filter(|c| !c.is_renderable())
                .cloned(),
        );

        let content: Vec<Vec<Word>> = content
            .into_iter()
            .map(|c| c.into_iter().filter(Word::is_renderable).collect())
            .collect();

        Self {
            kind,
            height: content.len() as u16,
            meta_info,
            content,
            offset: 0,
            scroll_offset: 0,
            focused: false,
            focused_index: 0,
            owning_details_ids: Vec::new(),
            hidden: false,
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
                .map(|c| c.iter().map(Word::content).collect::<Vec<_>>().join(""))
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
        if self.hidden { 0 } else { self.height }
    }

    #[must_use]
    pub fn raw_height(&self) -> u16 {
        self.height
    }

    #[must_use]
    pub fn owning_details_ids(&self) -> &[u32] {
        &self.owning_details_ids
    }

    pub fn prepend_owning_details_id(&mut self, id: u32) {
        self.owning_details_ids.insert(0, id);
    }

    pub fn set_owning_details_ids(&mut self, ids: Vec<u32>) {
        self.owning_details_ids = ids;
    }

    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    /// If this component is a `DetailsSummary`, set its `folded` field.
    /// Returns the new folded state on success, `None` if the component
    /// is not a `DetailsSummary`.
    pub fn set_details_folded(&mut self, folded: bool) -> Option<bool> {
        if let TextNode::DetailsSummary {
            id,
            folded: _,
            body_len,
        } = self.kind.clone()
        {
            self.kind = TextNode::DetailsSummary {
                id,
                folded,
                body_len,
            };
            Some(folded)
        } else {
            None
        }
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

    /// Mark a `DetailsSummary` component as focused. Unlike
    /// `visually_select`, no inner word changes kind — the renderer
    /// reads `is_focused()` directly to apply selection styling to the
    /// whole header line.
    pub fn visually_select_summary(&mut self) {
        self.focused = true;
    }

    /// Clear focus on a `DetailsSummary` component.
    pub fn deselect_summary(&mut self) {
        self.focused = false;
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

    fn links_iter(&self) -> impl Iterator<Item = &Word> {
        // Links live in `meta_info` as `LinkData`; `FootnoteInline` is the
        // footnote equivalent. `WordType::Link` words are renderable and only
        // ever appear in `content`, so they are intentionally not matched here.
        self.meta_info
            .iter()
            .filter(|c| matches!(c.kind(), WordType::LinkData | WordType::FootnoteInline))
    }

    pub fn highlight_link(&self) -> Result<&str, String> {
        self.links_iter()
            .nth(self.focused_index)
            .ok_or_else(|| "index out of bounds".to_string())
            .map(|w| w.content())
    }

    /// Non-mutating lookup of the Nth link's anchor text (URL / `#heading`)
    /// in this component's meta info. Returns `None` if `index` is out of
    /// range. The ordinal matches `num_links()`.
    #[must_use]
    pub fn link_anchor_at(&self, index: usize) -> Option<&str> {
        self.links_iter().nth(index).map(Word::content)
    }

    #[must_use]
    pub fn num_links(&self) -> usize {
        if self.hidden {
            return 0;
        }
        self.links_iter().count()
    }

    #[must_use]
    pub fn selected_heights(&self) -> Vec<usize> {
        let mut heights = Vec::new();
        if self.hidden {
            return heights;
        }

        if let TextNode::Table(widths, row_heights) = self.kind() {
            let column_count = widths.len();
            let iter = self.content.chunks(column_count).enumerate();

            for (i, line) in iter {
                if line
                    .iter()
                    .flatten()
                    .any(|c| c.kind() == WordType::Selected)
                {
                    let offset = 1
                        + row_heights.iter().take(i).copied().sum::<u16>() as usize
                        + usize::from(i > 0);
                    heights.push(offset);
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
            TextNode::LineBreak | TextNode::Heading | TextNode::DetailsSummary { .. } => {
                self.height = 1;
            }
            TextNode::Table(_, _) => {
                transform_table(self, width);
            }
            TextNode::HorizontalSeparator => self.height = 1,
            TextNode::Image => unreachable!("Image should not be transformed"),
            TextNode::Footnote => self.height = 0,
        }
    }
}

fn trim_word_leading_space(word: &mut Word) {
    // Cheap fast path — most words don't start with whitespace so skip the
    // clone-and-rebuild entirely.
    if !word.content().starts_with(|c: char| c.is_whitespace()) {
        return;
    }

    // Slow path: span-bearing words need the original string preserved so
    // the new span can refer back into it.
    let content = word.content().to_owned();
    let trimmed = content.trim_start();
    if let Some(span) = word.source_span() {
        let new_span = span.subspan_of_suffix(&content, trimmed, trimmed.len());
        *word = Word::new_with_source_span(trimmed.to_owned(), word.kind(), new_span);
    } else {
        word.set_content(trimmed.to_owned());
    }
}

fn split_and_wrap_long_word(
    word: &Word,
    width: usize,
    enable_hyphen: bool,
    lines: &mut Vec<Vec<Word>>,
    current_line: &mut Vec<Word>,
    current_line_len: &mut usize,
) {
    let content = word.content().to_owned();

    if width - *current_line_len < 4 {
        *current_line_len = 0;
        lines.push(std::mem::take(current_line));
    }

    let (mut head, mut tail) =
        split_word_initial_part(&content, width, *current_line_len, enable_hyphen);

    let mut hyphenated = false;
    if enable_hyphen && !head.ends_with('-') && !head.is_empty() {
        if let Some(last_char) = head.pop() {
            tail.insert(0, last_char);
        }
        hyphenated = true;
    }

    let head_word = Word::new_with_source_span(
        head.clone(),
        word.kind(),
        word.source_span()
            .and_then(|s| s.subspan(&content, 0, head.len())),
    );
    current_line.push(head_word);
    if hyphenated {
        current_line.push(Word::new("-".to_owned(), word.kind()));
    }
    lines.push(std::mem::take(current_line));

    let mut context = WrapContext {
        original_content: &content,
        width,
        enable_hyphen,
        lines,
        current_line,
        current_line_len,
    };

    wrap_remaining_tail(word, tail, &mut context);
}

/// One column is reserved for a trailing hyphen when a word will be split and
/// hyphenated (i.e. hyphenation is on and it doesn't already end in `-`).
fn hyphen_reserve(content: &str, enable_hyphen: bool) -> usize {
    usize::from(enable_hyphen && !content.ends_with('-'))
}

fn split_word_initial_part(
    content: &str,
    width: usize,
    current_line_len: usize,
    enable_hyphen: bool,
) -> (String, String) {
    let split_width = width - current_line_len - hyphen_reserve(content, enable_hyphen);
    split_by_width(content, split_width)
}

fn wrap_remaining_tail(word: &Word, mut tail: String, context: &mut WrapContext) {
    while display_width(&tail) > context.width {
        tail = process_tail_chunk(word, tail, context);
    }

    finalize_tail(word, tail, context);
}

fn process_tail_chunk(word: &Word, tail: String, context: &mut WrapContext) -> String {
    let (inner_head, next_tail, hyphenated) = split_and_hyphenate_tail(&tail, context);

    let head_word = create_head_word(word, &inner_head, &tail, context);

    let mut line = vec![head_word];
    if hyphenated {
        line.push(Word::new("-".to_owned(), word.kind()));
    }
    context.lines.push(line);
    next_tail
}

fn split_and_hyphenate_tail(tail: &str, context: &WrapContext) -> (String, String, bool) {
    let split_width = context.width - hyphen_reserve(tail, context.enable_hyphen);

    let (mut inner_head, next_tail) = split_by_width(tail, split_width);
    let mut hyphenated = false;

    if context.enable_hyphen
        && !tail.ends_with('-')
        && !inner_head.is_empty()
        && let Some(last_char) = inner_head.pop()
    {
        hyphenated = true;
        let mut next_tail_with_char = next_tail;
        next_tail_with_char.insert(0, last_char);
        return (inner_head, next_tail_with_char, hyphenated);
    }
    (inner_head, next_tail, hyphenated)
}

fn create_head_word(
    word: &Word,
    inner_head: &str,
    current_tail_content: &str,
    context: &WrapContext,
) -> Word {
    Word::new_with_source_span(
        inner_head.to_owned(),
        word.kind(),
        word.source_span().and_then(|s| {
            s.subspan_of_suffix(
                context.original_content,
                current_tail_content,
                inner_head.len(),
            )
        }),
    )
}

fn finalize_tail(word: &Word, tail: String, context: &mut WrapContext) {
    if tail.is_empty() {
        *context.current_line_len = 0;
        *context.current_line = Vec::new();
    } else {
        let current_tail_content = tail.clone();
        let tail_len = current_tail_content.len();
        let tail_word = Word::new_with_source_span(
            tail,
            word.kind(),
            word.source_span().and_then(|s| {
                s.subspan_of_suffix(context.original_content, &current_tail_content, tail_len)
            }),
        );
        *context.current_line_len = display_width(tail_word.content());
        *context.current_line = vec![tail_word];
    }
}

struct WrapContext<'a> {
    original_content: &'a str,
    width: usize,
    enable_hyphen: bool,
    lines: &'a mut Vec<Vec<Word>>,
    current_line: &'a mut Vec<Word>,
    current_line_len: &'a mut usize,
}

pub(crate) fn word_wrapping<'a>(
    words: impl IntoIterator<Item = &'a Word>,
    width: usize,
    allow_hyphen: bool,
) -> Vec<Vec<Word>> {
    let enable_hyphen = allow_hyphen && width > 4;

    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut line_len = 0;
    for word in words {
        let word_len = display_width(word.content());
        if line_len + word_len <= width {
            line_len += word_len;
            line.push(word.clone());
        } else if word_len <= width {
            lines.push(line);
            let mut word = word.clone();
            trim_word_leading_space(&mut word);
            line_len = display_width(word.content());
            line = vec![word];
        } else {
            split_and_wrap_long_word(
                word,
                width,
                enable_hyphen,
                &mut lines,
                &mut line,
                &mut line_len,
            );
        }
    }

    if !line.is_empty() {
        lines.push(line);
    }

    lines
}

pub(crate) fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn split_by_width(text: &str, max_width: usize) -> (String, String) {
    if max_width == 0 {
        return (String::new(), text.to_string());
    }

    let mut width = 0;
    let mut split_idx = 0;
    // Track the byte index where the visible width reaches (or just exceeds) max_width.
    for (i, c) in text.char_indices() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + char_width > max_width {
            if split_idx == 0 {
                split_idx = i + c.len_utf8();
            }
            break;
        }
        width += char_width;
        split_idx = i + c.len_utf8();
        if width == max_width {
            break;
        }
    }

    let (head, tail) = text.split_at(split_idx);
    (head.to_string(), tail.to_string())
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
    let language = component
        .meta_info()
        .first()
        .map(|w| w.content())
        .unwrap_or("");
    let highlight = highlight_code(language, &component.content_as_bytes());
    let content = component.content_as_lines().join("");

    if language.is_empty() {
        component.content.insert(
            0,
            vec![Word::new(String::new(), WordType::CodeBlock(Color::Reset))],
        );
    }

    match highlight {
        HighlightInfo::Highlighted(events) => {
            component.content = process_highlighted_code(&content, events);
        }
        HighlightInfo::Mermaid => {
            if let Some(final_content) = process_mermaid_code(&content) {
                component.content = final_content;
            }
        }
        HighlightInfo::Unhighlighted => (),
    }

    let max_line_len = component
        .content()
        .iter()
        .map(|inner| inner.iter().fold(0, |acc, x| acc + x.content().width()))
        .max()
        .unwrap_or(0);

    let height = component.content.len() as u16;
    component.height = height;

    component.meta_info.push(Word::new(
        String::new(),
        WordType::MetaInfo(MetaData::LineLength(max_line_len as u16)),
    ));
}

/// Flatten highlight events into colored words, returning the trailing color
/// (the highlight state after the last event) for the closing blank word.
fn highlight_events_to_words(content: &str, events: Vec<HighlightEvent>) -> (Vec<Word>, Color) {
    let mut color = Color::Reset;
    let mut words = Vec::new();
    for event in events {
        match event {
            HighlightEvent::Source { start, end } => words.push(Word::new(
                content[start..end].to_string(),
                WordType::CodeBlock(color),
            )),
            HighlightEvent::HighlightStart(index) => color = COLOR_MAP[index.0],
            HighlightEvent::HighlightEnd => color = Color::Reset,
        }
    }
    (words, color)
}

/// Append `word` to the current line, splitting on embedded newlines: each
/// `\n` flushes `inner` into `final_content` as a completed line, leaving the
/// trailing fragment in `inner`.
fn push_word_split_on_newlines(
    word: Word,
    inner: &mut Vec<Word>,
    final_content: &mut Vec<Vec<Word>>,
) {
    if !word.content().contains('\n') {
        inner.push(word);
        return;
    }
    let mut start = 0;
    for (i, c) in word.content().char_indices() {
        if c == '\n' {
            inner.push(Word::new(word.content()[start..i].to_string(), word.kind()));
            start = i + c.len_utf8();
            final_content.push(std::mem::take(inner));
        }
    }
    if start < word.content().len() {
        inner.push(Word::new(word.content()[start..].to_string(), word.kind()));
    }
}

fn process_highlighted_code(content: &str, events: Vec<HighlightEvent>) -> Vec<Vec<Word>> {
    let (words, color) = highlight_events_to_words(content, events);

    let mut final_content = Vec::new();
    let mut inner_content = Vec::new();
    for word in words {
        push_word_split_on_newlines(word, &mut inner_content, &mut final_content);
    }

    if !inner_content.is_empty() {
        final_content.push(std::mem::take(&mut inner_content));
    }
    final_content.push(vec![Word::new(String::new(), WordType::CodeBlock(color))]);
    final_content
}

fn process_mermaid_code(content: &str) -> Option<Vec<Vec<Word>>> {
    let output = render_with_width(content, Some(GENERAL_CONFIG.width as usize - 5)).ok()?;
    let mut final_content = Vec::new();

    final_content.push(vec![Word::new(String::new(), WordType::Normal)]);
    for line in output.lines() {
        final_content.push(vec![Word::new(line.to_owned(), WordType::Normal)]);
    }
    final_content.push(vec![Word::new(String::new(), WordType::Normal)]);

    Some(final_content)
}

struct ListTransformer<'a, I>
where
    I: Iterator<Item = (&'a Word, &'a Word)>,
{
    width: u16,
    zip_iter: I,
    o_list_counter_stack: Vec<usize>,
    max_stack_len: usize,
    indent: usize,
    extra_indent: usize,
    prev_indent_width: usize,
}

impl<'a, I> ListTransformer<'a, I>
where
    I: Iterator<Item = (&'a Word, &'a Word)>,
{
    fn new(width: u16, zip_iter: I) -> Self {
        Self {
            width,
            zip_iter,
            o_list_counter_stack: vec![0],
            max_stack_len: 1,
            indent: 0,
            extra_indent: 0,
            prev_indent_width: 0,
        }
    }

    fn transform(mut self, component: &mut TextComponent) {
        let mut lines = self.process_words(component);
        lines.retain(|l| l.iter().any(|c| !c.content().is_empty()));

        let corrections = self.calculate_alignment_corrections(&lines);
        self.apply_alignment_corrections(&mut lines, &corrections);

        component.height = lines.len() as u16;
        component.content = lines;
    }

    fn process_words(&mut self, component: &mut TextComponent) -> Vec<Vec<Word>> {
        let mut len = 0;
        let mut lines = Vec::new();
        let mut line = Vec::new();

        for word in component.content.iter_mut().flatten() {
            let word_len = display_width(word.content());
            if word_len + len < self.width as usize && word.kind() != WordType::ListMarker {
                len += word_len;
                line.push(word.clone());
            } else {
                let filler = self.create_filler_for_new_line(word);
                lines.push(std::mem::take(&mut line));

                let content = word.content().trim_start().to_owned();
                word.set_content(content);
                len = display_width(word.content()) + display_width(filler.content());
                line = vec![filler, word.clone()];
            }
        }
        lines.push(line);
        lines
    }

    fn create_filler_for_new_line(&mut self, word: &mut Word) -> Word {
        let filler_content = if word.kind() == WordType::ListMarker {
            self.update_indent_for_marker(word);
            " ".repeat(self.indent)
        } else {
            " ".repeat(self.indent + 2 + self.extra_indent)
        };
        Word::new(filler_content, WordType::Normal)
    }

    fn update_indent_for_marker(&mut self, word: &mut Word) {
        if let Some((meta, list_type)) = self.zip_iter.next() {
            let meta_width = display_width(meta.content());
            match self.prev_indent_width.cmp(&meta_width) {
                cmp::Ordering::Less => {
                    self.o_list_counter_stack.push(0);
                    self.max_stack_len += 1;
                }
                cmp::Ordering::Greater => {
                    self.o_list_counter_stack.pop();
                }
                cmp::Ordering::Equal => (),
            }

            if list_type.kind() == WordType::MetaInfo(MetaData::OList) {
                // Stack may be empty if the parser produced an ordered list
                // marker without a matching indent push. Fall back to "1." to
                // keep rendering instead of crashing the viewer.
                if let Some(counter) = self.o_list_counter_stack.last_mut() {
                    *counter += 1;
                    word.set_content(format!("{counter}. "));
                } else {
                    word.set_content("1. ".to_owned());
                }
                self.extra_indent = 1;
            } else {
                self.extra_indent = 0;
            }
            self.prev_indent_width = meta_width;
            self.indent = meta_width;
        } else {
            self.indent = 0;
        }
    }

    fn calculate_alignment_corrections(&self, lines: &[Vec<Word>]) -> Vec<usize> {
        let mut corrections = vec![0; self.max_stack_len];
        let mut idx: usize = 0;
        let mut current_indent_len = 0;

        for line in lines {
            if is_ordered_marker(&line[1]) {
                match current_indent_len.cmp(&display_width(line[0].content())) {
                    cmp::Ordering::Less => {
                        idx += 1;
                        current_indent_len = display_width(line[0].content());
                    }
                    cmp::Ordering::Greater => {
                        idx = idx.saturating_sub(1);
                        current_indent_len = display_width(line[0].content());
                    }
                    cmp::Ordering::Equal => (),
                }
                corrections[idx] = corrections[idx].max(display_width(line[1].content()));
            }
        }
        corrections
    }

    fn apply_alignment_corrections(&self, lines: &mut [Vec<Word>], corrections: &[usize]) {
        let mut idx: usize = 0;
        let mut current_indent_len = 0;
        let mut skip_unordered = true;

        for line in lines {
            if is_ordered_marker(&line[1]) {
                skip_unordered = false;
            }

            if line[1].content() == "• " || skip_unordered {
                skip_unordered = true;
                continue;
            }

            let amount = if is_ordered_marker(&line[1]) {
                match current_indent_len.cmp(&display_width(line[0].content())) {
                    cmp::Ordering::Less => {
                        idx += 1;
                        current_indent_len = display_width(line[0].content());
                    }
                    cmp::Ordering::Greater => {
                        idx = idx.saturating_sub(1);
                        current_indent_len = display_width(line[0].content());
                    }
                    cmp::Ordering::Equal => (),
                }
                corrections[idx].saturating_sub(display_width(line[1].content()))
                    + display_width(line[0].content())
            } else {
                (corrections[idx] + display_width(line[0].content())).saturating_sub(3)
            };
            line[0].set_content(" ".repeat(amount));
        }
    }
}

fn is_ordered_marker(word: &Word) -> bool {
    word.content()
        .strip_prefix(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
        .is_some_and(|c| c.ends_with(". "))
}

fn transform_list(component: &mut TextComponent, width: u16) {
    let indent_iter: Vec<Word> = component
        .meta_info
        .iter()
        .filter(|c| c.content().trim().is_empty())
        .cloned()
        .collect();
    let list_type_iter: Vec<Word> = component
        .meta_info
        .iter()
        .filter(|c| {
            matches!(
                c.kind(),
                WordType::MetaInfo(MetaData::OList | MetaData::UList)
            )
        })
        .cloned()
        .collect();

    let transformer = ListTransformer::new(width, indent_iter.iter().zip(list_type_iter.iter()));
    transformer.transform(component);
}

fn table_styling_width(column_count: usize) -> u16 {
    1 + column_count as u16 * (TABLE_CELL_PADDING * 2 + 1)
}

fn transform_table(component: &mut TextComponent, width: u16) {
    let width = width.saturating_sub(1);
    let column_count = component
        .meta_info
        .iter()
        .filter(|w| w.kind() == WordType::MetaInfo(MetaData::ColumnsCount))
        .count();

    if column_count == 0 || !component.content.len().is_multiple_of(column_count) {
        component.height = 1;
        component.kind = TextNode::Table(vec![], vec![]);
        return;
    }

    let row_count = component.content.len() / column_count;
    let initial_widths = calculate_initial_column_widths(&component.content, column_count);
    let styling_width = table_styling_width(column_count);
    let unbalanced_cells_width = initial_widths.iter().sum::<u16>();

    if width >= unbalanced_cells_width + styling_width {
        component.height = row_count as u16 + 3;
        component.kind = TextNode::Table(initial_widths, vec![1; row_count]);
        return;
    }

    let balanced_widths = calculate_balanced_column_widths(&initial_widths, width, styling_width);

    let mut heights = vec![1; row_count];
    for (row_i, row) in component
        .content
        .iter_mut()
        .chunks(column_count)
        .into_iter()
        .enumerate()
    {
        for (col_i, entry) in row.into_iter().enumerate() {
            let wrapped_lines = word_wrapping(
                entry.drain(..).as_ref(),
                balanced_widths[col_i] as usize,
                true,
            );

            if heights[row_i] < wrapped_lines.len() as u16 {
                heights[row_i] = wrapped_lines.len() as u16;
            }

            *entry = wrapped_lines.into_iter().flatten().collect();
        }
    }

    component.height = heights.iter().copied().sum::<u16>() + 3;
    component.kind = TextNode::Table(balanced_widths, heights);
}

fn calculate_initial_column_widths(content: &[Vec<Word>], column_count: usize) -> Vec<u16> {
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
}

fn calculate_balanced_column_widths(
    initial_widths: &[u16],
    total_width: u16,
    styling_width: u16,
) -> Vec<u16> {
    let column_count = initial_widths.len();
    let overflow_threshold = total_width.saturating_sub(styling_width) / column_count as u16;
    let mut overflowing_columns = vec![];
    let mut overflowing_width = 0;
    let mut non_overflowing_width = 0;

    for (i, &w) in initial_widths.iter().enumerate() {
        if w > overflow_threshold {
            overflowing_columns.push((i, w));
            overflowing_width += w;
        } else {
            non_overflowing_width += w;
        }
    }

    if overflowing_columns.is_empty() {
        return initial_widths.to_vec();
    }

    let mut available_balanced_width =
        total_width.saturating_sub(non_overflowing_width + styling_width);
    let mut available_overflowing_width = overflowing_width;
    let min_width = (available_balanced_width / (2 * overflowing_columns.len() as u16)).max(1);

    let mut balanced_widths = initial_widths.to_vec();
    for (column_i, old_column_width) in overflowing_columns.iter().sorted_by(|a, b| a.1.cmp(&b.1)) {
        let ratio = f32::from(*old_column_width) / f32::from(available_overflowing_width);
        let mut balanced_column_width =
            (ratio * f32::from(available_balanced_width)).floor() as u16;

        if balanced_column_width < min_width {
            balanced_column_width = min_width;
            available_overflowing_width -= *old_column_width;
            available_balanced_width =
                available_balanced_width.saturating_sub(balanced_column_width);
        }

        balanced_widths[*column_i] = balanced_column_width;
    }
    balanced_widths
}

#[must_use]
pub fn content_entry_len(words: &[Word]) -> usize {
    words.iter().map(|word| display_width(word.content())).sum()
}

pub(crate) fn byte_offset_at_width(content: &str, width: usize) -> usize {
    let mut current_width = 0;
    let mut split_index = 0;

    for (i, c) in content.char_indices() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > width {
            break;
        }
        current_width += char_width;
        split_index = i + c.len_utf8();
    }
    split_index
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render `process_highlighted_code` output as one string per line for easy
    /// assertions.
    fn lines(content: &str, events: Vec<HighlightEvent>) -> Vec<String> {
        process_highlighted_code(content, events)
            .iter()
            .map(|line| line.iter().map(Word::content).collect::<String>())
            .collect()
    }

    #[test]
    fn highlighted_code_splits_multiline_source_span_without_corruption() {
        // A single Source span covering several lines must split into one line
        // each, with no embedded newlines and nothing dropped. This was the
        // regression where every line re-included the previous content and the
        // final line was lost.
        let content = "a\nb\nc";
        let events = vec![HighlightEvent::Source {
            start: 0,
            end: content.len(),
        }];
        // Trailing entry is the blank-line sentinel.
        assert_eq!(lines(content, events), vec!["a", "b", "c", ""]);
    }

    #[test]
    fn highlighted_code_preserves_leading_whitespace_after_newline() {
        // Leading indentation following a newline inside one span must survive
        // (previously it collapsed to a single character).
        let content = "\n    x";
        let events = vec![HighlightEvent::Source {
            start: 0,
            end: content.len(),
        }];
        assert_eq!(lines(content, events), vec!["", "    x", ""]);
    }

    // --- split_by_width / byte_offset_at_width ----------------------------

    #[test]
    fn split_by_width_breaks_at_display_width() {
        assert_eq!(
            split_by_width("hello", 3),
            ("hel".to_owned(), "lo".to_owned())
        );
        // Exact fit takes the whole string.
        assert_eq!(split_by_width("hi", 2), ("hi".to_owned(), String::new()));
        // Zero width splits nothing off the head.
        assert_eq!(split_by_width("hi", 0), (String::new(), "hi".to_owned()));
        // A wide (CJK, width 2) char is not split mid-character.
        assert_eq!(
            split_by_width("世界", 3),
            ("世".to_owned(), "界".to_owned())
        );
    }

    #[test]
    fn byte_offset_and_entry_len_account_for_wide_chars() {
        // "世" is display width 2 / byte length 3.
        assert_eq!(byte_offset_at_width("世界", 2), 3);
        assert_eq!(byte_offset_at_width("世界", 1), 0);
        assert_eq!(
            content_entry_len(&[
                Word::new("ab".to_owned(), WordType::Normal),
                Word::new("世".to_owned(), WordType::Normal),
            ]),
            4
        );
    }

    // --- word_wrapping (covers split_and_wrap_long_word) ------------------

    fn wrap_text(words: &[Word], width: usize) -> Vec<String> {
        word_wrapping(words, width, false)
            .iter()
            .map(|line| line.iter().map(Word::content).collect::<String>())
            .collect()
    }

    #[test]
    fn word_wrapping_keeps_words_on_one_line_when_they_fit() {
        let words = vec![
            Word::new("hello".to_owned(), WordType::Normal),
            Word::new("world".to_owned(), WordType::Normal),
        ];
        assert_eq!(wrap_text(&words, 20), vec!["helloworld"]);
    }

    #[test]
    fn word_wrapping_wraps_at_width_boundary() {
        let words = vec![
            Word::new("hello".to_owned(), WordType::Normal),
            Word::new("world".to_owned(), WordType::Normal),
        ];
        assert_eq!(wrap_text(&words, 5), vec!["hello", "world"]);
    }

    #[test]
    fn word_wrapping_splits_a_single_overlong_word() {
        let words = vec![Word::new("abcdefghij".to_owned(), WordType::Normal)];
        let lines = wrap_text(&words, 5);
        assert_eq!(lines, vec!["abcde", "fghij"]);
        // Nothing is lost in the split.
        assert_eq!(lines.concat(), "abcdefghij");
    }

    // --- selected_heights -------------------------------------------------

    #[test]
    fn selected_heights_reports_rows_with_a_selection() {
        let comp = TextComponent::new_formatted(
            TextNode::Paragraph,
            vec![
                vec![Word::new("a".to_owned(), WordType::Normal)],
                vec![Word::new("b".to_owned(), WordType::Selected)],
                vec![Word::new("c".to_owned(), WordType::Normal)],
            ],
        );
        assert_eq!(comp.selected_heights(), vec![1]);
    }

    #[test]
    fn selected_heights_is_empty_when_hidden() {
        let mut comp = TextComponent::new_formatted(
            TextNode::Paragraph,
            vec![vec![Word::new("b".to_owned(), WordType::Selected)]],
        );
        comp.set_hidden(true);
        assert!(comp.selected_heights().is_empty());
    }

    // --- calculate_balanced_column_widths ---------------------------------

    #[test]
    fn balanced_widths_passthrough_when_no_column_overflows() {
        // threshold = 100/2 = 50; neither column exceeds it.
        assert_eq!(
            calculate_balanced_column_widths(&[2, 3], 100, 0),
            vec![2, 3]
        );
    }

    #[test]
    fn balanced_widths_shrink_the_overflowing_column() {
        // threshold = 50/2 = 25; col1 (90) overflows, col0 (10) does not.
        // available_balanced = 50 - 10 = 40; ratio 1.0 -> col1 becomes 40.
        assert_eq!(
            calculate_balanced_column_widths(&[10, 90], 50, 0),
            vec![10, 40]
        );
    }

    // --- list / table transform via the parser (ListTransformer, tables) --

    fn rendered_text(md: &str) -> String {
        crate::parser::parse_markdown(None, md, 80)
            .components()
            .iter()
            .flat_map(|c| c.content_as_lines())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn list_transform_renders_all_items() {
        let text = rendered_text("- alpha\n- beta\n");
        assert!(text.contains("alpha"), "got: {text:?}");
        assert!(text.contains("beta"), "got: {text:?}");
    }

    #[test]
    fn table_transform_renders_all_cells() {
        let text = rendered_text("| head | col |\n|------|-----|\n| one | two |\n");
        for cell in ["head", "col", "one", "two"] {
            assert!(text.contains(cell), "missing {cell} in: {text:?}");
        }
    }
}
