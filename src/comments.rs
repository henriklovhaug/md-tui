use crate::parser::SourceSpan;
use crate::util::Caret;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedRange {
    pub start: Caret,
    pub end: Caret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedCommentAnchor {
    pub source: SourceSpan,
    pub rendered: Vec<RenderedRange>,
}

impl ProjectedCommentAnchor {
    /// The anchor's full rendered extent as a `(start, end)` caret pair — the
    /// start of its first rendered range to the end of its last. `None` when
    /// the anchor projects to nothing (off-screen / orphaned span).
    #[must_use]
    pub fn full_range(&self) -> Option<(Caret, Caret)> {
        Some((self.rendered.first()?.start, self.rendered.last()?.end))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub source: SourceSpan,
    pub text: String,
    /// The markdown text covered by `source`, captured at save time, so the
    /// on-exit Sidemark dump can report what the comment was attached to.
    pub selected_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentModeSource {
    Caret,
    Comments,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CommentState {
    #[default]
    Off,
    Browsing,
    Selecting {
        anchor: Caret,
        source: CommentModeSource,
    },
    Editing {
        range: (Caret, Caret),
        draft: String,
        /// Byte offset of the insertion caret in `draft`, always kept on a
        /// `char` boundary (it only ever moves by whole `char`s).
        cursor: usize,
        /// Whether this edit will create a new comment or update an existing
        /// one in place.
        target: EditTarget,
        /// Which mode to restore after finishing or cancelling the edit.
        source: CommentModeSource,
    },
}

impl CommentState {
    #[must_use]
    pub fn shows_sidebar(&self) -> bool {
        matches!(
            self,
            Self::Browsing
                | Self::Editing { .. }
                | Self::Selecting {
                    source: CommentModeSource::Comments,
                    ..
                }
        )
    }
}

impl CommentModeSource {
    #[must_use]
    pub fn restored_state(self) -> CommentState {
        match self {
            Self::Caret => CommentState::Off,
            Self::Comments => CommentState::Browsing,
        }
    }
}

/// What `save_draft` does on commit:
/// - `New` — push the draft as a new entry at the end of `comments`.
/// - `Existing(i)` — update `comments[i].text` in place; range stays put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTarget {
    New,
    Existing(usize),
}

/// Returns `(start, end)` ordered so that `start <= end` in document order
/// (line first, then col). `end` is the exclusive end of the selection.
#[must_use]
pub fn normalize_range(a: Caret, b: Caret) -> (Caret, Caret) {
    if (a.line, a.col) <= (b.line, b.col) {
        (a, b)
    } else {
        (b, a)
    }
}

/// Given the current active index and the number of comments, return the
/// next index, wrapping around. Returns `None` if `len == 0`.
#[must_use]
pub fn next_index(active: Option<usize>, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match active {
        None => 0,
        Some(i) => (i + 1) % len,
    })
}

/// Given the current active index and the number of comments, return the
/// previous index, wrapping around. Returns `None` if `len == 0`.
#[must_use]
pub fn prev_index(active: Option<usize>, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match active {
        None => len - 1,
        Some(i) => (i + len - 1) % len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(line: u16, col: u16) -> Caret {
        Caret { line, col }
    }

    #[test]
    fn normalize_orders_by_line_then_col() {
        assert_eq!(normalize_range(c(3, 10), c(1, 5)), (c(1, 5), c(3, 10)));
        assert_eq!(normalize_range(c(2, 8), c(2, 4)), (c(2, 4), c(2, 8)));
        assert_eq!(normalize_range(c(0, 0), c(0, 0)), (c(0, 0), c(0, 0)));
    }

    #[test]
    fn next_index_wraps() {
        assert_eq!(next_index(None, 0), None);
        assert_eq!(next_index(None, 3), Some(0));
        assert_eq!(next_index(Some(0), 3), Some(1));
        assert_eq!(next_index(Some(2), 3), Some(0));
    }

    #[test]
    fn prev_index_wraps() {
        assert_eq!(prev_index(None, 0), None);
        assert_eq!(prev_index(None, 3), Some(2));
        assert_eq!(prev_index(Some(0), 3), Some(2));
        assert_eq!(prev_index(Some(2), 3), Some(1));
    }

    #[test]
    fn comment_state_default_is_off() {
        assert_eq!(CommentState::default(), CommentState::Off);
    }

    #[test]
    fn cycle_indices_handle_stale_active() {
        // If active is past the end, modular arithmetic still wraps to a
        // valid index. Pin this behaviour so a future deletion path stays sane.
        assert_eq!(next_index(Some(5), 3), Some(0)); // (5+1) % 3
        assert_eq!(prev_index(Some(5), 3), Some(1)); // (5+2) % 3
    }
}
