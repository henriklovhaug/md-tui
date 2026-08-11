use std::{cmp, fs::read_to_string};

use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use notify::{PollWatcher, Watcher};

use crate::{
    nodes::{root::ComponentRoot, word::WordType},
    pages::file_explorer::FileTree,
    parser::parse_markdown,
    util::{
        App, Boxes, Jump, LinkType, Mode,
        general::GENERAL_CONFIG,
        keys::{Action, key_to_action},
    },
};

pub enum KeyBoardAction {
    Continue,
    Edit,
    Exit,
}

/// Rows moved per wheel notch.
const MOUSE_SCROLL_LINES: u16 = 3;

/// Scroll the view down, clamped to the bottom of the document.
fn scroll_down(app: &mut App, markdown: &ComponentRoot, height: u16, lines: u16) {
    app.vertical_scroll = cmp::min(
        app.vertical_scroll.saturating_add(lines),
        markdown.height().saturating_sub(height / 2),
    );
}

/// Scroll the view up, clamped to the top of the document.
fn scroll_up(app: &mut App, lines: u16) {
    app.vertical_scroll = app.vertical_scroll.saturating_sub(lines);
}

pub fn handle_keyboard_input(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    file_tree: &mut FileTree,
    height: u16,
    watcher: &mut PollWatcher,
) -> KeyBoardAction {
    if key == KeyCode::Char('q') && app.boxes != Boxes::Search {
        return KeyBoardAction::Exit;
    }
    match app.mode {
        Mode::FileTree => keyboard_mode_file_tree(key, app, markdown, file_tree, height, watcher),
        Mode::View => keyboard_mode_view(key, app, markdown, height, watcher),
    }
}

/// Handle a mouse event. Only the wheel is wired up; clicks and drags are
/// ignored so that Shift-selection still behaves the way the terminal expects.
pub fn handle_mouse_input(
    mouse: MouseEvent,
    app: &mut App,
    markdown: &ComponentRoot,
    file_tree: &mut FileTree,
    height: u16,
) {
    let down = match mouse.kind {
        MouseEventKind::ScrollDown => true,
        MouseEventKind::ScrollUp => false,
        _ => return,
    };

    // An open box owns the viewport, so leave the scroll position alone.
    if app.boxes != Boxes::None {
        return;
    }

    match app.mode {
        // Scrolling moves the viewport only. Unlike `j`/`k` it never advances
        // link or details selection, which stays where the user put it.
        Mode::View => {
            if down {
                scroll_down(app, markdown, height, MOUSE_SCROLL_LINES);
            } else {
                scroll_up(app, MOUSE_SCROLL_LINES);
            }
        }
        // The file tree has no scroll offset independent of the selection, so
        // the wheel walks the selection the same way `j`/`k` do.
        Mode::FileTree => {
            for _ in 0..MOUSE_SCROLL_LINES {
                if down {
                    file_tree.next(height);
                } else {
                    file_tree.previous(height);
                }
            }
        }
    }
}

