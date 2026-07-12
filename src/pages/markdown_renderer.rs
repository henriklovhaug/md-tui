use std::borrow::Cow;
use std::cmp;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph, Widget},
};

use crate::{
    nodes::{
        textcomponent::{
            Clipping, TABLE_CELL_PADDING, TextComponent, TextNode, content_entry_len, word_wrapping,
        },
        word::{MetaData, Word, WordType},
    },
    util::{
        colors::{color_config, heading_colors},
        general::{Centering, GENERAL_CONFIG},
    },
};

/// Layout rectangle the markdown body is drawn into. Shared between the
/// renderer and any input handler that needs to map screen coords back to a
/// document position (e.g. mouse clicks).
#[must_use = "constructing the area without using it is almost certainly a bug"]
pub fn markdown_view_area(term_width: u16, term_height: u16, app_width: u16) -> Rect {
    let footer_h: u16 = u16::from(GENERAL_CONFIG.footer);
    let x = match GENERAL_CONFIG.centering {
        Centering::Left => 2,
        Centering::Center => {
            let x = (term_width / 2).saturating_sub(GENERAL_CONFIG.width / 2);
            if x > 2 { x } else { 2 }
        }
        Centering::Right => {
            let x = term_width.saturating_sub(GENERAL_CONFIG.width + 2);
            if x > 2 { x } else { 2 }
        }
    };
    let reserved_bottom = if GENERAL_CONFIG.help_menu { 5 } else { 0 } + footer_h;
    Rect {
        x,
        y: 0,
        width: cmp::min(app_width.saturating_sub(3), term_width.saturating_sub(1)),
        height: term_height.saturating_sub(reserved_bottom),
    }
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
            .unwrap_or_else(|| Word::new(String::new(), WordType::Normal));

        let area = Rect { height, y, ..area };

        match kind {
            TextNode::Paragraph => render_paragraph(area, buf, self, clips),
            TextNode::Heading => render_heading(area, buf, self),
            TextNode::Task => render_task(area, buf, self, clips, &meta_info),
            TextNode::List => render_list(area, buf, self, clips),
            TextNode::CodeBlock => render_code_block(area, buf, self, clips),
            TextNode::Table(widths, heights) => {
                render_table(area, buf, self, clips, widths, heights);
            }
            TextNode::Quote => render_quote(area, buf, self, clips),
            TextNode::LineBreak => (),
            TextNode::HorizontalSeparator => render_horizontal_separator(area, buf),
            TextNode::Image => render_paragraph(area, buf, self, clips),
            TextNode::Footnote => (),
            TextNode::DetailsSummary { folded, .. } => {
                render_details_summary(area, buf, self, folded);
            }
        }
    }
}

fn style_word_content<'a>(word: &Word, content: impl Into<Cow<'a, str>>) -> Span<'a> {
    match word.kind() {
        WordType::MetaInfo(_) | WordType::LinkData | WordType::FootnoteData => unreachable!(),
        WordType::Selected => Span::styled(
            content,
            Style::default()
                .fg(color_config().link_selected_fg_color)
                .bg(color_config().link_selected_bg_color),
        ),
        WordType::Normal => Span::raw(content),
        WordType::Code => Span::styled(content, Style::default().fg(color_config().code_fg_color))
            .bg(color_config().code_bg_color),
        WordType::Link | WordType::FootnoteInline => {
            Span::styled(content, Style::default().fg(color_config().link_color))
        }
        WordType::Italic => Span::styled(
            content,
            Style::default().fg(color_config().italic_color).italic(),
        ),
        WordType::Bold => Span::styled(
            content,
            Style::default().fg(color_config().bold_color).bold(),
        ),
        WordType::Strikethrough | WordType::Footnote => Span::styled(
            content,
            Style::default()
                .fg(color_config().striketrough_color)
                .add_modifier(Modifier::CROSSED_OUT),
        ),
        WordType::White => Span::styled(content, Style::default().fg(Color::White)),
        WordType::ListMarker => Span::styled(content, Style::default().fg(Color::White)),
        WordType::BoldItalic => Span::styled(
            content,
            Style::default()
                .fg(color_config().bold_italic_color)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC),
        ),
        WordType::CodeBlock(e) => Span::styled(content, e),
    }
}

fn style_word_owned(word: &Word) -> Span<'static> {
    style_word_content(word, word.content().to_owned())
}

fn wrap_table_cell(entry: &[Word], width: u16) -> Vec<Vec<Word>> {
    if width == 0 {
        return vec![Vec::new()];
    }

    word_wrapping(entry.iter(), width as usize, true)
}

