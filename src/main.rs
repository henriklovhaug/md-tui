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

use md_tui::bookmarks;
use md_tui::boxes::comment_sidebar::SIDEBAR_WIDTH;
use md_tui::event_handler::{KeyBoardAction, handle_keyboard_input};
use md_tui::nodes::image::ImageComponent;
use md_tui::nodes::root::{Component, ComponentRoot};
use md_tui::pages::file_explorer::{FileTree, MdFile};
use md_tui::pages::footer::render_footer;
use md_tui::pages::markdown_renderer::markdown_view_area;
use md_tui::parser::parse_markdown;
use md_tui::search::find_md_files_channel;
use md_tui::util::{App, Boxes, Mode, destruct_terminal, general::GENERAL_CONFIG};

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
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Clear},
};
use ratatui_image::{FilterType, Resize, StatefulImage};

fn enable_mouse() {
    let _ = execute!(io::stdout(), EnableMouseCapture);
}

fn disable_mouse() {
    let _ = execute!(io::stdout(), DisableMouseCapture);
}

const EMPTY_FILE: &str = "";

fn main() -> Result<(), Box<dyn Error>> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_cli(&raw_args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };

    // Set up panic handler. If not set up, the terminal will be left in a broken state if a panic
    // occurs
    panic::set_hook(Box::new(|panic_info| {
        destruct_terminal();
        better_panic::Settings::auto().create_panic_handler()(panic_info);
    }));

    let args: Vec<String> = std::env::args().collect();
    if args
        .get(1)
        .is_some_and(|a| a == "--version" || a == "-V" || a == "-v")
    {
        println!("mdt {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let mut terminal = ratatui::init();
    enable_mouse();

    // create app and run it
    let tick_rate = Duration::from_millis(100);
    let mut app = App::default();
    app.username = parsed.username;
    let res = run_app(&mut terminal, &mut app, tick_rate, parsed.file);

    disable_mouse();
    // Restore the main screen BEFORE dumping so the YAML lands in the user's
    // shell rather than the alternate buffer (which the terminal just exited).
    ratatui::restore();

    let dump_inputs = md_tui::sidemark::DumpInputs {
        document: res.as_ref().ok().and_then(|d| d.file_name.as_deref()),
        author: app.username.as_deref(),
        comments: &app.comments,
        raw_source: app.raw_source.as_deref(),
    };
    // `MDT_DUMP_PATH` redirects the YAML dump to a file so wrappers (e.g. a
    // tmux popup) can read it after exit without competing with the TUI for
    // stdout.
    dump_comments(
        dump_inputs,
        resolve_dump_target(env::var_os("MDT_DUMP_PATH")),
    );

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

/// Where the on-exit Sidemark dump is written.
#[derive(Debug, PartialEq, Eq)]
enum DumpTarget {
    File(std::path::PathBuf),
    Stdout,
}

/// Map the `MDT_DUMP_PATH` env value to a dump target. An unset or empty value
/// means "print to stdout"; any non-empty path redirects to that file.
fn resolve_dump_target(path: Option<std::ffi::OsString>) -> DumpTarget {
    match path {
        Some(p) if !p.is_empty() => DumpTarget::File(p.into()),
        _ => DumpTarget::Stdout,
    }
}

/// Emit the dump to the resolved target. Writing nothing (no comments) leaves
/// the file untouched, mirroring the stdout path which prints nothing.
fn dump_comments(dump_inputs: md_tui::sidemark::DumpInputs<'_>, target: DumpTarget) {
    match target {
        DumpTarget::File(path) => {
            if let Some(yaml) = md_tui::sidemark::render(dump_inputs)
                && let Err(e) = std::fs::write(&path, yaml)
            {
                eprintln!("warning: failed to write MDT_DUMP_PATH: {e}");
            }
        }
        // Stdout must be written AFTER the alternate screen is torn down,
        // otherwise the YAML lands in the alt buffer and vanishes on restore.
        DumpTarget::Stdout => {
            if let Some(yaml) = md_tui::sidemark::render(dump_inputs) {
                print!("{yaml}");
            }
        }
    }
}

struct RunOutcome {
    /// Path of the file shown at exit (or `None` for stdin / file tree).
    /// Captured here because `markdown` is owned inside `run_app`.
    file_name: Option<String>,
}

struct CliArgs {
    file: Option<String>,
    username: Option<String>,
}

fn parse_cli(args: &[String]) -> Result<CliArgs, String> {
    let mut file: Option<String> = None;
    let mut username: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-u" | "--username" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("error: {arg} requires a value"))?;
                username = Some(value.clone());
                i += 2;
            }
            s if s.starts_with("--username=") => {
                username = Some(s["--username=".len()..].to_string());
                i += 1;
            }
            _ => {
                if file.is_none() {
                    file = Some(arg.clone());
                }
                i += 1;
            }
        }
    }
    Ok(CliArgs { file, username })
}

