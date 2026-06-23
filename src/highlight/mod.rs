use std::cell::RefCell;
use std::collections::HashMap;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

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

#[derive(Debug)]
pub enum HighlightInfo {
    Highlighted(Vec<HighlightEvent>),
    Mermaid,
    Unhighlighted,
}

// With every `tree-sitter-*` feature disabled all numbered match arms gate
// out, leaving only the early-return arms — that makes the trailing match
// reachable only with at least one language feature, which is exactly the
// supported configuration.
#[allow(unused_variables, unreachable_code)]
#[must_use]
pub fn highlight_code(language: &str, lines: &[u8]) -> HighlightInfo {
    // The mapping from alias to (tree-sitter language, query) lives inline
    // here because each entry needs its own `#[cfg]` and the upstream crates
    // disagree on the query constant name (HIGHLIGHTS_QUERY vs HIGHLIGHT_QUERY)
    // and on the language constant name. The cache keyed by lang_name avoids
    // rebuilding the `HighlightConfiguration` on every call.
    let result: Result<Vec<HighlightEvent>, String> = match language {
        #[cfg(feature = "tree-sitter-bash")]
        "bash" | "sh" => highlight_with_language(
            lines,
            tree_sitter_bash::LANGUAGE.into(),
            "bash",
            tree_sitter_bash::HIGHLIGHT_QUERY,
        ),

        #[cfg(feature = "tree-sitter-c")]
        "c" => highlight_with_language(
            lines,
            tree_sitter_c::LANGUAGE.into(),
            "c",
            tree_sitter_c::HIGHLIGHT_QUERY,
        ),

        #[cfg(feature = "tree-sitter-cpp")]
        "cpp" => highlight_with_language(
            lines,
            tree_sitter_cpp::LANGUAGE.into(),
            "cpp",
            tree_sitter_cpp::HIGHLIGHT_QUERY,
        ),

        #[cfg(feature = "tree-sitter-css")]
        "css" => highlight_with_language(
            lines,
            tree_sitter_css::LANGUAGE.into(),
            "css",
            tree_sitter_css::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-diff")]
        "diff" | "patch" => highlight_with_language(
            lines,
            tree_sitter_diff::LANGUAGE.into(),
            "diff",
            tree_sitter_diff::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-elixir")]
        "elixir" => highlight_with_language(
            lines,
            tree_sitter_elixir::LANGUAGE.into(),
            "elixir",
            tree_sitter_elixir::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-go")]
        "go" => highlight_with_language(
            lines,
            tree_sitter_go::LANGUAGE.into(),
            "go",
            tree_sitter_go::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-html")]
        "html" => highlight_with_language(
            lines,
            tree_sitter_html::LANGUAGE.into(),
            "html",
            tree_sitter_html::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-java")]
        "java" => highlight_with_language(
            lines,
            tree_sitter_java::LANGUAGE.into(),
            "java",
            tree_sitter_java::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-javascript")]
        "javascript" | "js" => highlight_with_language(
            lines,
            tree_sitter_javascript::LANGUAGE.into(),
            "javascript",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
        ),

        #[cfg(feature = "tree-sitter-json")]
        "json" => highlight_with_language(
            lines,
            tree_sitter_json::LANGUAGE.into(),
            "json",
            tree_sitter_json::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-lua")]
        "lua" => highlight_with_language(
            lines,
            tree_sitter_lua::LANGUAGE.into(),
            "lua",
            tree_sitter_lua::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-ocaml")]
        "ocaml" => highlight_with_language(
            lines,
            tree_sitter_ocaml::LANGUAGE_OCAML_TYPE.into(),
            "ocaml",
            tree_sitter_ocaml::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-php")]
        "php" => highlight_with_language(
            lines,
            tree_sitter_php::LANGUAGE_PHP.into(),
            "php",
            tree_sitter_php::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-python")]
        "python" => highlight_with_language(
            lines,
            tree_sitter_python::LANGUAGE.into(),
            "python",
            tree_sitter_python::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-rust")]
        "rust" => highlight_with_language(
            lines,
            tree_sitter_rust::LANGUAGE.into(),
            "rust",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-scala")]
        "scala" => highlight_with_language(
            lines,
            tree_sitter_scala::LANGUAGE.into(),
            "scala",
            tree_sitter_scala::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-typescript")]
        "tsx" => highlight_with_language(
            lines,
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            "tsx",
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-typescript")]
        "typescript" | "ts" => highlight_with_language(
            lines,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "typescript",
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
        ),

        #[cfg(feature = "tree-sitter-yaml")]
        "yaml" | "yml" => highlight_with_language(
            lines,
            tree_sitter_yaml::LANGUAGE.into(),
            "yaml",
            tree_sitter_yaml::HIGHLIGHTS_QUERY,
        ),

        "mermaid" => return HighlightInfo::Mermaid,

        _ => return HighlightInfo::Unhighlighted,
    };

    match result {
        Ok(events) => HighlightInfo::Highlighted(events),
        Err(_) => HighlightInfo::Unhighlighted,
    }
}

thread_local! {
    /// Per-language `HighlightConfiguration` cache: lock-free, build-on-demand
    /// memoization that needs no synchronization at any thread count and matches
    /// tree-sitter's per-thread `Highlighter` model. (The type is `Sync`, so a
    /// shared `OnceLock`-per-language cache is also possible; not worth it since
    /// highlighting is main-thread-only.)
    static HIGHLIGHT_CONFIGS: RefCell<HashMap<&'static str, HighlightConfiguration>> =
        RefCell::new(HashMap::new());
}

pub fn highlight_with_language(
    lines: &[u8],
    language: tree_sitter::Language,
    lang_name: &'static str,
    query: &str,
) -> Result<Vec<HighlightEvent>, String> {
    HIGHLIGHT_CONFIGS.with(|cell| {
        let mut configs = cell.borrow_mut();
        if !configs.contains_key(lang_name) {
            let mut config = HighlightConfiguration::new(language, lang_name, query, "", "")
                .map_err(|e| e.to_string())?;
            config.configure(&HIGHLIGHT_NAMES);
            configs.insert(lang_name, config);
        }
        let config = configs.get(lang_name).expect("inserted above if missing");

        let mut highlighter = Highlighter::new();
        let events = highlighter
            .highlight(config, lines, None, |_| None)
            .map_err(|e| e.to_string())?;
        events
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_length() {
        assert_eq!(HIGHLIGHT_NAMES.len(), COLOR_MAP.len());
    }

    #[test]
    #[cfg(feature = "tree-sitter-typescript")]
    fn test_highlight_typescript() {
        let code = b"const x: number = 1;";
        let result = highlight_code("typescript", code);
        if let HighlightInfo::Highlighted(events) = result {
            assert!(!events.is_empty());
        } else {
            panic!("Expected Highlighted, got {:?}", result);
        }
    }

    #[test]
    #[cfg(feature = "tree-sitter-typescript")]
    fn test_highlight_tsx() {
        let code = b"const x = <div>hello</div>;";
        let result = highlight_code("tsx", code);
        if let HighlightInfo::Highlighted(events) = result {
            assert!(!events.is_empty());
        } else {
            panic!("Expected Highlighted, got {:?}", result);
        }
    }
}
