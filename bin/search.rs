use std::{collections::VecDeque, sync::mpsc::Sender};

use itertools::Itertools;
use strsim::damerau_levenshtein;

use md_tui::config::general::GENERAL_CONFIG;
use md_tui::nodes::word::Word;

use crate::pages::file_explorer::{FileTree, MdFile};

fn add_to_gitingore(path: &str, ignored_files: &mut Vec<String>) {
    let gitignore = std::fs::read_to_string(path);
    if let Ok(gitignore) = gitignore {
        for line in gitignore.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            ignored_files.push(line.to_string());
        }
    }
}

pub fn find_md_files_channel(tx: Sender<Option<MdFile>>) {
    let mut ignored_files = Vec::new();

    if GENERAL_CONFIG.gitignore {
        add_to_gitingore(".gitignore", &mut ignored_files);
    }

    let mut stack = VecDeque::new();

    stack.push_back(std::path::PathBuf::from("."));

    while let Some(path) = stack.pop_front() {
        for entry in if let Ok(entries) = std::fs::read_dir(&path) {
            entries
                .into_iter()
                .sorted_unstable_by(|a, b| {
                    let a = if let Ok(a) = a {
                        a
                    } else {
                        return std::cmp::Ordering::Equal;
                    };
                    let b = if let Ok(b) = b {
                        b
                    } else {
                        return std::cmp::Ordering::Equal;
                    };
                    a.path().cmp(&b.path())
                })
                .collect::<Vec<_>>()
        } else {
            continue;
        } {
            let path = if let Ok(path) = entry {
                path.path()
            } else {
                continue;
            };
            if path.is_dir() {
                stack.push_back(path);
            } else if path.extension().unwrap_or_default() == "md" {
                let (path_str, path_name) =
                    if let (Some(path_str), Some(path_name)) = (path.to_str(), path.file_name()) {
                        (path_str, path_name.to_str().unwrap_or("UNKNOWN"))
                    } else {
                        continue;
                    };
                // Check if the file is in the ignored files list
                if ignored_files
                    .iter()
                    .any(|ignored_file| !find(ignored_file, path_str, 0).is_empty())
                {
                    continue;
                }

                tx.send(Some(MdFile::new(
                    path_str.to_string(),
                    path_name.to_string(),
                )))
                .unwrap();
            } else if let (Some(file_name), Some(path)) = (path.file_name(), path.to_str()) {
                if GENERAL_CONFIG.gitignore && file_name == ".gitignore" {
                    add_to_gitingore(path, &mut ignored_files);
                }
            }
        }
    }

    tx.send(None).unwrap();
}

#[allow(dead_code)]
pub fn find_md_files() -> FileTree {
    let mut ignored_files = Vec::new();

    if GENERAL_CONFIG.gitignore {
        add_to_gitingore(".gitignore", &mut ignored_files);
    }

    let mut tree = FileTree::new();

    let mut stack = VecDeque::new();

    stack.push_back(std::path::PathBuf::from("."));

    while let Some(path) = stack.pop_front() {
        for entry in if let Ok(entries) = std::fs::read_dir(&path) {
            entries
        } else {
            continue;
        } {
            let path = if let Ok(path) = entry {
                path.path()
            } else {
                continue;
            };
            if path.is_dir() {
                stack.push_back(path);
            } else if path.extension().unwrap_or_default() == "md" {
                let (path_str, path_name) =
                    if let (Some(path_str), Some(path_name)) = (path.to_str(), path.file_name()) {
                        (path_str, path_name.to_str().unwrap_or("UNKNOWN"))
                    } else {
                        continue;
                    };
                // Check if the file is in the ignored files list
                if ignored_files
                    .iter()
                    .any(|ignored_file| !find(ignored_file, path_str, 0).is_empty())
                {
                    continue;
                }

                tree.add_file(MdFile::new(path_str.to_string(), path_name.to_string()));
            } else if let (Some(file_name), Some(path)) = (path.file_name(), path.to_str()) {
                if GENERAL_CONFIG.gitignore && file_name == ".gitignore" {
                    add_to_gitingore(path, &mut ignored_files);
                }
            }
        }
    }
    tree.sort_name();
    tree
}

