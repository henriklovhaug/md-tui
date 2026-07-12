use std::{cmp, fs::read_to_string};

use crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use notify::{PollWatcher, Watcher};

use crate::{
    bookmarks,
    comments::CommentState,
    nodes::{root::ComponentRoot, word::WordType},
    pages::{file_explorer::FileTree, markdown_renderer::markdown_view_area},
    parser::parse_markdown,
    util::{
        App, Boxes, Caret, Jump, LinkType, Mode, PendingInput, ScrollAction, SearchAction,
        SelectAction,
        general::GENERAL_CONFIG,
        keys::{Action, key_to_action},
    },
};

/// If the caret sits on a TOC link (`#` anchor), record the outline mark and
/// jump the viewport to that heading, centered. Caret follows unconditionally
/// (used for `''`-swap correctness even outside caret mode, where it stays
/// invisible). No-op when the caret isn't on a TOC link or the heading can't
/// be resolved.
fn try_outline_jump(app: &mut App, markdown: &mut ComponentRoot, vh: u16, height: u16) {
    let Some(idx) = markdown.link_index_at_caret(app.caret) else {
        return;
    };
    let Some(anchor) = markdown.link_anchor_at(idx) else {
        return;
    };
    if !anchor.starts_with('#') {
        return;
    }
    let Ok(heading_line) = markdown.heading_offset(anchor) else {
        return;
    };

    app.set_outline_mark();
    app.vertical_scroll = heading_line.saturating_sub(vh / 2);
    app.clamp_scroll(markdown.height(), height);
    // Caret tracks the heading even in scroll mode — it's internal state
    // there (not rendered) but `''` compares `caret.line != mark.line` to
    // decide between swap and clear, so it must move.
    app.caret.line = heading_line;
    app.caret.col = 0;
}

pub(crate) fn viewport_height(height: u16) -> u16 {
    let mut h = height;
    if GENERAL_CONFIG.help_menu {
        h = h.saturating_sub(5);
    }
    if GENERAL_CONFIG.footer {
        h = h.saturating_sub(1);
    }
    // Never return 0: callers divide and take the modulo by this (file
    // explorer paging), so a viewport of 0 on a tiny terminal would panic.
    h.max(1)
}

fn handle_comment_mode_key(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    vh: u16,
) -> bool {
    match &app.comment_state {
        CommentState::Off => handle_comment_state_none(key, app, vh),
        CommentState::Browsing => handle_comment_state_browsing(key, app, vh),
        CommentState::Selecting { .. } => handle_comment_state_selecting(key, app, markdown),
        CommentState::Editing { .. } => handle_comment_state_editing(key, app, markdown),
    }
}

fn handle_comment_state_none(key: KeyCode, app: &mut App, _vh: u16) -> bool {
    use Action::*;
    match key_to_action(key) {
        // `n`/`N` must NOT be hijacked here: while comment mode is off they
        // belong to search-next/previous (dispatched later in the handler).
        // Comment cycling lives in `Browsing` only.
        Enter => app.start_editing_active_or_caret(),
        _ => false,
    }
}

fn handle_comment_state_browsing(key: KeyCode, app: &mut App, vh: u16) -> bool {
    use Action::*;
    match key_to_action(key) {
        EnterCommentMode | Escape => {
            app.exit_comment_mode();
            true
        }
        StartCommentSelect => {
            let _ = app.start_selecting();
            true
        }
        SearchNext => {
            app.cycle_comment(true, vh);
            true
        }
        SearchPrevious => {
            app.cycle_comment(false, vh);
            true
        }
        Enter => app.start_editing_active_or_caret(),
        _ => false,
    }
}

fn handle_comment_state_selecting(key: KeyCode, app: &mut App, markdown: &ComponentRoot) -> bool {
    use Action::*;
    match key_to_action(key) {
        Enter => {
            app.commit_selection_to_editing(markdown);
            true
        }
        // Per the design, `c` is consumed only in `Browsing`; while `Selecting`
        // it is a no-op (it must not tear down the in-progress selection).
        EnterCommentMode => true,
        Escape => {
            app.cancel_editing();
            true
        }
        _ => false,
    }
}

fn handle_comment_state_editing(key: KeyCode, app: &mut App, markdown: &mut ComponentRoot) -> bool {
    match key {
        KeyCode::Esc => {
            app.cancel_editing();
            true
        }
        KeyCode::Enter => {
            app.save_draft(markdown);
            true
        }
        KeyCode::Backspace => {
            if let CommentState::Editing { draft, cursor, .. } = &mut app.comment_state
                && *cursor > 0
            {
                // `cursor` is a byte offset kept on a char boundary (it only
                // ever moves by whole `char`s), so step back to the previous
                // boundary and remove that whole `char` — correct for multi-byte
                // input, never a mid-codepoint panic.
                let prev = draft[..*cursor]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(i, _)| i);
                draft.remove(prev);
                *cursor = prev;
            }
            true
        }
        KeyCode::Char(c) => {
            if let CommentState::Editing { draft, cursor, .. } = &mut app.comment_state {
                // `cursor` is always on a char boundary, so inserting any
                // `char` is safe; advance by its UTF-8 byte length.
                draft.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            true
        }
        _ => true,
    }
}

fn persist_marks(markdown: &ComponentRoot, app: &mut App) {
    if let Some(path) = markdown.file_name() {
        let p = std::path::PathBuf::from(path);
        let _ = bookmarks::save_for(&p, &app.bookmarks, GENERAL_CONFIG.width);
        app.bookmark_origin_width = GENERAL_CONFIG.width;
    }
}

