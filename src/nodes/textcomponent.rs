use std::cmp;

use itertools::Itertools;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Cell, List, ListItem, Paragraph, Row, Table, Widget},
};
use tree_sitter_highlight::HighlightEvent;

use crate::{
    config::{
        colors::{COLOR_CONFIG, HEADER_COLOR},
        general::GENERAL_CONFIG,
    },
    highlight::{highlight_code, HighlightInfo, COLOR_MAP},
    nodes::word::MetaData, // config::0usize
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
                let height = (self.content.len() / cmp::max(self.meta_info().len(), 1)) as u16;
                self.height = height;
            }
            TextNode::HorizontalSeperator => self.height = 1,
            TextNode::Image => unreachable!("Image should not be transformed"),
        }
    }
}

enum Clipping {
    Both,
    Upper,
    Lower,
    None,
}

fn clips_upper_bound(_area: Rect, component: &TextComponent) -> bool {
    component.scroll_offset() > component.y_offset()
}

fn clips_lower_bound(area: Rect, component: &TextComponent) -> bool {
    (component.y_offset() + component.height()).saturating_sub(component.scroll_offset())
        > area.height
}

impl Widget for TextComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let kind = self.kind();

        let y = self.y_offset().saturating_sub(self.scroll_offset());

        let clips = if clips_upper_bound(area, &self) && clips_lower_bound(area, &self) {
            Clipping::Both
        } else if clips_upper_bound(area, &self) {
            Clipping::Upper
        } else if clips_lower_bound(area, &self) {
            Clipping::Lower
        } else {
            Clipping::None
        };

        let height = match clips {
            Clipping::Both => {
                let new_y = self.y_offset().saturating_sub(self.scroll_offset());
                let new_height = new_y;
                cmp::min(self.height(), area.height.saturating_sub(new_height))
            }

            Clipping::Upper => cmp::min(
                self.height(),
                (self.height() + self.y_offset()).saturating_sub(self.scroll_offset()),
            ),
            Clipping::Lower => {
                let new_y = self.y_offset() - self.scroll_offset();
                let new_height = new_y;
                cmp::min(self.height(), area.height.saturating_sub(new_height))
            }
            Clipping::None => self.height(),
        };

        let meta_info = self
            .meta_info()
            .to_owned()
            .first()
            .cloned()
            .unwrap_or_else(|| Word::new("".to_string(), WordType::Normal));

        let table_meta = self.meta_info().to_owned();

        let area = Rect { height, y, ..area };

        match kind {
            TextNode::Paragraph => render_paragraph(area, buf, self, clips),
            TextNode::Heading => render_heading(area, buf, self),
            TextNode::Task => render_task(area, buf, self, clips, &meta_info),
            TextNode::List => render_list(area, buf, self, clips),
            TextNode::CodeBlock => render_code_block(area, buf, self, clips),
            TextNode::Table => render_table(area, buf, self, clips, table_meta),
            TextNode::Quote => render_quote(area, buf, self, clips),
            TextNode::LineBreak => (),
            TextNode::HorizontalSeperator => render_horizontal_seperator(area, buf),
            TextNode::Image => todo!(),
        }
    }
}

fn render_heading(area: Rect, buf: &mut Buffer, component: TextComponent) {
    let indent = if let Some(meta) = component.meta_info().first() {
        match meta.kind() {
            WordType::MetaInfo(MetaData::HeadingLevel(e)) => e,
            _ => 1,
        }
    } else {
        1
    };

    let content: Vec<Span<'_>> = component
        .content()
        .iter()
        .flatten()
        .map(|c| style_heading(c, indent))
        .collect();

    let paragraph = match indent {
        1 => Paragraph::new(Line::from(content))
            .block(Block::default().style(Style::default().bg(COLOR_CONFIG.heading_bg_color)))
            .alignment(Alignment::Center),
        _ => Paragraph::new(Line::from(content)),
    };

    paragraph.render(area, buf);
}