fn table_border_line(
    widths: &[u16],
    left: &'static str,
    middle: &'static str,
    right: &'static str,
) -> Line<'static> {
    let mut spans = vec![Span::raw(left)];

    for (column_i, width) in widths.iter().enumerate() {
        spans.push(Span::raw(
            "─".repeat((width + TABLE_CELL_PADDING * 2) as usize),
        ));
        spans.push(Span::raw(if column_i + 1 == widths.len() {
            right
        } else {
            middle
        }));
    }

    Line::from(spans)
}

fn build_table_row_lines(
    row: &[Vec<Word>],
    widths: &[u16],
    row_height: u16,
    row_style: Option<Style>,
) -> Vec<Line<'static>> {
    let wrapped_cells = row
        .iter()
        .zip(widths.iter())
        .map(|(entry, width)| wrap_table_cell(entry, *width))
        .collect::<Vec<_>>();

    (0..row_height as usize)
        .map(|line_i| {
            let mut spans = vec![Span::raw("│")];

            for (column_i, cell_lines) in wrapped_cells.iter().enumerate() {
                spans.push(Span::raw(" ".repeat(TABLE_CELL_PADDING as usize)));

                if let Some(words) = cell_lines.get(line_i) {
                    spans.extend(words.iter().map(style_word_owned));

                    let padding = widths[column_i] as usize - content_entry_len(words);
                    if padding > 0 {
                        spans.push(Span::raw(" ".repeat(padding)));
                    }
                } else {
                    spans.push(Span::raw(" ".repeat(widths[column_i] as usize)));
                }

                spans.push(Span::raw(" ".repeat(TABLE_CELL_PADDING as usize)));
                spans.push(Span::raw("│"));
            }

            let line = Line::from(spans);
            if let Some(style) = row_style {
                line.patch_style(style)
            } else {
                line
            }
        })
        .collect()
}

fn build_table_lines(content: &[Vec<Word>], widths: &[u16], heights: &[u16]) -> Vec<Line<'static>> {
    let column_count = widths.len();
    let header_style = Style::default()
        .fg(color_config().table_header_fg_color)
        .bg(color_config().table_header_bg_color);

    let mut lines = vec![table_border_line(widths, "╭", "┬", "╮")];

    for (row_i, row) in content.chunks(column_count).enumerate() {
        let row_style = (row_i == 0).then_some(header_style);
        lines.extend(build_table_row_lines(
            row,
            widths,
            heights[row_i],
            row_style,
        ));

        if row_i == 0 {
            lines.push(table_border_line(widths, "├", "┼", "┤"));
        }
    }

    lines.push(table_border_line(widths, "╰", "┴", "╯"));
    lines
}

fn render_quote(area: Rect, buf: &mut Buffer, component: TextComponent, _clip: Clipping) {
    let meta = component.meta_info().to_owned();
    let (content, _) = component.clip_content(area.height);

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(style_word_owned).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let bar_color = if let Some(meta) = meta.first() {
        meta.content()
            .split_whitespace()
            .next()
            .map(str::to_lowercase)
            .map_or(color_config().quote_bg_color, |c| match c.as_str() {
                "[!tip]" => color_config().quote_tip,
                "[!warning]" => color_config().quote_warning,
                "[!caution]" => color_config().quote_caution,
                "[!important]" => color_config().quote_important,
                "[!note]" => color_config().quote_note,
                _ => color_config().quote_default,
            })
    } else {
        Color::White
    };
    let vertical_marker = Span::styled("\u{2588}", Style::default().fg(bar_color));

    let marker_paragraph = Paragraph::new(vec![Line::from(vertical_marker); content.len()])
        .bg(color_config().quote_bg_color);
    marker_paragraph.render(area, buf);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().style(Style::default().bg(color_config().quote_bg_color)));

    let area = Rect {
        x: area.x + 1,
        width: cmp::min(area.width, GENERAL_CONFIG.width).saturating_sub(1),
        ..area
    };

    paragraph.render(area, buf);
}

fn style_heading(word: &Word, indent: u8) -> Span<'_> {
    if word.kind() == WordType::Code {
        return style_word_content(word, word.content());
    }
    match indent {
        1 => Span::styled(
            word.content(),
            Style::default().fg(color_config().heading_fg_color),
        ),
        2 => Span::styled(
            word.content(),
            Style::default().fg(heading_colors().level_2),
        ),
        3 => Span::styled(
            word.content(),
            Style::default().fg(heading_colors().level_3),
        ),
        4 => Span::styled(
            word.content(),
            Style::default().fg(heading_colors().level_4),
        ),
        5 => Span::styled(
            word.content(),
            Style::default().fg(heading_colors().level_5),
        ),
        6 => Span::styled(
            word.content(),
            Style::default().fg(heading_colors().level_6),
        ),
        _ => Span::styled(
            word.content(),
            Style::default().fg(color_config().heading_fg_color),
        ),
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
            .block(Block::default().style(Style::default().bg(color_config().heading_bg_color)))
            .alignment(Alignment::Center),
        _ => Paragraph::new(Line::from(content)),
    };

    paragraph.render(area, buf);
}