struct AppEnvironment {
    markdown: ComponentRoot,
    watcher: Option<PollWatcher>,
}

impl AppEnvironment {
    fn new(width: u16, tx: mpsc::Sender<notify::Result<notify::Event>>) -> Self {
        let watcher = match PollWatcher::new(
            tx,
            Config::default().with_poll_interval(Duration::from_secs(1)),
        ) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("warning: file watcher unavailable, live reload disabled: {e}");
                None
            }
        };

        let markdown = parse_markdown(None, EMPTY_FILE, width.saturating_sub(1));

        Self { markdown, watcher }
    }

    fn load_initial_content(
        &mut self,
        app: &mut App,
        file_arg: Option<String>,
        potential_input: &io::Stdin,
    ) {
        let mut stdin_buf = String::new();

        if let Some(arg) = file_arg.as_deref() {
            if let Ok(file) = read_to_string(arg) {
                let path = std::path::Path::new(arg);
                if let Some(w) = self.watcher.as_mut() {
                    let _ = w.watch(path, notify::RecursiveMode::NonRecursive);
                }
                self.markdown = parse_markdown(Some(arg), &file, app.width() - 2);
                app.raw_source = Some(file);
                app.mode = Mode::View;
                app.help_box.set_mode(Mode::View);
                let (marks, w) = bookmarks::load_for(path);
                app.bookmarks = marks;
                app.bookmark_origin_width = w;
            } else {
                app.message_box
                    .set_message(format!("Could not open file {arg}"));
                app.boxes = Boxes::Error;
            }
        } else if !potential_input.is_terminal() {
            let _ = potential_input.lock().read_to_string(&mut stdin_buf);
            self.markdown = parse_markdown(None, &stdin_buf, app.width() - 2);
            app.raw_source = Some(stdin_buf.clone());
            app.mode = Mode::View;
            app.help_box.set_mode(Mode::View);
        }
    }
}

fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    tick_rate: Duration,
    file_arg: Option<String>,
) -> io::Result<RunOutcome> {
    let (f_tx, f_rx) = mpsc::channel::<Option<MdFile>>();
    thread::spawn(move || find_md_files_channel(f_tx.clone()));

    let (tx, rx) = mpsc::channel();
    let mut env = AppEnvironment::new(app.width(), tx);

    app.set_width(terminal.size()?.width - 1);
    env.load_initial_content(app, file_arg, &io::stdin());

    let mut last_tick = Instant::now();
    let mut file_tree = FileTree::default();

    loop {
        let height = terminal.size()?.height;

        handle_watcher_events(&rx, app, &mut env.markdown, height);

        if handle_resize(terminal, app, &mut env.markdown) {
            continue;
        }

        // Vertical resizes don't trigger handle_resize (which keys on width),
        // but they can leave vertical_scroll past the visible viewport. Clamp
        // every tick so terminal-height changes can't hide content.
        app.clamp_scroll(env.markdown.height(), height);

        env.markdown.set_scroll(app.vertical_scroll);

        terminal.draw(|f| {
            render_main_ui(f, app, &mut env.markdown, &mut file_tree, &f_rx);
            render_overlays(f, app, height);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)?
            && let Some(outcome) = handle_input(
                terminal,
                app,
                &mut env.markdown,
                &mut file_tree,
                &mut env.watcher,
                height,
            )?
        {
            return Ok(outcome);
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn handle_watcher_events(
    rx: &mpsc::Receiver<notify::Result<notify::Event>>,
    app: &mut App,
    markdown: &mut ComponentRoot,
    height: u16,
) {
    for event in rx.try_iter().flatten() {
        if let notify::EventKind::Modify(_) = event.kind
            && let Some(file_name) = markdown.file_name()
            && let Ok(file) = read_to_string(file_name)
        {
            *markdown = parse_markdown(Some(file_name), &file, app.width() - 2);
            app.raw_source = Some(file);
            app.mode = Mode::View;
            app.help_box.set_mode(Mode::View);
            app.resync_comments_after_reparse(markdown);
            app.clamp_scroll(markdown.height(), height);
            break;
        }
    }
}

fn handle_resize(terminal: &DefaultTerminal, app: &mut App, markdown: &mut ComponentRoot) -> bool {
    if let Ok(size) = terminal.size()
        && app.set_width(size.width - 1)
    {
        let url = if let Some(url) = markdown.file_name() {
            url
        } else {
            app.mode = Mode::FileTree;
            app.help_box.set_mode(Mode::FileTree);
            return true;
        };

        if let Ok(text) = read_to_string(url) {
            app.vertical_scroll = 0;
            *markdown = parse_markdown(Some(url), &text, app.width() - 2);
            app.raw_source = Some(text);
            app.resync_comments_after_reparse(markdown);
        } else {
            app.message_box
                .set_message(format!("Could not open file {url}"));
            app.boxes = Boxes::Error;
            app.mode = Mode::FileTree;
            app.help_box.set_mode(Mode::FileTree);
        }
        return true;
    }
    false
}

fn render_main_ui(
    f: &mut Frame,
    app: &App,
    markdown: &mut ComponentRoot,
    file_tree: &mut FileTree,
    f_rx: &mpsc::Receiver<Option<MdFile>>,
) {
    match app.mode {
        Mode::View => {
            render_markdown(f, app, markdown);
        }
        Mode::FileTree => {
            if !file_tree.loaded() {
                while let Ok(e) = f_rx.try_recv() {
                    match e {
                        Some(file) => file_tree.add_file(file),
                        None => {
                            file_tree.finish();
                            break;
                        }
                    }
                }
            }
            md_tui::pages::file_explorer::render_file_tree(f, app, file_tree.clone());
        }
    }
}

/// A `(width, height)` box centered horizontally on `app_width` and vertically
/// at `term_height / 2`. `saturating_sub` avoids an underflow panic when the
/// box is wider than the terminal.
fn centered(app_width: u16, term_height: u16, width: u16, height: u16) -> Rect {
    Rect {
        x: app_width.saturating_sub(width) / 2,
        y: term_height / 2,
        width,
        height,
    }
}

fn render_overlays(f: &mut Frame, app: &App, height: u16) {
    match app.boxes {
        Boxes::Search => {
            let (search_height, search_width) = app.search_box.dimensions();
            let search_area = Rect {
                x: app.search_box.x(),
                y: app.search_box.y(),
                width: search_width,
                height: search_height,
            };
            f.render_widget(app.search_box.clone(), search_area);
        }
        Boxes::Error => {
            let (error_height, error_width) = app.message_box.dimensions();
            if app.width() > error_width {
                let error_area = centered(app.width(), height, error_width, error_height);
                f.render_widget(Clear, error_area);
                f.render_widget(app.message_box.clone(), error_area);
            }
        }
        Boxes::LinkPreview => {
            let (link_height, link_width) = app.link_box.dimensions();
            let link_area = centered(app.width(), height, link_width, link_height);
            f.render_widget(Clear, link_area);
            f.render_widget(app.link_box.clone(), link_area);
        }
        Boxes::None => {}
    }
}

fn handle_input(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    markdown: &mut ComponentRoot,
    file_tree: &mut FileTree,
    watcher: &mut Option<PollWatcher>,
    height: u16,
) -> io::Result<Option<RunOutcome>> {
    match event::read()? {
        Event::Key(key) => {
            if key.kind != event::KeyEventKind::Press {
                return Ok(None);
            }
            match handle_keyboard_input(key.code, app, markdown, file_tree, height, watcher) {
                KeyBoardAction::Exit => {
                    return Ok(Some(RunOutcome {
                        file_name: markdown.file_name().map(str::to_string),
                    }));
                }
                KeyBoardAction::Continue => {}
                KeyBoardAction::Edit => {
                    // `terminal.draw`'s closure can't return a Result, so capture
                    // `open_editor`'s outcome and propagate it after the draw.
                    let mut edit_result = Ok(());
                    terminal.draw(|f| {
                        edit_result = open_editor(f, app, markdown.file_name());
                    })?;
                    edit_result?;
                    enable_mouse();
                }
            }
        }
        Event::Mouse(mouse) => {
            let term_width = terminal.size()?.width;
            md_tui::event_handler::handle_mouse_input(mouse, app, markdown, term_width, height);
        }
        _ => {}
    }
    Ok(None)
}

fn render_markdown(f: &mut Frame, app: &App, markdown: &mut ComponentRoot) {
    let size = f.area();
    let area = markdown_view_area(size.width, size.height, app.width());

    render_markdown_content(f, markdown, area);
    apply_comment_highlights(f, app, &area);
    render_caret(f, app, &area);

    // Detect the "user wants the sidebar but the terminal is too narrow"
    // case so the footer can surface it instead of silently dropping the
    // sidebar.
    let sb_x = area.x + area.width;
    let sidebar_fits = SIDEBAR_WIDTH.min(size.width.saturating_sub(sb_x)) > 0;
    let wants_sidebar = app.shows_comment_sidebar();
    let sidebar_hidden = wants_sidebar && !sidebar_fits;

    if wants_sidebar && sidebar_fits {
        render_comment_sidebar_ui(f, app, markdown, &area, size);
    }

    if GENERAL_CONFIG.help_menu {
        render_help_menu_ui(f, app, &area, size);
    }

    if GENERAL_CONFIG.footer {
        render_footer_ui(f, app, size, sidebar_hidden);
    }
}

fn render_markdown_content(f: &mut Frame, markdown: &mut ComponentRoot, area: Rect) {
    for child in markdown.children_mut() {
        match child {
            Component::TextComponent(comp) => {
                if comp.is_hidden() {
                    continue;
                }
                if comp.y_offset().saturating_sub(comp.scroll_offset()) >= area.height
                    || (comp.y_offset() + comp.height()).saturating_sub(comp.scroll_offset()) == 0
                {
                    continue;
                }
                f.render_widget(comp.clone(), area);
            }
            Component::Image(img) => {
                render_markdown_image(f, img, area);
            }
        }
    }
}

fn render_markdown_image(f: &mut Frame, img: &mut ImageComponent, area: Rect) {
    if img.y_offset().saturating_sub(img.scroll_offset()) >= area.height
        || (img.y_offset() + img.height()).saturating_sub(img.scroll_offset()) == 0
    {
        return;
    }

    let image = StatefulImage::default().resize(Resize::Fit(Some(FilterType::Nearest)));

    // Resize height based on clipping top and bottom
    let height = cmp::min(
        img.height(),
        (img.y_offset() + img.height()).saturating_sub(img.scroll_offset()),
    );
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

    f.render_stateful_widget(image, inner_area, img.image_mut());
}

fn render_caret(f: &mut Frame, app: &App, area: &Rect) {
    if let Some((cx, cy)) = caret_screen_pos(app, area) {
        let buf = f.buffer_mut();
        if let Some(cell) = buf.cell_mut((cx, cy)) {
            let prev = cell.style();
            cell.set_style(prev.add_modifier(Modifier::REVERSED));
        }
    }
}

/// Translate the current `Editing` comment state into an `EditingDraft`, or
/// `None` when no draft is being edited.
fn editing_draft<'a>(
    app: &'a App,
    markdown: &ComponentRoot,
) -> Option<md_tui::boxes::comment_sidebar::EditingDraft<'a>> {
    use md_tui::comments::{CommentState, EditTarget};
    match &app.comment_state {
        CommentState::Editing {
            range,
            draft,
            cursor,
            target,
            ..
        } => Some(md_tui::boxes::comment_sidebar::EditingDraft {
            range: *range,
            source: markdown.resolve_selection_to_source(range.0, range.1),
            draft,
            cursor: *cursor,
            replaces_saved_idx: match target {
                EditTarget::Existing(i) => Some(*i),
                EditTarget::New => None,
            },
        }),
        _ => None,
    }
}

/// Where the `i`-th comment's card sits in the sidebar. Prefers the projected
/// render position; an empty projection means the source span no longer maps
/// onto the view, so fall back to the comment's original source line so the
/// orphaned card stays near where it was instead of jumping to the top.
fn card_anchor(app: &App, i: usize, comment: &md_tui::comments::Comment) -> md_tui::util::Caret {
    app.comment_projections
        .get(i)
        .and_then(|p| p.rendered.first().map(|r| r.start))
        .unwrap_or(md_tui::util::Caret {
            line: u16::try_from(comment.source.start.line.saturating_sub(1)).unwrap_or(u16::MAX),
            col: 0,
        })
}

/// Build the sidebar cards: every saved comment (skipping the one being
/// edited, which is shown as the draft) followed by the draft card, if any.
fn build_comment_boxes<'a>(
    app: &'a App,
    draft: &'a Option<md_tui::boxes::comment_sidebar::EditingDraft<'a>>,
) -> Vec<md_tui::boxes::comment_sidebar::CommentBox<'a>> {
    use md_tui::boxes::comment_sidebar::{CommentBox, CommentBoxState};

    let replaced_idx = draft.as_ref().and_then(|d| d.replaces_saved_idx);
    let mut boxes = Vec::new();

    for (i, c) in app.comments.iter().enumerate() {
        if Some(i) == replaced_idx {
            continue;
        }
        let state = if app.active_comment == Some(i) && draft.is_none() {
            CommentBoxState::Active(c)
        } else {
            CommentBoxState::Inactive(c)
        };
        boxes.push(CommentBox {
            state,
            anchor: card_anchor(app, i, c),
            header: app.username.as_deref(),
        });
    }

    if let Some(d) = draft {
        boxes.push(CommentBox {
            state: CommentBoxState::Editing(d),
            anchor: d.range.0,
            header: app.username.as_deref(),
        });
    }

    boxes
}

