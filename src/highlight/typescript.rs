use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_typescript::LANGUAGE_TYPESCRIPT;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_typescript(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = LANGUAGE_TYPESCRIPT;

    let mut typescript_config = HighlightConfiguration::new(
        language.into(),
        "typescript",
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    typescript_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&typescript_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