pub enum KeyBoardAction {
    Continue,
    Edit,
    Exit,
}

/// Mouse-driven flow:
///
///   - `Down(Left)` on a TOC line → outline jump.
///   - `Down(Left)` inside a saved comment anchor → open it for editing
///     (same path as Enter in caret mode).
///   - `Down(Left)` elsewhere → arm a potential selection (no state change
///     until a `Drag` arrives, so a click without drag is just a caret
///     repositioning).
///   - `Drag(Left)` after an armed Down → first drag transitions to
///     `CommentState::Selecting` with the original Down position as anchor;
///     every drag updates `caret` to the drag position (clamped to the
///     area edges).
///   - `Up(Left)` → disarm. If `Selecting` was reached, the highlight stays
///     until Enter commits or Esc cancels.
///   - Scroll wheel → adjust `vertical_scroll`.
///
/// Mouse capture is on in both reading and caret mode, so this fires
/// uniformly. Right/middle clicks are intentionally ignored.
pub fn handle_mouse_input(
    mouse: MouseEvent,
    app: &mut App,
    markdown: &mut ComponentRoot,
    term_width: u16,
    term_height: u16,
) {
    if app.mode != Mode::View || app.boxes != Boxes::None {
        return;
    }

    let vh = viewport_height(term_height);
    // Scroll wheel always works, even while editing a comment — keeping
    // it captive made the sidebar feel stuck.
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.vertical_scroll = app.vertical_scroll.saturating_sub(3);
            return;
        }
        MouseEventKind::ScrollDown => {
            app.vertical_scroll = app.vertical_scroll.saturating_add(3);
            app.clamp_scroll(markdown.height(), vh);
            return;
        }
        _ => {}
    }

    // Clicks / drags inside the markdown view would move the caret and
    // disturb the comment's anchored range, so block those while editing.
    if matches!(app.comment_state, CommentState::Editing { .. }) {
        return;
    }

    let area = markdown_view_area(term_width, term_height, app.width());
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_mouse_down(mouse, app, markdown, &area, vh, term_height);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            handle_mouse_drag(mouse, app, markdown, &area);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.mouse_drag_anchor = None;
        }
        _ => {}
    }
}

/// Map a left-click at the mouse's (row, col) to a document caret, or `None`
/// when the click lands outside the markdown area or past rendered content.
fn project_click(
    mouse: MouseEvent,
    app: &App,
    markdown: &ComponentRoot,
    area: &ratatui::layout::Rect,
) -> Option<Caret> {
    let in_area = mouse.column >= area.x
        && mouse.column < area.x + area.width
        && mouse.row >= area.y
        && mouse.row < area.y + area.height;
    if !in_area {
        return None;
    }
    let line = app.vertical_scroll + (mouse.row - area.y);
    if line >= markdown.height() {
        return None;
    }
    Some(Caret {
        line,
        col: mouse.column - area.x,
    })
}

fn handle_mouse_down(
    mouse: MouseEvent,
    app: &mut App,
    markdown: &mut ComponentRoot,
    area: &ratatui::layout::Rect,
    vh: u16,
    term_height: u16,
) {
    let Some(click) = project_click(mouse, app, markdown, area) else {
        app.mouse_drag_anchor = None;
        return;
    };

    // Click on a TOC (`#`) link: jump the outline, don't start a selection.
    if let Some(idx) = markdown.link_index_at_caret(click)
        && let Some(anchor) = markdown.link_anchor_at(idx)
        && anchor.starts_with('#')
    {
        app.caret = click;
        try_outline_jump(app, markdown, vh, term_height);
        app.mouse_drag_anchor = None;
        return;
    }

    // A click in scroll mode promotes to caret mode (and remembers to demote
    // again once the comment edit it may start finishes).
    let was_in_scroll = !app.caret_mode;
    if was_in_scroll {
        app.toggle_caret_mode(vh);
    }
    if matches!(app.comment_state, CommentState::Selecting { .. }) {
        app.comment_state = CommentState::Off;
    }
    app.caret = click;
    if was_in_scroll {
        app.auto_caret_for_comment_edit = true;
    }

    // If the click landed on an existing comment, open it; otherwise arm a
    // drag so a subsequent move starts a selection.
    if app.start_editing_active_or_caret() {
        app.mouse_drag_anchor = None;
    } else {
        app.mouse_drag_anchor = Some(app.caret);
    }
}

fn handle_mouse_drag(
    mouse: MouseEvent,
    app: &mut App,
    markdown: &ComponentRoot,
    area: &ratatui::layout::Rect,
) {
    let Some(drag_anchor) = app.mouse_drag_anchor else {
        return;
    };

    let clamped_col = mouse
        .column
        .clamp(area.x, area.x + area.width.saturating_sub(1));
    let clamped_row = mouse
        .row
        .clamp(area.y, area.y + area.height.saturating_sub(1));
    let max_line = markdown.height().saturating_sub(1);
    let doc_line = cmp::min(app.vertical_scroll + (clamped_row - area.y), max_line);
    let doc_col = clamped_col - area.x;

    if !matches!(app.comment_state, CommentState::Selecting { .. }) {
        let _ = app.start_selecting_with_anchor(drag_anchor);
    }
    app.caret.line = doc_line;
    app.caret.col = doc_col;
}

