use std::{collections::VecDeque, sync::mpsc::Sender};

use itertools::Itertools;
use strsim::damerau_levenshtein;

use crate::{
    nodes::word::{Word, WordType},
    pages::file_explorer::{FileTree, MdFile},
    util::general::GENERAL_CONFIG,
};

/// Smart-case normalisation: leaves `text` untouched when the query has any
/// uppercase character, otherwise lower-cases it. Used by every search path
/// so case sensitivity is consistent across file-tree and document search.
fn smart_case_normalize(query: &str, text: &str) -> String {
    if query.chars().any(char::is_uppercase) {
        text.to_owned()
    } else {
        text.to_lowercase()
    }
}

/// Window size for the multi-word fuzzy search. Two slots per word
/// (the word itself plus a trailing space) minus one to drop the final
/// trailing space.
fn search_window_size(query: &str) -> usize {
    query
        .split_whitespace()
        .fold(0usize, |acc, _| acc + 2)
        .saturating_sub(1)
}

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

fn load_ignored_files() -> Vec<String> {
    let mut ignored_files = Vec::new();
    if GENERAL_CONFIG.gitignore {
        add_to_gitingore(".gitignore", &mut ignored_files);
    }
    ignored_files
}

fn get_sorted_entries(path: &std::path::Path) -> Vec<std::fs::DirEntry> {
    if let Ok(entries) = std::fs::read_dir(path) {
        entries
            .flatten()
            .sorted_unstable_by(|a, b| a.path().cmp(&b.path()))
            .collect()
    } else {
        Vec::new()
    }
}

fn should_skip_file(path: &std::path::Path, ignored_files: &[String]) -> bool {
    let Some(path_str) = path.to_str() else {
        return true;
    };

    ignored_files
        .iter()
        .any(|ignored_file| !find(ignored_file, path_str, 0).is_empty())
}

struct FileFinder {
    ignored_files: Vec<String>,
    stack: VecDeque<std::path::PathBuf>,
    tx: Sender<Option<MdFile>>,
}

impl FileFinder {
    fn new(tx: Sender<Option<MdFile>>) -> Self {
        Self {
            ignored_files: load_ignored_files(),
            stack: VecDeque::from([std::path::PathBuf::from(".")]),
            tx,
        }
    }

    fn find_files(mut self) {
        while let Some(path) = self.stack.pop_front() {
            for entry in get_sorted_entries(&path) {
                self.process_entry(entry);
            }
        }
        let _ = self.tx.send(None);
    }

    fn process_entry(&mut self, entry: std::fs::DirEntry) {
        let path = entry.path();
        if path.is_dir() {
            self.stack.push_back(path);
        } else if is_md_file(&path) {
            self.handle_md_file(&path);
        } else if is_gitignore(&path) && GENERAL_CONFIG.gitignore {
            self.handle_gitignore(&path);
        }
    }

    fn handle_md_file(&self, path: &std::path::Path) {
        if should_skip_file(path, &self.ignored_files) {
            return;
        }

        if let (Some(path_str), Some(path_name)) = (path.to_str(), path.file_name()) {
            let _ = self.tx.send(Some(MdFile::new(
                path_str.to_string(),
                path_name.to_string_lossy().to_string(),
            )));
        }
    }

    fn handle_gitignore(&mut self, path: &std::path::Path) {
        if let Some(path_str) = path.to_str() {
            add_to_gitingore(path_str, &mut self.ignored_files);
        }
    }
}

fn is_md_file(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|ext| ext == "md")
}

fn is_gitignore(path: &std::path::Path) -> bool {
    path.file_name().is_some_and(|name| name == ".gitignore")
}

pub fn find_md_files_channel(tx: Sender<Option<MdFile>>) {
    let finder = FileFinder::new(tx);
    finder.find_files();
}

