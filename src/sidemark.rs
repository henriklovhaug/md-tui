//! Sidemark (MRSF v1.0) sidecar emitter.
//!
//! On clean exit, all saved comments for the active markdown file are written
//! to stdout as a Sidemark YAML document so the user (or a wrapping shell)
//! can pipe them into `<doc>.review.yaml`. Spec: <https://sidemark.org/specification.html>.
//!
//! What we emit per comment (mapping from our internal `Comment`):
//! - `id`            — fresh UUIDv4 per comment, generated at dump time
//! - `author`        — `--username` if set, else `"anonymous"`
//! - `timestamp`     — current UTC moment in RFC 3339
//! - `text`          — the comment body
//! - `resolved`      — always `false` (we don't track resolution)
//! - `line` / `end_line` / `start_column` / `end_column` — recomputed from
//!   `raw_source` byte offsets when available. We don't trust pest's
//!   `SourcePos.line` / `.column`: they drift past the first paragraph and
//!   leak the half-open end across line boundaries (line N+1 col 1 instead
//!   of line N's last column). Inclusive end_line per the spec.
//! - `selected_text` — captured at save time from the raw markdown

use crate::comments::Comment;
use crate::parser::SourceSpan;

const ANONYMOUS_AUTHOR: &str = "anonymous";

#[derive(Debug, Clone, Copy)]
pub struct DumpInputs<'a> {
    pub document: Option<&'a str>,
    pub author: Option<&'a str>,
    pub comments: &'a [Comment],
    /// Raw markdown text for the active document. When supplied, line/column
    /// fields are recomputed from byte offsets — pest's `line_col()` shifts
    /// columns past the first paragraph, so we can't trust `SourcePos.line`
    /// / `.column` for the dump.
    pub raw_source: Option<&'a str>,
}

/// Slice the raw markdown text covered by `span`. Returns `None` if either
/// byte offset is out of bounds or doesn't fall on a UTF-8 char boundary —
/// emitting a partial / invalid `selected_text` would be worse than omitting
/// the field, which the spec allows.
#[must_use]
pub fn slice_source_span(raw: &str, span: SourceSpan) -> Option<String> {
    raw.get(span.start.byte..span.end.byte).map(str::to_string)
}

/// Render the Sidemark YAML document for the given inputs. Returns `None` if
/// there's nothing worth writing (no comments).
#[must_use]
pub fn render(inputs: DumpInputs<'_>) -> Option<String> {
    if inputs.comments.is_empty() {
        return None;
    }
    let timestamp = chrono::Utc::now().to_rfc3339();
    let document = inputs.document.unwrap_or("<stdin>");
    let author = inputs.author.unwrap_or(ANONYMOUS_AUTHOR);

    let mut out = String::new();
    out.push_str("mrsf_version: \"1.0\"\n");
    out.push_str(&format!("document: {}\n", yaml_quoted(document)));
    out.push_str("comments:\n");
    for c in inputs.comments {
        emit_comment(&mut out, c, author, &timestamp, inputs.raw_source);
    }
    Some(out)
}

/// Convert a comment's half-open source span ends into Sidemark's inclusive
/// `(end_line, end_col)`. The internal `SourceSpan` end is half-open (one past
/// the last selected byte); when it sits at column 1 of a later line — i.e. the
/// selection ended exactly at a newline — pull it back to the last column of
/// the previous line.
fn inclusive_end(
    raw: Option<&str>,
    start_line: u32,
    raw_end_line: u32,
    raw_end_col: u32,
) -> (u32, u32) {
    if raw_end_col == 1 && raw_end_line > start_line {
        let prev_line_chars = raw.map_or(0, |raw| line_char_count(raw, raw_end_line - 1));
        (raw_end_line - 1, prev_line_chars + 1)
    } else {
        (raw_end_line, raw_end_col)
    }
}

