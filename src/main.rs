use std::{
    cmp,
    error::Error,
    fs::read_to_string,
    io::{self},
    panic,
    time::{Duration, Instant},
};

use boxes::{errorbox::ErrorBox, searchbox::SearchBox};
use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use keyboard::{handle_keyboard_input, KeyBoardAction};
use nodes::RenderRoot;
use pages::file_explorer::FileTree;
use parser::parse_markdown;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    widgets::Clear,
    Frame, Terminal,
};
use search::find_md_files;

pub mod boxes;
mod keyboard;
pub mod nodes;
pub mod pages;
pub mod parser;
pub mod search;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    View,
    FileTree,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Boxes {
    Error,
    Search,
    None,
}

impl Default for Mode {
    fn default() -> Self {
        Self::FileTree
    }
}

impl Default for Boxes {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Default, Clone, Copy)]
struct App {
    pub vertical_scroll: u16,
    pub selected: bool,
    pub select_index: usize,
    pub mode: Mode,
    pub boxes: Boxes,
}

impl App {
    fn reset(&mut self) {
        self.vertical_scroll = 0;
        self.selected = false;
        self.select_index = 0;
        self.boxes = Boxes::None;
    }
}

enum LinkType<'a> {
    Internal(&'a str),
    External(&'a str),
    MarkdownFile(&'a str),
}

impl<'a> From<&'a str> for LinkType<'a> {
    fn from(s: &'a str) -> Self {
        if s.starts_with("http") {
            return Self::External(s);
        }
        if s.starts_with('/') {
            return Self::MarkdownFile(s);
        }
        Self::Internal(s)
    }
}

fn destruct_terminal() {
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();
}

fn main() -> Result<(), Box<dyn Error>> {
    // let text = read_to_string("./md_tests/test.md")?;
    // let mut markdown = parse_markdown(&text);
    // markdown.transform(80);
    // markdown.set_scroll(0);
    //
    // parser::print_from_root(&markdown);
    // // dbg!("{:?}", markdown.content());
    // return Ok(());

    // Set up panic handler. If not set up, the terminal will be left in a broken state if a panic
    // occurs
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

    let text = "# temp";
    let name = "temp";

    let mut markdown = parse_markdown(name, text);
    let mut width = cmp::min(terminal.size()?.width, 80);
    markdown.transform(width);

    let mut search_box = SearchBox::default();
    let mut error_box = ErrorBox::default();

    let mut file_tree = find_md_files()?;

    loop {
        let new_width = cmp::min(terminal.size()?.width, 80);
        if new_width != width {
            let text = if let Ok(file) = read_to_string(markdown.file_name()) {
                app.vertical_scroll = 0;
                file
            } else {
                error_box.set_message(format!("Could not open file {}", markdown.file_name()));
                app.boxes = Boxes::Error;
                app.mode = Mode::FileTree;
                continue;
            };
            markdown = parse_markdown(markdown.file_name(), &text);
            markdown.transform(new_width);
            width = new_width;
        }
        let height = terminal.size()?.height;

        markdown.set_scroll(app.vertical_scroll);

        terminal.draw(|f| {
            match app.mode {
                Mode::View => {
                    render_markdown(f, app, markdown.clone());
                }
                Mode::FileTree => {
                    render_file_tree(f, app, file_tree.clone());
                }
            };
            if app.boxes == Boxes::Search {
                let (search_height, search_width) = search_box.dimensions();
                let search_area = Rect {
                    x: height / 2,
                    y: height / 2,
                    width: search_width,
                    height: search_height,
                };
                f.render_widget(Clear, search_area);
                f.render_widget(search_box.clone(), search_area);
            } else if app.boxes == Boxes::Error {
                let (error_height, error_width) = error_box.dimensions();
                let error_area = Rect {
                    x: height / 2,
                    y: height / 2,
                    width: error_width,
                    height: error_height,
                };
                f.render_widget(Clear, error_area);
                f.render_widget(error_box.clone(), error_area);
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
                    &mut error_box,
                    &mut file_tree,
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

fn render_file_tree(f: &mut Frame, _app: App, file_tree: FileTree) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: size.width - 2,
        ..size
    };
    f.render_widget(file_tree, area);
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
