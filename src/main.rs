use std::{
    cmp,
    error::Error,
    fs::read_to_string,
    io::{self},
    panic,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nodes::RenderRoot;
use parser::parse_markdown;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    widgets::Clear,
    Frame, Terminal,
};
use searchbox::SearchBox;

pub mod nodes;
pub mod parser;
pub mod renderer;
pub mod search;
pub mod searchbox;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    View,
    Search,
}

impl Default for Mode {
    fn default() -> Self {
        Self::View
    }
}

#[derive(Default, Clone, Copy)]
struct App {
    pub vertical_scroll: u16,
    pub selected: bool,
    pub select_index: usize,
    pub mode: Mode,
}

fn destruct_terminal() {
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();
}

fn main() -> Result<(), Box<dyn Error>> {
    // let text = read_to_string("README.md")?;
    // let mut markdown = parse_markdown(&text);
    // markdown.transform(80);
    // markdown.set_scroll(0);
    //
    // parser::print_from_root(&markdown);
    // return Ok(());

    // Set up panic handler. If not set up, the terminal will be left in a broken state
    panic::set_hook(Box::new(|panic_info| {
        destruct_terminal();
        better_panic::Settings::auto().create_panic_handler()(panic_info);
    }));

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

    let mut search_box = SearchBox::default();

    loop {
        let new_width = cmp::min(terminal.size()?.width, 80);
        if new_width != width {
            markdown.transform(new_width);
            width = new_width;
        }
        let height = terminal.size()?.height;

        markdown.set_scroll(app.vertical_scroll);

        terminal.draw(|f| {
            render_markdown(f, app, markdown.clone());
            if app.mode == Mode::Search {
                let (search_height, search_width) = search_box.dimensions();
                let search_area = Rect {
                    x: height / 2,
                    y: height / 2,
                    width: search_width,
                    height: search_height,
                };
                f.render_widget(Clear, search_area);
                f.render_widget(search_box.clone(), search_area);
            }
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match handle_keyboard_input(
                    key.code,
                    &mut app,
                    &mut markdown,
                    &mut search_box,
                    height,
                ) {
                    KeyBoardAction::Continue => {}
                    KeyBoardAction::Exit => {
                        return Ok(());
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

enum KeyBoardAction {
    Continue,
    Exit,
}

fn handle_keyboard_input(
    key: KeyCode,
    app: &mut App,
    markdown: &mut RenderRoot,
    search_box: &mut SearchBox,
    height: u16,
) -> KeyBoardAction {
    match app.mode {
        Mode::View => match key {
            KeyCode::Char('q') => return KeyBoardAction::Exit,
            KeyCode::Char('j') => {
                if app.selected {
                    app.select_index = cmp::min(app.select_index + 1, markdown.num_links() - 1);
                    app.vertical_scroll =
                        markdown.select(app.select_index).saturating_sub(height / 3);
                } else {
                    app.vertical_scroll = cmp::min(
                        app.vertical_scroll + 1,
                        markdown.height().saturating_sub(height / 2),
                    );
                }
            }
            KeyCode::Char('k') => {
                if app.selected {
                    app.select_index = app.select_index.saturating_sub(1);
                    app.vertical_scroll =
                        markdown.select(app.select_index).saturating_sub(height / 3);
                } else {
                    app.vertical_scroll = app.vertical_scroll.saturating_sub(1);
                }
            }
            KeyCode::Char('g') => {
                app.vertical_scroll = 0;
            }
            KeyCode::Char('G') => {
                app.vertical_scroll = markdown.height().saturating_sub(height / 2);
            }

            KeyCode::Char('d') => {
                app.vertical_scroll += height / 2;
                app.vertical_scroll = cmp::min(
                    app.vertical_scroll,
                    markdown.height().saturating_sub(height / 2),
                );
            }
            KeyCode::Char('u') => {
                app.vertical_scroll = app.vertical_scroll.saturating_sub(height / 2);
            }
            KeyCode::Char('s') => {
                app.vertical_scroll = markdown.select(app.select_index).saturating_sub(height / 3);
                app.selected = true;
            }
            KeyCode::Esc => {
                app.selected = false;
                markdown.deselect();
            }
            KeyCode::Enter => {
                if !app.selected {
                    return KeyBoardAction::Continue;
                }
                let heading = markdown.selected();
                match LinkType::from(heading) {
                    LinkType::Internal(heading) => {
                        app.vertical_scroll = markdown.heading_offset(heading);
                    }
                    LinkType::External(url) => {
                        let _ = open::that(url);
                    }
                }
                markdown.deselect();
                app.selected = false;
            }
            KeyCode::Char('/') => {
                app.mode = Mode::Search;
            }
            _ => {}
        },
        Mode::Search => match key {
            KeyCode::Esc => {
                search_box.clear();
                app.mode = Mode::View;
            }
            // KeyCode::Enter => {
            //     let query = search_box.consume();
            //     let result = search::search(&query, markdown);
            //     if let Some(result) = result {
            //         app.vertical_scroll = result.saturating_sub(height / 3);
            //         app.selected = true;
            //     }
            //     app.mode = Mode::View;
            // }
            KeyCode::Char(c) => {
                search_box.insert(c);
            }
            KeyCode::Backspace => {
                search_box.delete();
            }
            _ => {}
        },
    }
    return KeyBoardAction::Continue;
}

fn render_markdown(f: &mut Frame, _app: App, markdown: RenderRoot) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: size.width - 2,
        ..size
    };
    f.render_widget(markdown, area);
}

enum LinkType<'a> {
    Internal(&'a str),
    External(&'a str),
}

impl<'a> From<&'a str> for LinkType<'a> {
    fn from(s: &'a str) -> Self {
        if s.starts_with("http") {
            return Self::External(s);
        }
        Self::Internal(s)
    }
}