/// Append one comment's YAML block to `out`. Columns are emitted 0-based per
/// the spec.
fn emit_comment(out: &mut String, c: &Comment, author: &str, timestamp: &str, raw: Option<&str>) {
    let id = uuid::Uuid::new_v4();
    let (start_line, start_col) = position_for(raw, c.source.start);
    let (raw_end_line, raw_end_col) = position_for(raw, c.source.end);
    let (end_line, end_col) = inclusive_end(raw, start_line, raw_end_line, raw_end_col);

    out.push_str(&format!("  - id: {id}\n"));
    out.push_str(&format!("    author: {}\n", yaml_quoted(author)));
    out.push_str(&format!("    timestamp: '{timestamp}'\n"));
    out.push_str(&format!("    text: {}\n", yaml_quoted(&c.text)));
    out.push_str("    resolved: false\n");
    out.push_str(&format!("    line: {start_line}\n"));
    out.push_str(&format!("    end_line: {end_line}\n"));
    out.push_str(&format!(
        "    start_column: {}\n",
        start_col.saturating_sub(1)
    ));
    out.push_str(&format!("    end_column: {}\n", end_col.saturating_sub(1)));
    if let Some(sel) = &c.selected_text {
        out.push_str(&format!("    selected_text: {}\n", yaml_quoted(sel)));
    }
}

fn position_for(raw: Option<&str>, pos: crate::parser::SourcePos) -> (u32, u32) {
    if let Some(raw) = raw {
        line_col_at(raw, pos.byte)
    } else {
        (pos.line, pos.column)
    }
}

/// 1-based `(line, column)` at the given byte position in `raw`. Counts UTF-8
/// characters within the line so multi-byte chars don't inflate the column.
fn line_col_at(raw: &str, byte_pos: usize) -> (u32, u32) {
    let pos = byte_pos.min(raw.len());
    let prefix = &raw[..pos];
    let line = 1 + prefix.bytes().filter(|&b| b == b'\n').count() as u32;
    let line_start = prefix
        .bytes()
        .rposition(|b| b == b'\n')
        .map_or(0, |i| i + 1);
    // Don't count a trailing `\r` that belongs to a CRLF line ending towards the
    // column, otherwise columns on CRLF files are inflated by one.
    let line_prefix = &raw[line_start..pos];
    let line_prefix = line_prefix.strip_suffix('\r').unwrap_or(line_prefix);
    let col = line_prefix.chars().count() as u32 + 1;
    (line, col)
}

/// Number of characters on the given 1-based line in `raw` (excluding the
/// trailing newline). Returns 0 if the line is past the end of the file.
fn line_char_count(raw: &str, line: u32) -> u32 {
    if line == 0 {
        return 0;
    }
    raw.split('\n').nth((line - 1) as usize).map_or(0, |s| {
        s.strip_suffix('\r').unwrap_or(s).chars().count() as u32
    })
}

