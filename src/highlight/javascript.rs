use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_javascript::language;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_javascript(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = language();

    let mut python_config = HighlightConfiguration::new(
        language,
        "javascript",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        "",
        "",
    )
    .unwrap();

    python_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&python_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}