use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::highlight::HIGHLIGHT_NAMES;

// Embedded from https://github.com/tree-sitter-grammars/tree-sitter-toml/blob/master/queries/highlights.scm
// MIT License - Copyright (c) tree-sitter-grammars contributors
// Modified to map @boolean/@comment/@number to HIGHLIGHT_NAMES equivalents
const HIGHLIGHTS_QUERY: &str = r#"
; Properties

(bare_key) @type

(quoted_key) @string

(pair
  (bare_key)) @property

(pair
  (dotted_key
    (bare_key) @property))

; Literals

(boolean) @constant

(comment) @property

(string) @string

[
  (integer)
  (float)
] @constant

[
  (offset_date_time)
  (local_date_time)
  (local_date)
  (local_time)
] @string.special

; Punctuation

[
  "."
  ","
] @punctuation.delimiter

"=" @operator

[
  "["
  "]"
  "[["
  "]]"
  "{"
  "}"
] @punctuation.bracket
"#;

pub fn highlight_toml(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlighter = Highlighter::new();
    let language = tree_sitter_toml_ng::LANGUAGE;

    let mut config = HighlightConfiguration::new(
        language.into(),
        "toml",
        HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    config.configure(&HIGHLIGHT_NAMES);

    if let Ok(lines) = highlighter.highlight(&config, lines, None, |_| None) {
        lines
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    } else {
        Err("Failed to highlight".to_string())
    }
}
