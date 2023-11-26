use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, List, ListItem, Paragraph, Widget, Wrap},
};

use crate::utils::{MdComponent, MdEnum};

pub fn render(kind: MdEnum, area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    match kind {
        MdEnum::Heading => render_heading(area, buf, content),
        MdEnum::Task => todo!(),
        MdEnum::UnorderedList => todo!(),
        MdEnum::ListContainer => render_list(area, buf, content),
        MdEnum::OrderedList => unreachable!(),
        MdEnum::CodeBlock => render_code_block(area, buf, content),
        MdEnum::Code => unreachable!(),
        MdEnum::Paragraph => render_paragraph(area, buf, content),
        MdEnum::Link => todo!(),
        MdEnum::Quote => todo!(),
        MdEnum::Table => (),
        MdEnum::EmptyLine => todo!(),
        MdEnum::Digit => todo!(),
        MdEnum::VerticalSeperator => (),
        MdEnum::Sentence => todo!(),
    }
}

fn render_heading(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content = content
        .iter()
        .map(|c| Line::from(c.content()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(content)
        .block(Block::default().style(Style::default().bg(Color::Blue)))
        .wrap(Wrap { trim: true });

    paragraph.render(area, buf);
}

fn render_paragraph(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content = content
        .iter()
        .filter_map(|c| {
            if c.kind() == MdEnum::VerticalSeperator {
                None
            } else {
                Some(Line::from(c.content()))
            }
        })
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: true });

    paragraph.render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content = content
        .iter()
        .filter_map(|c| {
            if c.kind() == MdEnum::VerticalSeperator {
                None
            } else {
                Some(ListItem::new(Line::from(c.content())))
            }
        })
        .collect::<Vec<_>>();

    let list = List::new(content);
    list.render(area, buf);
}

fn render_code_block(area: Rect, buf: &mut Buffer, content: Vec<MdComponent>) {
    let content = content
        .iter()
        .map(|c| Line::from(c.content()))
        .collect::<Vec<_>>();

    let area = Rect {
        x: area.x + 1,
        width: area.width - 2,
        height: area.height - 2,
        ..area
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().style(Style::default().bg(Color::DarkGray)))
        .wrap(Wrap { trim: true });

    paragraph.render(area, buf);
}
