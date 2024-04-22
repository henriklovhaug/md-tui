use ratatui::style::Color;
use tree_sitter_highlight::HighlightEvent;

use self::{
    c::highlight_c, cpp::highlight_cpp, elixir::highlight_elixir, go::highlight_go,
    java::highlight_java, javascript::highlight_javascript, json::highlight_json,
    lua::highlight_lua, python::highlight_python, rust::highlight_rust,
};

mod bash;
mod c;
mod cpp;
mod elixir;
mod go;
mod java;
mod javascript;
mod json;
mod lua;
mod ocaml;
mod python;
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
    Color::Reset,
    Color::Reset,
    Color::Reset,
];

pub enum HighlightInfo {
    Highlighted(Vec<HighlightEvent>),
    Unhighlighted,
}

pub fn highlight_code(language: &str, lines: &[u8]) -> HighlightInfo {
    match language {
        "bash" | "sh" => HighlightInfo::Highlighted(bash::highlight_bash(lines).unwrap()),
        "c" => HighlightInfo::Highlighted(highlight_c(lines).unwrap()),
        "cpp" => HighlightInfo::Highlighted(highlight_cpp(lines).unwrap()),
        "elixir" => HighlightInfo::Highlighted(highlight_elixir(lines).unwrap()),
        "go" => HighlightInfo::Highlighted(highlight_go(lines).unwrap()),
        "java" => HighlightInfo::Highlighted(highlight_java(lines).unwrap()),
        "javascript" | "js" => HighlightInfo::Highlighted(highlight_javascript(lines).unwrap()),
        "json" => HighlightInfo::Highlighted(highlight_json(lines).unwrap()),
        "lua" => HighlightInfo::Highlighted(highlight_lua(lines).unwrap()),
        "ocaml" => HighlightInfo::Highlighted(ocaml::highlight_ocaml(lines).unwrap()),
        "python" => HighlightInfo::Highlighted(highlight_python(lines).unwrap()),
        "rust" => HighlightInfo::Highlighted(highlight_rust(lines).unwrap()),
        _ => HighlightInfo::Unhighlighted,
    }
}

#[cfg(test)]
#[test]
fn test_equal_length() {
    assert_eq!(HIGHLIGHT_NAMES.len(), COLOR_MAP.len());
}