pub fn find_files(files: &[MdFile], query: &str) -> Vec<MdFile> {
    if query.is_empty() {
        return files.to_vec();
    }

    // Check if any char in the query is uppercase, making the search case sensitive
    let case_sensitive = query.chars().any(|c| c.is_uppercase());

    files
        .iter()
        .filter(|file| {
            let file_path = if case_sensitive {
                file.path.to_owned()
            } else {
                file.path.to_lowercase()
            };
            let res = char_windows(&file_path, query.len())
                .any(|window| damerau_levenshtein(window, query) == 0);
            res
        })
        .cloned()
        .collect()
}

#[allow(dead_code)]
pub fn find_with_backoff(query: &str, text: &str) -> Vec<usize> {
    let precision = 0;
    let mut result = find(query, text, precision);
    if result.is_empty() {
        let precision = 1;
        result = find(query, text, precision);
    }
    result
}

pub fn find(query: &str, text: &str, precision: usize) -> Vec<usize> {
    let mut result = Vec::new();

    let case_sensitive = query.chars().any(|c| c.is_uppercase());

    char_windows(text, query.len())
        .enumerate()
        .for_each(|(i, window)| {
            let window = if case_sensitive {
                window.to_owned()
            } else {
                window.to_lowercase()
            };
            let score = damerau_levenshtein(query, &window);
            if score <= precision {
                result.push(i);
            }
        });

    result
}

