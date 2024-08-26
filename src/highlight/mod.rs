use c::highlight_c;
use cpp::highlight_cpp;
use css::highlight_css;
use elixir::highlight_elixir;
use go::highlight_go;
use html::highlight_html;
use java::highlight_java;
use javascript::highlight_javascript;
use json::highlight_json;
use lua::highlight_lua;
use luau::highlight_luau;
use ocaml::highlight_ocaml;
use php::highlight_php;
use python::highlight_python;
use ratatui::style::Color;
use rust::highlight_rust;
use scala::highlight_scala;
use tree_sitter_highlight::HighlightEvent;
use tsx::highlight_tsx;
use typescript::highlight_typescript;
use yaml::highlight_yaml;

mod bash;
mod c;
mod cpp;
mod css;
mod elixir;
mod go;
mod html;
mod java;
mod javascript;
mod json;
mod lua;
mod luau;
mod ocaml;
mod php;
mod python;
mod rust;
mod scala;
mod tsx;
mod typescript;
mod yaml;

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
        "css" => HighlightInfo::Highlighted(highlight_css(lines).unwrap()),
        "elixir" => HighlightInfo::Highlighted(highlight_elixir(lines).unwrap()),
        "go" => HighlightInfo::Highlighted(highlight_go(lines).unwrap()),
        "html" => HighlightInfo::Highlighted(highlight_html(lines).unwrap()),
        "java" => HighlightInfo::Highlighted(highlight_java(lines).unwrap()),
        "javascript" | "js" => HighlightInfo::Highlighted(highlight_javascript(lines).unwrap()),
        "json" => HighlightInfo::Highlighted(highlight_json(lines).unwrap()),
        "lua" => HighlightInfo::Highlighted(highlight_lua(lines).unwrap()),
        "luau" => HighlightInfo::Highlighted(highlight_luau(lines).unwrap()),
        "ocaml" => HighlightInfo::Highlighted(highlight_ocaml(lines).unwrap()),
        "php" => HighlightInfo::Highlighted(highlight_php(lines).unwrap()),
        "python" => HighlightInfo::Highlighted(highlight_python(lines).unwrap()),
        "rust" => HighlightInfo::Highlighted(highlight_rust(lines).unwrap()),
        "scala" => HighlightInfo::Highlighted(highlight_scala(lines).unwrap()),
        "tsx" => HighlightInfo::Highlighted(highlight_tsx(lines).unwrap()),
        "typescript" | "ts" => HighlightInfo::Highlighted(highlight_typescript(lines).unwrap()),
        "yaml" | "yml" => HighlightInfo::Highlighted(highlight_yaml(lines).unwrap()),
        _ => HighlightInfo::Unhighlighted,
    }
}

#[cfg(test)]
#[test]
fn test_equal_length() {
    assert_eq!(HIGHLIGHT_NAMES.len(), COLOR_MAP.len());
}
