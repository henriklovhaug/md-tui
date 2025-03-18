use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_go(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = tree_sitter_go::LANGUAGE;

    let mut go_config = HighlightConfiguration::new(
        language.into(),
        "go",
        tree_sitter_go::HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    go_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&go_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
