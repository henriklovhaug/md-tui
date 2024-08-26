use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_yaml::language;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_yaml(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = language();

    let mut yaml_config =
        HighlightConfiguration::new(language, "yaml", tree_sitter_yaml::HIGHLIGHTS_QUERY, "", "")
            .unwrap();

    yaml_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&yaml_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