fn render_paragraph(area: Rect, buf: &mut Buffer, component: TextComponent, _clip: Clipping) {
    let (content, _) = component.clip_content(area.height);

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(style_word_owned).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_details_summary(area: Rect, buf: &mut Buffer, component: TextComponent, folded: bool) {
    let focused = component.is_focused();
    let mut style = Style::default().add_modifier(Modifier::BOLD);
    if focused {
        style = style
            .fg(color_config().link_selected_fg_color)
            .bg(color_config().link_selected_bg_color);
    }
    let marker = if folded { "▶ " } else { "▼ " };
    let mut spans: Vec<Span> = vec![Span::styled(marker, style)];
    for word in component.content_owned().into_iter().flatten() {
        spans.push(Span::styled(word.content().to_string(), style));
    }
    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, component: TextComponent, _clip: Clipping) {
    let (content, _) = component.clip_content(area.height);
    let content: Vec<ListItem<'_>> = content
        .iter()
        .map(|c| -> ListItem<'_> {
            ListItem::new(Line::from(
                c.iter().map(style_word_owned).collect::<Vec<_>>(),
            ))
        })
        .collect();

    let list = List::new(content);
    list.render(area, buf);
}

fn render_code_block(area: Rect, buf: &mut Buffer, component: TextComponent, clip: Clipping) {
    let lines = component
        .content()
        .iter()
        .map(|c| Line::from(c.iter().map(style_word_owned).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let content = clip_lines(lines, area.height, &component, clip);

    let max_width = cmp::max(
        component
            .meta_info()
            .iter()
            .find_map(|f| match f.kind() {
                WordType::MetaInfo(MetaData::LineLength(len)) => Some(len),
                _ => None,
            })
            .unwrap_or(area.width)
            + 2,
        area.width,
    );

    let block = Block::default().style(Style::default().bg(color_config().code_block_bg_color));

    let area = Rect {
        width: max_width,
        ..area
    };

    block.render(area, buf);

    let area = if let Some(word) = component.meta_info().first()
        && matches!(word.content(), "mermaid")
    {
        Rect {
            x: area.x + 1,
            width: buf.area().width,
            ..area
        }
    } else {
        Rect {
            x: area.x + 1,
            width: area.width - 1,
            ..area
        }
    };

    let paragraph = Paragraph::new(content);

    paragraph.render(area, buf);
}

fn clip_lines(
    mut lines: Vec<Line<'static>>,
    area_height: u16,
    component: &TextComponent,
    clip: Clipping,
) -> Vec<Line<'static>> {
    match clip {
        Clipping::Both => {
            let top = component
                .scroll_offset()
                .saturating_sub(component.y_offset());
            lines.drain(0..top as usize);
            lines.drain(area_height as usize..);
        }
        Clipping::Upper => {
            let offset = lines.len().saturating_sub(area_height as usize);
            lines.drain(0..offset);
        }
        Clipping::Lower => {
            lines.drain(area_height as usize..);
        }
        Clipping::None => (),
    }
    lines
}

fn render_table(
    area: Rect,
    buf: &mut Buffer,
    component: TextComponent,
    clip: Clipping,
    widths: Vec<u16>,
    heights: Vec<u16>,
) {
    let column_count = widths.len();

    if column_count == 0 {
        Paragraph::new(Line::from("Malformed table").fg(Color::Red)).render(area, buf);
        return;
    }

    let lines = build_table_lines(component.content(), &widths, &heights);
    let lines = clip_lines(lines, area.height, &component, clip);

    Paragraph::new(lines).render(area, buf);
}

fn render_task(
    area: Rect,
    buf: &mut Buffer,
    component: TextComponent,
    _clip: Clipping,
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

    let (content, _) = component.clip_content(area.height);

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(style_word_owned).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_horizontal_separator(area: Rect, buf: &mut Buffer) {
    // `GENERAL_CONFIG.width` is `u16::MAX` when the user configured "full
    // terminal width" (0), so clamp to the actual draw area to avoid allocating
    // a 65k-char string for every horizontal rule on every render.
    let width = cmp::min(area.width, GENERAL_CONFIG.width) as usize;
    let paragraph = Paragraph::new(Line::from(vec![Span::raw("\u{2014}".repeat(width))]));

    paragraph.render(area, buf);
}
