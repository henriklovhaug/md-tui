#[cfg(feature = "tree-sitter-bash")]
mod bash;
#[cfg(feature = "tree-sitter-c")]
mod c;
#[cfg(feature = "tree-sitter-cpp")]
mod cpp;
#[cfg(feature = "tree-sitter-css")]
mod css;
#[cfg(feature = "tree-sitter-elixir")]
mod elixir;
#[cfg(feature = "tree-sitter-go")]
mod go;
#[cfg(feature = "tree-sitter-html")]
mod html;
#[cfg(feature = "tree-sitter-java")]
mod java;
#[cfg(feature = "tree-sitter-javascript")]
mod javascript;
#[cfg(feature = "tree-sitter-json")]
mod json;
#[cfg(feature = "tree-sitter-lua")]
mod lua;
#[cfg(feature = "tree-sitter-ocaml")]
mod ocaml;
#[cfg(feature = "tree-sitter-php")]
mod php;
#[cfg(feature = "tree-sitter-python")]
mod python;
#[cfg(feature = "tree-sitter-rust")]
mod rust;
#[cfg(feature = "tree-sitter-scala")]
mod scala;
#[cfg(feature = "tree-sitter-typescript")]
mod tsx;
#[cfg(feature = "tree-sitter-typescript")]
mod typescript;
#[cfg(feature = "tree-sitter-yaml")]
mod yaml;

use tree_sitter_highlight::HighlightEvent;

use ratatui::style::Color;

#[allow(dead_code)]
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

#[allow(unused_variables)]
#[must_use]
pub fn highlight_code(language: &str, lines: &[u8]) -> HighlightInfo {
    match language {
        #[cfg(feature = "tree-sitter-bash")]
        "bash" | "sh" => HighlightInfo::Highlighted(bash::highlight_bash(lines).unwrap()),

        #[cfg(feature = "tree-sitter-c")]
        "c" => HighlightInfo::Highlighted(c::highlight_c(lines).unwrap()),

        #[cfg(feature = "tree-sitter-cpp")]
        "cpp" => HighlightInfo::Highlighted(cpp::highlight_cpp(lines).unwrap()),

        #[cfg(feature = "tree-sitter-css")]
        "css" => HighlightInfo::Highlighted(css::highlight_css(lines).unwrap()),

        #[cfg(feature = "tree-sitter-elixir")]
        "elixir" => HighlightInfo::Highlighted(elixir::highlight_elixir(lines).unwrap()),

        #[cfg(feature = "tree-sitter-go")]
        "go" => HighlightInfo::Highlighted(go::highlight_go(lines).unwrap()),

        #[cfg(feature = "tree-sitter-html")]
        "html" => HighlightInfo::Highlighted(html::highlight_html(lines).unwrap()),

        #[cfg(feature = "tree-sitter-java")]
        "java" => HighlightInfo::Highlighted(java::highlight_java(lines).unwrap()),

        #[cfg(feature = "tree-sitter-javascript")]
        "javascript" | "js" => {
            HighlightInfo::Highlighted(javascript::highlight_javascript(lines).unwrap())
        }

        #[cfg(feature = "tree-sitter-json")]
        "json" => HighlightInfo::Highlighted(json::highlight_json(lines).unwrap()),

        #[cfg(feature = "tree-sitter-lua")]
        "lua" => HighlightInfo::Highlighted(lua::highlight_lua(lines).unwrap()),

        #[cfg(feature = "tree-sitter-ocaml")]
        "ocaml" => HighlightInfo::Highlighted(ocaml::highlight_ocaml(lines).unwrap()),

        #[cfg(feature = "tree-sitter-php")]
        "php" => HighlightInfo::Highlighted(php::highlight_php(lines).unwrap()),

        #[cfg(feature = "tree-sitter-python")]
        "python" => HighlightInfo::Highlighted(python::highlight_python(lines).unwrap()),

        #[cfg(feature = "tree-sitter-rust")]
        "rust" => HighlightInfo::Highlighted(rust::highlight_rust(lines).unwrap()),

        #[cfg(feature = "tree-sitter-scala")]
        "scala" => HighlightInfo::Highlighted(scala::highlight_scala(lines).unwrap()),

        #[cfg(feature = "tree-sitter-typescript")]
        "tsx" => HighlightInfo::Highlighted(tsx::highlight_tsx(lines).unwrap()),

        #[cfg(feature = "tree-sitter-typescript")]
        "typescript" | "ts" => {
            HighlightInfo::Highlighted(typescript::highlight_typescript(lines).unwrap())
        }

        #[cfg(feature = "tree-sitter-yaml")]
        "yaml" | "yml" => HighlightInfo::Highlighted(yaml::highlight_yaml(lines).unwrap()),

        _ => HighlightInfo::Unhighlighted,
    }
}

#[cfg(test)]
#[test]
fn test_equal_length() {
    assert_eq!(HIGHLIGHT_NAMES.len(), COLOR_MAP.len());
}
