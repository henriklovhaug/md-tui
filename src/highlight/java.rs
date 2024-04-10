use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_java::language;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_java(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = language();

    let mut java_config =
        HighlightConfiguration::new(language, "java", tree_sitter_java::HIGHLIGHTS_QUERY, "", "")
            .unwrap();

    java_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&java_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
