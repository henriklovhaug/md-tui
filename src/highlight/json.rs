use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_json::language;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_json(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = language();

    let mut json_config =
        HighlightConfiguration::new(language, "json", tree_sitter_json::HIGHLIGHTS_QUERY, "", "")
            .unwrap();

    json_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&json_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
