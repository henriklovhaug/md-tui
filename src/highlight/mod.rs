use ratatui::style::Color;
use tree_sitter_highlight::HighlightEvent;

use self::{java::highlight_java, json::highlight_json, rust::highlight_rust};

mod java;
mod json;
mod rust;

static HIGHLIGHT_NAMES: [&str; 18] = [
    "attribute",
    "constant",
    "function.builtin",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

pub static COLOR_MAP: [Color; 18] = [
    Color::Yellow,
    Color::Yellow,
    Color::Green,
    Color::Green,
    Color::Red,
    Color::Red,
    Color::Blue,
    Color::Blue,
    Color::Blue,
    Color::Blue,
    Color::Magenta,
    Color::Magenta,
    Color::Cyan,
    Color::Cyan,
    Color::Cyan,
    Color::White,
    Color::White,
    Color::White,
];

pub enum HighlightInfo {
    Highlighted(Vec<HighlightEvent>),
    Unhighlighted,
}

pub fn highlight_code(language: &str, lines: &[u8]) -> HighlightInfo {
    match language {
        "java" => HighlightInfo::Highlighted(highlight_java(lines).unwrap()),
        "rust" => HighlightInfo::Highlighted(highlight_rust(lines).unwrap()),
        "json" => HighlightInfo::Highlighted(highlight_json(lines).unwrap()),
        _ => HighlightInfo::Unhighlighted,
    }
}

#[cfg(test)]
#[test]
fn test_equal_length() {
    assert_eq!(HIGHLIGHT_NAMES.len(), COLOR_MAP.len());
}