/// Returns line numbers that match the query with the given precision.
#[allow(dead_code)]
pub fn line_match(query: &str, text: Vec<&str>, precision: usize) -> Vec<usize> {
    text.iter()
        .enumerate()
        .filter_map(|(i, line)| {
            if find(query, line, precision).is_empty() {
                None
            } else {
                Some(i)
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn line_match_and_index(
    query: &str,
    lines: Vec<&str>,
    precision: usize,
) -> Vec<(usize, usize)> {
    lines
        .iter()
        .enumerate()
        .flat_map(|(i, line)| {
            find(query, line, precision)
                .into_iter()
                .map(move |j| (i, j))
        })
        .collect()
}

#[allow(dead_code)]
pub fn find_with_ref<'a>(query: &str, text: Vec<&'a Word>) -> Vec<&'a Word> {
    let window_size = query
        .split_whitespace()
        .fold(0usize, |acc, _| acc + 2)
        .saturating_sub(1);

    if window_size == 0 {
        return Vec::new();
    }

    text.windows(window_size)
        .filter(|word| {
            let mut words = word.iter().map(|c| c.content()).join("");
            let case_sensitive = query.chars().any(|c| c.is_uppercase());

            words = if case_sensitive {
                words.to_owned()
            } else {
                words.to_lowercase()
            };

            damerau_levenshtein(query, &words) == 0
        })
        .flatten()
        .copied()
        .collect::<Vec<_>>()
}

fn char_windows(src: &str, win_size: usize) -> impl Iterator<Item = &'_ str> {
    src.char_indices().flat_map(move |(from, _)| {
        src[from..]
            .char_indices()
            .nth(win_size - 1)
            .map(|(to, c)| &src[from..from + to + c.len_utf8()])
    })
}

#[cfg(test)]
mod tests {

    use md_tui::{
        nodes::{
            root::{Component, ComponentRoot},
            textcomponent::{TextComponent, TextNode},
            word::WordType,
        },
        parser::parse_markdown,
    };

    use super::*;

    #[test]
    fn test_find() {
        let text = "Hello, world!";
        let query = "world";
        let precision = 0;
        let result = find(query, text, precision);
        assert_eq!(result, vec![7]);
    }

    #[test]
    fn test_find_with_backoff() {
        let text = "Hello, world!";
        let query = "world";
        let result = find_with_backoff(query, text);
        assert_eq!(result, vec![7]);
    }

    #[test]
    fn test_find_with_backoff_with_typo() {
        let text = "Hello, world!";
        let query = "wrold";
        let result = find_with_backoff(query, text);
        assert_eq!(result, vec![7]);
    }

    #[test]
    fn test_vec_find() {
        let text = vec!["Hello", "hello", "world", "World"];
        let query = "world";
        let precision = 0;
        let result = line_match(query, text, precision);
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_vec_find_less_precision() {
        let text = vec!["Hello", "hello", "world", "World"];
        let query = "world";
        let precision = 1;
        let result = line_match(query, text, precision);
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_vec_find_with_typo() {
        let text = vec!["Hello", "hello", "world", "World"];
        let query = "wrold";
        let precision = 2;
        let result = line_match(query, text, precision);
        assert_eq!(result, vec![2, 3]);
    }

    #[test]
    fn test_find_line_match_and_index() {
        let text = vec!["Hello", "hello", "world", "hello world"];
        let query = "world";
        let precision = 0;
        let result = line_match_and_index(query, text, precision);
        assert_eq!(result, vec![(2, 0), (3, 6)]);
    }

    #[test]
    fn test_find_line_match_and_index_with_typo() {
        let text = vec!["Hello", "hello", "world", "hello world"];
        let query = "wrold";
        let precision = 2;
        let result = line_match_and_index(query, text, precision);
        assert_eq!(result, vec![(2, 0), (3, 6)]);
    }

    #[test]
    fn test_find_line_match_and_index_with_leading_space() {
        let text = vec!["Hello", "hello", "world", " hello world"];
        let query = "world";
        let precision = 0;
        let result = line_match_and_index(query, text, precision);
        assert_eq!(result, vec![(2, 0), (3, 7)]);
    }

    #[test]
    fn test_word_by_ref() {
        let text = vec![
            Word::new("Hello".to_string(), WordType::Bold),
            Word::new("hello".to_string(), WordType::White),
            Word::new("world".to_string(), WordType::Normal),
            Word::new("World".to_string(), WordType::BoldItalic),
        ];

        let componet = Component::TextComponent(TextComponent::new(TextNode::Paragraph, text));
        let root = ComponentRoot::new(None, vec![componet]);
        let query = "world";
        let result = find_with_ref(query, root.words());
        assert_eq!(result.len(), 2);
    }
    #[test]
    fn test_word_by_ref_span_multiple_words() {
        let text = vec![
            Word::new("Hello".to_string(), WordType::Bold),
            Word::new("hello".to_string(), WordType::White),
            Word::new(" ".to_string(), WordType::White),
            Word::new("world".to_string(), WordType::Normal),
            Word::new("World".to_string(), WordType::BoldItalic),
        ];

        let componet = Component::TextComponent(TextComponent::new(TextNode::Paragraph, text));
        let root = ComponentRoot::new(None, vec![componet]);
        let query = "hello world";
        let result = find_with_ref(query, root.words());
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_word_by_ref_span_multiple_words_using_reference() {
        let text = vec![
            Word::new("Hello".to_string(), WordType::Bold),
            Word::new("hello".to_string(), WordType::White),
            Word::new(" ".to_string(), WordType::White),
            Word::new("world".to_string(), WordType::Normal),
            Word::new("World".to_string(), WordType::BoldItalic),
        ];

        let componet = Component::TextComponent(TextComponent::new(TextNode::Paragraph, text));
        let root = ComponentRoot::new(None, vec![componet]);
        let query = "hello world";
        let result = find_with_ref(query, root.words());

        assert_ne!(result[0], root.words()[0]);
        assert_eq!(result[0], root.words()[1]);
        assert_eq!(result[1], root.words()[2]);
        assert_eq!(result[2], root.words()[3]);
    }

    #[test]
    fn test_long_match() {
        let text = "`MD-TUI` is a TUI application for viewing markdown files directly in your
terminal. I created it because I wasn't happy with how alternatives handled
links in their applications. While the full markdown specification is not yet
supported, it will slowly get there. It's a good solution for quickly viewing
your markdown notes, or opening external links from someones README.
";

        let markdown = parse_markdown(None, text, 80);

        let result = find_with_ref("in", markdown.words());
        dbg!(&result);
        assert_eq!(result.len(), 2);

        let result = find_with_ref("markdown notes,", markdown.words());
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_alphanumeric() {
        let s = "#Hello, world!";
        let filtered = s
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        assert_eq!(filtered, "Helloworld");
    }
}