pub fn keyboard_mode_file_tree(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    file_tree: &mut FileTree,
    height: u16,
    watcher: &mut PollWatcher,
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
                app.search_box.clear();
                file_tree.search(None);
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
                if app.search_box.content().is_none() {
                    app.boxes = Boxes::None;
                }
                app.search_box.delete();
                file_tree.search(app.search_box.content());
                let file_height = file_tree.height(height);
                app.search_box.set_position(10, file_height as u16 + 2);
            }
            _ => {}
        },
        Boxes::None => match key_to_action(key) {
            Action::Down => {
                file_tree.next(height);
            }

            Action::Up => {
                file_tree.previous(height);
            }

            Action::PageDown => {
                file_tree.next_page(height);
            }

            Action::PageUp => {
                file_tree.previous_page(height);
            }

            Action::ToTop => {
                file_tree.first();
            }

            Action::ToBottom => {
                file_tree.last(height);
            }

            Action::Enter => {
                let file = if let Some(file) = file_tree.selected() {
                    file
                } else {
                    app.message_box.set_message("No file selected".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };
                let text = if let Ok(file) = read_to_string(file.path_str()) {
                    app.reset();
                    file
                } else {
                    app.message_box
                        .set_message(format!("Could not open file {}", file.path_str()));
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };

                *markdown = parse_markdown(Some(file.path_str()), &text, app.width() - 2);
                let _ = watcher.watch(file.path(), notify::RecursiveMode::NonRecursive);
                app.mode = Mode::View;
                app.help_box.set_mode(Mode::View);
                app.select_index = 0;
            }
            Action::Search => {
                let file_height = file_tree.height(height);
                app.search_box.set_position(10, file_height as u16 + 2);
                app.search_box.set_width(20);
                app.boxes = Boxes::Search;
                app.help_box.close();
            }

            Action::Back => match app.history.pop() {
                Jump::File(e) => {
                    let text = if let Ok(file) = read_to_string(&e) {
                        app.vertical_scroll = 0;
                        file
                    } else {
                        app.message_box
                            .set_message(format!("Could not open file {e}"));
                        app.boxes = Boxes::Error;
                        return KeyBoardAction::Continue;
                    };
                    *markdown = parse_markdown(Some(&e), &text, app.width() - 2);
                    let path = std::path::Path::new(&e);
                    let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
                    app.reset();
                    app.mode = Mode::View;
                    app.help_box.set_mode(Mode::View);
                }
                Jump::FileTree => {
                    markdown.clear();
                    app.mode = Mode::FileTree;
                    app.help_box.set_mode(Mode::FileTree);
                }
            },
            Action::Help if GENERAL_CONFIG.help_menu => {
                app.help_box.toggle();
            }
            Action::Help => {}

            Action::Escape => {
                file_tree.unselect();
                file_tree.search(None);
            }

            Action::Sort => {
                file_tree.sort_name();
            }
            _ => {}
        },
        Boxes::LinkPreview => {
            if key == KeyCode::Esc {
                app.boxes = Boxes::None;
            }
        }
    }

    KeyBoardAction::Continue
}

