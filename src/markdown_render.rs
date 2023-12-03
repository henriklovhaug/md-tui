use std::cmp;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Cell, List, ListItem, Paragraph, Row, Table, Widget, Wrap},
};

use crate::nodes::{RenderComponent, RenderNode, Word, WordType};

impl Widget for RenderComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.height() + self.y_offset() > area.height
            || self.scroll_offset() > self.y_offset() + self.height()
        {
            return;
        }

        let height = cmp::min(
            self.height(),
            (self.height() + self.y_offset()).saturating_sub(self.scroll_offset()),
        );

        let area = Rect {
            height,
            y: self.y_offset().saturating_sub(self.scroll_offset()),
            ..area
        };

        match self.kind() {
            RenderNode::Paragraph => render_paragraph(area, buf, self.content()),
            RenderNode::Heading => render_heading(area, buf, self.content()),
            RenderNode::Task => todo!(),
            RenderNode::List => render_list(area, buf, self.content()),
            RenderNode::CodeBlock => render_code_block(area, buf, self.content()),
            RenderNode::LineBreak => (),
            RenderNode::Table => render_table(area, buf, self.content()),
        }
    }
}

fn render_heading(area: Rect, buf: &mut Buffer, content: &Vec<Vec<Word>>) {
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

fn render_paragraph(area: Rect, buf: &mut Buffer, content: &Vec<Vec<Word>>) {
    let mut lines = content
        .iter()
        .map(|c| {
            Line::from(
                c.iter()
                    .filter(|i| i.kind() != WordType::MetaInfo)
                    .map(|i| match i.kind() {
                        WordType::MetaInfo => unreachable!(),
                        WordType::Normal => Span::raw(format!("{} ", i.content())),
                        WordType::Code => Span::styled(
                            format!("{} ", i.content()),
                            Style::default().fg(Color::Red),
                        ),
                        WordType::Link => todo!(),
                        WordType::Italic => todo!(),
                        WordType::Bold => todo!(),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    lines.drain(..(lines.len() - area.height as usize));

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, content: &Vec<Vec<Word>>) {
    let mut content: Vec<ListItem<'_>> = content
        .iter()
        .map(|c| {
            ListItem::new(Line::from(
                c.iter()
                    .map(|i| format!("{} ", i.content()))
                    .collect::<String>(),
            ))
        })
        .collect();

    content.drain(..(content.len() - area.height as usize));

    let list = List::new(content);
    list.render(area, buf);
}

fn render_code_block(area: Rect, buf: &mut Buffer, content: &Vec<Vec<Word>>) {
    let mut content = content
        .first()
        .unwrap()
        .iter()
        .filter(|c| c.kind() != WordType::MetaInfo)
        .map(|c| Line::styled(c.content(), Style::default().fg(Color::Red)))
        .collect::<Vec<_>>();

    let area = Rect {
        x: area.x + 1,
        width: area.width - 2,
        ..area
    };

    content.drain(..(content.len() - area.height as usize));

    let paragraph = Paragraph::new(content)
        .block(Block::default().style(Style::default().bg(Color::Blue)))
        .wrap(Wrap { trim: false });

    paragraph.render(area, buf);
}

fn render_table(area: Rect, buf: &mut Buffer, content: &Vec<Vec<Word>>) {
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
        .map(|c| {
            Row::new(
                c.iter()
                    .map(|i| Cell::from(i.content()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    rows.drain(..(content.len() - area.height as usize));

    println!("Header len: {:?}", header);

    let table = Table::new(rows)
        .header(header.style(Style::default().fg(Color::Black)))
        .block(Block::default())
        .column_spacing(1)
        .widths(&widths);

    table.render(area, buf);
}
