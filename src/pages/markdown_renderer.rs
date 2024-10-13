use std::cmp;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Cell, List, ListItem, Paragraph, Row, Table, Widget},
};

use crate::{
    nodes::{
        textcomponent::{TextComponent, TextNode},
        word::{MetaData, Word, WordType},
    },
    util::{
        colors::{COLOR_CONFIG, HEADER_COLOR},
        general::GENERAL_CONFIG,
    },
};

fn clips_upper_bound(_area: Rect, component: &TextComponent) -> bool {
    component.scroll_offset() > component.y_offset()
}

fn clips_lower_bound(area: Rect, component: &TextComponent) -> bool {
    (component.y_offset() + component.height()).saturating_sub(component.scroll_offset())
        > area.height
}

enum Clipping {
    Both,
    Upper,
    Lower,
    None,
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

        let area = Rect { height, y, ..area };

        match kind {
            TextNode::Paragraph => render_paragraph(area, buf, self, clips),
            TextNode::Heading => render_heading(area, buf, self),
            TextNode::Task => render_task(area, buf, self, clips, &meta_info),
            TextNode::List => render_list(area, buf, self, clips),
            TextNode::CodeBlock => render_code_block(area, buf, self, clips),
            TextNode::Table(widths, heights) => {
                render_table(area, buf, self, clips, widths, heights)
            }
            TextNode::Quote => render_quote(area, buf, self, clips),
            TextNode::LineBreak => (),
            TextNode::HorizontalSeperator => render_horizontal_seperator(area, buf),
            TextNode::Image => todo!(),
        }
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
    widths: Vec<u16>,
    heights: Vec<u16>,
) {
    let scroll_offset = component.scroll_offset();
    let y_offset = component.y_offset();
    let height = component.height();

    let column_count = widths.len();

    let content = component.content_owned();
    let titles = content.chunks(column_count).next().unwrap().to_vec();
    let moved_content = content.chunks(column_count).skip(1).collect::<Vec<_>>();

    let header = Row::new(
        titles
            .iter()
            .enumerate()
            .map(|(column_i, entry)| {
                let mut line = vec![];
                let mut lines = vec![];
                let mut line_len = 0;
                for word in entry.iter() {
                    let word_len = word.content().len() as u16;
                    line_len += word_len;
                    if line_len <= widths[column_i] {
                        line.push(word);
                    } else {
                        lines.push(Line::from(
                            line.into_iter().map(style_word).collect::<Vec<_>>(),
                        ));
                        line = vec![word];
                        line_len -= widths[column_i]
                    }
                }

                lines.push(Line::from(
                    line.into_iter().map(style_word).collect::<Vec<_>>(),
                ));

                Cell::from(lines)
            })
            .collect::<Vec<_>>(),
    )
    .height(heights[0]);

    let (start_i, stop_i) = match clip {
        Clipping::Both => {
            let top = scroll_offset - y_offset;
            (top, top + area.height)
        }
        Clipping::Upper => {
            let offset = height.saturating_sub(area.height);
            (offset, height)
        }
        Clipping::Lower => (0, height.min(area.height)),
        Clipping::None => (0, height),
    };

    let (start_i, stop_i) = (
        start_i as usize,
        (stop_i).saturating_sub(heights[0]) as usize,
    );
    let mut line_i = 0;
    let rows = moved_content
        .iter()
        .enumerate()
        .filter_map(|(row_i, c)| {
            if (line_i + heights[row_i + 1] as usize) <= start_i {
                line_i += heights[row_i + 1] as usize;
                return None;
            } else if stop_i <= line_i {
                return None;
            }

            let start_cell_line_i = start_i.saturating_sub(line_i);
            let stop_cell_line_i = stop_i.saturating_sub(line_i);
            let n_cell_lines = (heights[row_i + 1] as usize)
                .min(stop_cell_line_i)
                .saturating_sub(start_cell_line_i);

            line_i += heights[row_i + 1] as usize;

            Some(
                Row::new(
                    c.iter()
                        .enumerate()
                        .map(|(column_i, entry)| {
                            let mut acc = vec![];
                            let mut lines = vec![];
                            let mut line_len = 0;
                            for word in entry.iter() {
                                let word_len = word.content().len() as u16;
                                line_len += word_len;
                                if line_len <= widths[column_i] {
                                    acc.push(word);
                                } else {
                                    lines.push(Line::from(
                                        acc.into_iter().map(style_word).collect::<Vec<_>>(),
                                    ));
                                    line_len = word_len;
                                    acc = vec![word];
                                }
                            }

                            lines.push(Line::from(
                                acc.into_iter().map(style_word).collect::<Vec<_>>(),
                            ));

                            lines.append(&mut vec![
                                Line::from("");
                                (start_cell_line_i + n_cell_lines)
                                    .saturating_sub(lines.len())
                            ]);

                            Cell::from(
                                lines
                                    .splice(
                                        start_cell_line_i..(start_cell_line_i + n_cell_lines),
                                        vec![],
                                    )
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
                .height(n_cell_lines as u16),
            )
        })
        .collect::<Vec<_>>();

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

fn render_horizontal_seperator(area: Rect, buf: &mut Buffer) {
    let paragraph = Paragraph::new(Line::from(vec![Span::raw(
        "\u{2014}".repeat(GENERAL_CONFIG.width.into()),
    )]));

    paragraph.render(area, buf);
}