pub fn handle_keyboard_input(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    file_tree: &mut FileTree,
    height: u16,
    watcher: &mut Option<PollWatcher>,
) -> KeyBoardAction {
    let is_in_textbox =
        app.boxes == Boxes::Search || matches!(app.comment_state, CommentState::Editing { .. });

    if key == KeyCode::Char(crate::util::keys::KEY_CONFIG.quit) && !is_in_textbox {
        return KeyBoardAction::Exit;
    }
    match app.mode {
        Mode::FileTree => keyboard_mode_file_tree(key, app, markdown, file_tree, height, watcher),
        Mode::View => keyboard_mode_view(key, app, markdown, height, watcher),
    }
}

pub fn keyboard_mode_file_tree(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    file_tree: &mut FileTree,
    height: u16,
    watcher: &mut Option<PollWatcher>,
) -> KeyBoardAction {
    match app.boxes {
        Boxes::Error => handle_error_box_key(key, app),
        Boxes::Search => handle_file_tree_search_key(key, app, file_tree, height),
        Boxes::None => {
            return handle_file_tree_none_box_key(key, app, markdown, file_tree, height, watcher);
        }
        Boxes::LinkPreview => {
            if key == KeyCode::Esc {
                app.boxes = Boxes::None;
            }
        }
    }

    KeyBoardAction::Continue
}

fn handle_file_tree_search_key(key: KeyCode, app: &mut App, file_tree: &mut FileTree, height: u16) {
    match key {
        KeyCode::Esc => {
            app.search_box.clear();
            file_tree.search(None);
            file_tree.restore_pre_search();
            app.boxes = Boxes::None;
        }
        KeyCode::Enter => {
            let query = app.search_box.consume();
            file_tree.search(Some(&query));
            app.boxes = Boxes::None;
        }
        KeyCode::Char(c) => {
            app.search_box.insert(c);
            file_tree.search(app.search_box.content());
            let file_height = file_tree.height(height);
            app.search_box.set_position(10, file_height as u16 + 2);
        }
        KeyCode::Backspace => {
            let was_empty = app.search_box.content().is_none();
            app.search_box.delete();
            file_tree.search(app.search_box.content());
            if was_empty {
                file_tree.restore_pre_search();
                app.boxes = Boxes::None;
            }
            let file_height = file_tree.height(height);
            app.search_box.set_position(10, file_height as u16 + 2);
        }
        _ => {}
    }
}

fn handle_file_tree_none_box_key(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    file_tree: &mut FileTree,
    height: u16,
    watcher: &mut Option<PollWatcher>,
) -> KeyBoardAction {
    match key_to_action(key) {
        Action::Down => file_tree.next(height),
        Action::Up => file_tree.previous(height),
        Action::PageDown => file_tree.next_page(height),
        Action::PageUp => file_tree.previous_page(height),
        Action::ToTop => file_tree.first(),
        Action::ToBottom => file_tree.last(height),
        Action::Enter => {
            if let Some(res) = handle_file_tree_enter_action(app, markdown, file_tree, watcher) {
                return res;
            }
        }
        Action::Search => {
            let file_height = file_tree.height(height);
            app.search_box.set_position(10, file_height as u16 + 2);
            app.search_box.set_width(20);
            app.boxes = Boxes::Search;
            file_tree.snapshot_pre_search();
            app.help_box.close();
        }
        Action::Back => handle_back_action(app, markdown, watcher),
        Action::Help if GENERAL_CONFIG.help_menu => app.help_box.toggle(),
        Action::Escape => {
            file_tree.unselect();
            file_tree.search(None);
        }
        Action::Sort | Action::Outline => file_tree.sort_name(),
        _ => {}
    }
    KeyBoardAction::Continue
}

/// Shared tail of the three file-open paths: parse `text` as the document
/// named `name` at `path`, point the file watcher at it, and load its
/// bookmarks. The caller owns `reset`/scroll/mode and must set `app.raw_source`
/// itself *after* any `reset` (which clears it).
fn open_document(
    app: &mut App,
    markdown: &mut ComponentRoot,
    path: &std::path::Path,
    name: &str,
    text: &str,
    watcher: &mut Option<PollWatcher>,
) {
    *markdown = parse_markdown(Some(name), text, app.width() - 2);
    if let Some(w) = watcher.as_mut() {
        let _ = w.watch(path, notify::RecursiveMode::NonRecursive);
    }
    let (marks, bw) = bookmarks::load_for(path);
    app.bookmarks = marks;
    app.bookmark_origin_width = bw;
}

fn handle_file_tree_enter_action(
    app: &mut App,
    markdown: &mut ComponentRoot,
    file_tree: &FileTree,
    watcher: &mut Option<PollWatcher>,
) -> Option<KeyBoardAction> {
    let file = if let Some(file) = file_tree.selected() {
        file
    } else {
        app.message_box.set_message("No file selected".to_string());
        app.boxes = Boxes::Error;
        return Some(KeyBoardAction::Continue);
    };

    let text = if let Ok(file) = read_to_string(file.path_str()) {
        app.reset();
        file
    } else {
        app.message_box
            .set_message(format!("Could not open file {}", file.path_str()));
        app.boxes = Boxes::Error;
        return Some(KeyBoardAction::Continue);
    };

    open_document(app, markdown, file.path(), file.path_str(), &text, watcher);
    app.raw_source = Some(text);
    app.mode = Mode::View;
    app.help_box.set_mode(Mode::View);
    app.select_index = 0;
    None
}

fn handle_error_box_key(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Enter | KeyCode::Esc => {
            app.boxes = Boxes::None;
        }
        _ => {}
    }
}