#[must_use]
pub fn find_md_files() -> FileTree {
    let (tx, rx) = std::sync::mpsc::channel();
    find_md_files_channel(tx);

    let mut tree = FileTree::new();
    while let Ok(Some(file)) = rx.recv() {
        tree.add_file(file);
    }
    tree.sort_name();
    tree
}

#[must_use]
pub fn find_files(files: &[MdFile], query: &str) -> Vec<MdFile> {
    if query.is_empty() {
        return files.to_vec();
    }

    files
        .iter()
        .filter(|file| {
            let file_path = smart_case_normalize(query, &file.path);
            char_windows(&file_path, query.chars().count())
                .any(|window| damerau_levenshtein(window, query) == 0)
        })
        .cloned()
        .collect()
}

#[must_use]
pub fn find_with_backoff(query: &str, text: &str) -> Vec<usize> {
    let precision = 0;
    let mut result = find(query, text, precision);
    if result.is_empty() {
        let precision = 1;
        result = find(query, text, precision);
    }
    result
}

#[must_use]
pub fn find(query: &str, text: &str, precision: usize) -> Vec<usize> {
    let mut result = Vec::new();

    char_windows(text, query.chars().count())
        .enumerate()
        .for_each(|(i, window)| {
            let window = smart_case_normalize(query, window);
            let score = damerau_levenshtein(query, &window);
            if score <= precision {
                result.push(i);
            }
        });

    result
}

#[must_use]
pub fn find_with_ref<'a>(query: &str, text: Vec<&'a Word>) -> Vec<&'a Word> {
    let window_size = search_window_size(query);
    if window_size == 0 {
        return Vec::new();
    }

    text.windows(window_size)
        .filter(|word| {
            let joined = word.iter().map(|c| c.content()).join("");
            let words = smart_case_normalize(query, &joined);
            damerau_levenshtein(query, &words) == 0
        })
        .flatten()
        .copied()
        .collect::<Vec<_>>()
}

pub fn find_and_mark<'a>(query: &str, text: &'a mut Vec<&'a mut Word>) {
    let window_size = search_window_size(query);
    if window_size == 0 {
        return;
    }

    windows_mut_for_each(text.as_mut_slice(), window_size, |window| {
        let joined = window.iter().map(|c| c.content()).join("");
        let words = smart_case_normalize(query, &joined);

        if damerau_levenshtein(query, &words) == 0 {
            window
                .iter_mut()
                .for_each(|word| word.set_kind(WordType::Selected));
        }
    });
}

fn windows_mut_for_each<T>(v: &mut [T], n: usize, f: impl Fn(&mut [T])) {
    let mut start = 0;
    let mut end = n;
    while end <= v.len() {
        f(&mut v[start..end]);
        start += 1;
        end += 1;
    }
}

fn char_windows(src: &str, win_size: usize) -> impl Iterator<Item = &'_ str> {
    src.char_indices().filter_map(move |(from, _)| {
        // Guard against `win_size == 0`, which would underflow `win_size - 1`.
        let last = win_size.checked_sub(1)?;
        src[from..]
            .char_indices()
            .nth(last)
            .map(|(to, c)| &src[from..from + to + c.len_utf8()])
    })
}

#[must_use]
pub fn compare_heading(link_header: &str, header: &[Vec<Word>]) -> bool {
    let header: String = header
        .iter()
        .flatten()
        .map(|word| word.content().to_lowercase())
        .join("-")
        .trim_start_matches('-')
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .dedup_by(|a, b| *a == '-' && *b == '-')
        .skip_while(|c| *c == '-')
        .collect();

    link_header == header
}

#[cfg(test)]
mod tests {

    use crate::{
        nodes::{
            root::{Component, ComponentRoot},
            textcomponent::{TextComponent, TextNode},
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
    fn find_matches_multibyte_query() {
        // "naïve" is 5 chars but 6 bytes; the window must be sized in chars or
        // the multi-byte query never matches. 'n' is the 3rd char (index 2).
        let text = "a naïve approach";
        assert_eq!(find("naïve", text, 0), vec![2]);
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
