use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_rust(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = tree_sitter_rust::LANGUAGE;

    let mut rust_config = HighlightConfiguration::new(
        language.into(),
        "rust",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    rust_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&rust_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