fn render_comment_sidebar_ui(
    f: &mut Frame,
    app: &App,
    markdown: &ComponentRoot,
    area: &Rect,
    size: Rect,
) {
    use md_tui::boxes::comment_sidebar::CommentSideBar;

    let sb_x = area.x + area.width;
    let sb_w = SIDEBAR_WIDTH.min(size.width.saturating_sub(sb_x));
    if sb_w == 0 {
        return;
    }

    let sb_area = Rect {
        x: sb_x,
        y: area.y,
        width: sb_w,
        height: area.height,
    };

    let draft = editing_draft(app, markdown);
    let sidebar = CommentSideBar {
        boxes: build_comment_boxes(app, &draft),
        markdown_scroll: app.vertical_scroll,
    };

    f.render_widget(Clear, sb_area);
    f.render_widget(sidebar, sb_area);
}

fn render_help_menu_ui(f: &mut Frame, app: &App, area: &Rect, size: Rect) {
    const HELP_BLOCK_HEIGHT: u16 = 30;
    const HELP_CONTENT_HEIGHT: u16 = 28;
    let footer_h: u16 = if GENERAL_CONFIG.footer { 1 } else { 0 };

    let block = Block::default().bg(Color::Black);

    // Per-state sizing. `block_h`/`content_basis` drive the bottom-anchored
    // y offsets; `content_h` is the content box's own height. (Collapsed keeps
    // a 3-row content box positioned as if 1 row — preserved verbatim.)
    let (block_h, content_basis, content_h) = if app.help_box.expanded() {
        (HELP_BLOCK_HEIGHT, HELP_CONTENT_HEIGHT, HELP_CONTENT_HEIGHT)
    } else {
        (3, 1, 3)
    };
    let block_area = Rect {
        x: area.x,
        y: size.height.saturating_sub(block_h + 1 + footer_h),
        width: area.width - 1,
        height: cmp::min(block_h, size.height),
    };
    let help_area = Rect {
        x: area.x + 2,
        y: size.height.saturating_sub(content_basis + 2 + footer_h),
        width: app.width() - 5,
        height: cmp::min(content_h, size.height),
    };

    f.render_widget(Clear, block_area);
    f.render_widget(block, block_area);

    if app.boxes != Boxes::Search {
        f.render_widget(app.help_box, help_area);
    }
}