/// YAML double-quoted scalar with the escapes the spec leans on. Keeps
/// strings deterministic (no flow-style guessing) and survives newlines /
/// special chars without breaking the document structure.
fn yaml_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Other C0 controls — escape via \xNN so YAML stays parseable.
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourcePos;

    fn span(
        line_s: u32,
        col_s: u32,
        line_e: u32,
        col_e: u32,
        byte_s: usize,
        byte_e: usize,
    ) -> SourceSpan {
        SourceSpan {
            start: SourcePos {
                byte: byte_s,
                line: line_s,
                column: col_s,
            },
            end: SourcePos {
                byte: byte_e,
                line: line_e,
                column: col_e,
            },
        }
    }

    #[test]
    fn empty_comment_list_renders_nothing() {
        assert!(
            render(DumpInputs {
                document: Some("foo.md"),
                author: Some("a"),
                comments: &[],
                raw_source: None,
            })
            .is_none()
        );
    }

    #[test]
    fn single_comment_round_trips_required_fields() {
        let comments = vec![Comment {
            source: span(12, 43, 12, 74, 0, 31),
            text: "Is this phrasing correct?".into(),
            selected_text: Some("While many concepts are represented".into()),
        }];
        let yaml = render(DumpInputs {
            document: Some("docs/architecture.md"),
            author: Some("Wictor (wictorwilen)"),
            comments: &comments,
            raw_source: None,
        })
        .expect("expected render to produce yaml");

        assert!(yaml.starts_with("mrsf_version: \"1.0\"\n"));
        assert!(yaml.contains("document: \"docs/architecture.md\"\n"));
        assert!(yaml.contains("comments:\n"));
        assert!(yaml.contains("    author: \"Wictor (wictorwilen)\"\n"));
        assert!(yaml.contains("    text: \"Is this phrasing correct?\"\n"));
        assert!(yaml.contains("    resolved: false\n"));
        assert!(yaml.contains("    line: 12\n"));
        assert!(yaml.contains("    end_line: 12\n"));
        // Spec wants 0-based columns; our SourcePos is 1-based, so 43 -> 42.
        assert!(yaml.contains("    start_column: 42\n"));
        assert!(yaml.contains("    end_column: 73\n"));
        assert!(yaml.contains("    selected_text: \"While many concepts are represented\"\n"));
    }

    #[test]
    fn quoting_escapes_control_chars_and_quotes() {
        let comments = vec![Comment {
            source: span(1, 1, 1, 5, 0, 4),
            text: "line1\nline2 \"quoted\"\tend".into(),
            selected_text: None,
        }];
        let yaml = render(DumpInputs {
            document: Some("a.md"),
            author: None,
            comments: &comments,
            raw_source: None,
        })
        .unwrap();
        // \n, \t and embedded quotes must all be escape-encoded so the doc
        // stays parseable.
        assert!(yaml.contains("\"line1\\nline2 \\\"quoted\\\"\\tend\""));
        // selected_text is absent when None — keeps optional fields clean.
        assert!(!yaml.contains("selected_text"));
    }

    #[test]
    fn missing_document_falls_back_to_stdin_marker() {
        let comments = vec![Comment {
            source: span(1, 1, 1, 5, 0, 4),
            text: "x".into(),
            selected_text: None,
        }];
        let yaml = render(DumpInputs {
            document: None,
            author: None,
            comments: &comments,
            raw_source: None,
        })
        .unwrap();
        assert!(yaml.contains("document: \"<stdin>\"\n"));
        // Default author when --username isn't set.
        assert!(yaml.contains("author: \"anonymous\""));
    }

    #[test]
    fn slice_source_span_returns_substring() {
        let raw = "hello world";
        let s = slice_source_span(raw, span(1, 1, 1, 6, 0, 5));
        assert_eq!(s.as_deref(), Some("hello"));
    }

    #[test]
    fn slice_source_span_rejects_out_of_bounds() {
        let raw = "abc";
        let s = slice_source_span(raw, span(1, 1, 1, 100, 0, 100));
        assert!(s.is_none());
    }

    #[test]
    fn raw_source_overrides_pest_line_col_for_single_line_selection() {
        // Reproduces the user-reported bug: a single-line selection that
        // ended at a trailing space showed `end_line` one past the source
        // line because pest's `line_col()` reports the position past the
        // first paragraph with a shifted column. With raw_source provided,
        // we recompute from byte offsets and produce the right line/col.
        //
        // The Comment's `source` carries deliberately wrong pest values
        // (line 4 col 19 for the end) — exactly the shape the parser was
        // emitting. The dumper must IGNORE these and use byte 0..18 against
        // the raw text to derive line=1 col=1 .. line=1 col=19.
        let raw = "Start the program running.\nrecursively from where.\n";
        let comments = vec![Comment {
            source: span(
                /*sline*/ 1, /*scol*/ 2, /*eline*/ 4, /*ecol*/ 19, 0, 18,
            ),
            text: "Hello".into(),
            selected_text: Some("Start the program ".into()),
        }];
        let yaml = render(DumpInputs {
            document: Some("notes.md"),
            author: Some("@a"),
            comments: &comments,
            raw_source: Some(raw),
        })
        .unwrap();
        assert!(yaml.contains("    line: 1\n"), "got:\n{yaml}");
        assert!(
            yaml.contains("    end_line: 1\n"),
            "end_line must collapse to start_line for a single-line selection; got:\n{yaml}"
        );
        assert!(yaml.contains("    start_column: 0\n"), "got:\n{yaml}");
        assert!(yaml.contains("    end_column: 18\n"), "got:\n{yaml}");
    }

    #[test]
    fn end_at_newline_pulls_back_to_previous_line() {
        // Edge case: a selection that lands exactly at a newline boundary.
        // The half-open end is at col 1 of the next line, but the inclusive
        // `end_line` should be the previous line. The renderer clamps
        // accordingly using the raw source to find the last column of the
        // previous line.
        let raw = "abc\ndef\n";
        let comments = vec![Comment {
            // Selection covers "abc\n" (bytes 0..4), so end is at line 2 col 1.
            source: span(1, 1, 2, 1, 0, 4),
            text: "comment".into(),
            selected_text: Some("abc\n".into()),
        }];
        let yaml = render(DumpInputs {
            document: Some("a.md"),
            author: None,
            comments: &comments,
            raw_source: Some(raw),
        })
        .unwrap();
        assert!(yaml.contains("    line: 1\n"));
        assert!(yaml.contains("    end_line: 1\n"));
        // end_column = last col of line 1 (3 chars + 1 = 4 in 1-based, 3 in 0-based).
        assert!(yaml.contains("    end_column: 3\n"), "got:\n{yaml}");
    }

    #[test]
    fn crlf_line_endings_do_not_inflate_columns() {
        // "abc\r\ndef\r\n": selecting "abc\r\n" (bytes 0..5) lands the half-open
        // end at col 1 of line 2, which pulls back to line 1. The pulled-back
        // end column must be 3 (chars in "abc"), not 4 — i.e. the trailing
        // `\r` is excluded from the char count.
        let raw = "abc\r\ndef\r\n";
        let comments = vec![Comment {
            source: span(1, 1, 2, 1, 0, 5),
            text: "x".into(),
            selected_text: Some("abc\r\n".into()),
        }];
        let yaml = render(DumpInputs {
            document: Some("a.md"),
            author: None,
            comments: &comments,
            raw_source: Some(raw),
        })
        .unwrap();
        assert!(yaml.contains("    line: 1\n"), "got:\n{yaml}");
        assert!(yaml.contains("    end_line: 1\n"), "got:\n{yaml}");
        // 1-based end col 4 -> 0-based 3; a counted `\r` would make it 4.
        assert!(yaml.contains("    end_column: 3\n"), "got:\n{yaml}");
    }

    #[test]
    fn multibyte_chars_counted_as_one_column_each() {
        // "héllo world": é is two bytes. "world" starts at byte 7 but column 7
        // (1-based) because the preceding chars are counted, not bytes.
        let raw = "héllo world";
        let comments = vec![Comment {
            source: span(1, 1, 1, 1, 7, 12),
            text: "c".into(),
            selected_text: Some("world".into()),
        }];
        let yaml = render(DumpInputs {
            document: Some("a.md"),
            author: None,
            comments: &comments,
            raw_source: Some(raw),
        })
        .unwrap();
        // 1-based start col 7 -> 0-based 6; byte-counting would give 7.
        assert!(yaml.contains("    start_column: 6\n"), "got:\n{yaml}");
        assert!(yaml.contains("    end_column: 11\n"), "got:\n{yaml}");
    }

    #[test]
    fn multiple_comments_each_get_a_unique_id_in_order() {
        let comments = vec![
            Comment {
                source: span(1, 1, 1, 4, 0, 3),
                text: "first".into(),
                selected_text: None,
            },
            Comment {
                source: span(2, 1, 2, 4, 4, 7),
                text: "second".into(),
                selected_text: None,
            },
        ];
        let yaml = render(DumpInputs {
            document: Some("a.md"),
            author: None,
            comments: &comments,
            raw_source: Some("abc\ndef\n"),
        })
        .unwrap();
        let ids: Vec<&str> = yaml
            .lines()
            .filter_map(|l| l.trim().strip_prefix("- id: "))
            .collect();
        assert_eq!(ids.len(), 2, "one id per comment");
        assert_ne!(ids[0], ids[1], "ids must be unique per comment");
        // Order is preserved: "first" appears before "second".
        let first_at = yaml.find("first").unwrap();
        let second_at = yaml.find("second").unwrap();
        assert!(first_at < second_at, "comments emitted in input order");
    }

    #[test]
    fn full_envelope_shape_matches_spec_example() {
        // Snapshot of the document-level envelope: required top-level keys
        // appear in the order the spec example shows (mrsf_version, document,
        // comments) and no others sneak in.
        let comments = vec![Comment {
            source: span(9, 1, 9, 50, 0, 49),
            text: "Needs clarity.".into(),
            selected_text: Some("The gateway component routes inbound traffic.".into()),
        }];
        let yaml = render(DumpInputs {
            document: Some("docs/architecture.md"),
            author: Some("rev (rev)"),
            comments: &comments,
            raw_source: None,
        })
        .unwrap();
        let mut lines = yaml.lines();
        assert_eq!(lines.next(), Some("mrsf_version: \"1.0\""));
        assert_eq!(lines.next(), Some("document: \"docs/architecture.md\""));
        assert_eq!(lines.next(), Some("comments:"));
        // The first comment line begins with `  - id:` per spec layout.
        assert!(lines.next().unwrap().starts_with("  - id:"));
    }
}