fn handle_search_box_key(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    height: u16,
) -> KeyBoardAction {
    match key {
        KeyCode::Esc => {
            app.search_box.clear();
            app.boxes = Boxes::None;
        }
        KeyCode::Enter => {
            let query = app.search_box.content_str();

            markdown.deselect();

            markdown.find_and_mark(query);

            let heights = markdown.search_results_heights();

            if heights.is_empty() {
                app.message_box
                    .set_message(format!("No results found for\n {query}"));
                app.boxes = Boxes::Error;
                return KeyBoardAction::Continue;
            }

            let next = heights
                .iter()
                .find(|row| **row >= (app.vertical_scroll as usize + height as usize / 2));

            if let Some(index) = next {
                app.vertical_scroll = (*index as u16).saturating_sub(height / 2);
                app.clamp_scroll(markdown.height(), height);
            }

            app.boxes = Boxes::None;
        }
        KeyCode::Char(c) => {
            app.search_box.insert(c);
        }
        KeyCode::Backspace => {
            app.search_box.delete();
        }
        _ => {}
    }
    KeyBoardAction::Continue
}

fn handle_enter_action(
    app: &mut App,
    markdown: &mut ComponentRoot,
    vh: u16,
    height: u16,
    watcher: &mut Option<PollWatcher>,
) -> Option<KeyBoardAction> {
    // In caret mode the caret position — not any prior j/k or
    // outline-cycle selection — is what Enter activates. If the
    // caret is on a link line, select that link; otherwise
    // clear any stale selection so Enter is a no-op.
    if app.caret_mode {
        if let Some(idx) = markdown.link_index_at_caret(app.caret) {
            if markdown.select(idx).is_ok() {
                app.select_index = idx;
                app.selected = true;
            }
        } else {
            markdown.deselect();
            app.selected = false;
        }
    }

    if !app.selected {
        return None;
    }
    let Some(link) = markdown.selected() else {
        markdown.deselect();
        app.selected = false;
        return Some(KeyBoardAction::Continue);
    };
    let prev_type = markdown
        .selected_underlying_type()
        .unwrap_or(WordType::Normal);

    if prev_type == WordType::FootnoteInline {
        app.message_box.set_message(markdown.find_footnote(link));
        app.boxes = Boxes::Error;
        markdown.deselect();
        app.selected = false;
        return Some(KeyBoardAction::Continue);
    }

    match LinkType::from(link) {
        LinkType::Internal { heading } => {
            handle_internal_link(heading, app, markdown, vh, height);
        }
        LinkType::External(url) => handle_external_link(url),
        LinkType::MarkdownFile { path, heading } => {
            handle_markdown_file_link(path, heading, app, markdown, height, watcher);
        }
    }
    markdown.deselect();
    app.selected = false;
    Some(KeyBoardAction::Continue)
}

fn handle_external_link(url: &str) {
    let _ = open::that(url);
}

fn handle_internal_link(
    heading: &str,
    app: &mut App,
    markdown: &ComponentRoot,
    vh: u16,
    height: u16,
) {
    if let Ok(index) = markdown.heading_offset(heading) {
        // Center the heading line in the viewport.
        let centered = index.saturating_sub(vh / 2);
        app.vertical_scroll = centered;
        app.clamp_scroll(markdown.height(), height);
        if app.caret_mode {
            app.caret.line = index;
            app.caret.col = 0;
        }
    } else {
        app.message_box
            .set_message(format!("Could not find heading {heading}"));
        app.boxes = Boxes::Error;
    }
}

fn handle_markdown_file_link(
    path: String,
    heading: Option<String>,
    app: &mut App,
    markdown: &mut ComponentRoot,
    height: u16,
    watcher: &mut Option<PollWatcher>,
) {
    let text = if let Ok(file) = read_to_string(&path) {
        file
    } else {
        app.message_box
            .set_message(format!("Could not open file {path}"));
        app.boxes = Boxes::Error;
        return;
    };

    // Record where we came from before `markdown` is replaced below.
    if let Some(file_name) = markdown.file_name() {
        app.history.push(Jump::File(file_name.to_string()));
    }

    // Reset the *old* document's state (caret, comments, scroll, …) before
    // loading the new one — must happen before `raw_source` is set, since
    // `reset` clears it (and would also wipe a heading-not-found error box).
    app.reset();

    let path_buf = std::path::Path::new(&path);
    open_document(app, markdown, path_buf, &path, &text, watcher);
    app.raw_source = Some(text);

    if let Some(heading) = heading {
        if let Ok(index) = markdown.heading_offset(&format!("#{heading}")) {
            app.vertical_scroll = index;
            app.clamp_scroll(markdown.height(), height);
        } else {
            app.message_box
                .set_message(format!("Could not find heading {heading}"));
            app.boxes = Boxes::Error;
        }
    }
}

fn handle_caret_mode(
    key: KeyCode,
    action: &Action,
    app: &mut App,
    markdown: &ComponentRoot,
    vh: u16,
) -> bool {
    if !app.caret_mode {
        return false;
    }

    let max = markdown.height();

    // Caret moves expressed as a `(dy, dx)` delta. Horizontal moves arrive
    // both as arrow keys and as `h`/`l` (which map to the HalfPage actions),
    // so both spellings collapse to the same delta here.
    let delta = match key {
        KeyCode::Left => Some((0, -1)),
        KeyCode::Right => Some((0, 1)),
        _ => match action {
            Action::Down => Some((1, 0)),
            Action::Up => Some((-1, 0)),
            Action::HalfPageDown => Some((0, 1)),
            Action::HalfPageUp => Some((0, -1)),
            Action::PageDown => Some(((vh / 2) as i32, 0)),
            Action::PageUp => Some((-((vh / 2) as i32), 0)),
            _ => None,
        },
    };
    if let Some((dy, dx)) = delta {
        app.move_caret(dy, dx, max, vh);
        return true;
    }

    match action {
        Action::ToTop => {
            app.caret_to_top(vh);
            true
        }
        Action::ToBottom => {
            app.caret_to_bottom(max, vh);
            true
        }
        Action::CaretLineStart => {
            app.caret_to_line_start();
            true
        }
        Action::CaretLineEnd => {
            app.caret_to_line_end();
            true
        }
        Action::Escape => {
            // Esc always exits caret mode; any outline mark is cleared as
            // a side effect so the next caret session starts fresh.
            app.clear_outline_mark();
            app.toggle_caret_mode(vh);
            true
        }
        _ => false,
    }
}

