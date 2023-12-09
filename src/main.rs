use std::{
    cmp,
    error::Error,
    fs::read_to_string,
    io::{self},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nodes::RenderRoot;
use parser::parse_markdown;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Frame, Terminal,
};

pub mod markdown_render;
pub mod nodes;
pub mod parser;

#[derive(Default)]
struct App {
    pub vertical_scroll: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    // let text = read_to_string("README.md")?;
    // let mut markdown = parse_markdown(&text);
    // markdown.transform(80);
    // markdown.set_scroll(0);
    //
    // parser::print_from_root(&markdown);
    // return Ok(());

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

    let text = read_to_string("README.md")?;

    let mut markdown = parse_markdown(&text);
    let mut width = cmp::min(terminal.size()?.width, 80);
    markdown.transform(width);

    loop {
        let new_width = cmp::min(terminal.size()?.width, 80);
        if new_width != width {
            markdown.transform(new_width);
            width = new_width;
        }

        markdown.set_scroll(app.vertical_scroll);

        terminal.draw(|f| ui(f, markdown.clone()))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') => {
                        app.vertical_scroll += 1;
                    }
                    KeyCode::Char('k') => {
                        app.vertical_scroll = app.vertical_scroll.saturating_sub(1);
                    }
                    KeyCode::Char('g') => {
                        app.vertical_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        app.vertical_scroll = markdown.height().saturating_sub(1);
                    }
                    KeyCode::Char('r') => {
                        markdown.transform(new_width)
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

fn ui(f: &mut Frame, lines: RenderRoot) {
    let size = f.size();
    f.render_widget(lines, size);
}
