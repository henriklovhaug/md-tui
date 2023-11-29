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
use parser::parse_markdown;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::ScrollbarState,
    Frame, Terminal,
};
use utils::MdComponentTree;

pub mod markdown_render;
pub mod parser;
pub mod utils;

#[derive(Default)]
struct App {
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
    pub vertical_scroll: u16,
    pub horizontal_scroll: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    // let text = read_to_string("README.md")?;
    // let mut markdown = parse_markdown(&text);
    // markdown.set_height(80);
    // markdown.set_y_offset(0);
    // parser::print_tree(markdown.root(), 0);
    // for ci in markdown.root().children() {
    //     println!("Kind: {:?}, height: {}", ci.kind(), ci.height());
    // }
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

    loop {
        let width = terminal.size()?.width;
        let height = terminal.size()?.height;
        markdown.set_height(height, cmp::min(width, 80));
        markdown.set_y_offset(app.vertical_scroll);

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
                        app.vertical_scroll_state = app
                            .vertical_scroll_state
                            .position(app.vertical_scroll as usize);
                    }
                    KeyCode::Char('k') => {
                        app.vertical_scroll = app.vertical_scroll.saturating_sub(1);
                        app.vertical_scroll_state = app
                            .vertical_scroll_state
                            .position(app.vertical_scroll as usize);
                    }
                    KeyCode::Char('h') => {
                        app.horizontal_scroll += 1;
                        app.horizontal_scroll_state =
                            app.horizontal_scroll_state.position(app.horizontal_scroll);
                    }
                    KeyCode::Char('l') => {
                        app.horizontal_scroll -= 1;
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

fn ui(f: &mut Frame, lines: MdComponentTree) {
    let size = f.size();
    f.render_widget(lines, size);
}
