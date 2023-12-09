use std::cmp;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Cell, List, ListItem, Paragraph, Row, Table, Widget, Wrap},
};

use crate::nodes::{RenderComponent, RenderNode, Word, WordType};

fn clips_upper_bound(_area: Rect, component: &RenderComponent) -> bool {
    component.scroll_offset() > component.y_offset()
}

fn clips_lower_bound(area: Rect, component: &RenderComponent) -> bool {
    component
        .y_offset()
        .saturating_sub(component.scroll_offset())
        > area.height
        || component.y_offset() + component.height() > area.height
}

enum Clipping {
    Upper,
    Lower,
    None,
}

impl Widget for RenderComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.y_offset().saturating_sub(self.scroll_offset()) > area.height
            || self.scroll_offset() > self.y_offset() + self.height()
        {
            return;
        }
        let kind = self.kind();

        let y = self.y_offset().saturating_sub(self.scroll_offset());

        let clips = if clips_upper_bound(area, &self) {
            Clipping::Upper
        } else if clips_lower_bound(area, &self) {
            Clipping::Lower
        } else {
            Clipping::None
        };

        let height = match clips {
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

        let area = Rect { height, y, ..area };

        match kind {
            RenderNode::Paragraph => render_paragraph(area, buf, self.content_owned(), clips),
            RenderNode::Heading => render_heading(area, buf, self.content_owned()),
            RenderNode::Task => render_task(area, buf, self.content_owned(), clips),
            RenderNode::List => render_list(area, buf, self.content_owned(), clips),
            RenderNode::CodeBlock => render_code_block(area, buf, self.content_owned(), clips),
            RenderNode::LineBreak => (),
            RenderNode::Table => render_table(area, buf, self.content_owned(), clips),
            RenderNode::Quote => todo!(),
        }
    }
}

fn style_word(word: &Word) -> Span<'_> {
    match word.kind() {
        WordType::MetaInfo => unreachable!(),
        WordType::Normal => Span::raw(word.content()),
        WordType::Code => Span::styled(word.content(), Style::default().fg(Color::Red)),
        WordType::Link => todo!(),
        WordType::Italic => {
            Span::styled(word.content(), Style::default().fg(Color::Green).italic())
        }
        WordType::Bold => Span::styled(word.content(), Style::default().fg(Color::Green).bold()),
        WordType::Strikethrough => Span::styled(
            word.content(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::CROSSED_OUT),
        ),
    }
}

fn render_heading(area: Rect, buf: &mut Buffer, content: Vec<Vec<Word>>) {
    let content = content
        .first()
        .unwrap()
        .iter()
        .map(|c| Line::styled(c.content(), Style::default().fg(Color::Black)))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(content)
        .block(Block::default().style(Style::default().bg(Color::Blue)))
        .alignment(Alignment::Center);

    paragraph.render(area, buf);
}

fn render_paragraph(area: Rect, buf: &mut Buffer, content: Vec<Vec<Word>>, clip: Clipping) {
    let content = match clip {
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
        .map(|c| {
            Line::from(
                c.iter()
                    .filter(|i| i.kind() != WordType::MetaInfo)
                    .map(|i| style_word(i))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, content: Vec<Vec<Word>>, clip: Clipping) {
    let content = match clip {
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
            ListItem::new(Line::from(
                c.iter().map(|i| style_word(i)).collect::<Vec<_>>(),
            ))
        })
        .collect();

    let list = List::new(content)
        .highlight_symbol("*")
        .repeat_highlight_symbol(true);
    list.render(area, buf);
}

fn render_code_block(area: Rect, buf: &mut Buffer, content: Vec<Vec<Word>>, clip: Clipping) {
    let mut content = content
        .iter()
        .filter(|c| c.iter().any(|i| i.kind() != WordType::MetaInfo))
        .map(|c| {
            Line::from(
                c.iter()
                    .map(|i| Span::styled(i.content(), Style::default().fg(Color::Red)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    match clip {
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            content.drain(0..offset);
        }
        Clipping::Lower => {
            content.drain(area.height as usize..);
        }
        Clipping::None => (),
    }

    let area = Rect {
        x: area.x + 1,
        width: area.width - 2,
        ..area
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().style(Style::default().bg(Color::Blue)))
        .wrap(Wrap { trim: false });

    paragraph.render(area, buf);
}

fn render_table(area: Rect, buf: &mut Buffer, content: Vec<Vec<Word>>, clip: Clipping) {
    let titles = content.first().unwrap();

    let widths = titles
        .iter()
        .map(|c| Constraint::Length(c.content().len() as u16))
        .collect::<Vec<_>>();

    let moved_content = content.to_owned();
    let header = Row::new(
        titles
            .iter()
            .map(|c| Cell::from(c.content()).style(Style::default().fg(Color::Red)))
            .collect::<Vec<_>>(),
    );

    let mut rows = moved_content
        .iter()
        .map(|c| Row::new(c.iter().map(|i| style_word(i)).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    match clip {
        Clipping::Upper => {
            let len = rows.len();
            let height = area.height;
            let offset = len - height as usize;
            rows.drain(0..offset);
        }
        Clipping::Lower => {
            rows.drain(area.height as usize..);
        }
        Clipping::None => (),
    }

    let table = Table::new(rows)
        .header(header.style(Style::default().fg(Color::Black)))
        .block(Block::default())
        .column_spacing(1)
        .widths(&widths);

    table.render(area, buf);
}

fn render_task(area: Rect, buf: &mut Buffer, content: Vec<Vec<Word>>, clip: Clipping) {
    let content = match clip {
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
        .map(|c| {
            Line::from(
                c.iter()
                    .filter(|i| i.kind() != WordType::MetaInfo)
                    .map(|i| style_word(i))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}
