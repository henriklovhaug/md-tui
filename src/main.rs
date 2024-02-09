use std::{
    cmp,
    error::Error,
    fs::read_to_string,
    io::{self},
    panic,
    process::exit,
    time::{Duration, Instant},
};

use boxes::{errorbox::ErrorBox, searchbox::SearchBox};
use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nodes::RenderRoot;
use pages::file_explorer::{FileTree, MdFile};
use parser::parse_markdown;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    widgets::Clear,
    Frame, Terminal,
};
use search::find_line_match_and_index;

pub mod boxes;
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
        } else if s.starts_with('/') {
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

enum KeyBoardAction {
    Continue,
    Exit,
}

fn handle_keyboard_input(
    key: KeyCode,
    app: &mut App,
    markdown: &mut RenderRoot,
    search_box: &mut SearchBox,
    error_box: &mut ErrorBox,
    file_tree: &mut FileTree,
    height: u16,
) -> KeyBoardAction {
    if key == KeyCode::Char('q') && app.boxes != Boxes::Search {
        return KeyBoardAction::Exit;
    }
    match app.mode {
        Mode::View => keyboard_mode_view(key, app, markdown, search_box, error_box, height),
        Mode::FileTree => {
            keyboard_mode_file_tree(key, app, markdown, search_box, error_box, file_tree, height)
        }
    }
}

fn keyboard_mode_file_tree(
    key: KeyCode,
    app: &mut App,
    markdown: &mut RenderRoot,
    _search_box: &mut SearchBox,
    error_box: &mut ErrorBox,
    file_tree: &mut FileTree,
    _height: u16,
) -> KeyBoardAction {
    match app.boxes {
        Boxes::Error => match key {
            KeyCode::Enter | KeyCode::Esc => {
                app.boxes = Boxes::None;
            }
            _ => {}
        },
        Boxes::Search => todo!(),
        Boxes::None => match key {
            KeyCode::Char('j') => {
                file_tree.next();
            }
            KeyCode::Char('k') => {
                file_tree.previous();
            }
            KeyCode::Enter => {
                let file = if let Some(file) = file_tree.selected() {
                    file
                } else {
                    error_box.set_message("No file selected".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };
                let text = if let Ok(file) = read_to_string(file.path()) {
                    app.reset();
                    file
                } else {
                    error_box.set_message(format!("Could not open file {}", file.path()));
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };
                *markdown = parse_markdown(file.path(), &text);
                markdown.transform(80);
                app.mode = Mode::View;
            }
            KeyCode::Esc => {
                file_tree.unselect();
            }
            _ => {}
        },
    }

    KeyBoardAction::Continue
}

fn keyboard_mode_view(
    key: KeyCode,
    app: &mut App,
    markdown: &mut RenderRoot,
    search_box: &mut SearchBox,
    error_box: &mut ErrorBox,
    height: u16,
) -> KeyBoardAction {
    match app.boxes {
        Boxes::Error => match key {
            KeyCode::Enter | KeyCode::Esc => {
                app.boxes = Boxes::None;
            }
            _ => {}
        },
        Boxes::Search => match key {
            KeyCode::Esc => {
                search_box.clear();
                app.boxes = Boxes::None;
            }
            KeyCode::Enter => {
                let query = search_box.consume();
                let lines = markdown.content();
                let search =
                    find_line_match_and_index(&query, lines.iter().map(|s| &**s).collect(), 0);
                if search.is_empty() {
                    error_box.set_message(format!("No results for {}", query));
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                }
                markdown
                    .mark_word(
                        search.first().unwrap().0,
                        search.first().unwrap().1,
                        query.len(),
                    )
                    .unwrap();
                app.boxes = Boxes::None;
            }
            KeyCode::Char(c) => {
                search_box.insert(c);
            }
            KeyCode::Backspace => {
                search_box.delete();
            }
            _ => {}
        },
        Boxes::None => match key {
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
            KeyCode::Char('f') | KeyCode::Char('/') => {
                app.boxes = Boxes::Search;
            }
            KeyCode::Char('t') => {
                app.mode = Mode::FileTree;
            }
            KeyCode::Char('r') => {
                let text = if let Ok(file) = read_to_string(markdown.file_name()) {
                    app.vertical_scroll = 0;
                    file
                } else {
                    error_box.set_message(format!("Could not open file {}", markdown.file_name()));
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };
                *markdown = parse_markdown(markdown.file_name(), text.as_ref());
                markdown.transform(80);
                app.mode = Mode::View;
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
                        app.vertical_scroll = if let Ok(index) = markdown.heading_offset(heading) {
                            index
                        } else {
                            error_box.set_message(format!("Could not find heading {}", heading));
                            app.boxes = Boxes::Error;
                            return KeyBoardAction::Continue;
                        };
                    }
                    LinkType::External(url) => {
                        let _ = open::that(url);
                    }
                    LinkType::MarkdownFile(url) => {
                        // Remove the first character, which is a '/'
                        let url = &url[1..];
                        let text = if let Ok(file) = read_to_string(url) {
                            app.vertical_scroll = 0;
                            file
                        } else {
                            error_box.set_message(format!("Could not open file {}", url));
                            app.boxes = Boxes::Error;
                            return KeyBoardAction::Continue;
                        };
                        *markdown = parse_markdown(url, &text);
                        markdown.transform(80);
                        app.mode = Mode::View;
                    }
                }
                markdown.deselect();
                app.selected = false;
            }
            _ => {}
        },
    }
    KeyBoardAction::Continue
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

fn find_md_files() -> Result<FileTree, io::Error> {
    let mut tree = FileTree::new();
    let mut stack = vec![std::path::PathBuf::from(".")];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().unwrap_or_default() == "md" {
                let name = path.file_name().unwrap().to_str().unwrap().to_string();
                let path = path.to_str().unwrap().to_string();
                tree.add_file(MdFile::new(path, name));
            }
        }
    }
    tree.sort_2();
    Ok(tree)
}