fn render_paragraph(area: Rect, buf: &mut Buffer, component: TextComponent, clip: Clipping) {
    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());
    let mut content = component.content_owned();
    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(style_word).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, component: TextComponent, clip: Clipping) {
    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());
    let mut content = component.content_owned();
    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };
    let content: Vec<ListItem<'_>> = content
        .iter()
        .map(|c| -> ListItem<'_> {
            ListItem::new(Line::from(c.iter().map(style_word).collect::<Vec<_>>()))
        })
        .collect();

    let list = List::new(content);
    list.render(area, buf);
}

fn render_code_block(area: Rect, buf: &mut Buffer, component: TextComponent, clip: Clipping) {
    let mut content = component
        .content()
        .iter()
        .map(|c| Line::from(c.iter().map(style_word).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    match clip {
        Clipping::Both => {
            let top = component.scroll_offset() - component.y_offset();
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            // panic!("offset: {}, height: {}, len: {}", offset, height, len);
            content.drain(0..offset);
        }
        Clipping::Lower => {
            content.drain(area.height as usize..);
        }
        Clipping::None => (),
    }

    let block = Block::default().style(Style::default().bg(COLOR_CONFIG.code_block_bg_color));

    block.render(area, buf);

    let area = Rect {
        x: area.x + 1,
        width: area.width - 1,
        ..area
    };

    let paragraph = Paragraph::new(content);

    paragraph.render(area, buf);
}

fn render_table(
    area: Rect,
    buf: &mut Buffer,
    component: TextComponent,
    clip: Clipping,
    meta_info: Vec<Word>,
) {
    let column_count = meta_info.len();

    let content = component.content();

    let titles = content.chunks(column_count).next().unwrap().to_vec();

    let mut widths = vec![0; column_count];

    content.chunks(column_count).for_each(|c| {
        c.iter().enumerate().for_each(|(i, c)| {
            let len = c.iter().map(|c| c.content().len()).sum::<usize>() + 1;
            if len > widths[i] as usize {
                widths[i] = len as u16;
            }
        });
    });

    let widths = widths
        .into_iter()
        .map(Constraint::Length)
        .collect::<Vec<_>>();

    let moved_content = content.chunks(column_count).skip(1).collect::<Vec<_>>();

    let header = Row::new(
        titles
            .iter()
            .map(|c| Cell::from(Line::from(c.iter().map(style_word).collect::<Vec<_>>())))
            .collect::<Vec<_>>(),
    );

    let mut rows = moved_content
        .iter()
        .map(|c| {
            Row::new(
                c.iter()
                    .map(|i| Cell::from(Line::from(i.iter().map(style_word).collect::<Vec<_>>())))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    match clip {
        Clipping::Both => {
            let top = component.scroll_offset() - component.y_offset();
            rows.drain(0..top as usize);
            rows.drain(area.height as usize..);
        }
        Clipping::Upper => {
            let len = rows.len();
            let height = area.height as usize;
            let offset = len.saturating_sub(height) + 1;
            // panic!("offset: {}, height: {}, len: {}", offset, height, len);
            if offset < len {
                rows.drain(0..offset);
            }
        }
        Clipping::Lower => {
            let drain_area = cmp::min(area.height, rows.len() as u16);
            rows.drain(drain_area as usize..);
        }
        Clipping::None => (),
    }

    let table = Table::new(rows, widths)
        .header(
            header.style(
                Style::default()
                    .fg(COLOR_CONFIG.table_header_fg_color)
                    .bg(COLOR_CONFIG.table_header_bg_color),
            ),
        )
        .block(Block::default())
        .column_spacing(1);

    table.render(area, buf);
}

fn render_task(
    area: Rect,
    buf: &mut Buffer,
    component: TextComponent,
    clip: Clipping,
    meta_info: &Word,
) {
    const CHECKBOX: &str = "✅ ";
    const UNCHECKED: &str = "❌ ";

    let checkbox = if meta_info.content() == "- [ ] " {
        UNCHECKED
    } else {
        CHECKBOX
    };

    let paragraph = Paragraph::new(checkbox);

    paragraph.render(area, buf);

    let area = Rect {
        x: area.x + 4,
        width: area.width - 4,
        ..area
    };

    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());

    let mut content = component.content_owned();

    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(style_word).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_quote(area: Rect, buf: &mut Buffer, component: TextComponent, clip: Clipping) {
    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());

    let meta = component.meta_info().to_owned();

    let mut content = component.content_owned();
    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(style_word).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let bar_color = if let Some(meta) = meta.first() {
        meta.content()
            .split_whitespace()
            .next()
            .map(|c| c.to_lowercase())
            .map(|c| match c.as_str() {
                "[!tip]" => COLOR_CONFIG.quote_tip,
                "[!warning]" => COLOR_CONFIG.quote_warning,
                "[!caution]" => COLOR_CONFIG.quote_caution,
                "[!important]" => COLOR_CONFIG.quote_important,
                "[!note]" => COLOR_CONFIG.quote_note,
                _ => COLOR_CONFIG.quote_default,
            })
            .unwrap_or(COLOR_CONFIG.quote_bg_color)
    } else {
        Color::White
    };
    let vertical_marker = Span::styled("\u{2588}", Style::default().fg(bar_color));

    let marker_paragraph = Paragraph::new(vec![Line::from(vertical_marker); content.len()])
        .bg(COLOR_CONFIG.quote_bg_color);
    marker_paragraph.render(area, buf);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().style(Style::default().bg(COLOR_CONFIG.quote_bg_color)));

    let area = Rect {
        x: area.x + 1,
        width: cmp::min(area.width, GENERAL_CONFIG.width) - 1,
        ..area
    };

    paragraph.render(area, buf);
}

fn render_horizontal_seperator(area: Rect, buf: &mut Buffer) {
    let paragraph = Paragraph::new(Line::from(vec![Span::raw(
        "\u{2014}".repeat(GENERAL_CONFIG.width.into()),
    )]));

    paragraph.render(area, buf);
}

fn style_heading(word: &Word, indent: u8) -> Span<'_> {
    match indent {
        1 => Span::styled(
            word.content(),
            Style::default().fg(COLOR_CONFIG.heading_fg_color),
        ),
        2 => Span::styled(word.content(), Style::default().fg(HEADER_COLOR.level_2)),
        3 => Span::styled(word.content(), Style::default().fg(HEADER_COLOR.level_3)),
        4 => Span::styled(word.content(), Style::default().fg(HEADER_COLOR.level_4)),
        5 => Span::styled(word.content(), Style::default().fg(HEADER_COLOR.level_5)),
        6 => Span::styled(word.content(), Style::default().fg(HEADER_COLOR.level_6)),
        _ => Span::styled(
            word.content(),
            Style::default().fg(COLOR_CONFIG.heading_fg_color),
        ),
    }
}

fn style_word(word: &Word) -> Span<'_> {
    match word.kind() {
        WordType::MetaInfo(_) | WordType::LinkData => unreachable!(),
        WordType::Selected => Span::styled(
            word.content(),
            Style::default()
                .fg(COLOR_CONFIG.link_selected_fg_color)
                .bg(COLOR_CONFIG.link_selected_bg_color),
        ),
        WordType::Normal => Span::raw(word.content()),
        WordType::Code => Span::styled(
            word.content(),
            Style::default().fg(COLOR_CONFIG.code_fg_color),
        )
        .bg(COLOR_CONFIG.code_bg_color),
        WordType::Link => {
            Span::styled(word.content(), Style::default().fg(COLOR_CONFIG.link_color))
        }
        WordType::Italic => Span::styled(
            word.content(),
            Style::default().fg(COLOR_CONFIG.italic_color).italic(),
        ),
        WordType::Bold => Span::styled(
            word.content(),
            Style::default().fg(COLOR_CONFIG.bold_color).bold(),
        ),
        WordType::Strikethrough => Span::styled(
            word.content(),
            Style::default()
                .fg(COLOR_CONFIG.striketrough_color)
                .add_modifier(Modifier::CROSSED_OUT),
        ),
        WordType::White => Span::styled(word.content(), Style::default().fg(Color::White)),
        WordType::ListMarker => Span::styled(word.content(), Style::default().fg(Color::White)),
        WordType::BoldItalic => Span::styled(
            word.content(),
            Style::default()
                .fg(COLOR_CONFIG.bold_italic_color)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC),
        ),
        WordType::CodeBlock(e) => Span::styled(word.content(), e),
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

    for line in lines.iter_mut() {
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
