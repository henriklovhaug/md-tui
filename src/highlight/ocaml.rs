use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_ocaml::LANGUAGE_OCAML_TYPE;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_ocaml(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = LANGUAGE_OCAML_TYPE;

    let mut ocaml_config = HighlightConfiguration::new(
        language.into(),
        "ocaml",
        tree_sitter_ocaml::HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    ocaml_config.configure(&HIGHLIGHT_NAMES);

    let highlights: Result<Vec<HighlightEvent>, String> =
        if let Ok(lines) = highlither.highlight(&ocaml_config, lines, None, |_| None) {
            lines
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        } else {
            Err("Failed to highlight".to_string())
        };

    highlights
}