fn keyboard_mode_view(
    key: KeyCode,
    app: &mut App,
    markdown: &mut ComponentRoot,
    height: u16,
    watcher: &mut PollWatcher,
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
                    app.vertical_scroll = cmp::min(
                        (*index as u16).saturating_sub(height / 2),
                        markdown.height().saturating_sub(height / 2),
                    );
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
        },
        Boxes::None => match key_to_action(key) {
            Action::Down => {
                if app.selected {
                    app.select_index = cmp::min(app.select_index + 1, markdown.num_links() - 1);
                    app.vertical_scroll = if let Ok(scroll) = markdown.select(app.select_index) {
                        app.selected = true;
                        scroll.saturating_sub(height / 3)
                    } else {
                        app.vertical_scroll
                    };
                } else if app.details_selected {
                    let max_idx = markdown.num_details().saturating_sub(1);
                    app.details_select_index = cmp::min(app.details_select_index + 1, max_idx);
                    app.vertical_scroll =
                        if let Ok(scroll) = markdown.select_details(app.details_select_index) {
                            app.details_selected = true;
                            scroll.saturating_sub(height / 3)
                        } else {
                            app.vertical_scroll
                        };
                } else {
                    scroll_down(app, markdown, height, 1);
                }
            }
            Action::Up => {
                if app.selected {
                    app.select_index = app.select_index.saturating_sub(1);
                    app.vertical_scroll = if let Ok(scroll) = markdown.select(app.select_index) {
                        app.selected = true;
                        scroll.saturating_sub(height / 3)
                    } else {
                        app.vertical_scroll
                    };
                } else if app.details_selected {
                    app.details_select_index = app.details_select_index.saturating_sub(1);
                    app.vertical_scroll =
                        if let Ok(scroll) = markdown.select_details(app.details_select_index) {
                            app.details_selected = true;
                            scroll.saturating_sub(height / 3)
                        } else {
                            app.vertical_scroll
                        };
                } else {
                    scroll_up(app, 1);
                }
            }
            Action::ToTop => {
                app.vertical_scroll = 0;
            }
            Action::ToBottom => {
                app.vertical_scroll = markdown.height().saturating_sub(height / 2);
            }

            Action::HalfPageDown => {
                scroll_down(app, markdown, height, height / 2);
            }
            Action::HalfPageUp => {
                scroll_up(app, height / 2);
            }

            Action::PageDown => {
                scroll_down(app, markdown, height, height);
            }

            Action::PageUp => {
                scroll_up(app, height);
            }

            Action::Hover => {
                if app.selected {
                    let link = markdown.selected();

                    let prev_type = markdown.selected_underlying_type();

                    if prev_type == WordType::FootnoteInline {
                        app.link_box
                            .set_message(format!("Footnote: {}", markdown.find_footnote(link)));
                        app.boxes = Boxes::LinkPreview;
                        return KeyBoardAction::Continue;
                    }

                    let message = match LinkType::from(link) {
                        LinkType::Internal(e) => format!("Internal link: {e}"),
                        LinkType::External(e) => format!("External link: {e}"),
                        LinkType::MarkdownFile(e) => format!("Markdown file: {e}"),
                    };

                    app.link_box.set_message(message);
                    app.boxes = Boxes::LinkPreview;
                } else {
                    app.message_box.set_message("No link selected".to_string());
                    app.boxes = Boxes::Error;
                }
            }

            // Find the link closest to the middle, searching both ways
            Action::SelectLinkAlt => {
                let links = markdown.link_index_and_height();
                if links.is_empty() {
                    app.message_box.set_message("No links found".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                }

                let next = links
                    .iter()
                    .min_by_key(|(_, row)| (*row).abs_diff(app.vertical_scroll + height / 3));

                if let Some((index, _)) = next {
                    app.vertical_scroll = if let Ok(scroll) = markdown.select(*index) {
                        app.select_index = *index;
                        scroll.saturating_sub(height / 3)
                    } else {
                        app.vertical_scroll
                    };
                    app.selected = true;
                    app.details_selected = false;
                    markdown.deselect_details();
                } else {
                    // Something weird must have happened at this point
                    markdown.deselect();
                }
            }

            // Find the link closest to the to the top, searching downwards
            Action::SelectLink => {
                let mut links = markdown.link_index_and_height();
                if links.is_empty() {
                    app.message_box.set_message("No links found".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                }

                let mut index = usize::MAX;
                while let Some(top) = links.pop() {
                    if top.1 >= app.vertical_scroll || index == usize::MAX {
                        index = top.0;
                    } else {
                        break;
                    }
                }

                app.select_index = index;
                app.selected = true;
                app.details_selected = false;
                markdown.deselect_details();
                app.vertical_scroll = if let Ok(scroll) = markdown.select(app.select_index) {
                    scroll.saturating_sub(height / 3)
                } else {
                    app.vertical_scroll
                };
            }

            // Cycle to the details summary nearest (and at-or-below) the
            // current scroll position. Mirrors `SelectLink` but for
            // `<details>` blocks. Mutually exclusive with link selection.
            Action::SelectDetails => {
                let details = markdown.details_index_and_height();
                if details.is_empty() {
                    app.message_box
                        .set_message("No details blocks found".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                }

                // Clear any link selection first — the two modes are
                // mutually exclusive.
                app.selected = false;
                markdown.deselect();

                let next_idx = if app.details_selected {
                    // Already in details mode — advance to the next.
                    cmp::min(app.details_select_index + 1, details.len() - 1)
                } else {
                    // Pick the first summary at or below the current
                    // scroll position, else the last one above.
                    details
                        .iter()
                        .find(|(_, y)| *y >= app.vertical_scroll)
                        .map(|(i, _)| *i)
                        .unwrap_or(details.last().map(|(i, _)| *i).unwrap_or(0))
                };

                app.details_select_index = next_idx;
                app.details_selected = true;
                app.vertical_scroll = if let Ok(scroll) = markdown.select_details(next_idx) {
                    scroll.saturating_sub(height / 3)
                } else {
                    app.vertical_scroll
                };
            }

            Action::Search => {
                app.search_box.clear();
                app.search_box.set_position(2, height - 3);
                app.search_box.set_width(GENERAL_CONFIG.width - 3);
                app.boxes = Boxes::Search;
                app.help_box.close();
            }

            Action::ToFileTree => {
                app.mode = Mode::FileTree;
                app.help_box.set_mode(Mode::FileTree);
                if let Some(file) = markdown.file_name() {
                    app.history.push(Jump::File(file.to_string()));
                }
                app.reset();
            }

            Action::SearchNext => {
                let heights = markdown.search_results_heights();

                let next = heights
                    .iter()
                    .find(|row| **row > (app.vertical_scroll as usize + height as usize / 2));

                if let Some(index) = next {
                    app.vertical_scroll = cmp::min(
                        (*index as u16).saturating_sub(height / 2),
                        markdown.height().saturating_sub(height / 2),
                    );
                }
            }

            Action::SearchPrevious => {
                let heights = markdown.search_results_heights();

                let next = heights
                    .iter()
                    .rev()
                    .find(|row| **row < (app.vertical_scroll as usize + height as usize / 2));

                if let Some(index) = next {
                    app.vertical_scroll = cmp::min(
                        (*index as u16).saturating_sub(height / 2),
                        markdown.height().saturating_sub(height / 2),
                    );
                }
            }

            Action::Edit => return KeyBoardAction::Edit,

            Action::Escape => {
                app.selected = false;
                markdown.deselect();
                app.details_selected = false;
                markdown.deselect_details();
            }

            Action::Enter => {
                // A focused `<details>` summary toggles its fold state
                // and stays in selection mode so the user can chain
                // multiple toggles without re-pressing `D`.
                if app.details_selected {
                    if markdown.toggle_selected_details().is_ok() {
                        markdown.set_scroll(app.vertical_scroll);
                    }
                    return KeyBoardAction::Continue;
                }

                if !app.selected {
                    return KeyBoardAction::Continue;
                }
                let link = markdown.selected();
                let prev_type = markdown.selected_underlying_type();

                if prev_type == WordType::FootnoteInline {
                    app.message_box.set_message(markdown.find_footnote(link));
                    app.boxes = Boxes::Error;
                    markdown.deselect();
                    app.selected = false;
                    return KeyBoardAction::Continue;
                }

                match LinkType::from(link) {
                    LinkType::Internal(heading) => {
                        app.vertical_scroll = if let Ok(index) = markdown.heading_offset(heading) {
                            cmp::min(index, markdown.height().saturating_sub(height / 2))
                        } else {
                            app.message_box
                                .set_message(format!("Could not find heading {heading}"));
                            app.boxes = Boxes::Error;
                            markdown.deselect();
                            return KeyBoardAction::Continue;
                        };
                    }
                    LinkType::External(url) => {
                        let _ = open::that(url);
                    }
                    LinkType::MarkdownFile(url) => {
                        // Remove the first character, which is a '/'
                        let url = if let Some(url) = url.strip_prefix('/') {
                            url
                        } else {
                            url
                        };

                        let (url, heading) = if let Some((url, heading)) = url.split_once('#') {
                            (url.to_string(), Some(heading.to_string().to_lowercase()))
                        } else {
                            (url.to_string(), None)
                        };

                        let url = if url.ends_with(".md") {
                            url
                        } else {
                            format!("{url}.md")
                        };

                        let text = if let Ok(file) = read_to_string(&url) {
                            app.vertical_scroll = 0;
                            file
                        } else {
                            app.message_box
                                .set_message(format!("Could not open file {url}"));
                            app.boxes = Boxes::Error;
                            return KeyBoardAction::Continue;
                        };

                        if let Some(file_name) = markdown.file_name() {
                            app.history.push(Jump::File(file_name.to_string()));
                        }

                        let path = std::path::Path::new(&url);
                        let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
                        *markdown = parse_markdown(Some(&url), &text, app.width() - 2);
                        let index = if let Some(heading) = heading {
                            if let Ok(index) = markdown.heading_offset(&format!("#{heading}")) {
                                cmp::min(index, markdown.height().saturating_sub(height / 2))
                            } else {
                                app.message_box
                                    .set_message(format!("Could not find heading {heading}"));
                                app.boxes = Boxes::Error;
                                0
                            }
                        } else {
                            0
                        };

                        app.reset();
                        app.vertical_scroll = index;
                    }
                }
                markdown.deselect();
                app.selected = false;
            }

            Action::Back => match app.history.pop() {
                Jump::File(e) => {
                    let text = if let Ok(file) = read_to_string(&e) {
                        app.vertical_scroll = 0;
                        file
                    } else {
                        app.message_box
                            .set_message(format!("Could not open file {e}"));
                        app.boxes = Boxes::Error;
                        return KeyBoardAction::Continue;
                    };
                    *markdown = parse_markdown(Some(&e), &text, app.width() - 2);
                    let path = std::path::Path::new(&e);
                    let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
                    app.reset();
                    app.mode = Mode::View;
                    app.help_box.set_mode(Mode::View);
                }
                Jump::FileTree => {
                    markdown.clear();
                    app.mode = Mode::FileTree;
                    app.help_box.set_mode(Mode::FileTree);
                }
            },

            Action::Help if GENERAL_CONFIG.help_menu => {
                app.help_box.toggle();
            }
            _ => {}
        },
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
    use crossterm::event::KeyModifiers;

    const TERM_HEIGHT: u16 = 20;

    /// A document comfortably taller than the viewport.
    fn long_markdown() -> ComponentRoot {
        let body = (0..100)
            .map(|i| format!("Line {i}\n\n"))
            .collect::<String>();
        parse_markdown(None, &body, 80)
    }

    fn view_app() -> App {
        let mut app = App::default();
        app.mode = Mode::View;
        app
    }

    fn wheel(kind: MouseEventKind) -> MouseEvent {
        MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn scroll(app: &mut App, markdown: &ComponentRoot, kind: MouseEventKind) {
        handle_mouse_input(
            wheel(kind),
            app,
            markdown,
            &mut FileTree::default(),
            TERM_HEIGHT,
        );
    }

    #[test]
    fn wheel_down_scrolls_the_view() {
        let markdown = long_markdown();
        let mut app = view_app();

        scroll(&mut app, &markdown, MouseEventKind::ScrollDown);

        assert_eq!(app.vertical_scroll, MOUSE_SCROLL_LINES);
    }

    #[test]
    fn wheel_up_stops_at_the_top() {
        let markdown = long_markdown();
        let mut app = view_app();
        app.vertical_scroll = 1;

        scroll(&mut app, &markdown, MouseEventKind::ScrollUp);

        assert_eq!(app.vertical_scroll, 0);
    }

    #[test]
    fn wheel_down_stops_at_the_bottom() {
        let markdown = long_markdown();
        let max = markdown.height().saturating_sub(TERM_HEIGHT / 2);
        let mut app = view_app();
        app.vertical_scroll = max;

        scroll(&mut app, &markdown, MouseEventKind::ScrollDown);

        assert_eq!(app.vertical_scroll, max);
    }

    #[test]
    fn scrolling_leaves_link_selection_alone() {
        let markdown = long_markdown();
        let mut app = view_app();
        app.selected = true;
        app.select_index = 4;

        scroll(&mut app, &markdown, MouseEventKind::ScrollDown);

        assert!(app.selected);
        assert_eq!(app.select_index, 4);
        assert_eq!(app.vertical_scroll, MOUSE_SCROLL_LINES);
    }

    #[test]
    fn an_open_box_swallows_the_wheel() {
        let markdown = long_markdown();
        let mut app = view_app();
        app.boxes = Boxes::Search;

        scroll(&mut app, &markdown, MouseEventKind::ScrollDown);

        assert_eq!(app.vertical_scroll, 0);
    }

    #[test]
    fn non_wheel_events_are_ignored() {
        let markdown = long_markdown();
        let mut app = view_app();
        app.vertical_scroll = 7;

        scroll(&mut app, &markdown, MouseEventKind::Moved);
        scroll(&mut app, &markdown, MouseEventKind::ScrollLeft);

        assert_eq!(app.vertical_scroll, 7);
    }
}
