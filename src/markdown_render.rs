use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Cell, List, ListItem, Paragraph, Row, Table, Widget, Wrap},
};

use crate::utils::{MdComponent, MdEnum};

impl Widget for MdComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.height() + self.y_offset() > area.height
            || self.scroll_offset() > self.y_offset() + area.y
        {
            return;
        }

        let _removed = area.y + self.y_offset() - self.scroll_offset();

        // let height = cmp::min(self.height(), removed + 1);

        let area = Rect {
            height: self.height(),
            y: area.y + self.y_offset() - self.scroll_offset(),
            width: [self.width(), area.width, 80]
                .iter()
                .fold(u16::MAX, |a, &b| a.min(b)),
            ..area
        };

        let kind = self.kind();

        let content: Vec<MdComponent> = self
            .children_owned()
            .into_iter()
            .filter(|c| c.kind() != MdEnum::VerticalSeperator)
            .collect();

        // println!("{:?}: {}, {}", kind, content.len(), height);
        //
        // if content.len() > height as usize && kind != MdEnum::VerticalSeperator {
        //     content.drain(content.len() - height as usize..);
        // }

        match kind {
            MdEnum::Heading => render_heading(area, buf, content),
            MdEnum::ListContainer => render_list(area, buf, content),
            MdEnum::CodeBlock => render_code_block(area, buf, content),
            MdEnum::Paragraph => render_paragraph(area, buf, content),
            MdEnum::Table => render_table(area, buf, content),
            MdEnum::Task => todo!(),
            MdEnum::Code => unreachable!(),
            MdEnum::Link => todo!(),
            MdEnum::Quote => todo!(),
            MdEnum::Digit => todo!(),
            MdEnum::VerticalSeperator => (),
            MdEnum::Sentence => todo!(),
            MdEnum::TableRow => todo!(),
            MdEnum::BlockSeperator => (),
            _ => panic!("{:?} should not be reachable", kind),
        }
    }
}

fn render_heading(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content = content
        .iter()
        .map(|c| Line::styled(c.content(), Style::default().fg(Color::Black)))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(content)
        .block(Block::default().style(Style::default().bg(Color::Blue)))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    paragraph.render(area, buf);
}

fn render_paragraph(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content = Line::from(
        content
            .iter()
            .map(|c| match c.kind() {
                MdEnum::Code => {
                    Span::styled(c.content(), Style::new().green().italic().on_dark_gray())
                }
                _ => Span::raw(format!(" {} ", c.content().trim())),
            })
            .collect::<Vec<_>>(),
    );

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: true });

    paragraph.render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content: Vec<ListItem<'_>> = content
        .iter()
        .map(|c| ListItem::new(Line::from(c.content())))
        .collect();

    let list = List::new(content);
    list.render(area, buf);
}

fn render_code_block(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content: Vec<Line<'_>> = content.iter().map(|c| Line::from(c.content())).collect();

    let area = Rect {
        x: area.x + 1,
        width: area.width - 2,
        ..area
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().style(Style::default().bg(Color::DarkGray)))
        .wrap(Wrap { trim: true });

    paragraph.render(area, buf);
}

fn render_table(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let titles = content.first().unwrap();

    let widths = titles
        .children()
        .iter()
        .map(|c| Constraint::Length(c.content().len() as u16))
        .collect::<Vec<_>>();

    let mut moved_content = content.to_owned();
    moved_content.drain(0..2);

    let header = Row::new(
        titles
            .children()
            .iter()
            .map(|c| Cell::from(c.content().trim()))
            .collect::<Vec<_>>(),
    );

    let rows = moved_content
        .iter()
        .map(|c| {
            Row::new(
                c.children()
                    .iter()
                    .map(|i| Cell::from(i.content()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let table = Table::new(rows)
        .header(header)
        .block(Block::default())
        .column_spacing(1)
        .widths(&widths);

    table.render(area, buf);
}
