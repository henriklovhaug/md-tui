use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_diff(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlighter = Highlighter::new();
    let language = tree_sitter_diff::LANGUAGE;

    let mut diff_config = HighlightConfiguration::new(
        language.into(),
        "diff",
        tree_sitter_diff::HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    diff_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlighter.highlight(&diff_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