fn render_footer_ui(f: &mut Frame, app: &App, size: Rect, sidebar_hidden: bool) {
    let footer_area = Rect {
        x: 0,
        y: size.height.saturating_sub(1),
        width: size.width,
        height: 1,
    };
    render_footer(f, app, footer_area, sidebar_hidden);
}

/// Paints `style` over cells in the document-space range `[start, end)`.
/// `end.col` is treated as exclusive. `vertical_scroll` is the document line
/// currently rendered at `area.y`. Lines outside the visible scroll window
/// are skipped silently.
fn paint_range(
    buf: &mut ratatui::buffer::Buffer,
    area: &Rect,
    vertical_scroll: u16,
    start: md_tui::util::Caret,
    end: md_tui::util::Caret,
    style: Style,
) {
    if start == end {
        return;
    }
    let line_min = vertical_scroll;
    let line_max = vertical_scroll + area.height;
    for line in start.line..=end.line {
        if line < line_min || line >= line_max {
            continue;
        }
        paint_line_range(buf, area, line, line_min, start, end, style);
    }
}

fn paint_line_range(
    buf: &mut ratatui::buffer::Buffer,
    area: &Rect,
    line: u16,
    line_min: u16,
    start: md_tui::util::Caret,
    end: md_tui::util::Caret,
    style: Style,
) {
    let row = area.y + (line - line_min);
    let col_start = if line == start.line { start.col } else { 0 };
    let col_end = if line == end.line {
        end.col
    } else {
        area.width
    };
    let x0 = area.x + col_start.min(area.width.saturating_sub(1));
    let x1 = area.x + col_end.min(area.width);
    for x in x0..x1 {
        if let Some(cell) = buf.cell_mut((x, row)) {
            let prev = cell.style();
            cell.set_style(prev.patch(style));
        }
    }
}

fn apply_comment_highlights(f: &mut Frame, app: &App, area: &Rect) {
    apply_persisted_comment_highlights(f, app, area);
    apply_active_selection_highlight(f, app, area);
}

fn apply_persisted_comment_highlights(f: &mut Frame, app: &App, area: &Rect) {
    let dim_style = Style::default().bg(Color::DarkGray);
    let active_style = Style::default().bg(Color::Blue);

    for (i, projection) in app.comment_projections.iter().enumerate() {
        let style = if app.active_comment == Some(i) {
            active_style
        } else {
            dim_style
        };
        let buf = f.buffer_mut();
        for range in &projection.rendered {
            paint_range(
                buf,
                area,
                app.vertical_scroll,
                range.start,
                range.end,
                style,
            );
        }
    }
}

fn apply_active_selection_highlight(f: &mut Frame, app: &App, area: &Rect) {
    use md_tui::comments::CommentState;
    let active_style = Style::default().bg(Color::Blue);

    if let CommentState::Selecting { anchor, .. } = app.comment_state {
        let (s, e) = md_tui::comments::normalize_range(anchor, app.caret);
        let e_inclusive = md_tui::util::Caret {
            line: e.line,
            col: e.col.saturating_add(1),
        };
        let buf = f.buffer_mut();
        paint_range(buf, area, app.vertical_scroll, s, e_inclusive, active_style);
    }
}

