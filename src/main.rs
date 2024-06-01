use std::{
    cmp, env,
    error::Error,
    fs::read_to_string,
    io, panic,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event_handler::{handle_keyboard_input, KeyBoardAction};
use nodes::root::{Component, ComponentRoot};
use notify::{Config, PollWatcher, Watcher};
use pages::file_explorer::FileTree;
use parser::parse_markdown;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    style::{Color, Stylize},
    widgets::{Block, Clear, Paragraph},
    Frame, Terminal,
};
use ratatui_image::{FilterType, Resize, StatefulImage};
use search::find_md_files_and_send;
use util::{destruct_terminal, App, Boxes, Mode};

mod boxes;
mod event_handler;
pub mod highlight;
mod nodes;
mod pages;
pub mod parser;
pub mod search;
mod util;

const EMPTY_FILE: &str = "";

fn main() -> Result<(), Box<dyn Error>> {
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

    let (tx, rx) = mpsc::channel();

    let mut watcher = PollWatcher::new(
        tx,
        Config::default().with_poll_interval(Duration::from_secs(1)),
    )
    .unwrap();

    let (f_tx, f_rx) = mpsc::channel::<FileTree>();

    thread::spawn(move || loop {
        find_md_files_and_send(f_tx.clone())
    });

    app.set_width(terminal.size()?.width);
    let mut markdown = parse_markdown(None, EMPTY_FILE, app.width() - 2);

    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        if let Ok(file) = read_to_string(arg) {
            let path = std::path::Path::new(arg);
            let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
            markdown = parse_markdown(Some(arg), &file, app.width() - 2);
            app.mode = Mode::View;
        } else {
            app.error_box
                .set_message(format!("Could not open file {:?}", arg));
            app.boxes = Boxes::Error;
            app.mode = Mode::Loading;
        }
    }

    let mut file_tree = FileTree::default();

    loop {
        let height = terminal.size()?.height;

        for event in rx.try_iter() {
            if event.is_err() {
                continue;
            }
            let event = event.unwrap();

            if let notify::EventKind::Modify(_) = event.kind {
                if let Ok(file) = read_to_string(markdown.file_name().unwrap()) {
                    markdown =
                        parse_markdown(Some(markdown.file_name().unwrap()), &file, app.width() - 2);
                    app.mode = Mode::View;
                    app.vertical_scroll = cmp::min(
                        app.vertical_scroll,
                        markdown.height().saturating_sub(height / 2),
                    );
                }

                break;
            }
        }
        if app.set_width(terminal.size()?.width) {
            let url = if let Some(url) = markdown.file_name() {
                url
            } else {
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
            markdown = parse_markdown(markdown.file_name(), &text, app.width() - 2);
        }

        markdown.set_scroll(app.vertical_scroll);

        terminal.draw(|f| {
            match app.mode {
                Mode::View => {
                    render_markdown(f, &app, &mut markdown);
                }
                Mode::FileTree => {
                    if !file_tree.loaded() {
                        app.mode = Mode::Loading;
                    }
                    render_file_tree(f, &app, file_tree.clone());
                }
                Mode::Loading => {
                    render_loading(f, &app);
                    if let Ok(e) = f_rx.try_recv() {
                        file_tree = e;
                        app.mode = Mode::FileTree;
                    }
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
                    x: app.width() / 2 - error_width / 2,
                    y: height / 2,
                    width: error_width,
                    height: error_height,
                };

                if app.width() > error_width {
                    f.render_widget(Clear, error_area);
                    f.render_widget(app.error_box.clone(), error_area);
                }
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
                    &mut watcher,
                ) {
                    KeyBoardAction::Exit => {
                        return Ok(());
                    }
                    KeyBoardAction::Continue => {}
                    KeyBoardAction::Edit => {
                        terminal.draw(|f| {
                            open_editor(f, &mut app, markdown.file_name());
                        })?;
                    }
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
        width: app.width() - 3,
        ..size
    };
    f.render_widget(file_tree, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: 4,
            y: size.height - 14,
            height: 13,
            width: app.width() - 5,
        }
    } else {
        Rect {
            x: 4,
            y: size.height - 4,
            height: 3,
            width: app.width() - 5,
        }
    };

    f.render_widget(Clear, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: 4,
            y: size.height - 13,
            height: 10,
            width: app.width() - 5,
        }
    } else {
        Rect {
            x: 4,
            y: size.height - 5,
            height: 3,
            width: app.width() - 5,
        }
    };

    f.render_widget(app.help_box, area);
}

fn render_markdown(f: &mut Frame, app: &App, markdown: &mut ComponentRoot) {
    let size = f.size();
    let area = Rect {
        x: 2,
        width: app.width() - 3,
        height: size.height - 5,
        ..size
    };

    for child in markdown.children_mut() {
        match child {
            Component::TextComponent(comp) => {
                if comp.y_offset().saturating_sub(comp.scroll_offset()) >= area.height
                    || (comp.y_offset() + comp.height()).saturating_sub(comp.scroll_offset()) == 0
                {
                    continue;
                }

                f.render_widget(comp.clone(), area)
            }
            Component::Image(img) => {
                if img.y_offset().saturating_sub(img.scroll_offset()) >= area.height
                    || (img.y_offset() + img.height()).saturating_sub(img.scroll_offset()) == 0
                {
                    continue;
                }

                let image = StatefulImage::new(None).resize(Resize::Fit(Some(FilterType::Nearest)));

                // Resize height based on clipping top
                let height = cmp::min(
                    img.height(),
                    (img.y_offset() + img.height()).saturating_sub(img.scroll_offset()),
                );

                // Resize height based on clipping bottom
                let height = cmp::min(
                    height,
                    area.height
                        .saturating_add(img.scroll_offset())
                        .saturating_sub(img.y_offset()),
                );

                let inner_area = Rect::new(
                    area.x,
                    img.y_offset().saturating_sub(img.scroll_offset()),
                    area.width,
                    height,
                );

                f.render_stateful_widget(image, inner_area, img.image_mut())
            }
        }
    }

    // Render a block at the bottom to show the current mode
    let block = Block::default().bg(Color::Black);
    let area = if !app.help_box.expanded() {
        Rect {
            y: size.height - 4,
            height: 3,
            ..area
        }
    } else {
        Rect {
            y: size.height - 19,
            height: 18,
            ..area
        }
    };
    f.render_widget(Clear, area);

    f.render_widget(block, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: 4,
            y: size.height - 18,
            height: 16,
            width: app.width() - 5,
        }
    } else {
        Rect {
            x: 4,
            y: size.height - 3,
            height: 3,
            width: app.width() - 5,
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
        width: size.width - 3,
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

fn open_editor(f: &mut Frame, app: &mut App, file_name: Option<&str>) {
    let editor = if let Ok(editor) = env::var("EDITOR") {
        editor
    } else {
        app.error_box
            .set_message("No editor found. Please set the EDITOR environment variable".to_owned());
        app.boxes = Boxes::Error;
        return;
    };

    let file_name = if let Some(file_name) = file_name {
        file_name
    } else {
        app.error_box
            .set_message("No file found to open in editor".to_owned());
        app.boxes = Boxes::Error;
        return;
    };

    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();

    let _ = std::process::Command::new(editor)
        .arg(file_name)
        .spawn()
        .expect("Failed to open editor")
        .wait();

    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();

    app.boxes = Boxes::None;
    f.render_widget(Clear, f.size());
}
