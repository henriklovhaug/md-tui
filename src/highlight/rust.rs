use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_rust::language;

use crate::highlight::HIGHLIGHT_NAMES;

pub fn highlight_rust(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlither = Highlighter::new();
    let language = language();

    let mut rust_config =
        HighlightConfiguration::new(language, "rust", tree_sitter_rust::HIGHLIGHT_QUERY, "", "")
            .unwrap();

    rust_config.configure(&HIGHLIGHT_NAMES);

    let highlights = highlither
        .highlight(&rust_config, lines, None, |_| None)
        .unwrap()
        .collect::<Vec<_>>();

    // Unpack the results
    let new: Result<Vec<HighlightEvent>, String> = highlights
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());

    new
}