fn caret_screen_pos(app: &App, area: &Rect) -> Option<(u16, u16)> {
    // Caret is internal state (used to drive jumps and outline-mark logic
    // even in reading mode), but only RENDERED in caret mode. Otherwise the
    // user sees a stray highlighted cell wherever they last clicked.
    if !app.caret_mode {
        return None;
    }
    if area.width == 0 || area.height == 0 {
        return None;
    }
    if app.caret.line < app.vertical_scroll {
        return None;
    }
    let rel_y = app.caret.line - app.vertical_scroll;
    if rel_y >= area.height {
        return None;
    }
    let cy = area.y + rel_y;
    let cx = area.x + app.caret.col.min(area.width - 1);
    Some((cx, cy))
}

fn open_editor(f: &mut Frame, app: &mut App, file_name: Option<&str>) -> io::Result<()> {
    let editor = if let Ok(editor) = env::var("EDITOR") {
        editor
    } else {
        app.message_box
            .set_message("No editor found. Please set the EDITOR environment variable".to_owned());
        app.boxes = Boxes::Error;
        return Ok(());
    };

    let file_name = if let Some(file_name) = file_name {
        file_name
    } else {
        app.message_box
            .set_message("No file found to open in editor".to_owned());
        app.boxes = Boxes::Error;
        return Ok(());
    };

    // Terminal-mode transitions are propagated rather than `unwrap`ed: a failure
    // here would otherwise panic mid-teardown and strand the terminal in a
    // half-restored state. On `Err`, `main` still runs `ratatui::restore()`.
    disable_raw_mode()?;
    // Drop the terminal's mouse mode so the child editor doesn't inherit it.
    // run_app's loop will reapply on the next iteration based on caret_mode.
    disable_mouse();
    execute!(io::stdout(), LeaveAlternateScreen)?;
    execute!(io::stdout(), cursor::Show)?;

    match std::process::Command::new(&editor).arg(file_name).spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => {
            app.message_box
                .set_message(format!("Failed to open editor '{editor}': {e}"));
            app.boxes = Boxes::Error;
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    app.boxes = Boxes::None;
    f.render_widget(Clear, f.area());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use md_tui::comments::{
        Comment, CommentModeSource, CommentState, EditTarget, ProjectedCommentAnchor, RenderedRange,
    };
    use md_tui::parser::{SourcePos, SourceSpan};
    use md_tui::util::Caret;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    fn backend_text(buf: &Buffer) -> String {
        let area = buf.area;
        let mut s = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    fn caret(line: u16, col: u16) -> Caret {
        Caret { line, col }
    }

    fn dummy_span() -> SourceSpan {
        SourceSpan {
            start: SourcePos {
                byte: 0,
                line: 1,
                column: 1,
            },
            end: SourcePos {
                byte: 1,
                line: 1,
                column: 1,
            },
        }
    }

    // --- paint_range geometry ---------------------------------------------

    #[test]
    fn paint_range_paints_single_line_segment_exclusive_end() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let style = Style::default().bg(Color::Blue);
        paint_range(&mut buf, &area, 0, caret(0, 2), caret(0, 5), style);

        assert_eq!(
            buf[(1, 0)].style().bg,
            Some(Color::Reset),
            "before the range untouched"
        );
        assert_eq!(buf[(2, 0)].style().bg, Some(Color::Blue));
        assert_eq!(buf[(4, 0)].style().bg, Some(Color::Blue));
        assert_eq!(
            buf[(5, 0)].style().bg,
            Some(Color::Reset),
            "end column is exclusive"
        );
    }

    #[test]
    fn paint_range_patches_style_keeping_existing_content() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        // Pre-existing rendered content: a red 'X'.
        if let Some(cell) = buf.cell_mut((2, 0)) {
            cell.set_symbol("X");
            cell.set_style(Style::default().fg(Color::Red));
        }
        paint_range(
            &mut buf,
            &area,
            0,
            caret(0, 2),
            caret(0, 3),
            Style::default().bg(Color::Blue),
        );
        let cell = &buf[(2, 0)];
        assert_eq!(cell.symbol(), "X", "highlight preserves the glyph");
        assert_eq!(cell.style().fg, Some(Color::Red), "fg preserved");
        assert_eq!(cell.style().bg, Some(Color::Blue), "bg added via patch");
    }

    #[test]
    fn paint_range_clips_above_viewport() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        // vertical_scroll 5 -> visible lines 5..8; a line-0 range paints nothing.
        paint_range(
            &mut buf,
            &area,
            5,
            caret(0, 0),
            caret(0, 4),
            Style::default().bg(Color::Blue),
        );
        for x in 0..10 {
            assert_eq!(buf[(x, 0)].style().bg, Some(Color::Reset));
        }
    }

    #[test]
    fn paint_range_spans_three_line_segments() {
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        let style = Style::default().bg(Color::Blue);
        // start col 3 on line 0, full middle line 1, up to col 4 on line 2.
        paint_range(&mut buf, &area, 0, caret(0, 3), caret(2, 4), style);

        // First line: from start.col to end of width.
        assert_eq!(buf[(2, 0)].style().bg, Some(Color::Reset));
        assert_eq!(buf[(3, 0)].style().bg, Some(Color::Blue));
        assert_eq!(buf[(9, 0)].style().bg, Some(Color::Blue));
        // Middle line: full width.
        assert_eq!(buf[(0, 1)].style().bg, Some(Color::Blue));
        assert_eq!(buf[(9, 1)].style().bg, Some(Color::Blue));
        // Last line: up to (exclusive) end.col.
        assert_eq!(buf[(3, 2)].style().bg, Some(Color::Blue));
        assert_eq!(
            buf[(4, 2)].style().bg,
            Some(Color::Reset),
            "end column exclusive"
        );
    }

    #[test]
    fn paint_range_noop_when_start_equals_end() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        paint_range(
            &mut buf,
            &area,
            0,
            caret(1, 2),
            caret(1, 2),
            Style::default().bg(Color::Blue),
        );
        for y in 0..3 {
            for x in 0..10 {
                assert_eq!(buf[(x, y)].style().bg, Some(Color::Reset));
            }
        }
    }

    // --- caret_screen_pos --------------------------------------------------

    #[test]
    fn caret_screen_pos_none_outside_caret_mode() {
        let mut app = App::default();
        app.caret_mode = false;
        app.caret = caret(1, 1);
        assert_eq!(caret_screen_pos(&app, &Rect::new(0, 0, 10, 5)), None);
    }

    #[test]
    fn caret_screen_pos_maps_visible_caret() {
        let mut app = App::default();
        app.caret_mode = true;
        app.caret = caret(3, 4);
        app.vertical_scroll = 0;
        assert_eq!(
            caret_screen_pos(&app, &Rect::new(0, 0, 10, 5)),
            Some((4, 3))
        );
    }

    #[test]
    fn caret_screen_pos_none_when_scrolled_past() {
        let mut app = App::default();
        app.caret_mode = true;
        // Above the viewport.
        app.caret = caret(2, 0);
        app.vertical_scroll = 5;
        assert_eq!(caret_screen_pos(&app, &Rect::new(0, 0, 10, 5)), None);
        // Below the viewport.
        app.caret = caret(10, 0);
        app.vertical_scroll = 0;
        assert_eq!(caret_screen_pos(&app, &Rect::new(0, 0, 10, 5)), None);
    }

    #[test]
    fn caret_screen_pos_clamps_column_to_width() {
        let mut app = App::default();
        app.caret_mode = true;
        app.caret = caret(0, 20);
        app.vertical_scroll = 0;
        // col clamped to width-1 = 9.
        assert_eq!(
            caret_screen_pos(&app, &Rect::new(0, 0, 10, 5)),
            Some((9, 0))
        );
    }

    // --- highlight wrappers (need a Frame) --------------------------------

    fn projection(rendered: Vec<RenderedRange>) -> ProjectedCommentAnchor {
        ProjectedCommentAnchor {
            source: dummy_span(),
            rendered,
        }
    }

    #[test]
    fn persisted_highlights_use_blue_for_active_dim_for_rest() {
        let mut app = App::default();
        app.comment_projections = vec![
            projection(vec![RenderedRange {
                start: caret(0, 0),
                end: caret(0, 3),
            }]),
            projection(vec![RenderedRange {
                start: caret(1, 0),
                end: caret(1, 3),
            }]),
        ];
        app.active_comment = Some(0);

        let area = Rect::new(0, 0, 10, 5);
        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        terminal
            .draw(|f| apply_comment_highlights(f, &app, &area))
            .unwrap();
        let buf = terminal.backend().buffer();

        assert_eq!(
            buf[(0, 0)].style().bg,
            Some(Color::Blue),
            "active comment is blue"
        );
        assert_eq!(
            buf[(0, 1)].style().bg,
            Some(Color::DarkGray),
            "inactive comment is dim gray"
        );
    }

    #[test]
    fn active_selection_highlight_is_inclusive_of_caret() {
        let mut app = App::default();
        app.comment_state = CommentState::Selecting {
            anchor: caret(0, 1),
            source: CommentModeSource::Caret,
        };
        app.caret = caret(0, 3);

        let area = Rect::new(0, 0, 10, 5);
        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        terminal
            .draw(|f| apply_comment_highlights(f, &app, &area))
            .unwrap();
        let buf = terminal.backend().buffer();

        assert_eq!(
            buf[(0, 0)].style().bg,
            Some(Color::Reset),
            "before anchor untouched"
        );
        assert_eq!(buf[(1, 0)].style().bg, Some(Color::Blue), "anchor painted");
        assert_eq!(
            buf[(3, 0)].style().bg,
            Some(Color::Blue),
            "caret cell included (inclusive end)"
        );
        assert_eq!(
            buf[(4, 0)].style().bg,
            Some(Color::Reset),
            "one past caret untouched"
        );
    }

    // --- sidebar orchestration (render_comment_sidebar_ui) ----------------

    fn comment(text: &str, src_line: u32) -> Comment {
        Comment {
            source: SourceSpan {
                start: SourcePos {
                    byte: 0,
                    line: src_line,
                    column: 1,
                },
                end: SourcePos {
                    byte: 1,
                    line: src_line,
                    column: 1,
                },
            },
            text: text.to_string(),
            selected_text: None,
        }
    }

    /// Render the sidebar into an 80x20 backend and return its visible text.
    fn render_sidebar_text(app: &App) -> String {
        let md = md_tui::parser::parse_markdown(None, "", 40);
        let area = Rect::new(0, 0, 40, 20);
        let size = Rect::new(0, 0, 80, 20);
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|f| render_comment_sidebar_ui(f, app, &md, &area, size))
            .unwrap();
        backend_text(terminal.backend().buffer())
    }

    #[test]
    fn editing_card_replaces_its_saved_card() {
        let mut app = App::default();
        app.comments = vec![comment("OLDTEXT", 1), comment("SECONDC", 2)];
        app.comment_projections = vec![
            projection(vec![RenderedRange {
                start: caret(0, 0),
                end: caret(0, 3),
            }]),
            projection(vec![RenderedRange {
                start: caret(1, 0),
                end: caret(1, 3),
            }]),
        ];
        app.active_comment = Some(0);
        app.comment_state = CommentState::Editing {
            range: (caret(0, 0), caret(0, 1)),
            draft: "NEWDRAFT".to_string(),
            cursor: 8,
            target: EditTarget::Existing(0),
            source: CommentModeSource::Comments,
        };

        let text = render_sidebar_text(&app);
        assert!(text.contains("NEWDRAFT"), "the draft is shown");
        assert!(text.contains("SECONDC"), "other saved comments still shown");
        assert!(
            !text.contains("OLDTEXT"),
            "the saved card being edited is hidden (replaced by the draft)"
        );
    }

    #[test]
    fn orphaned_comment_still_renders_a_card() {
        // A comment whose projection has no rendered ranges (its source span no
        // longer maps onto the view) must still get a card via the source-line
        // fallback anchor, not be silently dropped.
        let mut app = App::default();
        app.comments = vec![comment("ORPHANED", 5)];
        app.comment_projections = vec![projection(vec![])];
        app.comment_state = CommentState::Browsing;

        assert!(
            render_sidebar_text(&app).contains("ORPHANED"),
            "orphaned comment keeps a card"
        );
    }

    // --- MDT_DUMP_PATH routing --------------------------------------------

    #[test]
    fn resolve_dump_target_unset_or_empty_is_stdout() {
        assert_eq!(resolve_dump_target(None), DumpTarget::Stdout);
        assert_eq!(
            resolve_dump_target(Some(std::ffi::OsString::new())),
            DumpTarget::Stdout
        );
    }

    #[test]
    fn resolve_dump_target_path_is_file() {
        assert_eq!(
            resolve_dump_target(Some(std::ffi::OsString::from("/tmp/out.yaml"))),
            DumpTarget::File(std::path::PathBuf::from("/tmp/out.yaml"))
        );
    }

    #[test]
    fn dump_comments_writes_yaml_to_file_target() {
        let path = std::env::temp_dir().join(format!("mdt_dump_test_{}.yaml", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let comments = vec![comment("a note", 1)];
        let inputs = md_tui::sidemark::DumpInputs {
            document: Some("doc.md"),
            author: Some("me"),
            comments: &comments,
            raw_source: Some("hello"),
        };
        dump_comments(inputs, DumpTarget::File(path.clone()));

        let written = std::fs::read_to_string(&path).expect("dump file written");
        assert!(written.contains("mrsf_version: \"1.0\""));
        assert!(written.contains("a note"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dump_comments_file_target_writes_nothing_without_comments() {
        let path = std::env::temp_dir().join(format!("mdt_dump_empty_{}.yaml", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let inputs = md_tui::sidemark::DumpInputs {
            document: Some("doc.md"),
            author: None,
            comments: &[],
            raw_source: None,
        };
        dump_comments(inputs, DumpTarget::File(path.clone()));

        assert!(
            !path.exists(),
            "no comments -> render returns None -> file untouched"
        );
    }
}
