use std::{
    cmp,
    error::Error,
    fs::read_to_string,
    io::{self},
    panic,
    time::{Duration, Instant},
};

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
use util::{destruct_terminal, App, Boxes, Mode, CONFIG};

mod boxes;
mod event_handler;
pub mod nodes;
mod pages;
pub mod parser;
pub mod search;
mod util;

const EMPTY_FILE: &str = "";

fn main() -> Result<(), Box<dyn Error>> {
    // let text = read_to_string("./md_tests/test.md")?;
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

    terminal.draw(|f| {
        render_loading(f, &app);
    })?;

    let mut file_tree = find_md_files();

    let mut markdown = parse_markdown(None, EMPTY_FILE);
    let mut width = cmp::min(terminal.size()?.width, CONFIG.width);
    markdown.transform(width);

    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        if let Ok(file) = read_to_string(arg) {
            markdown = parse_markdown(Some(arg), &file);
            markdown.transform(width);
            app.mode = Mode::View;
        } else {
            app.error_box
                .set_message(format!("Could not open file {:?}", arg));
            app.boxes = Boxes::Error;
            app.mode = Mode::FileTree;
        }
    }

    loop {
        let new_width = cmp::min(terminal.size()?.width, CONFIG.width);
        if new_width != width {
            let url = if let Some(url) = markdown.file_name() {
                url
            } else {
                app.error_box.set_message("No file".to_string());
                app.boxes = Boxes::Error;
                app.mode = Mode::FileTree;
                continue;
            };
            let text = if let Ok(file) = read_to_string(url) {
                app.vertical_scroll = 0;
                file
            } else {
                app.error_box
                    .set_message(format!("Could not open file {:?}", markdown.file_name()));
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
                    render_markdown(f, &app, markdown.clone());
                }
                Mode::FileTree => {
                    render_file_tree(f, &app, file_tree.clone());
                }
            };
            if app.boxes == Boxes::Search {
                let (search_height, search_width) = app.search_box.dimensions();
                let search_area = Rect {
                    x: app.search_box.x(),
                    y: app.search_box.y(),
                    width: search_width,
                    height: search_height,
                };
                f.render_widget(app.search_box.clone(), search_area);
            } else if app.boxes == Boxes::Error {
                let (error_height, error_width) = app.error_box.dimensions();
                let error_area = Rect {
                    x: height / 2,
                    y: height / 2,
                    width: error_width,
                    height: error_height,
                };
                f.render_widget(Clear, error_area);
                f.render_widget(app.error_box.clone(), error_area);
            } else if app.boxes == Boxes::LinkPreview {
                let (link_height, link_width) = app.link_box.dimensions();
                let link_area = Rect {
                    x: height / 2,
                    y: height / 2,
                    width: link_width,
                    height: link_height,
                };
                f.render_widget(Clear, link_area);
                f.render_widget(app.link_box.clone(), link_area);
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
                    &mut file_tree,
                    height,
                ) {
                    KeyBoardAction::Exit => {
                        return Ok(());
                    }
                    KeyBoardAction::Continue => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn render_file_tree(f: &mut Frame, app: &App, file_tree: FileTree) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: size.width - 2,
        ..size
    };
    f.render_widget(file_tree, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: 4,
            y: size.height - 14,
            height: 13,
            width: CONFIG.width,
        }
    } else {
        Rect {
            x: 4,
            y: size.height - 4,
            height: 3,
            width: CONFIG.width,
        }
    };

    f.render_widget(Clear, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: 4,
            y: size.height - 13,
            height: 10,
            width: CONFIG.width,
        }
    } else {
        Rect {
            x: 4,
            y: size.height - 4,
            height: 3,
            width: CONFIG.width,
        }
    };

    f.render_widget(app.help_box, area);
}

fn render_markdown(f: &mut Frame, app: &App, markdown: RenderRoot) {
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
    let area = if !app.help_box.expanded() {
        Rect {
            y: size.height - 4,
            height: 3,
            width: CONFIG.width,
            ..area
        }
    } else {
        Rect {
            y: size.height - 17,
            height: 16,
            width: CONFIG.width,
            ..area
        }
    };
    f.render_widget(Clear, area);

    f.render_widget(block, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: 4,
            y: size.height - 16,
            height: 14,
            width: CONFIG.width,
        }
    } else {
        Rect {
            x: 4,
            y: size.height - 3,
            height: 3,
            width: CONFIG.width,
        }
    };

    if app.boxes != Boxes::Search {
        f.render_widget(app.help_box, area)
    }
}

fn render_loading(f: &mut Frame, _app: &App) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: size.width - 2,
        height: size.height - 5,
        ..size
    };

    const LOADING_TEXT: &str = r#"
  _        ___       _      ____    ___   _   _    ____             
 | |      / _ \     / \    |  _ \  |_ _| | \ | |  / ___|            
 | |     | | | |   / _ \   | | | |  | |  |  \| | | |  _             
 | |___  | |_| |  / ___ \  | |_| |  | |  | |\  | | |_| |  _   _   _ 
 |_____|  \___/  /_/   \_\ |____/  |___| |_| \_|  \____| (_) (_) (_)
                                                                    
"#;

    let page = Paragraph::new(LOADING_TEXT);

    f.render_widget(page, area);
}
