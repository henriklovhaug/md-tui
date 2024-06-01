use std::{cmp, fs::read_to_string};

use crossterm::event::KeyCode;
use notify::{PollWatcher, Watcher};

use crate::{
    nodes::root::ComponentRoot,
    pages::file_explorer::FileTree,
    parser::parse_markdown,
    util::{
        colors::COLOR_CONFIG,
        keys::{key_to_action, Action},
        App, Boxes, Jump, LinkType, Mode,
    },
};

pub enum KeyBoardAction {
    Continue,
    Edit,
    Exit,
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
        Mode::Loading => KeyBoardAction::Continue,
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
                    app.error_box.set_message("No file selected".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };
                let text = if let Ok(file) = read_to_string(file.path_str()) {
                    app.reset();
                    file
                } else {
                    app.error_box
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
                        app.error_box
                            .set_message(format!("Could not open file {}", e));
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
            Action::Help => {
                app.help_box.toggle();
            }

            Action::Escape => {
                file_tree.unselect();
                file_tree.search(None);
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
                    app.error_box
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
                } else {
                    app.vertical_scroll = cmp::min(
                        app.vertical_scroll + 1,
                        markdown.height().saturating_sub(height / 2),
                    );
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
                } else {
                    app.vertical_scroll = app.vertical_scroll.saturating_sub(1);
                }
            }
            Action::ToTop => {
                app.vertical_scroll = 0;
            }
            Action::ToBottom => {
                app.vertical_scroll = markdown.height().saturating_sub(height / 2);
            }

            Action::HalfPageDown => {
                app.vertical_scroll += height / 2;
                app.vertical_scroll = cmp::min(
                    app.vertical_scroll,
                    markdown.height().saturating_sub(height / 2),
                );
            }
            Action::HalfPageUp => {
                app.vertical_scroll = app.vertical_scroll.saturating_sub(height / 2);
            }

            Action::PageDown => {
                app.vertical_scroll = cmp::min(
                    app.vertical_scroll + height,
                    markdown.height().saturating_sub(height / 2),
                );
            }

            Action::PageUp => {
                app.vertical_scroll = app.vertical_scroll.saturating_sub(height);
            }

            Action::Hover => {
                if app.selected {
                    let link = markdown.selected();

                    let message = match LinkType::from(link) {
                        LinkType::Internal(e) => format!("Internal link: {}", e),
                        LinkType::External(e) => format!("External link: {}", e),
                        LinkType::MarkdownFile(e) => format!("Markdown file: {}", e),
                    };

                    app.link_box.set_message(message);
                    app.boxes = Boxes::LinkPreview;
                } else {
                    app.error_box.set_message("No link selected".to_string());
                    app.boxes = Boxes::Error;
                }
            }

            // Find the link closest to the middle, searching both ways
            Action::SelectLinkAlt => {
                let links = markdown.link_index_and_height();
                if links.is_empty() {
                    app.error_box.set_message("No links found".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                }

                let next = links.iter().min_by_key(|(_, row)| {
                    if *row > (app.vertical_scroll + height / 3) {
                        *row - (app.vertical_scroll + height / 3)
                    } else {
                        (app.vertical_scroll + height / 3) - *row
                    }
                });

                if let Some((index, _)) = next {
                    app.vertical_scroll = if let Ok(scroll) = markdown.select(*index) {
                        app.select_index = *index;
                        scroll.saturating_sub(height / 3)
                    } else {
                        app.vertical_scroll
                    };
                    app.selected = true;
                } else {
                    // Something weird must have happened at this point
                    markdown.deselect();
                }
            }

            // Find the link closest to the to the top, searching downwards
            Action::SelectLink => {
                let mut links = markdown.link_index_and_height();
                if links.is_empty() {
                    app.error_box.set_message("No links found".to_string());
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
                app.vertical_scroll = if let Ok(scroll) = markdown.select(app.select_index) {
                    scroll.saturating_sub(height / 3)
                } else {
                    app.vertical_scroll
                };
            }

            Action::Search => {
                app.search_box.clear();
                app.search_box.set_position(2, height - 3);
                app.search_box.set_width(COLOR_CONFIG.width - 3);
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
            }

            Action::Enter => {
                if !app.selected {
                    return KeyBoardAction::Continue;
                }
                let link = markdown.selected();
                match LinkType::from(link) {
                    LinkType::Internal(heading) => {
                        app.vertical_scroll = if let Ok(index) = markdown.heading_offset(heading) {
                            cmp::min(index, markdown.height().saturating_sub(height / 2))
                        } else {
                            app.error_box
                                .set_message(format!("Could not find heading {}", heading));
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
                        if !url.ends_with("md") {
                            let _ = open::that(url);
                            return KeyBoardAction::Continue;
                        }
                        let text = if let Ok(file) = read_to_string(url) {
                            app.vertical_scroll = 0;
                            file
                        } else {
                            app.error_box
                                .set_message(format!("Could not open file {}", url));
                            app.boxes = Boxes::Error;
                            return KeyBoardAction::Continue;
                        };

                        if let Some(file_name) = markdown.file_name() {
                            app.history.push(Jump::File(file_name.to_string()));
                        }

                        let path = std::path::Path::new(&url);
                        let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
                        *markdown = parse_markdown(Some(url), &text, app.width() - 2);
                        app.reset();
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
                        app.error_box
                            .set_message(format!("Could not open file {}", e));
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

            Action::Help => {
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
