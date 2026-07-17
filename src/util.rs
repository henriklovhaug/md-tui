use std::{cmp, collections::BTreeMap, io};

use crossterm::{
    cursor,
    event::DisableMouseCapture,
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};
use general::GENERAL_CONFIG;

use crate::boxes::{errorbox::ErrorBox, help_box::HelpBox, linkbox::LinkBox, searchbox::SearchBox};
use crate::comments::{
    Comment, CommentModeSource, CommentState, EditTarget, ProjectedCommentAnchor,
};
use crate::nodes::root::ComponentRoot;
use crate::nodes::word::WordType;

pub mod colors;
pub mod general;
pub mod keys;

/// Build the user config layered from `~/.config/mdt/config.toml` (optional)
/// plus `MDT_*` environment variables. Falls back to an empty `Config` if
/// `$HOME` is unset, the path is non-UTF-8, or the builder fails — every
/// downstream `get::<T>(...).unwrap_or(default)` already handles missing keys.
pub(crate) fn load_user_config() -> config::Config {
    let mut builder = config::Config::builder();
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".config").join("mdt").join("config.toml");
        if let Some(s) = path.to_str() {
            builder = builder.add_source(config::File::with_name(s).required(false));
        }
    }
    builder
        .add_source(config::Environment::with_prefix("MDT").separator("_"))
        .build()
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Mode {
    View,
    #[default]
    FileTree,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Boxes {
    Error,
    Search,
    LinkPreview,
    #[default]
    None,
}

impl From<JumpHistory> for Mode {
    fn from(jump_history: JumpHistory) -> Self {
        match jump_history.history.last() {
            Some(jump) => match jump {
                Jump::File(_) => Mode::View,
                Jump::FileTree => Mode::FileTree,
            },
            None => Mode::FileTree,
        }
    }
}

#[derive(Default, Clone)]
pub struct App {
    pub vertical_scroll: u16,
    width: u16,
    pub selected: bool,
    pub select_index: usize,
    pub details_selected: bool,
    pub details_select_index: usize,
    pub mode: Mode,
    pub boxes: Boxes,
    pub history: JumpHistory,
    pub search_box: SearchBox,
    pub message_box: ErrorBox,
    pub help_box: HelpBox,
    pub link_box: LinkBox,
    pub caret_mode: bool,
    pub caret: Caret,
    pub bookmarks: BTreeMap<char, Caret>,
    pub bookmark_origin_width: u16,
    pub pending_input: Option<PendingInput>,
    /// Sticky mark set by `o` at the outline item the user jumped from.
    /// `''` jumps the caret back to it; pressing `''` again while already
    /// at the mark, or pressing ESC, clears it. Independent of bookmarks.
    pub outline_mark: Option<Caret>,
    /// Position recorded on a mouse Down. The first Drag event after this
    /// promotes the click to a comment selection anchored at this position.
    /// Cleared on mouse Up. None when no left button is currently pressed.
    pub mouse_drag_anchor: Option<Caret>,
    /// True if `caret_mode` was forced ON by the mouse handler (because the
    /// user clicked while in scroll mode). Set so that when the resulting
    /// edit / selection cycle ends (save_draft or cancel_editing), we can
    /// flip back to scroll mode rather than stranding the user in caret
    /// mode. Cleared by manual `toggle_caret_mode` so explicit user intent
    /// overrides the auto-restore.
    pub auto_caret_for_comment_edit: bool,
    pub comments: Vec<Comment>,
    pub comment_projections: Vec<ProjectedCommentAnchor>,
    pub comment_state: CommentState,
    pub active_comment: Option<usize>,
    /// Author label shown in each comment card's headline. Set once at startup
    /// from `--username/-u`; `None` falls back to no header (matches the
    /// pre-flag layout).
    pub username: Option<String>,
    /// Raw markdown text most recently parsed. Used to slice `selected_text`
    /// for new comments and for the on-exit Sidemark dump. Reset alongside
    /// `comments` whenever we navigate to a different file.
    pub raw_source: Option<String>,
}

pub enum ScrollAction {
    Down,
    Up,
    ToTop,
    ToBottom,
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
}

pub enum SearchAction {
    Next,
    Previous,
}

pub enum SelectAction {
    Hover,
    SelectLink,
    SelectLinkAlt,
}

impl App {
    pub fn start_search(&mut self, height: u16) {
        self.search_box.clear();
        self.search_box.set_position(2, height.saturating_sub(3));
        self.search_box
            .set_width(GENERAL_CONFIG.width.saturating_sub(3));
        self.boxes = Boxes::Search;
        self.help_box.close();
    }

    pub fn navigate_to_file_tree(&mut self, markdown: &ComponentRoot) {
        self.mode = Mode::FileTree;
        self.help_box.set_mode(Mode::FileTree);
        if let Some(file) = markdown.file_name() {
            self.history.push(Jump::File(file.to_string()));
        }
        self.reset();
    }

    pub fn handle_selection(
        &mut self,
        action: SelectAction,
        markdown: &mut ComponentRoot,
        viewport_height: u16,
    ) {
        match action {
            SelectAction::Hover => self.hover_link(markdown),
            SelectAction::SelectLinkAlt => self.select_closest_link(markdown, viewport_height),
            SelectAction::SelectLink => self.select_top_link(markdown, viewport_height),
        }
    }

    fn hover_link(&mut self, markdown: &ComponentRoot) {
        if !self.selected {
            self.message_box.set_message("No link selected".to_string());
            self.boxes = Boxes::Error;
            return;
        }

        let Some(link) = markdown.selected() else {
            self.message_box.set_message("No link selected".to_string());
            self.boxes = Boxes::Error;
            return;
        };
        if markdown.selected_underlying_type() == Some(WordType::FootnoteInline) {
            self.link_box
                .set_message(format!("Footnote: {}", markdown.find_footnote(link)));
            self.boxes = Boxes::LinkPreview;
            return;
        }

        let message = match LinkType::from(link) {
            LinkType::Internal { heading } => format!("Internal link: {heading}"),
            LinkType::External(e) => format!("External link: {e}"),
            LinkType::MarkdownFile { path, .. } => format!("Markdown file: {path}"),
        };

        self.link_box.set_message(message);
        self.boxes = Boxes::LinkPreview;
    }

    fn select_closest_link(&mut self, markdown: &mut ComponentRoot, viewport_height: u16) {
        let links = markdown.link_index_and_height();
        if links.is_empty() {
            self.message_box.set_message("No links found".to_string());
            self.boxes = Boxes::Error;
            return;
        }

        let next = links
            .iter()
            .min_by_key(|(_, row)| (*row).abs_diff(self.vertical_scroll + viewport_height / 3));

        if let Some((index, _)) = next {
            self.select_index = *index;
            self.scroll_to_selected(markdown, viewport_height);
            self.selected = true;
        }
    }

    fn select_top_link(&mut self, markdown: &mut ComponentRoot, viewport_height: u16) {
        let mut links = markdown.link_index_and_height();
        if links.is_empty() {
            self.message_box.set_message("No links found".to_string());
            self.boxes = Boxes::Error;
            return;
        }

        let mut index = usize::MAX;
        while let Some(top) = links.pop() {
            if top.1 >= self.vertical_scroll || index == usize::MAX {
                index = top.0;
            } else {
                break;
            }
        }

        self.select_index = index;
        self.selected = true;
        self.scroll_to_selected(markdown, viewport_height);
    }

    /// Select the link at `self.select_index` and scroll it to roughly a third
    /// down the viewport (the resting position links share across navigation).
    fn scroll_to_selected(&mut self, markdown: &mut ComponentRoot, viewport_height: u16) {
        if let Ok(scroll) = markdown.select(self.select_index) {
            self.vertical_scroll = scroll.saturating_sub(viewport_height / 3);
        }
    }

    pub fn handle_search(
        &mut self,
        action: SearchAction,
        markdown: &ComponentRoot,
        viewport_height: u16,
    ) {
        let heights = markdown.search_results_heights();
        let mid_viewport = self.vertical_scroll as usize + viewport_height as usize / 2;

        let target = match action {
            SearchAction::Next => heights.iter().find(|&&row| row > mid_viewport),
            SearchAction::Previous => heights.iter().rev().find(|&&row| row < mid_viewport),
        };

        if let Some(&index) = target {
            self.vertical_scroll = (index as u16).saturating_sub(viewport_height / 2);
            self.clamp_scroll(markdown.height(), viewport_height);
        }
    }

    pub fn scroll(
        &mut self,
        action: ScrollAction,
        markdown: &mut ComponentRoot,
        viewport_height: u16,
    ) {
        match action {
            ScrollAction::Down => {
                if self.selected {
                    self.select_index = cmp::min(
                        self.select_index + 1,
                        markdown.num_links().saturating_sub(1),
                    );
                    self.scroll_to_selected(markdown, viewport_height);
                } else {
                    self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                    self.clamp_scroll(markdown.height(), viewport_height);
                }
            }
            ScrollAction::Up => {
                if self.selected {
                    self.select_index = self.select_index.saturating_sub(1);
                    self.scroll_to_selected(markdown, viewport_height);
                } else {
                    self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                }
            }
            ScrollAction::ToTop => self.vertical_scroll = 0,
            ScrollAction::ToBottom => {
                self.vertical_scroll = u16::MAX;
                self.clamp_scroll(markdown.height(), viewport_height);
            }
            ScrollAction::HalfPageDown => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(viewport_height / 2);
                self.clamp_scroll(markdown.height(), viewport_height);
            }
            ScrollAction::HalfPageUp => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(viewport_height / 2);
            }
            ScrollAction::PageDown => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(viewport_height);
                self.clamp_scroll(markdown.height(), viewport_height);
            }
            ScrollAction::PageUp => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(viewport_height);
            }
        }
    }

    pub fn reset(&mut self) {
        self.vertical_scroll = 0;
        self.selected = false;
        self.select_index = 0;
        self.details_selected = false;
        self.details_select_index = 0;
        self.boxes = Boxes::None;
        self.help_box.close();
        self.caret = Caret::default();
        self.caret_mode = false;
        self.pending_input = None;
        self.outline_mark = None;
        self.mouse_drag_anchor = None;
        self.auto_caret_for_comment_edit = false;
        self.comments.clear();
        self.comment_projections.clear();
        self.comment_state = CommentState::Off;
        self.active_comment = None;
        self.raw_source = None;
    }

    pub fn set_width(&mut self, width: u16) -> bool {
        let temp_width = self.width;
        self.width = cmp::min(width, GENERAL_CONFIG.width);
        temp_width != self.width
    }

    #[must_use]
    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn move_caret(&mut self, dy: i32, dx: i32, max_line: u16, viewport_height: u16) {
        let line_max = max_line.saturating_sub(1) as i32;
        let col_max = self.width.saturating_sub(1) as i32;
        let new_line = (self.caret.line as i32 + dy).clamp(0, line_max.max(0)) as u16;
        let new_col = (self.caret.col as i32 + dx).clamp(0, col_max.max(0)) as u16;
        self.caret.line = new_line;
        self.caret.col = new_col;
        self.ensure_caret_visible(viewport_height);
    }

    pub fn ensure_caret_visible(&mut self, viewport_height: u16) {
        if viewport_height == 0 {
            return;
        }
        if self.caret.line < self.vertical_scroll {
            self.vertical_scroll = self.caret.line;
        } else if self.caret.line >= self.vertical_scroll + viewport_height {
            self.vertical_scroll = self.caret.line.saturating_sub(viewport_height - 1);
        }
    }

    /// True when the caret's line is not within the current viewport (or the
    /// viewport has zero rows).
    fn caret_offscreen(&self, viewport_height: u16) -> bool {
        viewport_height == 0
            || self.caret.line < self.vertical_scroll
            || self.caret.line >= self.vertical_scroll + viewport_height
    }

    /// Move the caret to `target` and center its line in the viewport. Used by
    /// outline-mark, bookmark, and comment-cycle jumps.
    fn jump_caret_centered(&mut self, target: Caret, viewport_height: u16) {
        self.caret = target;
        if viewport_height > 0 {
            self.vertical_scroll = target.line.saturating_sub(viewport_height / 2);
        }
    }

    pub fn clamp_scroll(&mut self, markdown_height: u16, viewport_height: u16) {
        self.vertical_scroll = cmp::min(
            self.vertical_scroll,
            markdown_height.saturating_sub(viewport_height / 2),
        );
    }

    pub fn toggle_caret_mode(&mut self, viewport_height: u16) {
        // Manual toggle overrides any auto-entry: if the user is explicitly
        // flipping mode, we shouldn't sneak them back into the previous mode
        // when they next save/cancel a comment edit.
        self.auto_caret_for_comment_edit = false;
        self.caret_mode = !self.caret_mode;
        if self.caret_mode {
            // Snap caret into the visible viewport if it's currently outside.
            if self.caret_offscreen(viewport_height) {
                self.caret.line = self.vertical_scroll;
                self.caret.col = 0;
            }
        } else if matches!(
            self.comment_state,
            CommentState::Selecting { .. } | CommentState::Editing { .. }
        ) {
            // Selecting/Editing rely on the caret cursor — bail out of them
            // when caret mode goes away. Browsing is mode-independent and
            // keeps running so the user can still navigate comments.
            self.exit_comment_mode();
        }
    }

    pub fn caret_to_line_start(&mut self) {
        self.caret.col = 0;
    }

    pub fn caret_to_line_end(&mut self) {
        self.caret.col = self.width.saturating_sub(1);
    }

    pub fn caret_to_top(&mut self, viewport_height: u16) {
        self.caret.line = 0;
        self.ensure_caret_visible(viewport_height);
    }

    pub fn caret_to_bottom(&mut self, max_line: u16, viewport_height: u16) {
        self.caret.line = max_line.saturating_sub(1);
        self.ensure_caret_visible(viewport_height);
    }

    pub fn set_bookmark(&mut self, ch: char) {
        self.bookmarks.insert(ch, self.caret);
    }

    /// Records the current caret position as the outline mark. Used by `o`
    /// to remember the outline item we're jumping from so `''` can return.
    pub fn set_outline_mark(&mut self) {
        self.outline_mark = Some(self.caret);
    }

    /// Clears the outline mark. Called by ESC and when `''` is pressed while
    /// already at the marked location.
    pub fn clear_outline_mark(&mut self) {
        self.outline_mark = None;
    }

    /// `''` semantics: if a mark is set and the caret isn't already on its
    /// line, jump the caret to the mark and center the viewport. If the
    /// caret is already on the mark's line, clear the mark. No-op when no
    /// mark is set.
    pub fn jump_to_outline_mark(&mut self, viewport_height: u16) {
        let Some(mark) = self.outline_mark else {
            return;
        };
        if self.caret.line == mark.line {
            self.outline_mark = None;
            return;
        }
        self.jump_caret_centered(mark, viewport_height);
    }

    #[must_use = "the bool reports whether the named bookmark exists"]
    pub fn jump_bookmark(&mut self, ch: char, viewport_height: u16) -> bool {
        if let Some(target) = self.bookmarks.get(&ch).copied() {
            self.jump_caret_centered(target, viewport_height);
            true
        } else {
            false
        }
    }

    #[must_use = "the bool reports whether the transition was made"]
    pub fn enter_comment_mode(&mut self) -> bool {
        if self.comment_state != CommentState::Off {
            return false;
        }
        self.pending_input = None;
        self.comment_state = CommentState::Browsing;
        true
    }

    #[must_use = "the bool reports whether comment mode toggled"]
    pub fn toggle_comment_mode(&mut self) -> bool {
        match self.comment_state {
            CommentState::Off => self.enter_comment_mode(),
            _ => {
                self.exit_comment_mode();
                true
            }
        }
    }

    pub fn exit_comment_mode(&mut self) {
        self.comment_state = CommentState::Off;
        self.active_comment = None;
    }

    #[must_use = "the bool reports whether selection actually started"]
    pub fn start_selecting(&mut self) -> bool {
        self.start_selecting_with_anchor(self.caret)
    }

    /// Like `start_selecting` but with an explicit anchor — used by the mouse
    /// handler so the anchor is the *original* click position rather than the
    /// caret's current value (which the drag has already moved).
    #[must_use = "the bool reports whether selection actually started"]
    pub fn start_selecting_with_anchor(&mut self, anchor: Caret) -> bool {
        let Some(source) = self.comment_interaction_source() else {
            return false;
        };
        self.pending_input = None;
        self.comment_state = CommentState::Selecting { anchor, source };
        true
    }

    pub fn commit_selection_to_editing(&mut self, markdown: &ComponentRoot) {
        if let CommentState::Selecting { anchor, source } = self.comment_state {
            self.pending_input = None;

            let Some(source_span) = markdown.resolve_selection_to_source(anchor, self.caret) else {
                // If selection doesn't touch any source-backed words, just cancel
                self.comment_state = source.restored_state();
                return;
            };

            // The selection's own caret range, used for a new comment and as
            // the fallback when an existing comment hasn't projected. End col
            // is exclusive; clamp to the pane width so it never points past the
            // rendered grid (and never wraps).
            let (s, e) = crate::comments::normalize_range(anchor, self.caret);
            let selection_range = (
                s,
                Caret {
                    line: e.line,
                    col: e.col.saturating_add(1).min(self.width),
                },
            );

            // Range is the stable identity for saved comments: an existing
            // comment keeps its projected range, a new one takes the selection.
            let (target, draft, range) =
                if let Some(i) = self.comments.iter().position(|c| c.source == source_span) {
                    self.active_comment = Some(i);
                    let range = self
                        .comment_projections
                        .get(i)
                        .and_then(ProjectedCommentAnchor::full_range)
                        .unwrap_or(selection_range);
                    (
                        EditTarget::Existing(i),
                        self.comments[i].text.clone(),
                        range,
                    )
                } else {
                    (EditTarget::New, String::new(), selection_range)
                };

            let cursor = draft.len();
            self.comment_state = CommentState::Editing {
                range,
                draft,
                cursor,
                target,
                source,
            };
        }
    }

    fn start_editing_comment(&mut self, idx: usize, source: CommentModeSource) {
        let Some(comment) = self.comments.get(idx) else {
            return;
        };
        self.pending_input = None;
        let draft = comment.text.clone();
        let cursor = draft.len();

        // Visual range from the projection (empty/missing -> origin fallback).
        let range = self
            .comment_projections
            .get(idx)
            .and_then(ProjectedCommentAnchor::full_range)
            .unwrap_or((Caret::default(), Caret::default()));

        self.comment_state = CommentState::Editing {
            range,
            draft,
            cursor,
            target: EditTarget::Existing(idx),
            source,
        };
    }

    /// Enter Editing mode on the focused comment, or the comment under the
    /// caret if no explicit focus is set.
    #[must_use = "the bool reports whether editing started"]
    pub fn start_editing_active_or_caret(&mut self) -> bool {
        let Some(source) = self.comment_interaction_source() else {
            return false;
        };

        let idx = match source {
            // Browsing: prefer `active_comment` (set by n/N navigation, the
            // only meaningful "focus" signal in scroll mode where the caret
            // isn't rendered). Fall back to caret position if no active idx
            // or the index is stale.
            CommentModeSource::Comments => self
                .active_comment
                .filter(|&i| i < self.comments.len())
                .or_else(|| self.comment_index_under_caret()),
            // Off + caret_mode: the caret position alone decides. Any stale
            // `active_comment` left over from a prior Browsing session is
            // ignored.
            CommentModeSource::Caret => self.comment_index_under_caret(),
        };

        let Some(idx) = idx else {
            return false;
        };
        self.active_comment = Some(idx);
        self.start_editing_comment(idx, source);
        true
    }

    fn comment_index_under_caret(&self) -> Option<usize> {
        self.comment_projections.iter().position(|p| {
            p.rendered
                .iter()
                .any(|r| caret_in_range(self.caret, (r.start, r.end)))
        })
    }

    pub fn save_draft(&mut self, markdown: &ComponentRoot) {
        // `save_draft` is only meaningful while Editing; bail (without taking
        // the state) otherwise so a stray call can't flip a live mode to Off.
        if !matches!(self.comment_state, CommentState::Editing { .. }) {
            return;
        }
        let CommentState::Editing {
            target,
            draft,
            source,
            range,
            ..
        } = std::mem::take(&mut self.comment_state)
        else {
            return; // unreachable: guarded above
        };

        let draft_is_empty = draft.trim().is_empty();
        match target {
            EditTarget::New => {
                // Discard an empty draft instead of persisting a blank comment
                // that can never be removed (there is no separate delete path).
                if !draft_is_empty
                    && let Some(source_span) =
                        markdown.resolve_selection_to_source(range.0, range.1)
                {
                    let selected_text = self
                        .raw_source
                        .as_deref()
                        .and_then(|raw| crate::sidemark::slice_source_span(raw, source_span));
                    let comment = Comment {
                        source: source_span,
                        text: draft,
                        selected_text,
                    };
                    self.comments.push(comment);
                }
                self.active_comment = None;
            }
            EditTarget::Existing(i) => {
                if i < self.comments.len() {
                    if draft_is_empty {
                        // Clearing the text of an existing comment deletes it,
                        // doubling as the delete affordance.
                        self.comments.remove(i);
                        self.active_comment = None;
                    } else {
                        self.comments[i].text = draft;
                    }
                }
            }
        }
        self.comment_projections = markdown.project_comments(&self.comments);
        self.comment_state = source.restored_state();
        self.restore_scroll_mode_if_auto_entered();
    }

    pub fn cancel_editing(&mut self) {
        let restored = match self.comment_state {
            CommentState::Editing { source, .. } | CommentState::Selecting { source, .. } => {
                Some(source.restored_state())
            }
            _ => None,
        };
        if let Some(state) = restored {
            self.comment_state = state;
            self.restore_scroll_mode_if_auto_entered();
        }
    }

    /// Re-synchronise comment state after the document is reparsed (terminal
    /// resize or an external file change). The rendered layout — and therefore
    /// every projected highlight — has shifted, so an in-flight selection or
    /// draft is anchored to a stale layout and is cancelled, and the cached
    /// projections are recomputed against the freshly parsed document.
    pub fn resync_comments_after_reparse(&mut self, markdown: &ComponentRoot) {
        self.cancel_editing();
        self.comment_projections = markdown.project_comments(&self.comments);
    }

    /// If `caret_mode` was auto-entered by the mouse handler for the cycle
    /// that's just ending, flip back to scroll mode so the user lands where
    /// they came from. No-op when `caret_mode` was already on at click time
    /// (or after a manual `toggle_caret_mode` cleared the flag).
    fn restore_scroll_mode_if_auto_entered(&mut self) {
        if self.auto_caret_for_comment_edit {
            self.auto_caret_for_comment_edit = false;
            self.caret_mode = false;
        }
    }

    pub fn cycle_comment(&mut self, forward: bool, viewport_height: u16) {
        if !matches!(
            self.comment_state,
            CommentState::Off | CommentState::Browsing
        ) {
            return;
        }
        let len = self.comments.len();
        if len == 0 {
            return;
        }
        let next = if forward {
            crate::comments::next_index(self.active_comment, len)
        } else {
            crate::comments::prev_index(self.active_comment, len)
        };
        self.active_comment = next;
        if let Some(idx) = next
            && let Some(projection) = self.comment_projections.get(idx)
            && let Some(first_range) = projection.rendered.first()
        {
            self.jump_caret_centered(first_range.start, viewport_height);
        }
    }

    pub fn shows_comment_sidebar(&self) -> bool {
        self.comment_state.shows_sidebar()
    }

    fn comment_interaction_source(&self) -> Option<CommentModeSource> {
        match self.comment_state {
            // Caret-rooted selection needs caret mode on (otherwise the
            // anchor would come from a non-rendered cursor).
            CommentState::Off if self.caret_mode => Some(CommentModeSource::Caret),
            // Browsing is mode-independent: n/N navigation and Enter-to-edit
            // work in scroll mode too.
            CommentState::Browsing => Some(CommentModeSource::Comments),
            _ => None,
        }
    }
}

