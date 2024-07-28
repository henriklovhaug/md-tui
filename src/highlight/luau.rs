use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_luau_fork::language;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_luau(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = language();

    let mut luau_config = HighlightConfiguration::new(
        language,
        "luau",
        tree_sitter_luau_fork::HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    luau_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&luau_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
