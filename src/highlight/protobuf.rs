use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::highlight::HIGHLIGHT_NAMES;

// Embedded from https://github.com/coder3101/tree-sitter-proto/blob/main/queries/highlights.scm
// MIT License - Copyright (c) 2024-2025 Mohammad Ashar Khan
const HIGHLIGHTS_QUERY: &str = r#"
[
  "syntax"
  "edition"
  "package"
  "option"
  "import"
  "service"
  "rpc"
  "returns"
  "message"
  "enum"
  "oneof"
  "repeated"
  "reserved"
  "to"
] @keyword

[
  (key_type)
  (type)
  (message_name)
  (enum_name)
  (service_name)
  (rpc_name)
]@type

(string) @string

[
  (int_lit)
  (float_lit)
] @constant

[
  (true)
  (false)
] @constant

(comment) @property

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket
"#;

pub fn highlight_protobuf(lines: &[u8]) -> Result<Vec<HighlightEvent>, String> {
    let mut highlighter = Highlighter::new();
    let language = tree_sitter_proto::LANGUAGE;

    let mut proto_config = HighlightConfiguration::new(
        language.into(),
        "protobuf",
        HIGHLIGHTS_QUERY,
        "",
        "",
    )
    .unwrap();

    proto_config.configure(&HIGHLIGHT_NAMES);

    if let Ok(lines) = highlighter.highlight(&proto_config, lines, None, |_| None) {
        lines
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    } else {
        Err("Failed to highlight".to_string())
    }
}
