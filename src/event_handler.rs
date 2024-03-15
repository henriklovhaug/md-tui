use std::{cmp, fs::read_to_string};

use crossterm::event::KeyCode;

use crate::{
    nodes::RenderRoot,
    pages::file_explorer::FileTree,
    parser::parse_markdown,
    search::find_line_match_and_index,
    util::{App, Boxes, Jump, LinkType, Mode},
};

pub enum KeyBoardAction {
    Continue,
    Exit,
}

pub fn handle_keyboard_input(
    key: KeyCode,
    app: &mut App,
    markdown: &mut RenderRoot,
    file_tree: &mut FileTree,
    height: u16,
) -> KeyBoardAction {
    if key == KeyCode::Char('q') && app.boxes != Boxes::Search {
        return KeyBoardAction::Exit;
    }
    match app.mode {
        Mode::View => keyboard_mode_view(key, app, markdown, height),
        Mode::FileTree => keyboard_mode_file_tree(key, app, markdown, file_tree, height),
    }
}

pub fn keyboard_mode_file_tree(
    key: KeyCode,
    app: &mut App,
    markdown: &mut RenderRoot,
    file_tree: &mut FileTree,
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
        Boxes::None => match key {
            KeyCode::Char('j') => {
                file_tree.next(height);
            }

            KeyCode::Char('k') => {
                file_tree.previous(height);
            }

            KeyCode::Char('l') => {
                file_tree.next_page(height);
            }

            KeyCode::Char('h') => {
                file_tree.previous_page(height);
            }

            KeyCode::Char('g') => {
                file_tree.first();
            }

            KeyCode::Char('G') => {
                file_tree.last(height);
            }

            KeyCode::Enter => {
                let file = if let Some(file) = file_tree.selected() {
                    file
                } else {
                    app.error_box.set_message("No file selected".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };
                let text = if let Ok(file) = read_to_string(file.path()) {
                    app.reset();
                    file
                } else {
                    app.error_box
                        .set_message(format!("Could not open file {}", file.path()));
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };

                *markdown = parse_markdown(Some(file.path()), &text);
                markdown.transform(80);
                app.mode = Mode::View;
                app.help_box.set_mode(Mode::View);
                app.select_index = 0;
            }
            KeyCode::Char('f') | KeyCode::Char('/') => {
                let file_height = file_tree.height(height);
                app.search_box.set_position(10, file_height as u16 + 2);
                app.search_box.set_width(20);
                app.boxes = Boxes::Search;
                app.help_box.close();
            }

            KeyCode::Char('b') => match app.history.pop() {
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
                    *markdown = parse_markdown(Some(&e), &text);
                    markdown.transform(80);
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
            KeyCode::Char('?') => {
                app.help_box.toggle();
            }

            KeyCode::Esc => {
                file_tree.unselect();
                file_tree.search(None);
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
                app.search_box.clear();
                app.boxes = Boxes::None;
            }
            KeyCode::Enter => {
                let query = app.search_box.consume();
                let lines = markdown.content();
                let search =
                    find_line_match_and_index(&query, lines.iter().map(|s| &**s).collect(), 0);
                if search.is_empty() {
                    app.error_box
                        .set_message(format!("No results for {}", query));
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                }

                markdown.deselect();

                let closest = search.iter().min_by_key(|(index, _)| {
                    if *index as u16 > app.vertical_scroll + height / 2 {
                        *index as u16 - app.vertical_scroll - height / 2
                    } else {
                        app.vertical_scroll + height / 2 - *index as u16
                    }
                });

                if let Some((index, _)) = closest {
                    app.vertical_scroll = cmp::min(
                        (*index as u16).saturating_sub(height / 2),
                        markdown.height().saturating_sub(height / 2),
                    );
                }

                for ele in search.iter() {
                    let _ = markdown.mark_word(ele.0, ele.1, query.len());
                    // Scroll to closest match
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
        Boxes::None => match key {
            KeyCode::Char('j') => {
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
            KeyCode::Char('k') => {
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

            KeyCode::Char('l') => {
                app.vertical_scroll = cmp::min(
                    app.vertical_scroll + height,
                    markdown.height().saturating_sub(height / 2),
                );
            }

            KeyCode::Char('h') => {
                app.vertical_scroll = app.vertical_scroll.saturating_sub(height);
            }

            KeyCode::Char('s') => {
                app.vertical_scroll = if let Ok(scroll) = markdown.select(app.select_index) {
                    app.selected = true;
                    scroll.saturating_sub(height / 3)
                } else {
                    app.vertical_scroll
                };
            }
            KeyCode::Char('f') | KeyCode::Char('/') => {
                app.search_box.set_position(2, height - 3);
                app.search_box.set_width(80);
                app.boxes = Boxes::Search;
                app.help_box.close();
            }
            KeyCode::Char('t') => {
                app.mode = Mode::FileTree;
                app.help_box.set_mode(Mode::FileTree);
                if let Some(file) = markdown.file_name() {
                    app.history.push(Jump::File(file.to_string()));
                }
                app.reset();
            }
            KeyCode::Char('r') => {
                let url = if let Some(url) = markdown.file_name() {
                    url
                } else {
                    app.error_box.set_message("No file".to_string());
                    app.boxes = Boxes::Error;
                    return KeyBoardAction::Continue;
                };
                let text = if let Ok(file) = read_to_string(url) {
                    app.vertical_scroll = 0;
                    file
                } else {
                    app.error_box
                        .set_message(format!("Could not open file {:?}", markdown.file_name()));
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
                        let url = &url[1..];
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

                        *markdown = parse_markdown(Some(url), &text);
                        markdown.transform(80);
                        app.reset();
                    }
                }
                markdown.deselect();
                app.selected = false;
            }

            KeyCode::Char('b') => match app.history.pop() {
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
                    *markdown = parse_markdown(Some(&e), &text);
                    markdown.transform(80);
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

            KeyCode::Char('?') => {
                app.help_box.toggle();
            }
            _ => {}
        },
    }
    KeyBoardAction::Continue
}