fn handle_pending_input(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    vh: u16,
) -> bool {
    if let Some(pending) = app.pending_input.take() {
        if let KeyCode::Char(c) = key {
            match pending {
                PendingInput::BookmarkSet if c.is_ascii_lowercase() => {
                    app.set_bookmark(c);
                    persist_marks(markdown, app);
                }
                PendingInput::BookmarkJump => {
                    if c == crate::util::keys::KEY_CONFIG.bookmark_jump {
                        // Double single quote: jump to outline mark
                        app.jump_to_outline_mark(vh);
                    } else if c.is_ascii_lowercase() {
                        let _ = app.jump_bookmark(c, vh);
                    }
                }
                _ => {}
            }
        }
        return true;
    }
    false
}

fn handle_none_box_key(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    height: u16,
    watcher: &mut Option<PollWatcher>,
) -> KeyBoardAction {
    let vh = viewport_height(height);

    // Pending-input capture (bookmark set/jump) takes the very next key, so it
    // must run before comment dispatch — otherwise a `n`/`N` capture target
    // would be swallowed by comment cycling while a bookmark name is pending.
    if handle_pending_input(key, app, markdown, vh) {
        return KeyBoardAction::Continue;
    }

    // Comment mode dispatch. Returns true when the key was consumed, false to
    // fall through to the rest of the handler.
    if handle_comment_mode_key(key, app, markdown, vh) {
        return KeyBoardAction::Continue;
    }

    let action = key_to_action(key);

    if handle_caret_mode(key, &action, app, markdown, vh) {
        return KeyBoardAction::Continue;
    }

    match action {
        // Universal actions, available regardless of mode.
        Action::ToggleCaretMode => app.toggle_caret_mode(vh),
        Action::EnterCommentMode => {
            let _ = app.toggle_comment_mode();
        }
        Action::StartCommentSelect => {
            if app.caret_mode {
                let _ = app.start_selecting();
            }
        }
        Action::BookmarkSetPending => app.pending_input = Some(PendingInput::BookmarkSet),
        Action::BookmarkJumpPending => app.pending_input = Some(PendingInput::BookmarkJump),
        Action::Outline => try_outline_jump(app, markdown, vh, height),

        Action::Down => app.scroll(ScrollAction::Down, markdown, height),
        Action::Up => app.scroll(ScrollAction::Up, markdown, height),
        Action::ToTop => app.scroll(ScrollAction::ToTop, markdown, height),
        Action::ToBottom => app.scroll(ScrollAction::ToBottom, markdown, height),
        Action::HalfPageDown => app.scroll(ScrollAction::HalfPageDown, markdown, height),
        Action::HalfPageUp => app.scroll(ScrollAction::HalfPageUp, markdown, height),
        Action::PageDown => app.scroll(ScrollAction::PageDown, markdown, height),
        Action::PageUp => app.scroll(ScrollAction::PageUp, markdown, height),

        Action::Hover => app.handle_selection(SelectAction::Hover, markdown, height),
        Action::SelectLinkAlt => {
            app.handle_selection(SelectAction::SelectLinkAlt, markdown, height)
        }
        Action::SelectLink => app.handle_selection(SelectAction::SelectLink, markdown, height),

        Action::SearchNext => app.handle_search(SearchAction::Next, markdown, height),
        Action::SearchPrevious => app.handle_search(SearchAction::Previous, markdown, height),

        Action::Search => app.start_search(height),

        Action::ToFileTree => app.navigate_to_file_tree(markdown),

        Action::Edit => return KeyBoardAction::Edit,

        Action::Escape => {
            app.selected = false;
            markdown.deselect();
            app.clear_outline_mark();
        }

        Action::Enter => {
            if let Some(res) = handle_enter_action(app, markdown, vh, height, watcher) {
                return res;
            }
        }

        Action::Back => handle_back_action(app, markdown, watcher),

        Action::Help if GENERAL_CONFIG.help_menu => {
            app.help_box.toggle();
        }
        _ => {}
    }

    KeyBoardAction::Continue
}

fn handle_back_action(
    app: &mut App,
    markdown: &mut ComponentRoot,
    watcher: &mut Option<PollWatcher>,
) {
    match app.history.pop() {
        Jump::File(e) => {
            let text = if let Ok(file) = read_to_string(&e) {
                app.vertical_scroll = 0;
                file
            } else {
                app.message_box
                    .set_message(format!("Could not open file {e}"));
                app.boxes = Boxes::Error;
                return;
            };
            let path = std::path::Path::new(&e);
            app.reset();
            open_document(app, markdown, path, &e, &text, watcher);
            app.raw_source = Some(text);
            app.mode = Mode::View;
            app.help_box.set_mode(Mode::View);
        }
        Jump::FileTree => {
            markdown.clear();
            app.reset();
            app.mode = Mode::FileTree;
            app.help_box.set_mode(Mode::FileTree);
        }
    }
}

