use std::{
    cmp, env,
    error::Error,
    fs::read_to_string,
    io::{self, IsTerminal, Read},
    panic,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use md_tui::event_handler::{KeyBoardAction, handle_keyboard_input};
use md_tui::nodes::root::{Component, ComponentRoot};
use md_tui::pages::file_explorer::{FileTree, MdFile};
use md_tui::parser::parse_markdown;
use md_tui::search::find_md_files_channel;
use md_tui::util::{self, App, Boxes, Mode, destruct_terminal, general::GENERAL_CONFIG};

use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use notify::{Config, PollWatcher, Watcher};
use ratatui::{
    DefaultTerminal, Frame,
    layout::Rect,
    style::{Color, Stylize},
    widgets::{Block, Clear},
};
use ratatui_image::{FilterType, Resize, StatefulImage};

const EMPTY_FILE: &str = "";

fn main() -> Result<(), Box<dyn Error>> {
    // Set up panic handler. If not set up, the terminal will be left in a broken state if a panic
    // occurs
    panic::set_hook(Box::new(|panic_info| {
        destruct_terminal();
        better_panic::Settings::auto().create_panic_handler()(panic_info);
    }));

    let mut terminal = ratatui::init();

    // create app and run it
    let tick_rate = Duration::from_millis(100);
    let app = App::default();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    ratatui::restore();

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut DefaultTerminal, mut app: App, tick_rate: Duration) -> io::Result<()> {
    let (f_tx, f_rx) = mpsc::channel::<Option<MdFile>>();

    thread::spawn(move || find_md_files_channel(f_tx.clone()));

    let mut last_tick = Instant::now();

    let (tx, rx) = mpsc::channel();

    let mut watcher = PollWatcher::new(
        tx,
        Config::default().with_poll_interval(Duration::from_secs(1)),
    )
    .unwrap();

    app.set_width(terminal.size()?.width);
    let mut markdown = parse_markdown(None, EMPTY_FILE, app.width() - 2);

    let potential_input = io::stdin();
    let mut stdin_buf = String::new();

    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        if let Ok(file) = read_to_string(arg) {
            let path = std::path::Path::new(arg);
            let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
            markdown = parse_markdown(Some(arg), &file, app.width() - 2);
            app.mode = Mode::View;
        } else {
            app.message_box
                .set_message(format!("Could not open file {arg}"));
            app.boxes = Boxes::Error;
        }
    } else if !potential_input.is_terminal() {
        let _ = potential_input.lock().read_to_string(&mut stdin_buf);
        markdown = parse_markdown(None, &stdin_buf, app.width() - 2);
        app.mode = Mode::View;
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
                app.message_box
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
                        while let Ok(e) = f_rx.try_recv() {
                            match e {
                                Some(file) => {
                                    file_tree.add_file(file);
                                }
                                None => {
                                    file_tree = file_tree.clone().finish();
                                    break;
                                }
                            }
                        }
                    }
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
                let (error_height, error_width) = app.message_box.dimensions();
                let error_area = Rect {
                    x: app.width() / 2 - error_width / 2,
                    y: height / 2,
                    width: error_width,
                    height: error_height,
                };

                if app.width() > error_width {
                    f.render_widget(Clear, error_area);
                    f.render_widget(app.message_box.clone(), error_area);
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

        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
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
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn render_file_tree(f: &mut Frame, app: &App, file_tree: FileTree) {
    let size = f.area();
    let x = match GENERAL_CONFIG.centering {
        util::general::Centering::Left => 2,
        util::general::Centering::Center => {
            let x = (size.width / 2).saturating_sub(GENERAL_CONFIG.width / 2);

            if x > 2 { x } else { 2 }
        }
        util::general::Centering::Right => {
            let x = size.width.saturating_sub(GENERAL_CONFIG.width + 2);
            if x > 2 { x } else { 2 }
        }
    };
    let area = Rect {
        x,
        width: app.width() - 3,
        ..size
    };
    f.render_widget(file_tree, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: x + 2,
            y: size.height - 14,
            height: 13,
            width: app.width() - 5,
        }
    } else {
        Rect {
            x: x + 2,
            y: size.height - 4,
            height: 3,
            width: app.width() - 5,
        }
    };

    f.render_widget(Clear, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: x + 2,
            y: size.height - 13,
            height: 10,
            width: app.width() - 5,
        }
    } else {
        Rect {
            x: x + 2,
            y: size.height - 5,
            height: 3,
            width: app.width() - 5,
        }
    };

    f.render_widget(app.help_box, area);
}

fn render_markdown(f: &mut Frame, app: &App, markdown: &mut ComponentRoot) {
    let size = f.area();

    let x = match GENERAL_CONFIG.centering {
        util::general::Centering::Left => 2,
        util::general::Centering::Center => {
            let x = (size.width / 2).saturating_sub(GENERAL_CONFIG.width / 2);

            if x > 2 { x } else { 2 }
        }
        util::general::Centering::Right => {
            let x = size.width.saturating_sub(GENERAL_CONFIG.width + 2);
            if x > 2 { x } else { 2 }
        }
    };

    let area = Rect {
        width: app.width() - 3,
        height: size.height - 5,
        x,
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

                let image = StatefulImage::default().resize(Resize::Fit(Some(FilterType::Nearest)));

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
            x,
            ..area
        }
    } else {
        Rect {
            y: size.height - 19,
            height: 18,
            x,
            ..area
        }
    };
    f.render_widget(Clear, area);

    f.render_widget(block, area);

    let area = if app.help_box.expanded() {
        Rect {
            x: x + 2,
            y: size.height - 18,
            height: 16,
            width: app.width() - 5,
        }
    } else {
        Rect {
            x: x + 2,
            y: size.height - 3,
            height: 3,
            width: app.width() - 5,
        }
    };

    if app.boxes != Boxes::Search {
        f.render_widget(app.help_box, area)
    }
}

fn open_editor(f: &mut Frame, app: &mut App, file_name: Option<&str>) {
    let editor = if let Ok(editor) = env::var("EDITOR") {
        editor
    } else {
        app.message_box
            .set_message("No editor found. Please set the EDITOR environment variable".to_owned());
        app.boxes = Boxes::Error;
        return;
    };

    let file_name = if let Some(file_name) = file_name {
        file_name
    } else {
        app.message_box
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
    f.render_widget(Clear, f.area());
}
