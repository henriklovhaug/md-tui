use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};

pub mod utils;
pub mod parser;

#[derive(Default)]
struct App {
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub horizontal_scroll: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::default();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    let text = File::open("README.md")?;

    let buffer = BufReader::new(text);

    let lines = buffer.lines();

    let mut text_lines = Vec::new();

    for line in lines {
        text_lines.push(Line::from(line.unwrap()));
    }

    loop {
        terminal.draw(|f| ui(f, &mut app, &text_lines))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') => {
                        app.vertical_scroll = app.vertical_scroll + 1;
                        app.vertical_scroll_state =
                            app.vertical_scroll_state.position(app.vertical_scroll);
                    }
                    KeyCode::Char('k') => {
                        app.vertical_scroll = app.vertical_scroll + 1;
                        app.vertical_scroll_state =
                            app.vertical_scroll_state.position(app.vertical_scroll);
                    }
                    KeyCode::Char('h') => {
                        app.horizontal_scroll = app.horizontal_scroll + 1;
                        app.horizontal_scroll_state =
                            app.horizontal_scroll_state.position(app.horizontal_scroll);
                    }
                    KeyCode::Char('l') => {
                        app.horizontal_scroll = app.horizontal_scroll + 1;
                        app.horizontal_scroll_state =
                            app.horizontal_scroll_state.position(app.horizontal_scroll);
                    }
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &mut App, lines: &Vec<Line>) {
    let size = f.size();
    let paragraph = Paragraph::new(lines.clone())
        .block(Block::default().borders(Borders::ALL).title("Paragraph"))
        .scroll((app.vertical_scroll as u16, 0))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, size);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Vertical Scrollbar"),
        size,
    );
    let mut size = f.size();
    let height = &size.height.to_string();
    let table = Table::new(vec![
        // Row can be created from simple strings.
        Row::new(vec!["Row11", "Row12", "Row13"]),
        // You can style the entire row.
        Row::new(vec!["Row21", "Row22", "Row23"]).style(Style::default().fg(Color::Blue)),
        // If you need more control over the styling you may need to create Cells directly
        Row::new(vec![
            Cell::from("Row31"),
            Cell::from("Row32").style(Style::default().fg(Color::Yellow)),
            Cell::from(Line::from(vec![
                Span::raw("Row"),
                Span::styled("33", Style::default().fg(Color::Green)),
            ])),
        ]),
        // If a Row need to display some content over multiple lines, you just have to change
        // its height.
        Row::new(vec![
            Cell::from("Row\n41"),
            Cell::from("Row\n42"),
            Cell::from("Row\n43"),
        ])
        .height(2),
    ])
    // You can set the style of the entire Table.
    .style(Style::default().fg(Color::White))
    // It has an optional header, which is simply a Row always visible at the top.
    .header(
        Row::new(vec!["Col1", "Col2", height])
            .style(Style::default().fg(Color::Yellow))
            // If you want some space between the header and the rest of the rows, you can always
            // specify some margin at the bottom.
            .bottom_margin(1),
    )
    // As any other widget, a Table can be wrapped in a Block.
    .block(Block::default().title("Table"))
    // Columns widths are constrained in the same way as Layout...
    .widths(&[
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(10),
    ])
    // ...and they can be separated by a fixed spacing.
    .column_spacing(1)
    // If you wish to highlight a row in any specific way when it is selected...
    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    // ...and potentially show a symbol in front of the selection.
    .highlight_symbol(">>");

    size.height -= 10;
    size.width -= 10;
    size.y += 10;
    // size.x += 10;
    f.render_widget(table, size);
}