fn keyboard_mode_view(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    height: u16,
    watcher: &mut Option<PollWatcher>,
) -> KeyBoardAction {
    match app.boxes {
        Boxes::Error => handle_error_box_key(key, app),
        Boxes::Search => return handle_search_box_key(key, app, markdown, height),
        Boxes::None => return handle_none_box_key(key, app, markdown, height, watcher),
        Boxes::LinkPreview => {
            if key == KeyCode::Esc {
                app.boxes = Boxes::None;
            }
        }
    }
    KeyBoardAction::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::{
        Comment, CommentModeSource, CommentState, EditTarget, ProjectedCommentAnchor, RenderedRange,
    };
    use crate::parser::{SourcePos, SourceSpan};
    use crate::util::keys::KEY_CONFIG;
    use crossterm::event::KeyModifiers;
    use ratatui::layout::Rect;

    const VH: u16 = 18;

    fn caret(line: u16, col: u16) -> Caret {
        Caret { line, col }
    }

    fn empty_markdown() -> ComponentRoot {
        parse_markdown(None, "", 40)
    }

    fn doc() -> ComponentRoot {
        // A real paragraph so the view has height and words carry source spans.
        parse_markdown(None, "hello world foo bar", 40)
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

    fn push_comment(app: &mut App, start: Caret, end: Caret) {
        app.comments.push(Comment {
            source: dummy_span(),
            text: String::new(),
            selected_text: None,
        });
        app.comment_projections.push(ProjectedCommentAnchor {
            source: dummy_span(),
            rendered: vec![RenderedRange { start, end }],
        });
    }

    fn draft_of(app: &App) -> Option<(String, usize)> {
        if let CommentState::Editing { draft, cursor, .. } = &app.comment_state {
            Some((draft.clone(), *cursor))
        } else {
            None
        }
    }

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        }
    }

    // --- key dispatch ------------------------------------------------------

    #[test]
    fn browsing_c_exits_comment_mode() {
        let mut app = App::default();
        app.comment_state = CommentState::Browsing;
        let mut md = empty_markdown();
        assert!(handle_comment_mode_key(
            KeyCode::Char(KEY_CONFIG.comment),
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn browsing_select_key_starts_selecting() {
        let mut app = App::default();
        app.comment_state = CommentState::Browsing;
        app.caret = caret(0, 2);
        let mut md = empty_markdown();
        // Derive the key from config so a rebind can't silently break dispatch.
        assert!(handle_comment_mode_key(
            KeyCode::Char(KEY_CONFIG.comment_select),
            &mut app,
            &mut md,
            VH
        ));
        assert!(matches!(
            app.comment_state,
            CommentState::Selecting {
                anchor,
                source: CommentModeSource::Comments,
            } if anchor == caret(0, 2)
        ));
    }

    #[test]
    fn browsing_n_cycles_to_active_comment() {
        let mut app = App::default();
        app.comment_state = CommentState::Browsing;
        push_comment(&mut app, caret(0, 0), caret(0, 5));
        let mut md = empty_markdown();
        assert!(handle_comment_mode_key(
            KeyCode::Char(KEY_CONFIG.search_next),
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(app.active_comment, Some(0));
    }

    #[test]
    fn selecting_escape_returns_to_browsing() {
        let mut app = App::default();
        app.comment_state = CommentState::Selecting {
            anchor: caret(0, 0),
            source: CommentModeSource::Comments,
        };
        let mut md = empty_markdown();
        assert!(handle_comment_mode_key(KeyCode::Esc, &mut app, &mut md, VH));
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn selecting_c_is_a_consumed_noop() {
        let mut app = App::default();
        let state = CommentState::Selecting {
            anchor: caret(0, 0),
            source: CommentModeSource::Comments,
        };
        app.comment_state = state.clone();
        let mut md = empty_markdown();
        // The comment key is consumed (returns true) but must NOT tear down
        // the in-progress selection.
        assert!(handle_comment_mode_key(
            KeyCode::Char(KEY_CONFIG.comment),
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(app.comment_state, state);
    }

    #[test]
    fn editing_char_inserts_and_backspace_deletes() {
        let mut app = App::default();
        app.comment_state = CommentState::Editing {
            range: (caret(0, 0), caret(0, 0)),
            draft: String::new(),
            cursor: 0,
            target: EditTarget::New,
            source: CommentModeSource::Caret,
        };
        let mut md = empty_markdown();

        handle_comment_mode_key(KeyCode::Char('h'), &mut app, &mut md, VH);
        handle_comment_mode_key(KeyCode::Char('i'), &mut app, &mut md, VH);
        assert_eq!(draft_of(&app), Some(("hi".to_string(), 2)));

        handle_comment_mode_key(KeyCode::Backspace, &mut app, &mut md, VH);
        assert_eq!(draft_of(&app), Some(("h".to_string(), 1)));
    }

    #[test]
    fn editing_accepts_non_ascii_and_backspaces_whole_chars() {
        let mut app = App::default();
        app.comment_state = CommentState::Editing {
            range: (caret(0, 0), caret(0, 0)),
            draft: String::new(),
            cursor: 0,
            target: EditTarget::New,
            source: CommentModeSource::Caret,
        };
        let mut md = empty_markdown();

        // 'é' is 2 bytes: it is inserted and the byte cursor advances by 2.
        assert!(handle_comment_mode_key(
            KeyCode::Char('é'),
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(draft_of(&app), Some(("é".to_string(), 2)));

        // Backspace removes the whole multi-byte char without panicking.
        assert!(handle_comment_mode_key(
            KeyCode::Backspace,
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(draft_of(&app), Some((String::new(), 0)));
    }

    #[test]
    fn editing_enter_saves_new_comment() {
        let mut app = App::default();
        app.raw_source = Some("hello world foo bar".to_string());
        app.comment_state = CommentState::Editing {
            // Cols 0..5 cover the first word "hello".
            range: (caret(0, 0), caret(0, 5)),
            draft: "a note".to_string(),
            cursor: 6,
            target: EditTarget::New,
            source: CommentModeSource::Caret,
        };
        let mut md = doc();
        assert!(handle_comment_mode_key(
            KeyCode::Enter,
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(app.comments.len(), 1, "Enter persists the draft");
        assert_eq!(app.comments[0].text, "a note");
        // Caret-sourced edit restores to Off on save.
        assert_eq!(app.comment_state, CommentState::Off);
    }

    // --- mouse flow --------------------------------------------------------

    #[test]
    fn mouse_down_arms_selection_without_changing_state() {
        let mut app = App::default();
        let mut md = doc();
        let area = Rect::new(0, 0, 40, 20);
        handle_mouse_down(
            mouse(MouseEventKind::Down(MouseButton::Left), 2, 0),
            &mut app,
            &mut md,
            &area,
            VH,
            20,
        );
        assert_eq!(app.mouse_drag_anchor, Some(caret(0, 2)));
        assert_eq!(
            app.comment_state,
            CommentState::Off,
            "a bare click is not a selection"
        );
        assert!(
            app.caret_mode,
            "clicking from scroll mode enters caret mode"
        );
    }

    #[test]
    fn mouse_drag_promotes_to_selecting_with_down_anchor() {
        let mut app = App::default();
        let mut md = doc();
        let area = Rect::new(0, 0, 40, 20);
        handle_mouse_down(
            mouse(MouseEventKind::Down(MouseButton::Left), 2, 0),
            &mut app,
            &mut md,
            &area,
            VH,
            20,
        );
        handle_mouse_drag(
            mouse(MouseEventKind::Drag(MouseButton::Left), 6, 0),
            &mut app,
            &md,
            &area,
        );
        assert!(
            matches!(
                app.comment_state,
                CommentState::Selecting { anchor, .. } if anchor == caret(0, 2)
            ),
            "selection anchors at the original down position, not the drag point"
        );
        assert_eq!(app.caret, caret(0, 6), "caret follows the drag");
    }

    #[test]
    fn mouse_click_inside_comment_opens_editing() {
        let mut app = App::default();
        push_comment(&mut app, caret(0, 0), caret(0, 5));
        let mut md = doc();
        let area = Rect::new(0, 0, 40, 20);
        handle_mouse_down(
            mouse(MouseEventKind::Down(MouseButton::Left), 2, 0),
            &mut app,
            &mut md,
            &area,
            VH,
            20,
        );
        assert!(matches!(app.comment_state, CommentState::Editing { .. }));
        assert_eq!(
            app.mouse_drag_anchor, None,
            "opening an edit clears the drag arm"
        );
    }

    #[test]
    fn mouse_up_disarms_drag_anchor() {
        let mut app = App::default();
        app.mode = Mode::View;
        app.comment_state = CommentState::Selecting {
            anchor: caret(0, 2),
            source: CommentModeSource::Caret,
        };
        app.mouse_drag_anchor = Some(caret(0, 2));
        let mut md = doc();
        handle_mouse_input(
            mouse(MouseEventKind::Up(MouseButton::Left), 6, 0),
            &mut app,
            &mut md,
            40,
            20,
        );
        assert_eq!(app.mouse_drag_anchor, None);
        // Up leaves the selection highlight in place until Enter/Esc.
        assert!(matches!(app.comment_state, CommentState::Selecting { .. }));
    }

    #[test]
    fn mouse_input_ignored_outside_view_mode() {
        // The dispatcher guard (mode == View && boxes == None) must swallow
        // clicks in the file tree — the helper-level tests bypass this guard,
        // so pin it here. App::default() is Mode::FileTree.
        let mut app = App::default();
        let mut md = doc();
        handle_mouse_input(
            mouse(MouseEventKind::Down(MouseButton::Left), 2, 0),
            &mut app,
            &mut md,
            40,
            20,
        );
        assert_eq!(app.mouse_drag_anchor, None);
        assert!(
            !app.caret_mode,
            "a file-tree click must not enter caret mode"
        );
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn mouse_input_ignored_when_a_box_is_open() {
        let mut app = App::default();
        app.mode = Mode::View;
        app.boxes = Boxes::Search;
        let mut md = doc();
        handle_mouse_input(
            mouse(MouseEventKind::Down(MouseButton::Left), 2, 0),
            &mut app,
            &mut md,
            40,
            20,
        );
        assert_eq!(app.mouse_drag_anchor, None, "search box swallows the click");
    }

    // --- handle_pending_input ---------------------------------------------

    #[test]
    fn pending_input_noop_without_pending() {
        let mut app = App::default();
        let mut md = empty_markdown();
        assert!(!handle_pending_input(
            KeyCode::Char('a'),
            &mut app,
            &mut md,
            VH
        ));
    }

    #[test]
    fn pending_input_bookmark_set_records_caret() {
        let mut app = App::default();
        app.pending_input = Some(PendingInput::BookmarkSet);
        app.caret = caret(5, 3);
        let mut md = empty_markdown(); // no file name -> persist is a no-op
        assert!(handle_pending_input(
            KeyCode::Char('a'),
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(app.bookmarks.get(&'a'), Some(&caret(5, 3)));
        assert!(app.pending_input.is_none());
    }

    #[test]
    fn pending_input_bookmark_jump_moves_caret() {
        let mut app = App::default();
        app.bookmarks.insert('b', caret(9, 2));
        app.pending_input = Some(PendingInput::BookmarkJump);
        let mut md = empty_markdown();
        assert!(handle_pending_input(
            KeyCode::Char('b'),
            &mut app,
            &mut md,
            VH
        ));
        assert_eq!(app.caret, caret(9, 2));
    }

    // --- handle_caret_mode ------------------------------------------------

    #[test]
    fn caret_mode_dispatch_ignored_when_off() {
        let mut app = App::default();
        let md = doc();
        let key = KeyCode::Right;
        assert!(!handle_caret_mode(
            key,
            &key_to_action(key),
            &mut app,
            &md,
            VH
        ));
    }

    #[test]
    fn caret_mode_moves_caret_horizontally() {
        let mut app = App::default();
        app.set_width(10);
        app.caret_mode = true;
        let md = doc();
        let right = KeyCode::Right;
        assert!(handle_caret_mode(
            right,
            &key_to_action(right),
            &mut app,
            &md,
            VH
        ));
        assert_eq!(app.caret.col, 1);
        let left = KeyCode::Left;
        assert!(handle_caret_mode(
            left,
            &key_to_action(left),
            &mut app,
            &md,
            VH
        ));
        assert_eq!(app.caret.col, 0);
    }

    #[test]
    fn caret_mode_escape_exits() {
        let mut app = App::default();
        app.caret_mode = true;
        let md = doc();
        assert!(handle_caret_mode(
            KeyCode::Esc,
            &Action::Escape,
            &mut app,
            &md,
            VH
        ));
        assert!(!app.caret_mode);
    }

    // --- handle_none_box_key ----------------------------------------------

    #[test]
    fn none_box_key_toggles_caret_mode() {
        let mut app = App::default();
        let mut md = doc();
        let mut watcher: Option<PollWatcher> = None;
        let _ = handle_none_box_key(
            KeyCode::Char(KEY_CONFIG.toggle_caret),
            &mut app,
            &mut md,
            20,
            &mut watcher,
        );
        assert!(app.caret_mode);
    }

    #[test]
    fn none_box_key_arms_bookmark_pending() {
        let mut app = App::default();
        let mut md = doc();
        let mut watcher: Option<PollWatcher> = None;
        let _ = handle_none_box_key(
            KeyCode::Char(KEY_CONFIG.bookmark_set),
            &mut app,
            &mut md,
            20,
            &mut watcher,
        );
        assert_eq!(app.pending_input, Some(PendingInput::BookmarkSet));
    }

    #[test]
    fn none_box_key_escape_clears_selection() {
        let mut app = App::default();
        app.selected = true;
        let mut md = doc();
        let mut watcher: Option<PollWatcher> = None;
        let _ = handle_none_box_key(KeyCode::Esc, &mut app, &mut md, 20, &mut watcher);
        assert!(!app.selected);
    }

    // --- handle_enter_action (no-link paths; link-follow does IO) ---------

    #[test]
    fn enter_action_none_when_nothing_selected() {
        let mut app = App::default();
        let mut md = doc();
        let mut watcher: Option<PollWatcher> = None;
        assert!(handle_enter_action(&mut app, &mut md, VH, 20, &mut watcher).is_none());
    }

    #[test]
    fn enter_action_in_caret_mode_off_a_link_clears_selection() {
        let mut app = App::default();
        app.caret_mode = true;
        app.selected = true;
        app.caret = caret(0, 0); // "hello world foo bar" has no links
        let mut md = doc();
        let mut watcher: Option<PollWatcher> = None;
        assert!(handle_enter_action(&mut app, &mut md, VH, 20, &mut watcher).is_none());
        assert!(!app.selected);
    }

    // --- intentional behavior contracts (locked against regression) -------

    #[test]
    fn search_keys_fall_through_when_comment_mode_off() {
        // With comment mode Off, `n`/`N` belong to search and must NOT be
        // consumed by comment dispatch (so they reach the search handler).
        let mut app = App::default();
        push_comment(&mut app, caret(5, 0), caret(5, 1)); // a comment exists
        let n = KeyCode::Char(KEY_CONFIG.search_next);
        assert!(
            !handle_comment_state_none(n, &mut app, VH),
            "n must not be consumed in CommentState::Off"
        );
        assert_eq!(app.active_comment, None, "no comment cycling while Off");
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn armed_bookmark_wins_over_comment_cycling() {
        // Browsing with a comment present: an armed bookmark capture must take
        // the next key (`n`) instead of comment-cycling consuming it.
        let mut app = App::default();
        app.comment_state = CommentState::Browsing;
        app.caret = caret(7, 2);
        push_comment(&mut app, caret(5, 0), caret(5, 1));
        app.pending_input = Some(PendingInput::BookmarkSet);
        let mut md = doc();
        let mut watcher: Option<PollWatcher> = None;
        let _ = handle_none_box_key(
            KeyCode::Char(KEY_CONFIG.search_next),
            &mut app,
            &mut md,
            20,
            &mut watcher,
        );
        assert_eq!(
            app.bookmarks.get(&'n'),
            Some(&caret(7, 2)),
            "n set the bookmark"
        );
        assert!(app.pending_input.is_none());
        assert_eq!(app.active_comment, None, "comment was not cycled");
    }
}
