use std::{
    cmp,
    error::Error,
    fs::read_to_string,
    io::{self},
    panic,
    time::{Duration, Instant},
};

use boxes::{errorbox::ErrorBox, help_box::HelpBox, searchbox::SearchBox};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event_handler::{handle_keyboard_input, KeyBoardAction};
use nodes::RenderRoot;
use pages::file_explorer::FileTree;
use parser::parse_markdown;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    style::{Color, Stylize},
    widgets::{Block, Clear, Paragraph},
    Frame, Terminal,
};
use search::find_md_files;
use util::{destruct_terminal, App, Boxes, Mode};

pub mod boxes;
mod event_handler;
pub mod nodes;
pub mod pages;
pub mod parser;
pub mod search;
pub mod util;

const EMPTY_FILE: &str = "";

fn main() -> Result<(), Box<dyn Error>> {
    // let text = read_to_string("./README.md")?;
    // let mut markdown = parse_markdown(Some("kek"), &text);
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

    let mut file_tree = FileTree::default();

    let mut search_box = SearchBox::default();
    let mut error_box = ErrorBox::default();
    let mut help_box = HelpBox::default();

    terminal.draw(|f| {
        render_loading(f, &app);
    })?;

    if let Ok(files) = find_md_files() {
        file_tree = files;
    } else {
        error_box.set_message("MDT does not have permissions to view directories".to_string());
        app.boxes = Boxes::Error;
        app.mode = Mode::FileTree;
    }

    let mut markdown = parse_markdown(None, EMPTY_FILE);
    let mut width = cmp::min(terminal.size()?.width, 80);
    markdown.transform(width);

    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        if let Ok(file) = read_to_string(arg) {
            markdown = parse_markdown(Some(arg), &file);
            markdown.transform(width);
            app.mode = Mode::View;
        } else {
            error_box.set_message(format!("Could not open file {:?}", arg));
            app.boxes = Boxes::Error;
            app.mode = Mode::FileTree;
        }
    }

    loop {
        let new_width = cmp::min(terminal.size()?.width, 80);
        if new_width != width {
            let url = if let Some(url) = markdown.file_name() {
                url
            } else {
                error_box.set_message("No file".to_string());
                app.boxes = Boxes::Error;
                app.mode = Mode::FileTree;
                continue;
            };
            let text = if let Ok(file) = read_to_string(url) {
                app.vertical_scroll = 0;
                file
            } else {
                error_box.set_message(format!("Could not open file {:?}", markdown.file_name()));
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
                    render_markdown(f, &app, markdown.clone(), help_box);
                }
                Mode::FileTree => {
                    render_file_tree(f, &app, file_tree.clone(), help_box);
                }
            };
            if app.boxes == Boxes::Search {
                let (search_height, search_width) = search_box.dimensions();
                let search_area = Rect {
                    x: search_box.x(),
                    y: search_box.y(),
                    width: search_width,
                    height: search_height,
                };
                // f.render_widget(Clear, search_area);
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
                    &mut help_box,
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

fn render_file_tree(f: &mut Frame, _app: &App, file_tree: FileTree, help_box: HelpBox) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: size.width - 2,
        ..size
    };
    f.render_widget(file_tree, area);

    let area = if help_box.expanded() {
        Rect {
            x: 4,
            y: size.height - 13,
            height: 10,
            width: 80,
        }
    } else {
        Rect {
            x: 4,
            y: size.height - 4,
            height: 3,
            width: 80,
        }
    };

    f.render_widget(Clear, area);
    f.render_widget(help_box, area);
}

fn render_markdown(f: &mut Frame, _app: &App, markdown: RenderRoot, _help_box: HelpBox) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: size.width - 2,
        height: size.height - 5,
        ..size
    };
    f.render_widget(markdown, area);

    // Render a block at the bottom to show the current mode
    let block = Block::default().bg(Color::Black);
    let area = Rect {
        y: size.height - 4,
        height: 3,
        width: 80,
        ..area
    };
    f.render_widget(block, area);
}

fn render_loading(f: &mut Frame, _app: &App) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: size.width - 2,
        height: size.height - 5,
        ..size
    };

    let loading_text = r#"
  _        ___       _      ____    ___   _   _    ____             
 | |      / _ \     / \    |  _ \  |_ _| | \ | |  / ___|            
 | |     | | | |   / _ \   | | | |  | |  |  \| | | |  _             
 | |___  | |_| |  / ___ \  | |_| |  | |  | |\  | | |_| |  _   _   _ 
 |_____|  \___/  /_/   \_\ |____/  |___| |_| \_|  \____| (_) (_) (_)
                                                                    
"#;

    let page = Paragraph::new(loading_text);

    f.render_widget(page, area);
}