/// Returns `true` if `point` lies inside `[start, end]`, inclusive on both
/// ends. We treat the end as inclusive here even though stored ranges are
/// half-open: the user's caret commonly lands one cell past the visible
/// highlight (the end-exclusive position), and "click anywhere on or just
/// past the highlight" should still target the comment.
fn caret_in_range(point: Caret, range: (Caret, Caret)) -> bool {
    let p = (point.line, point.col);
    let s = (range.0.line, range.0.col);
    let e = (range.1.line, range.1.col);
    s <= p && p <= e
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Caret {
    pub line: u16,
    pub col: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingInput {
    BookmarkSet,
    BookmarkJump,
}

pub enum LinkType<'a> {
    Internal {
        heading: &'a str,
    },
    External(&'a str),
    MarkdownFile {
        path: String,
        heading: Option<String>,
    },
}

impl<'a> From<&'a str> for LinkType<'a> {
    fn from(s: &'a str) -> Self {
        if s.starts_with('#') {
            return Self::Internal { heading: s };
        }

        // A URL with an explicit scheme (http://, https://, mailto:, …) is
        // always external, even when its path ends in `.md` — otherwise
        // `https://example.com/page.md` would be mangled into a local file.
        if s.contains("://") || s.starts_with("mailto:") {
            return Self::External(s);
        }

        if s.ends_with(".md") || !s.contains('.') || s.contains(".md#") {
            let s = s.strip_prefix('/').unwrap_or(s);
            let (path, heading) = if let Some((path, heading)) = s.split_once('#') {
                (path.to_string(), Some(heading.to_string().to_lowercase()))
            } else {
                (s.to_string(), None)
            };

            let path = if path.ends_with(".md") {
                path
            } else {
                format!("{path}.md")
            };

            return Self::MarkdownFile { path, heading };
        }

        Self::External(s)
    }
}

pub fn destruct_terminal() {
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();
}

#[derive(Debug, Clone)]
pub struct JumpHistory {
    history: Vec<Jump>,
}

impl JumpHistory {
    #[must_use]
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn push(&mut self, jump: Jump) {
        self.history.push(jump);
    }

    pub fn pop(&mut self) -> Jump {
        if let Some(jump) = self.history.pop() {
            jump
        } else {
            Jump::FileTree
        }
    }
}

impl Default for JumpHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Jump {
    File(String),
    FileTree,
}

#[cfg(test)]
pub(crate) mod test_utils {
    use super::*;
    use crate::comments::{ProjectedCommentAnchor, RenderedRange};
    use crate::nodes::{
        root::ComponentRoot,
        textcomponent::{TextComponent, TextNode},
        word::{Word, WordType},
    };
    use crate::parser::{SourcePos, SourceSpan};

    pub fn mock_pos(byte: usize) -> SourcePos {
        SourcePos {
            byte,
            line: (byte / 100) as u32,
            column: (byte % 100) as u32,
        }
    }

    pub fn mock_span(start: usize, end: usize) -> SourceSpan {
        SourceSpan {
            start: mock_pos(start),
            end: mock_pos(end),
        }
    }

    pub fn mock_markdown() -> ComponentRoot {
        ComponentRoot::new(None, Vec::new())
    }

    pub fn mock_markdown_with_span(
        line: u16,
        col: u16,
        len: u16,
        span: SourceSpan,
    ) -> ComponentRoot {
        let mut words = Vec::new();
        if col > 0 {
            words.push(Word::new(" ".repeat(col as usize), WordType::Normal));
        }
        words.push(Word::new_with_source_span(
            " ".repeat(len as usize),
            WordType::Normal,
            Some(span),
        ));
        let mut comp = TextComponent::new(TextNode::Paragraph, words);
        comp.set_y_offset(line);
        ComponentRoot::new(
            None,
            vec![crate::nodes::root::Component::TextComponent(comp)],
        )
    }

    /// A document `rows` lines tall (one component), for scroll-clamp tests.
    pub fn tall_markdown(rows: u16) -> ComponentRoot {
        let content: Vec<Vec<Word>> = (0..rows)
            .map(|_| vec![Word::new("x".to_owned(), WordType::Normal)])
            .collect();
        let comp = TextComponent::new_formatted(TextNode::Paragraph, content);
        ComponentRoot::new(
            None,
            vec![crate::nodes::root::Component::TextComponent(comp)],
        )
    }

    /// A parsed document containing a single link.
    pub fn markdown_with_link() -> ComponentRoot {
        crate::parser::parse_markdown(None, "[text](https://example.com)", 40)
    }

    pub fn push_comment(app: &mut App, line: u16, col: u16, len: u16, text: &str) {
        use crate::comments::Comment;
        let span = mock_span(
            (line * 100 + col) as usize,
            (line * 100 + col + len) as usize,
        );
        app.comments.push(Comment {
            source: span,
            text: text.into(),
            selected_text: None,
        });
        app.comment_projections.push(ProjectedCommentAnchor {
            source: span,
            rendered: vec![RenderedRange {
                start: Caret { line, col },
                end: Caret {
                    line,
                    col: col + len + 1,
                },
            }],
        });
    }

    pub fn app_with_width(w: u16) -> App {
        App {
            width: w,
            ..App::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::{CommentModeSource, RenderedRange};
    use test_utils::*;

    #[test]
    fn link_type_classifies_schemes_paths_and_anchors() {
        // Scheme'd URLs are external even when the path ends in `.md`.
        assert!(matches!(
            LinkType::from("https://example.com/page.md"),
            LinkType::External("https://example.com/page.md")
        ));
        assert!(matches!(
            LinkType::from("http://example.com"),
            LinkType::External(_)
        ));
        assert!(matches!(
            LinkType::from("mailto:a@b.com"),
            LinkType::External(_)
        ));

        // In-page anchors are internal.
        assert!(matches!(
            LinkType::from("#heading"),
            LinkType::Internal {
                heading: "#heading"
            }
        ));

        // Local markdown files (with/without extension, with anchor).
        assert!(matches!(
            LinkType::from("notes.md"),
            LinkType::MarkdownFile { path, heading: None } if path == "notes.md"
        ));
        assert!(matches!(
            LinkType::from("notes.md#intro"),
            LinkType::MarkdownFile { path, heading: Some(h) } if path == "notes.md" && h == "intro"
        ));
    }

    #[test]
    fn scroll_to_top_and_up_floor_at_zero() {
        let mut app = app_with_width(40);
        app.vertical_scroll = 50;
        app.scroll(ScrollAction::ToTop, &mut tall_markdown(100), 10);
        assert_eq!(app.vertical_scroll, 0);
        // Scrolling up from the top stays at 0 (saturating).
        app.scroll(ScrollAction::Up, &mut tall_markdown(100), 10);
        assert_eq!(app.vertical_scroll, 0);
    }

    #[test]
    fn scroll_half_page_and_page_up_subtract_without_underflow() {
        let mut app = app_with_width(40);
        app.vertical_scroll = 10;
        app.scroll(ScrollAction::HalfPageUp, &mut tall_markdown(100), 10);
        assert_eq!(app.vertical_scroll, 5); // 10 - 10/2
        app.scroll(ScrollAction::PageUp, &mut tall_markdown(100), 8);
        assert_eq!(app.vertical_scroll, 0); // 5 - 8 saturates to 0
    }

    #[test]
    fn scroll_down_and_to_bottom_clamp_to_document_height() {
        // height 30, viewport 10: clamp ceiling is 30 - 10/2 = 25.
        let mut app = app_with_width(40);
        app.scroll(ScrollAction::Down, &mut tall_markdown(30), 10);
        assert_eq!(app.vertical_scroll, 1);
        app.scroll(ScrollAction::ToBottom, &mut tall_markdown(30), 10);
        assert_eq!(app.vertical_scroll, 25);
    }

    #[test]
    fn select_top_link_errors_when_document_has_no_links() {
        let mut app = app_with_width(40);
        app.select_top_link(&mut mock_markdown(), 10);
        assert!(!app.selected);
        assert_eq!(app.boxes, Boxes::Error);
    }

    #[test]
    fn select_top_link_marks_selected_when_a_link_exists() {
        let mut app = app_with_width(40);
        app.select_top_link(&mut markdown_with_link(), 10);
        assert!(app.selected);
        assert_ne!(app.boxes, Boxes::Error);
    }

    #[test]
    fn test_jump_history() {
        let mut jump_history = JumpHistory::default();
        jump_history.push(Jump::File("file".to_string()));
        jump_history.push(Jump::File("file2".to_string()));
        jump_history.push(Jump::FileTree);
        assert_eq!(jump_history.pop(), Jump::FileTree);
        assert_eq!(jump_history.pop(), Jump::File("file2".to_string()));
        assert_eq!(jump_history.pop(), Jump::File("file".to_string()));
        assert_eq!(jump_history.pop(), Jump::FileTree);
        assert_eq!(jump_history.pop(), Jump::FileTree);
        assert_eq!(jump_history.pop(), Jump::FileTree);
    }

    #[test]
    fn caret_motion_clamps_to_bounds() {
        let mut app = app_with_width(40);
        app.move_caret(-5, -5, 100, 20); // already at (0, 0)
        assert_eq!(app.caret, Caret { line: 0, col: 0 });

        app.move_caret(1000, 1000, 100, 20);
        assert_eq!(app.caret, Caret { line: 99, col: 39 });
    }

    #[test]
    fn caret_motion_scrolls_viewport_when_leaving() {
        let mut app = app_with_width(40);
        // Move caret down past viewport bottom (height 20).
        app.move_caret(25, 0, 100, 20);
        assert_eq!(app.caret.line, 25);
        assert_eq!(app.vertical_scroll, 25 - (20 - 1));
        // Move caret back above the top.
        app.move_caret(-30, 0, 100, 20);
        assert_eq!(app.caret.line, 0);
        assert_eq!(app.vertical_scroll, 0);
    }

    #[test]
    fn ensure_caret_visible_no_op_inside_viewport() {
        let mut app = app_with_width(40);
        app.vertical_scroll = 10;
        app.caret = Caret { line: 15, col: 0 };
        app.ensure_caret_visible(20);
        assert_eq!(app.vertical_scroll, 10);
    }

    #[test]
    fn toggle_caret_mode_snaps_when_offscreen() {
        let mut app = app_with_width(40);
        app.vertical_scroll = 50;
        app.caret = Caret { line: 5, col: 7 }; // off-screen above
        app.toggle_caret_mode(20);
        assert!(app.caret_mode);
        assert_eq!(app.caret, Caret { line: 50, col: 0 });
    }

    #[test]
    fn toggle_caret_mode_keeps_position_when_visible() {
        let mut app = app_with_width(40);
        app.vertical_scroll = 50;
        app.caret = Caret { line: 55, col: 7 };
        app.toggle_caret_mode(20);
        assert!(app.caret_mode);
        assert_eq!(app.caret, Caret { line: 55, col: 7 });
    }

    #[test]
    fn jump_bookmark_centers_viewport() {
        let mut app = app_with_width(40);
        app.caret = Caret { line: 12, col: 5 };
        app.set_bookmark('a');
        app.caret = Caret { line: 200, col: 0 };
        app.vertical_scroll = 180;
        let hit = app.jump_bookmark('a', 20);
        assert!(hit);
        assert_eq!(app.caret, Caret { line: 12, col: 5 });
        assert_eq!(app.vertical_scroll, 12u16.saturating_sub(10));
    }

    #[test]
    fn jump_bookmark_returns_false_for_missing() {
        let mut app = app_with_width(40);
        assert!(!app.jump_bookmark('z', 20));
    }

    #[test]
    fn reset_clears_comment_state() {
        use crate::comments::{Comment, CommentState};
        let mut app = app_with_width(40);
        app.comments.push(Comment {
            source: mock_span(0, 4),
            text: "x".into(),
            selected_text: None,
        });
        app.comment_state = CommentState::Browsing;
        app.active_comment = Some(0);
        app.reset();
        assert!(app.comments.is_empty());
        assert_eq!(app.comment_state, CommentState::Off);
        assert_eq!(app.active_comment, None);
    }

    #[test]
    fn default_app_has_off_comment_state() {
        use crate::comments::CommentState;
        let app = App::default();
        assert!(app.comments.is_empty());
        assert_eq!(app.comment_state, CommentState::Off);
        assert_eq!(app.active_comment, None);
    }

    #[test]
    fn enter_comment_mode_works_without_caret_mode() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        // Browsing is mode-independent: 'c' from scroll mode must enter
        // comment mode without first toggling caret on.
        assert!(!app.caret_mode);
        assert!(app.enter_comment_mode());
        assert_eq!(app.comment_state, CommentState::Browsing);
        assert!(!app.caret_mode, "caret mode must stay off after `c`");
    }

    #[test]
    fn toggle_comment_mode_round_trip_in_scroll_mode() {
        use crate::comments::CommentState;
        // Regression: pressing the comment key from scroll mode used to be
        // a silent no-op because both enter/toggle gated on caret_mode.
        let mut app = app_with_width(40);
        assert!(!app.caret_mode);
        assert!(app.toggle_comment_mode());
        assert_eq!(app.comment_state, CommentState::Browsing);
        assert!(app.toggle_comment_mode());
        assert_eq!(app.comment_state, CommentState::Off);
        assert!(!app.caret_mode, "scroll/caret state must not leak");
    }

    #[test]
    fn exit_comment_mode_clears_state_keeps_comments() {
        use crate::comments::{Comment, CommentState};
        let mut app = app_with_width(40);
        app.comments.push(Comment {
            source: mock_span(0, 1),
            text: "y".into(),
            selected_text: None,
        });
        app.comment_state = CommentState::Browsing;
        app.active_comment = Some(0);
        app.exit_comment_mode();
        assert_eq!(app.comment_state, CommentState::Off);
        assert_eq!(app.active_comment, None);
        assert_eq!(app.comments.len(), 1);
    }

    #[test]
    fn start_selecting_anchors_at_current_caret() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.caret = Caret { line: 5, col: 12 };
        app.comment_state = CommentState::Browsing;
        assert!(app.start_selecting());
        assert_eq!(
            app.comment_state,
            CommentState::Selecting {
                anchor: Caret { line: 5, col: 12 },
                source: CommentModeSource::Comments,
            }
        );
    }

    #[test]
    fn start_selecting_from_caret_mode_keeps_comment_mode_off() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.caret = Caret { line: 2, col: 7 };
        assert!(app.start_selecting());
        assert_eq!(
            app.comment_state,
            CommentState::Selecting {
                anchor: Caret { line: 2, col: 7 },
                source: CommentModeSource::Caret,
            }
        );
        assert!(!app.shows_comment_sidebar());
    }

    #[test]
    fn start_selecting_requires_caret_mode() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.comment_state = CommentState::Off;
        assert!(!app.start_selecting());
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn commit_selection_to_editing_normalizes_range() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.caret = Caret { line: 1, col: 4 };
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 3, col: 10 },
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(1, 4, 30, mock_span(10, 40));
        app.commit_selection_to_editing(&markdown);
        match app.comment_state {
            CommentState::Editing {
                range,
                draft,
                cursor,
                target,
                source,
            } => {
                // End col is bumped by 1 so the stored range covers the
                // cells the user visually highlighted (caret cell included).
                assert_eq!(
                    range,
                    (Caret { line: 1, col: 4 }, Caret { line: 3, col: 11 })
                );
                assert!(draft.is_empty());
                assert_eq!(cursor, 0);
                assert_eq!(target, crate::comments::EditTarget::New);
                assert_eq!(source, CommentModeSource::Comments);
            }
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    #[test]
    fn screenshot_scenario_does_not_duplicate_comment() {
        // Reproduces the bug from the user's screenshot: an existing
        // comment "World" at L21c0..L21c11. The user puts the caret
        // inside that range (e.g. L21c8) and presses Enter. The fix must
        // edit the existing comment in place rather than create a
        // duplicate. This is a simple point-in-range check.
        use crate::comments::{Comment, CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        let span1 = mock_span(0, 5);
        let span2 = mock_span(210, 221);
        app.comments.push(Comment {
            source: span1,
            text: "Hello".into(),
            selected_text: None,
        });
        app.comments.push(Comment {
            source: span2,
            text: "World".into(),
            selected_text: None,
        });

        // Mock markdown that projects these comments
        let markdown = mock_markdown_with_span(21, 0, 11, span2);
        app.comment_projections = vec![
            ProjectedCommentAnchor {
                source: span1,
                rendered: vec![RenderedRange {
                    start: Caret { line: 0, col: 0 },
                    end: Caret { line: 0, col: 6 },
                }],
            },
            ProjectedCommentAnchor {
                source: span2,
                rendered: vec![RenderedRange {
                    start: Caret { line: 21, col: 0 },
                    end: Caret { line: 21, col: 12 },
                }],
            },
        ];

        // Selecting the whole "World" range (L21c0..L21c11).
        app.caret = Caret { line: 21, col: 11 };
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 21, col: 0 },
            source: CommentModeSource::Comments,
        };
        app.commit_selection_to_editing(&markdown);
        match &app.comment_state {
            CommentState::Editing {
                range,
                target,
                draft,
                ..
            } => {
                // Editor adopts the EXISTING range and text.
                assert_eq!(
                    *range,
                    (Caret { line: 21, col: 0 }, Caret { line: 21, col: 12 })
                );
                assert_eq!(draft, "World");
                assert_eq!(*target, EditTarget::Existing(1));
            }
            other => panic!("expected Editing(Existing), got {other:?}"),
        }
        // Saving must update World in place, NOT push a third comment.
        app.save_draft(&markdown);
        assert_eq!(
            app.comments.len(),
            2,
            "save_draft must not duplicate when target is Existing"
        );
        assert_eq!(app.comments[1].text, "World");
    }

    #[test]
    fn caret_in_range_basics() {
        let caret = |l: u16, c: u16| Caret { line: l, col: c };
        let r = (caret(2, 0), caret(2, 5));
        // Inside
        assert!(caret_in_range(caret(2, 0), r));
        assert!(caret_in_range(caret(2, 4), r));
        // End position INCLUDED (deliberately forgiving — user's caret often
        // lands one past the visible highlight).
        assert!(caret_in_range(caret(2, 5), r));
        // Beyond end
        assert!(!caret_in_range(caret(2, 6), r));
        // Before start
        assert!(!caret_in_range(caret(1, 99), r));
        // Different line
        assert!(!caret_in_range(caret(3, 0), r));
        // Multi-line range, end inclusive
        let r2 = (caret(1, 5), caret(3, 2));
        assert!(caret_in_range(caret(1, 5), r2));
        assert!(caret_in_range(caret(2, 0), r2));
        assert!(caret_in_range(caret(3, 2), r2));
        assert!(!caret_in_range(caret(3, 3), r2));
        assert!(!caret_in_range(caret(1, 4), r2));
    }

    #[test]
    fn caret_at_range_end_still_edits_existing() {
        // Repro of the second screenshot: comment at L16c0..L16c8, caret at
        // L16c8 (the half-open end position, which is one past the visible
        // highlight). Must edit in place, not duplicate.
        use crate::comments::{Comment, CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        let span = mock_span(160, 168);
        app.comments.push(Comment {
            source: span,
            text: "hello".into(),
            selected_text: None,
        });
        app.comment_projections = vec![ProjectedCommentAnchor {
            source: span,
            rendered: vec![RenderedRange {
                start: Caret { line: 16, col: 0 },
                end: Caret { line: 16, col: 9 },
            }],
        }];
        // Selecting the whole range to edit in place.
        app.caret = Caret { line: 16, col: 8 };
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 16, col: 0 },
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(16, 0, 8, span);
        app.commit_selection_to_editing(&markdown);
        match &app.comment_state {
            CommentState::Editing {
                range,
                target,
                draft,
                ..
            } => {
                assert_eq!(
                    *range,
                    (Caret { line: 16, col: 0 }, Caret { line: 16, col: 9 })
                );
                assert_eq!(draft, "hello");
                assert_eq!(*target, EditTarget::Existing(0));
            }
            other => panic!("expected Editing(Existing), got {other:?}"),
        }
        app.save_draft(&markdown);
        assert_eq!(app.comments.len(), 1, "must not duplicate");
        assert_eq!(app.comments[0].text, "hello");
    }

    #[test]
    fn commit_selection_matching_existing_edits_in_place() {
        use crate::comments::{Comment, CommentModeSource, CommentState, EditTarget};
        let stored_range = (Caret { line: 7, col: 0 }, Caret { line: 7, col: 6 });
        let span = mock_span(700, 705);
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comments.push(Comment {
            source: span,
            text: "existing".into(),
            selected_text: None,
        });
        app.comment_projections = vec![ProjectedCommentAnchor {
            source: span,
            rendered: vec![RenderedRange {
                start: stored_range.0,
                end: stored_range.1,
            }],
        }];
        // Selecting the whole range (L7c0..L7c5) exactly.
        app.caret = Caret { line: 7, col: 5 };
        app.comment_state = CommentState::Selecting {
            anchor: stored_range.0,
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(7, 0, 5, span);
        app.commit_selection_to_editing(&markdown);
        match &app.comment_state {
            CommentState::Editing {
                range: r,
                draft,
                cursor,
                target,
                ..
            } => {
                assert_eq!(*r, (stored_range.0, Caret { line: 7, col: 6 }));
                assert_eq!(draft, "existing");
                assert_eq!(*cursor, "existing".len());
                assert_eq!(*target, EditTarget::Existing(0));
            }
            other => panic!("expected Editing, got {other:?}"),
        }
        assert_eq!(app.active_comment, Some(0));
        // Length unchanged — no duplicate.
        assert_eq!(app.comments.len(), 1);
    }

    #[test]
    fn commit_selection_prefers_exact_range_identity_over_overlap() {
        use crate::comments::{Comment, CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        let span_outer = mock_span(400, 410);
        let span_inner = mock_span(403, 408);
        app.comments.push(Comment {
            source: span_outer,
            text: "outer".into(),
            selected_text: None,
        });
        app.comments.push(Comment {
            source: span_inner,
            text: "inner".into(),
            selected_text: None,
        });
        app.comment_projections = vec![
            ProjectedCommentAnchor {
                source: span_outer,
                rendered: vec![RenderedRange {
                    start: Caret { line: 4, col: 0 },
                    end: Caret { line: 4, col: 11 },
                }],
            },
            ProjectedCommentAnchor {
                source: span_inner,
                rendered: vec![RenderedRange {
                    start: Caret { line: 4, col: 3 },
                    end: Caret { line: 4, col: 9 },
                }],
            },
        ];
        // The caret sits inside both comments, so a point-in-range lookup
        // alone would pick the wrong one. The selected range (L4c3..L4c8)
        // uniquely identifies the inner comment.
        app.caret = Caret { line: 4, col: 8 };
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 4, col: 3 },
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(4, 3, 5, span_inner);
        app.commit_selection_to_editing(&markdown);
        match &app.comment_state {
            CommentState::Editing {
                range,
                draft,
                target,
                ..
            } => {
                assert_eq!(
                    *range,
                    (Caret { line: 4, col: 3 }, Caret { line: 4, col: 9 })
                );
                assert_eq!(draft, "inner");
                assert_eq!(*target, EditTarget::Existing(1));
            }
            other => panic!("expected Editing(Existing inner), got {other:?}"),
        }
    }

    #[test]
    fn save_draft_pushes_comment_and_browses() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        // Range (0, 6) will resolve to (0, 5) source span in save_draft.
        let range = (Caret { line: 0, col: 0 }, Caret { line: 0, col: 6 });
        let span = mock_span(0, 5);
        app.comment_state = CommentState::Editing {
            range,
            draft: "hello".into(),
            cursor: 5,
            target: crate::comments::EditTarget::New,
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(0, 0, 5, span);
        app.save_draft(&markdown);
        assert_eq!(app.comments.len(), 1);
        assert_eq!(app.comments[0].source.start, span.start);
        assert_eq!(app.comments[0].source.end, span.end);
        assert_eq!(app.comments[0].text, "hello");
        // Newly saved comment is NOT auto-focused; it renders with the same
        // style as other saved cards. The active highlight is reserved for
        // explicit n/N cycling.
        assert_eq!(app.active_comment, None);
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn save_draft_snapshots_selected_text_from_raw_source() {
        // The Sidemark dump on exit needs `selected_text` per comment;
        // capturing it at save time keeps the snapshot accurate even if the
        // file is rewritten externally before the user quits.
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        // mock_span builds spans with byte = (line*100 + col); we feed a
        // raw_source whose byte 0..5 is "hello " — so the slice should be
        // exactly "hello".
        app.raw_source = Some("hello world".into());
        let range = (Caret { line: 0, col: 0 }, Caret { line: 0, col: 6 });
        let span = mock_span(0, 5);
        app.comment_state = CommentState::Editing {
            range,
            draft: "comment text".into(),
            cursor: 12,
            target: crate::comments::EditTarget::New,
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(0, 0, 5, span);
        app.save_draft(&markdown);
        assert_eq!(
            app.comments[0].selected_text.as_deref(),
            Some("hello"),
            "save_draft must capture selected_text from raw_source"
        );
    }

    #[test]
    fn save_draft_leaves_selected_text_none_without_raw_source() {
        // If raw_source is missing (e.g. comments came from somewhere other
        // than the open-file path), selected_text stays None — the spec
        // allows omitting it, which is better than emitting bogus bytes.
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.raw_source = None;
        let range = (Caret { line: 0, col: 0 }, Caret { line: 0, col: 6 });
        let span = mock_span(0, 5);
        app.comment_state = CommentState::Editing {
            range,
            draft: "x".into(),
            cursor: 1,
            target: crate::comments::EditTarget::New,
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(0, 0, 5, span);
        app.save_draft(&markdown);
        assert!(app.comments[0].selected_text.is_none());
    }

    #[test]
    fn save_draft_from_caret_mode_restores_off() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        let range = (Caret { line: 1, col: 1 }, Caret { line: 1, col: 3 });
        let span = mock_span(11, 13);
        app.comment_state = CommentState::Editing {
            range,
            draft: "x".into(),
            cursor: 1,
            target: crate::comments::EditTarget::New,
            source: CommentModeSource::Caret,
        };
        let markdown = mock_markdown_with_span(1, 1, 2, span);
        app.save_draft(&markdown);
        assert_eq!(app.comment_state, CommentState::Off);
        assert_eq!(app.comments.len(), 1);
    }

    #[test]
    fn cycle_comment_next_wraps_and_moves_caret_to_start() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        push_comment(&mut app, 5, 0, 3, "a");
        push_comment(&mut app, 20, 2, 3, "b");
        // First call from None goes to index 0
        app.cycle_comment(true, 20);
        assert_eq!(app.active_comment, Some(0));
        assert_eq!(app.caret, Caret { line: 5, col: 0 });
        // Next goes to 1
        app.cycle_comment(true, 20);
        assert_eq!(app.active_comment, Some(1));
        assert_eq!(app.caret, Caret { line: 20, col: 2 });
        // Wraps back to 0
        app.cycle_comment(true, 20);
        assert_eq!(app.active_comment, Some(0));
    }

    #[test]
    fn cycle_comment_no_op_when_empty() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        app.cycle_comment(true, 20);
        assert_eq!(app.active_comment, None);
    }

    #[test]
    fn cycle_comment_centers_main_pane() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        push_comment(&mut app, 50, 0, 1, "x");
        app.cycle_comment(true, 20);
        assert_eq!(app.vertical_scroll, 50u16.saturating_sub(10));
    }

    #[test]
    fn toggle_caret_off_keeps_browsing() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        app.active_comment = Some(0);
        app.toggle_caret_mode(20);
        assert!(!app.caret_mode);
        // Browsing is mode-independent — it must persist across a caret toggle.
        assert_eq!(app.comment_state, CommentState::Browsing);
        assert_eq!(app.active_comment, Some(0));
    }

    #[test]
    fn toggle_caret_off_exits_caret_dependent_states() {
        use crate::comments::{CommentModeSource, CommentState};
        // Selecting and Editing rely on the caret cursor, so they must
        // bail out when caret mode is turned off.
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 0, col: 0 },
            source: CommentModeSource::Caret,
        };
        app.toggle_caret_mode(20);
        assert!(!app.caret_mode);
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn enter_comment_mode_no_op_when_already_on() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        assert!(!app.enter_comment_mode());
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn cycle_comment_from_off_selects_anchor_without_entering_comment_mode() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Off;
        push_comment(&mut app, 5, 0, 1, "x");
        app.cycle_comment(true, 20);
        assert_eq!(app.active_comment, Some(0));
        assert_eq!(app.caret, Caret { line: 5, col: 0 });
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn cycle_comment_no_op_when_selecting() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 0, col: 0 },
            source: CommentModeSource::Comments,
        };
        push_comment(&mut app, 5, 0, 1, "x");
        app.cycle_comment(true, 20);
        assert_eq!(app.active_comment, None);
        assert_eq!(app.caret, Caret { line: 0, col: 0 });
    }

    #[test]
    fn cancel_editing_from_editing_returns_to_browsing() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.comment_state = CommentState::Editing {
            range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 5 }),
            draft: "in progress".into(),
            cursor: 11,
            target: crate::comments::EditTarget::New,
            source: CommentModeSource::Comments,
        };
        app.cancel_editing();
        assert_eq!(app.comment_state, CommentState::Browsing);
        assert!(app.comments.is_empty());
    }

    #[test]
    fn cancel_editing_from_selecting_returns_to_browsing() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 1, col: 0 },
            source: CommentModeSource::Comments,
        };
        app.cancel_editing();
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn cancel_editing_from_caret_selection_returns_to_off() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 1, col: 0 },
            source: CommentModeSource::Caret,
        };
        app.cancel_editing();
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn start_editing_active_or_caret_loads_existing_text_at_end() {
        use crate::comments::{CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        push_comment(&mut app, 2, 4, 5, "hello");
        app.active_comment = Some(0);
        assert!(app.start_editing_active_or_caret());
        match &app.comment_state {
            CommentState::Editing {
                range,
                draft,
                cursor,
                target,
                source,
            } => {
                assert_eq!(
                    *range,
                    (Caret { line: 2, col: 4 }, Caret { line: 2, col: 10 })
                );
                assert_eq!(draft, "hello");
                assert_eq!(*cursor, 5);
                assert_eq!(*target, EditTarget::Existing(0));
                assert_eq!(*source, CommentModeSource::Comments);
            }
            other => panic!("expected Editing, got {other:?}"),
        }
    }

    #[test]
    fn start_editing_active_or_caret_no_op_without_active_or_caret_match() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        app.active_comment = None;
        assert!(!app.start_editing_active_or_caret());
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn start_editing_active_or_caret_reopens_comment_under_caret() {
        use crate::comments::{CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Off;
        push_comment(&mut app, 9, 2, 4, "note");
        app.caret = Caret { line: 9, col: 4 };
        assert!(app.start_editing_active_or_caret());
        match &app.comment_state {
            CommentState::Editing {
                range,
                draft,
                cursor,
                target,
                source,
            } => {
                assert_eq!(
                    *range,
                    (Caret { line: 9, col: 2 }, Caret { line: 9, col: 7 })
                );
                assert_eq!(draft, "note");
                assert_eq!(*cursor, 4);
                assert_eq!(*target, EditTarget::Existing(0));
                assert_eq!(*source, CommentModeSource::Caret);
            }
            other => panic!("expected Editing, got {other:?}"),
        }
        assert_eq!(app.active_comment, Some(0));
    }

    #[test]
    fn start_editing_active_or_caret_no_op_without_match() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Off;
        app.caret = Caret { line: 9, col: 4 };
        assert!(!app.start_editing_active_or_caret());
        assert_eq!(app.comment_state, CommentState::Off);
        assert_eq!(app.active_comment, None);
    }

    #[test]
    fn start_editing_active_or_caret_ignores_stale_active_index() {
        use crate::comments::{CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Off;
        app.active_comment = Some(99);
        push_comment(&mut app, 12, 1, 3, "ok");
        app.caret = Caret { line: 12, col: 2 };
        assert!(app.start_editing_active_or_caret());
        match &app.comment_state {
            CommentState::Editing { target, source, .. } => {
                assert_eq!(*target, EditTarget::Existing(0));
                assert_eq!(*source, CommentModeSource::Caret);
            }
            other => panic!("expected Editing, got {other:?}"),
        }
        assert_eq!(app.active_comment, Some(0));
    }

    #[test]
    fn start_editing_active_or_caret_from_caret_mode_requires_caret_on_anchor() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Off;
        push_comment(&mut app, 12, 1, 3, "ok");
        app.active_comment = Some(0);
        app.caret = Caret { line: 0, col: 0 };
        assert!(!app.start_editing_active_or_caret());
        assert_eq!(app.comment_state, CommentState::Off);
        assert_eq!(app.active_comment, Some(0));
    }

    #[test]
    fn save_draft_updates_existing_in_place() {
        use crate::comments::{CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        push_comment(&mut app, 0, 0, 5, "old");
        app.active_comment = Some(0);
        app.comment_state = CommentState::Editing {
            range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 6 }),
            draft: "new text".into(),
            cursor: 8,
            target: EditTarget::Existing(0),
            source: CommentModeSource::Comments,
        };
        app.save_draft(&mock_markdown());
        // Length unchanged — existing comment was updated in place.
        assert_eq!(app.comments.len(), 1);
        assert_eq!(app.comments[0].text, "new text");
        // Active focus retained.
        assert_eq!(app.active_comment, Some(0));
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn resync_after_reparse_cancels_edit_and_reprojects() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        // An in-flight draft anchored to the old layout.
        app.comment_state = CommentState::Editing {
            range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 6 }),
            draft: "wip".into(),
            cursor: 3,
            target: crate::comments::EditTarget::New,
            source: CommentModeSource::Comments,
        };
        // A stale projection that should be replaced by the reprojection.
        app.comment_projections = vec![ProjectedCommentAnchor {
            source: mock_span(0, 5),
            rendered: vec![RenderedRange {
                start: Caret { line: 99, col: 0 },
                end: Caret { line: 99, col: 9 },
            }],
        }];
        // The freshly parsed document has no comments, so projections clear.
        app.resync_comments_after_reparse(&mock_markdown());
        assert_eq!(app.comment_state, CommentState::Browsing);
        assert!(app.comment_projections.is_empty());
    }

    #[test]
    fn save_draft_discards_empty_new_comment() {
        use crate::comments::{CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        let range = (Caret { line: 0, col: 0 }, Caret { line: 0, col: 6 });
        let span = mock_span(0, 5);
        app.comment_state = CommentState::Editing {
            range,
            draft: "   ".into(),
            cursor: 3,
            target: EditTarget::New,
            source: CommentModeSource::Comments,
        };
        let markdown = mock_markdown_with_span(0, 0, 5, span);
        app.save_draft(&markdown);
        // A whitespace-only draft must not persist a blank, unremovable comment.
        assert!(app.comments.is_empty());
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn save_draft_empty_existing_deletes_comment() {
        use crate::comments::{CommentModeSource, CommentState, EditTarget};
        let mut app = app_with_width(40);
        push_comment(&mut app, 0, 0, 5, "to delete");
        app.active_comment = Some(0);
        app.comment_state = CommentState::Editing {
            range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 6 }),
            draft: "".into(),
            cursor: 0,
            target: EditTarget::Existing(0),
            source: CommentModeSource::Comments,
        };
        app.save_draft(&mock_markdown());
        // Clearing an existing comment's text deletes it (the delete affordance).
        assert!(app.comments.is_empty());
        assert_eq!(app.active_comment, None);
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn entering_comment_mode_clears_pending_input() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.pending_input = Some(PendingInput::BookmarkSet);
        assert!(app.enter_comment_mode());
        assert!(app.pending_input.is_none());
        assert_eq!(app.comment_state, CommentState::Browsing);
    }

    #[test]
    fn start_selecting_clears_pending_input() {
        use crate::comments::CommentState;
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.comment_state = CommentState::Browsing;
        app.pending_input = Some(PendingInput::BookmarkSet);
        assert!(app.start_selecting());
        assert!(app.pending_input.is_none());
    }

    #[test]
    fn commit_to_editing_clears_pending_input() {
        use crate::comments::{CommentModeSource, CommentState};
        let mut app = app_with_width(40);
        app.caret_mode = true;
        app.caret = Caret { line: 0, col: 0 };
        app.comment_state = CommentState::Selecting {
            anchor: Caret { line: 0, col: 0 },
            source: CommentModeSource::Comments,
        };
        app.pending_input = Some(PendingInput::BookmarkJump);
        app.commit_selection_to_editing(&mock_markdown());
        assert!(app.pending_input.is_none());
    }

    #[test]
    fn jump_to_outline_mark_no_op_without_mark() {
        let mut app = App {
            caret: Caret { line: 5, col: 2 },
            vertical_scroll: 5,
            ..App::default()
        };
        app.jump_to_outline_mark(20);
        // No mark set → no state change.
        assert_eq!(app.caret, Caret { line: 5, col: 2 });
        assert_eq!(app.vertical_scroll, 5);
        assert!(app.outline_mark.is_none());
    }

    #[test]
    fn jump_to_outline_mark_swaps_caret_and_centers_when_off_mark_line() {
        let mut app = App {
            outline_mark: Some(Caret { line: 3, col: 7 }),
            caret: Caret { line: 100, col: 0 },
            vertical_scroll: 90,
            ..App::default()
        };
        app.jump_to_outline_mark(20);
        // Caret moved to mark, viewport centered on caret line, mark stays.
        assert_eq!(app.caret, Caret { line: 3, col: 7 });
        assert_eq!(app.vertical_scroll, 0); // 3 - 10 saturating
        assert_eq!(app.outline_mark, Some(Caret { line: 3, col: 7 }));
    }

    #[test]
    fn jump_to_outline_mark_clears_when_already_on_mark_line() {
        let mut app = App {
            outline_mark: Some(Caret { line: 3, col: 7 }),
            caret: Caret { line: 3, col: 0 }, // same line, different col
            vertical_scroll: 5,
            ..App::default()
        };
        app.jump_to_outline_mark(20);
        // Caret already on mark.line → mark cleared, no caret/scroll change.
        assert!(app.outline_mark.is_none());
        assert_eq!(app.caret, Caret { line: 3, col: 0 });
        assert_eq!(app.vertical_scroll, 5);
    }

    #[test]
    fn jump_to_outline_mark_double_press_swap_then_clear() {
        let mut app = App {
            outline_mark: Some(Caret { line: 2, col: 0 }),
            caret: Caret { line: 50, col: 0 },
            vertical_scroll: 40,
            ..App::default()
        };
        app.jump_to_outline_mark(20);
        assert_eq!(app.caret.line, 2);
        assert!(app.outline_mark.is_some());
        // Second press: caret already at mark line → clear.
        app.jump_to_outline_mark(20);
        assert!(app.outline_mark.is_none());
    }

    #[test]
    fn save_draft_restores_scroll_when_auto_entered() {
        let mut app = App {
            caret_mode: true,
            auto_caret_for_comment_edit: true,
            comment_state: CommentState::Editing {
                range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 1 }),
                draft: String::new(),
                cursor: 0,
                target: EditTarget::New,
                source: CommentModeSource::Caret,
            },
            ..App::default()
        };
        app.save_draft(&mock_markdown());
        assert!(!app.caret_mode, "auto-entered caret mode should flip back");
        assert!(!app.auto_caret_for_comment_edit, "flag should be cleared");
        assert_eq!(app.comment_state, CommentState::Off);
    }

    #[test]
    fn save_draft_keeps_caret_mode_when_not_auto_entered() {
        let mut app = App {
            caret_mode: true,
            auto_caret_for_comment_edit: false,
            comment_state: CommentState::Editing {
                range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 1 }),
                draft: String::new(),
                cursor: 0,
                target: EditTarget::New,
                source: CommentModeSource::Caret,
            },
            ..App::default()
        };
        app.save_draft(&mock_markdown());
        assert!(
            app.caret_mode,
            "user manually entered caret mode — should stay"
        );
    }

    #[test]
    fn cancel_editing_restores_scroll_when_auto_entered() {
        let mut app = App {
            caret_mode: true,
            auto_caret_for_comment_edit: true,
            comment_state: CommentState::Editing {
                range: (Caret { line: 0, col: 0 }, Caret { line: 0, col: 1 }),
                draft: "abc".into(),
                cursor: 3,
                target: EditTarget::New,
                source: CommentModeSource::Caret,
            },
            ..App::default()
        };
        app.cancel_editing();
        assert!(!app.caret_mode);
        assert!(!app.auto_caret_for_comment_edit);
    }

    #[test]
    fn manual_toggle_caret_mode_clears_auto_flag() {
        let mut app = App {
            caret_mode: true,
            auto_caret_for_comment_edit: true,
            ..App::default()
        };
        app.toggle_caret_mode(20);
        assert!(!app.caret_mode);
        assert!(
            !app.auto_caret_for_comment_edit,
            "explicit user toggle should override the auto-restore intent"
        );
    }
}
