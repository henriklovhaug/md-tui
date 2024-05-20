use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_php::language_php;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_php(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = language_php();

    let mut php_config =
        HighlightConfiguration::new(language, "php", tree_sitter_php::HIGHLIGHTS_QUERY, "", "")
            .unwrap();

    php_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&php_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
