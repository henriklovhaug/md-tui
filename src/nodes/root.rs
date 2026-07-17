use std::collections::HashSet;

use crate::comments::{Comment, ProjectedCommentAnchor, RenderedRange};
use crate::parser::SourceSpan;
use crate::search::{compare_heading, find_and_mark};

use super::{
    image::ImageComponent,
    textcomponent::{TextComponent, TextNode, display_width},
    word::{Word, WordType},
};

pub struct ComponentRoot {
    file_name: Option<String>,
    components: Vec<Component>,
    is_focused: bool,
}

impl ComponentRoot {
    #[must_use]
    pub fn new(file_name: Option<String>, components: Vec<Component>) -> Self {
        Self {
            file_name,
            components,
            is_focused: false,
        }
    }

    #[must_use]
    pub fn children(&self) -> Vec<&Component> {
        self.components.iter().collect()
    }

    pub fn children_mut(&mut self) -> Vec<&mut Component> {
        self.components.iter_mut().collect()
    }

    #[must_use]
    pub fn components(&self) -> Vec<&TextComponent> {
        self.components
            .iter()
            .filter_map(|c| match c {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .collect()
    }

    pub fn components_mut(&mut self) -> Vec<&mut TextComponent> {
        self.components
            .iter_mut()
            .filter_map(|c| match c {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .collect()
    }

    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    #[must_use]
    pub fn words(&self) -> Vec<&Word> {
        self.components
            .iter()
            .filter_map(|c| match c {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .flat_map(|c| c.content().iter().flatten())
            .collect()
    }

    pub fn find_and_mark(&mut self, search: &str) {
        let mut words = self
            .components
            .iter_mut()
            .filter_map(|c| match c {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .flat_map(|c| c.words_mut())
            .collect::<Vec<_>>();
        find_and_mark(search, &mut words);
    }

    #[must_use]
    pub fn search_results_heights(&self) -> Vec<usize> {
        self.components
            .iter()
            .filter_map(|c| match c {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .flat_map(|c| {
                let mut heights = c.selected_heights();
                heights.iter_mut().for_each(|h| *h += c.y_offset() as usize);
                heights
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.file_name = None;
        self.components.clear();
    }

    pub fn select(&mut self, index: usize) -> Result<u16, String> {
        self.deselect();
        self.is_focused = true;
        let mut count = 0;
        for comp in self.components.iter_mut().filter_map(|f| match f {
            Component::TextComponent(comp) => Some(comp),
            Component::Image(_) => None,
        }) {
            let link_inside_comp = index - count < comp.num_links();
            if link_inside_comp {
                comp.visually_select(index - count)?;
                return Ok(comp.y_offset());
            }
            count += comp.num_links();
        }
        Err(format!("Index out of bounds: {index} >= {count}"))
    }

    pub fn deselect(&mut self) {
        self.is_focused = false;
        for comp in self.components.iter_mut().filter_map(|f| match f {
            Component::TextComponent(comp) => Some(comp),
            Component::Image(_) => None,
        }) {
            comp.deselect();
        }
    }

    /// Non-mutating equivalent of `select` + `selected` + `deselect`: returns
    /// the link anchor (URL or `#heading`) at the document-wide ordinal
    /// `index`, without changing focus or visual highlighting. The ordinal
    /// matches what `select` expects.
    #[must_use]
    pub fn link_anchor_at(&self, index: usize) -> Option<&str> {
        let mut count = 0;
        for comp in self.components.iter().filter_map(|f| match f {
            Component::TextComponent(comp) => Some(comp),
            Component::Image(_) => None,
        }) {
            let n = comp.num_links();
            if index < count + n {
                return comp.link_anchor_at(index - count);
            }
            count += n;
        }
        None
    }

    #[must_use]
    pub fn find_footnote(&self, search: &str) -> String {
        let footnote = self
            .components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(text_component) => {
                    if text_component.kind() == TextNode::Footnote {
                        Some(text_component)
                    } else {
                        None
                    }
                }
                Component::Image(_) => None,
            })
            .filter(|f| {
                if let Some(foot_ref) = f.meta_info().iter().next() {
                    foot_ref.content() == search
                } else {
                    false
                }
            })
            .flat_map(|f| f.content().iter().flatten())
            .filter(|f| f.kind() == WordType::Footnote)
            .map(Word::content)
            .collect::<String>();

        if footnote.is_empty() {
            String::from("Footnote not found")
        } else {
            footnote
        }
    }

    /// The visible (non-hidden) text components in document order.
    fn visible_text_components(&self) -> impl Iterator<Item = &TextComponent> {
        self.components
            .iter()
            .filter_map(|c| match c {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .filter(|comp| !comp.is_hidden())
    }

    #[must_use]
    pub fn link_index_and_height(&self) -> Vec<(usize, u16)> {
        let mut indexes = Vec::new();
        let mut base = 0;
        // One ordinal per link *group* (a link can render as several
        // consecutive words), numbered continuously across components so the
        // index matches `select` / `num_links`.
        for comp in self.visible_text_components() {
            let groups = comp_link_groups(comp, comp.y_offset());
            for &(ord, line) in &groups {
                indexes.push((base + ord, line));
            }
            base += groups.len();
        }
        indexes
    }

    /// Resolve the link ordinal under a caret position, column-aware.
    ///
    /// When a line carries more than one link, this returns the one whose
    /// rendered cells contain `caret.col`; if the caret isn't on any link, it
    /// falls back to the first link on the line (so caret-at-column-0 and
    /// outline jumps still behave as before). Ordinals match
    /// `link_index_and_height` / `select` / `link_anchor_at`.
    pub fn link_index_at_caret(&self, caret: crate::util::Caret) -> Option<usize> {
        let mut base = 0;
        let mut first_on_line = None;
        for comp in self.visible_text_components() {
            let hit = comp_link_at_caret(comp, comp.y_offset(), caret);
            if let Some(ord) = hit.hit {
                return Some(base + ord);
            }
            if first_on_line.is_none() {
                first_on_line = hit.first_on_line.map(|ord| base + ord);
            }
            base += hit.group_count;
        }
        first_on_line
    }

    /// Sets the y offset of the components
    pub fn set_scroll(&mut self, scroll: u16) {
        let mut y_offset = 0;
        for component in &mut self.components {
            component.set_y_offset(y_offset);
            component.set_scroll_offset(scroll);
            y_offset += component.height();
        }
    }

    pub fn heading_offset(&self, heading: &str) -> Result<u16, String> {
        let mut y_offset = 0;
        for component in &self.components {
            match component {
                Component::TextComponent(comp) => {
                    if comp.kind() == TextNode::Heading
                        && compare_heading(&heading[1..], comp.content())
                    {
                        return Ok(y_offset);
                    }
                    y_offset += comp.height();
                }
                Component::Image(e) => y_offset += e.height(),
            }
        }
        Err(format!("Heading not found: {heading}"))
    }

    /// Return the content of the components, where each element a line
    #[must_use]
    pub fn content(&self) -> Vec<String> {
        self.components()
            .iter()
            .flat_map(|c| c.content_as_lines())
            .collect()
    }

    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        let block = self
            .components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .find(|c| c.is_focused())?;
        block.highlight_link().ok()
    }

    #[must_use]
    pub fn selected_underlying_type(&self) -> Option<WordType> {
        let block = self
            .components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .find(|c| c.is_focused())?;
        block
            .content()
            .iter()
            .flatten()
            .find(|c| c.kind() == WordType::Selected)
            .map(Word::previous_type)
    }

    /// Transforms the content of the components to fit the given width
    pub fn transform(&mut self, width: u16) {
        for component in self.components_mut() {
            component.transform(width);
        }
    }

    /// Because of the parsing, every table has a missing newline at the end
    #[must_use]
    pub fn add_missing_components(self) -> Self {
        let mut components = Vec::new();
        let mut iter = self.components.into_iter().peekable();
        while let Some(component) = iter.next() {
            let kind = component.kind();
            let curr_ids: Vec<u32> = match &component {
                Component::TextComponent(tc) => tc.owning_details_ids().to_vec(),
                Component::Image(_) => Vec::new(),
            };
            components.push(component);
            if let Some(next) = iter.peek()
                && kind != TextNode::LineBreak
                && next.kind() != TextNode::LineBreak
            {
                let next_ids: Vec<u32> = match next {
                    Component::TextComponent(tc) => tc.owning_details_ids().to_vec(),
                    Component::Image(_) => Vec::new(),
                };
                // An inserted LineBreak inherits the longest common
                // outermost prefix of its two neighbors' owning-details
                // chains, so it is hidden iff both neighbors are inside
                // the same folded `<details>` body.
                let shared_ids: Vec<u32> = curr_ids
                    .iter()
                    .zip(next_ids.iter())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| *a)
                    .collect();
                let mut lb = TextComponent::new(TextNode::LineBreak, Vec::new());
                lb.set_owning_details_ids(shared_ids);
                components.push(Component::TextComponent(lb));
            }
        }
        Self {
            file_name: self.file_name,
            components,
            is_focused: self.is_focused,
        }
    }

    #[must_use]
    pub fn height(&self) -> u16 {
        self.components.iter().map(ComponentProps::height).sum()
    }

    #[must_use]
    pub fn num_links(&self) -> usize {
        self.components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .map(TextComponent::num_links)
            .sum()
    }

    /// Walk all components and set their `hidden` flag based on whether
    /// any of their `owning_details_ids` references a currently-folded
    /// `<details>` block. Must be called after parse and after every
    /// fold-toggle so that `height()`, `num_links()`, etc. return the
    /// post-fold values used by `set_scroll` and the renderer.
    pub fn recompute_visibility(&mut self) {
        let folded: HashSet<u32> = self
            .components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .filter_map(|tc| match tc.kind() {
                TextNode::DetailsSummary {
                    id, folded: true, ..
                } => Some(id),
                _ => None,
            })
            .collect();

        for c in self.components.iter_mut() {
            if let Component::TextComponent(tc) = c {
                let hidden = tc.owning_details_ids().iter().any(|id| folded.contains(id));
                tc.set_hidden(hidden);
            }
        }
    }

    /// Count of `<details>` summary headers that are currently *visible*
    /// (i.e. not hidden by an outer folded block). Used by the event
    /// handler to bound the cyclable selection index.
    #[must_use]
    pub fn num_details(&self) -> usize {
        self.components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .filter(|comp| {
                !comp.is_hidden() && matches!(comp.kind(), TextNode::DetailsSummary { .. })
            })
            .count()
    }

    /// Returns `(index, y_offset)` for each visible details summary, in
    /// document order. Parallels `link_index_and_height` — callers use it
    /// to pick the summary nearest the current scroll position.
    #[must_use]
    pub fn details_index_and_height(&self) -> Vec<(usize, u16)> {
        let mut out = Vec::new();
        let mut idx = 0usize;
        for c in &self.components {
            if let Component::TextComponent(comp) = c
                && !comp.is_hidden()
                && matches!(comp.kind(), TextNode::DetailsSummary { .. })
            {
                out.push((idx, comp.y_offset()));
                idx += 1;
            }
        }
        out
    }

    /// Visually mark the `index`-th visible details summary as focused,
    /// returning its `y_offset` so the caller can scroll it into view.
    /// Clears any prior details focus first.
    pub fn select_details(&mut self, index: usize) -> Result<u16, String> {
        self.deselect_details();
        let mut count = 0;
        for c in self.components.iter_mut() {
            if let Component::TextComponent(comp) = c
                && !comp.is_hidden()
                && matches!(comp.kind(), TextNode::DetailsSummary { .. })
            {
                if count == index {
                    comp.visually_select_summary();
                    return Ok(comp.y_offset());
                }
                count += 1;
            }
        }
        Err(format!("Details index out of bounds: {index} >= {count}"))
    }

    /// Clear focus from whichever details summary currently has it.
    pub fn deselect_details(&mut self) {
        for c in self.components.iter_mut() {
            if let Component::TextComponent(comp) = c
                && matches!(comp.kind(), TextNode::DetailsSummary { .. })
            {
                comp.deselect_summary();
            }
        }
    }

    /// Flip the `folded` flag on the currently-focused details summary
    /// and recompute visibility. Returns `Err` if no details summary is
    /// focused.
    pub fn toggle_selected_details(&mut self) -> Result<(), String> {
        let mut toggled = false;
        for c in self.components.iter_mut() {
            if let Component::TextComponent(comp) = c
                && comp.is_focused()
                && let TextNode::DetailsSummary { folded, .. } = comp.kind()
            {
                comp.set_details_folded(!folded);
                toggled = true;
                break;
            }
        }
        if !toggled {
            return Err("No details summary is focused".to_string());
        }
        self.recompute_visibility();
        Ok(())
    }

    pub fn project_comments(&self, comments: &[Comment]) -> Vec<ProjectedCommentAnchor> {
        let mut projections = Vec::new();

        for comment in comments {
            let mut rendered_ranges: Vec<RenderedRange> = Vec::new();

            for component in &self.components {
                self.project_comment_on_component(component, comment, &mut rendered_ranges);
            }

            projections.push(ProjectedCommentAnchor {
                source: comment.source,
                rendered: rendered_ranges,
            });
        }

        projections
    }

    fn project_comment_on_component(
        &self,
        component: &Component,
        comment: &Comment,
        rendered_ranges: &mut Vec<RenderedRange>,
    ) {
        if let Component::TextComponent(comp) = component {
            let comp_y = comp.y_offset();
            for (row_idx, row) in comp.content().iter().enumerate() {
                let line_y = comp_y + row_idx as u16;
                self.project_comment_on_row(row, line_y, comment, rendered_ranges);
            }
        }
    }

    fn project_comment_on_row(
        &self,
        row: &[Word],
        line_y: u16,
        comment: &Comment,
        rendered_ranges: &mut Vec<RenderedRange>,
    ) {
        let mut current_x = 0;
        for word in row {
            let word_width = display_width(word.content()) as u16;
            self.project_comment_on_word(
                word,
                line_y,
                current_x,
                word_width,
                comment,
                rendered_ranges,
            );
            current_x += word_width;
        }
    }

    fn project_comment_on_word(
        &self,
        word: &Word,
        line_y: u16,
        start_col: u16,
        word_width: u16,
        comment: &Comment,
        rendered_ranges: &mut Vec<RenderedRange>,
    ) {
        let Some(word_span) = word.source_span() else {
            return;
        };

        // Check for overlap: max(start) < min(end)
        if word_span.start.byte < comment.source.end.byte
            && comment.source.start.byte < word_span.end.byte
        {
            let end_col = start_col + word_width;

            let range = RenderedRange {
                start: crate::util::Caret {
                    line: line_y,
                    col: start_col,
                },
                end: crate::util::Caret {
                    line: line_y,
                    col: end_col,
                },
            };

            // Coalesce if adjacent on same line
            if let Some(last) = rendered_ranges.last_mut()
                && last.end.line == range.start.line
                && last.end.col == range.start.col
            {
                last.end = range.end;
                return;
            }
            rendered_ranges.push(range);
        }
    }

    fn resolve_range_pos(
        &self,
        start: crate::util::Caret,
        end: crate::util::Caret,
    ) -> (
        Option<crate::parser::SourcePos>,
        Option<crate::parser::SourcePos>,
    ) {
        let (start, end) = crate::comments::normalize_range(start, end);
        let mut first_pos: Option<crate::parser::SourcePos> = None;
        let mut last_pos: Option<crate::parser::SourcePos> = None;

        for component in &self.components {
            self.resolve_selection_on_component(
                component,
                start,
                end,
                &mut first_pos,
                &mut last_pos,
            );
        }
        (first_pos, last_pos)
    }

    pub fn resolve_selection_to_source(
        &self,
        start: crate::util::Caret,
        end: crate::util::Caret,
    ) -> Option<SourceSpan> {
        match self.resolve_range_pos(start, end) {
            (Some(s), Some(e)) => Some(SourceSpan::new(s, e)),
            _ => None,
        }
    }

    fn resolve_selection_on_component(
        &self,
        component: &Component,
        start: crate::util::Caret,
        end: crate::util::Caret,
        first_pos: &mut Option<crate::parser::SourcePos>,
        last_pos: &mut Option<crate::parser::SourcePos>,
    ) {
        if let Component::TextComponent(comp) = component {
            let comp_y = comp.y_offset();
            for (row_idx, row) in comp.content().iter().enumerate() {
                let line_y = comp_y + row_idx as u16;
                if line_y < start.line || line_y > end.line {
                    continue;
                }

                let mut context = SelectionContext {
                    line_y,
                    start,
                    end,
                    first_pos,
                    last_pos,
                };
                self.resolve_selection_on_row(row, &mut context);
            }
        }
    }

    fn resolve_selection_on_row(&self, row: &[Word], context: &mut SelectionContext) {
        let mut current_x = 0;
        for word in row {
            let word_width = display_width(word.content()) as u16;
            let word_start_x = current_x;
            let word_end_x = current_x + word_width;

            if self.word_overlaps_selection(word_start_x, word_end_x, context) {
                self.resolve_selection_on_word(word, word_start_x, context);
            }
            current_x += word_width;
        }
    }

    fn word_overlaps_selection(
        &self,
        word_start_x: u16,
        word_end_x: u16,
        context: &SelectionContext,
    ) -> bool {
        if context.line_y == context.start.line && context.line_y == context.end.line {
            if context.start.col == context.end.col {
                word_start_x <= context.start.col && word_end_x >= context.start.col
            } else {
                word_start_x < context.end.col && word_end_x > context.start.col
            }
        } else if context.line_y == context.start.line {
            word_end_x > context.start.col
        } else if context.line_y == context.end.line {
            word_start_x < context.end.col
        } else {
            true
        }
    }

    fn resolve_selection_on_word(
        &self,
        word: &Word,
        word_start_x: u16,
        context: &mut SelectionContext,
    ) {
        let Some(word_span) = word.source_span() else {
            return;
        };

        let word_content = word.content();
        let start_offset = if context.line_y == context.start.line {
            super::textcomponent::byte_offset_at_width(
                word_content,
                context.start.col.saturating_sub(word_start_x) as usize,
            )
        } else {
            0
        };

        let end_offset = if context.line_y == context.end.line {
            super::textcomponent::byte_offset_at_width(
                word_content,
                context.end.col.saturating_sub(word_start_x) as usize,
            )
        } else {
            word_content.len()
        };

        if let Some(precise_span) = word_span.subspan(word_content, start_offset, end_offset) {
            if context
                .first_pos
                .is_none_or(|pos| precise_span.start.byte < pos.byte)
            {
                *context.first_pos = Some(precise_span.start);
            }
            if context
                .last_pos
                .is_none_or(|pos| precise_span.end.byte > pos.byte)
            {
                *context.last_pos = Some(precise_span.end);
            }
        }
    }
}

/// A word that participates in a link (its display word or a footnote ref).
fn is_link_word(kind: WordType) -> bool {
    matches!(
        kind,
        WordType::Link | WordType::Selected | WordType::FootnoteInline
    )
}

/// One rendered link word: its group `ord` (local to the component, shared by
/// every word of the same link), the row it's on, and its `[x_start, x_end)`
/// column span.
struct LinkCell {
    ord: usize,
    line: u16,
    x_start: u16,
    x_end: u16,
}

/// All link-word cells in `comp` plus the total link-group count. A link can
/// render as several consecutive words (display text split on spaces or wrapped
/// across rows); each run of consecutive link words shares one `ord`.
fn comp_link_cells(comp: &TextComponent, base_y: u16) -> (Vec<LinkCell>, usize) {
    let mut cells = Vec::new();
    let mut ord = 0;
    let mut in_link = false;
    for (i, row) in comp.content().iter().enumerate() {
        let line_y = base_y + i as u16;
        let mut x = 0u16;
        for word in row {
            let width = display_width(word.content()) as u16;
            if is_link_word(word.kind()) {
                cells.push(LinkCell {
                    ord,
                    line: line_y,
                    x_start: x,
                    x_end: x + width,
                });
                in_link = true;
            } else if in_link {
                ord += 1;
                in_link = false;
            }
            x += width;
        }
    }
    let count = if in_link { ord + 1 } else { ord };
    (cells, count)
}

/// `(local ordinal, start row)` of each link group — the first cell of each
/// distinct `ord`.
fn comp_link_groups(comp: &TextComponent, base_y: u16) -> Vec<(usize, u16)> {
    let (cells, _) = comp_link_cells(comp, base_y);
    let mut out = Vec::new();
    let mut last = None;
    for c in &cells {
        if last != Some(c.ord) {
            out.push((c.ord, c.line));
            last = Some(c.ord);
        }
    }
    out
}

/// Outcome of resolving a caret within one component's link groups. Ordinals
/// are local to the component; the caller offsets them by a running base.
struct CaretLink {
    /// Group whose rendered cells contain the caret column, if any.
    hit: Option<usize>,
    /// First link group on the caret's line (column-independent fallback).
    first_on_line: Option<usize>,
    /// Total number of link groups in the component (for ordinal accounting).
    group_count: usize,
}

fn comp_link_at_caret(comp: &TextComponent, base_y: u16, caret: crate::util::Caret) -> CaretLink {
    let (cells, count) = comp_link_cells(comp, base_y);
    let mut first_on_line = None;
    for c in &cells {
        if c.line == caret.line {
            first_on_line.get_or_insert(c.ord);
            if (c.x_start..c.x_end).contains(&caret.col) {
                return CaretLink {
                    hit: Some(c.ord),
                    first_on_line,
                    group_count: count,
                };
            }
        }
    }
    CaretLink {
        hit: None,
        first_on_line,
        group_count: count,
    }
}

struct SelectionContext<'a> {
    line_y: u16,
    start: crate::util::Caret,
    end: crate::util::Caret,
    first_pos: &'a mut Option<crate::parser::SourcePos>,
    last_pos: &'a mut Option<crate::parser::SourcePos>,
}

pub trait ComponentProps {
    fn height(&self) -> u16;
    fn set_y_offset(&mut self, y_offset: u16);
    fn set_scroll_offset(&mut self, scroll: u16);
    fn kind(&self) -> TextNode;
}

pub enum Component {
    TextComponent(TextComponent),
    Image(ImageComponent),
}

impl From<TextComponent> for Component {
    fn from(comp: TextComponent) -> Self {
        Component::TextComponent(comp)
    }
}

impl ComponentProps for Component {
    fn height(&self) -> u16 {
        match self {
            Component::TextComponent(comp) => comp.height(),
            Component::Image(comp) => comp.height(),
        }
    }

    fn set_y_offset(&mut self, y_offset: u16) {
        match self {
            Component::TextComponent(comp) => comp.set_y_offset(y_offset),
            Component::Image(comp) => comp.set_y_offset(y_offset),
        }
    }

    fn set_scroll_offset(&mut self, scroll: u16) {
        match self {
            Component::TextComponent(comp) => comp.set_scroll_offset(scroll),
            Component::Image(comp) => comp.set_scroll_offset(scroll),
        }
    }

    fn kind(&self) -> TextNode {
        match self {
            Component::TextComponent(comp) => comp.kind(),
            Component::Image(comp) => comp.kind(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Caret;

    fn root_with_two_links_on_one_line() -> ComponentRoot {
        // Layout: "[a]" at cols 0..3, a space at col 3, "[b]" at cols 4..7.
        let words = vec![
            Word::new("[a]".to_owned(), WordType::Link),
            Word::new(" ".to_owned(), WordType::Normal),
            Word::new("[b]".to_owned(), WordType::Link),
        ];
        let mut comp = TextComponent::new(TextNode::Paragraph, words);
        comp.set_y_offset(0);
        ComponentRoot::new(None, vec![Component::TextComponent(comp)])
    }

    #[test]
    fn link_index_at_caret_disambiguates_by_column() {
        let root = root_with_two_links_on_one_line();
        // Caret inside the first link.
        assert_eq!(root.link_index_at_caret(Caret { line: 0, col: 1 }), Some(0));
        // Caret inside the second link.
        assert_eq!(root.link_index_at_caret(Caret { line: 0, col: 5 }), Some(1));
    }

    #[test]
    fn link_index_at_caret_falls_back_to_first_link_on_line() {
        let root = root_with_two_links_on_one_line();
        // Caret on the gap between links resolves to the first link on the line.
        assert_eq!(root.link_index_at_caret(Caret { line: 0, col: 3 }), Some(0));
    }

    #[test]
    fn link_index_at_caret_none_when_line_has_no_link() {
        let root = root_with_two_links_on_one_line();
        assert_eq!(root.link_index_at_caret(Caret { line: 9, col: 0 }), None);
    }

    #[test]
    fn link_index_at_caret_counts_a_multiword_link_as_one_ordinal() {
        // A single link whose display text was split into two consecutive Link
        // words (e.g. wrapped across a row) must advance the ordinal once, so a
        // following link is index 1 — matching `num_links` / `link_anchor_at`.
        // Cols: "click"=0..5, "here"=5..9, space=9..10, "[b]"=10..13.
        let words = vec![
            Word::new("click".to_owned(), WordType::Link),
            Word::new("here".to_owned(), WordType::Link),
            Word::new(" ".to_owned(), WordType::Normal),
            Word::new("[b]".to_owned(), WordType::Link),
        ];
        let mut comp = TextComponent::new(TextNode::Paragraph, words);
        comp.set_y_offset(0);
        let root = ComponentRoot::new(None, vec![Component::TextComponent(comp)]);

        // Both words of the first link resolve to ordinal 0.
        assert_eq!(root.link_index_at_caret(Caret { line: 0, col: 2 }), Some(0));
        assert_eq!(root.link_index_at_caret(Caret { line: 0, col: 6 }), Some(0));
        // The second link is ordinal 1, not 2.
        assert_eq!(
            root.link_index_at_caret(Caret { line: 0, col: 11 }),
            Some(1)
        );
    }

    // --- Comment anchoring: caret <-> source-span round trips -------------
    //
    // These build components out of words carrying *real* source spans (byte
    // offsets into a notional raw document) so the precise byte math in
    // `subspan` / `byte_offset_at_width` is exercised, not just the
    // single-word ASCII fixtures used by the `util` state-machine tests.

    use crate::comments::Comment;
    use crate::parser::{SourcePos, SourceSpan};

    /// A word whose source span starts at `start_byte` and runs for
    /// `content.len()` bytes — mirroring how the parser anchors a word.
    fn word(start_byte: usize, content: &str) -> Word {
        let span = SourceSpan {
            start: SourcePos {
                byte: start_byte,
                line: 1,
                column: 1,
            },
            end: SourcePos {
                byte: start_byte + content.len(),
                line: 1,
                column: 1,
            },
        };
        Word::new_with_source_span(content.to_owned(), WordType::Normal, Some(span))
    }

    fn paragraph(y: u16, words: Vec<Word>) -> ComponentRoot {
        let mut comp = TextComponent::new(TextNode::Paragraph, words);
        comp.set_y_offset(y);
        ComponentRoot::new(None, vec![Component::TextComponent(comp)])
    }

    fn multiline(y: u16, rows: Vec<Vec<Word>>) -> ComponentRoot {
        let mut comp = TextComponent::new_formatted(TextNode::Paragraph, rows);
        comp.set_y_offset(y);
        ComponentRoot::new(None, vec![Component::TextComponent(comp)])
    }

    fn comment(span: SourceSpan) -> Comment {
        Comment {
            source: span,
            text: String::new(),
            selected_text: None,
        }
    }

    #[test]
    fn resolve_selection_maps_columns_to_byte_offsets() {
        // "Hello" at bytes 0..5, cols 0..5. Select cols 1..4 -> bytes 1..4.
        let root = paragraph(0, vec![word(0, "Hello")]);
        let span = root
            .resolve_selection_to_source(Caret { line: 0, col: 1 }, Caret { line: 0, col: 4 })
            .expect("selection overlaps a source-backed word");
        assert_eq!(span.start.byte, 1);
        assert_eq!(span.end.byte, 4);
    }

    #[test]
    fn resolve_selection_handles_multibyte_chars() {
        // "café": c(1) a(1) f(1) é(2 bytes, width 1). Content is 5 bytes wide 4.
        // Selecting the whole word by *column* must yield a 5-byte span, proving
        // width->byte conversion runs (a naive col==byte would give 4).
        let root = paragraph(0, vec![word(0, "café")]);
        let span = root
            .resolve_selection_to_source(Caret { line: 0, col: 0 }, Caret { line: 0, col: 4 })
            .expect("selection overlaps the word");
        assert_eq!(span.start.byte, 0);
        assert_eq!(span.end.byte, 5, "é is two bytes wide");
    }

    #[test]
    fn resolve_selection_handles_wide_cjk_chars() {
        // "中文": each char is 3 bytes and 2 columns wide. Selecting cols 0..2
        // covers only "中" -> bytes 0..3.
        let root = paragraph(0, vec![word(0, "中文")]);
        let span = root
            .resolve_selection_to_source(Caret { line: 0, col: 0 }, Caret { line: 0, col: 2 })
            .expect("selection overlaps the word");
        assert_eq!(span.start.byte, 0);
        assert_eq!(span.end.byte, 3);
    }

    #[test]
    fn resolve_selection_spans_multiple_lines() {
        // Raw "foo\nbar": foo=0..3, '\n'=3, bar=4..7.
        // Select from (line0,col0) to (line1,col2) == "foo\nba" -> bytes 0..6.
        let root = multiline(0, vec![vec![word(0, "foo")], vec![word(4, "bar")]]);
        let span = root
            .resolve_selection_to_source(Caret { line: 0, col: 0 }, Caret { line: 1, col: 2 })
            .expect("multi-line selection resolves");
        assert_eq!(span.start.byte, 0);
        assert_eq!(span.end.byte, 6);
    }

    #[test]
    fn resolve_selection_envelope_is_tight_across_words() {
        // Three words on a line; select from inside the first to inside the
        // third. Result must be min(start)..max(end) across touched words.
        // "Hello World !!": Hello=0..5, ' '=5..6, World=6..11.
        let root = paragraph(0, vec![word(0, "Hello"), word(5, " "), word(6, "World")]);
        let span = root
            .resolve_selection_to_source(Caret { line: 0, col: 2 }, Caret { line: 0, col: 9 })
            .expect("selection overlaps multiple words");
        assert_eq!(span.start.byte, 2); // inside "Hello"
        assert_eq!(span.end.byte, 9); // inside "World" (col 9 -> "Wor")
    }

    #[test]
    fn resolve_selection_none_when_no_source_word_touched() {
        // A word with no source span contributes nothing; selecting only it
        // yields None.
        let root = paragraph(0, vec![Word::new("plain".to_owned(), WordType::Normal)]);
        assert!(
            root.resolve_selection_to_source(Caret { line: 0, col: 0 }, Caret { line: 0, col: 3 })
                .is_none()
        );
    }

    #[test]
    fn project_comment_coalesces_adjacent_words_on_one_line() {
        // Comment spanning all three words must collapse to a single rendered
        // range covering cols 0..11, not three separate ranges.
        let root = paragraph(0, vec![word(0, "Hello"), word(5, " "), word(6, "World")]);
        let projections = root.project_comments(&[comment(SourceSpan {
            start: SourcePos {
                byte: 0,
                line: 1,
                column: 1,
            },
            end: SourcePos {
                byte: 11,
                line: 1,
                column: 1,
            },
        })]);
        assert_eq!(projections.len(), 1);
        let ranges = &projections[0].rendered;
        assert_eq!(ranges.len(), 1, "adjacent words must coalesce");
        assert_eq!(ranges[0].start, Caret { line: 0, col: 0 });
        assert_eq!(ranges[0].end, Caret { line: 0, col: 11 });
    }

    #[test]
    fn project_comment_spans_multiple_lines_without_coalescing() {
        let root = multiline(0, vec![vec![word(0, "foo")], vec![word(4, "bar")]]);
        let projections = root.project_comments(&[comment(SourceSpan {
            start: SourcePos {
                byte: 0,
                line: 1,
                column: 1,
            },
            end: SourcePos {
                byte: 7,
                line: 1,
                column: 1,
            },
        })]);
        let ranges = &projections[0].rendered;
        assert_eq!(ranges.len(), 2, "ranges on different lines stay separate");
        assert_eq!(ranges[0].start, Caret { line: 0, col: 0 });
        assert_eq!(ranges[0].end, Caret { line: 0, col: 3 });
        assert_eq!(ranges[1].start, Caret { line: 1, col: 0 });
        assert_eq!(ranges[1].end, Caret { line: 1, col: 3 });
    }

    #[test]
    fn project_comment_orphaned_span_yields_no_ranges() {
        // A comment whose byte range falls outside every word's span produces
        // an empty projection (the orphaned-card / no-highlight path).
        let root = paragraph(0, vec![word(0, "Hello")]);
        let projections = root.project_comments(&[comment(SourceSpan {
            start: SourcePos {
                byte: 100,
                line: 1,
                column: 1,
            },
            end: SourcePos {
                byte: 110,
                line: 1,
                column: 1,
            },
        })]);
        assert_eq!(projections.len(), 1);
        assert!(projections[0].rendered.is_empty());
    }

    #[test]
    fn resolve_then_project_round_trips() {
        // Resolve a selection to a span, then project that span back: the
        // rendered range should cover the originally selected columns.
        let root = paragraph(0, vec![word(0, "Hello"), word(5, " "), word(6, "World")]);
        let span = root
            .resolve_selection_to_source(Caret { line: 0, col: 0 }, Caret { line: 0, col: 11 })
            .expect("resolves");
        let projections = root.project_comments(&[comment(span)]);
        let ranges = &projections[0].rendered;
        assert_eq!(ranges.first().unwrap().start, Caret { line: 0, col: 0 });
        assert_eq!(ranges.last().unwrap().end, Caret { line: 0, col: 11 });
    }

    // --- link_index_and_height / link_anchor_at / select ------------------

    /// A single-row paragraph `Component` at `y` built from `words`.
    fn para_comp(y: u16, words: Vec<Word>) -> Component {
        let mut comp = TextComponent::new(TextNode::Paragraph, words);
        comp.set_y_offset(y);
        Component::TextComponent(comp)
    }

    /// One Link word plus its `LinkData` (URL) word, as the parser emits them.
    fn link_words(text: &str, url: &str) -> Vec<Word> {
        vec![
            Word::new(text.to_owned(), WordType::Link),
            Word::new(url.to_owned(), WordType::LinkData),
        ]
    }

    #[test]
    fn link_index_and_height_groups_consecutive_link_words() {
        // "click here" (two Link words) is one link; "[b]" is a second.
        let words = vec![
            Word::new("click".to_owned(), WordType::Link),
            Word::new("here".to_owned(), WordType::Link),
            Word::new(" ".to_owned(), WordType::Normal),
            Word::new("[b]".to_owned(), WordType::Link),
        ];
        let root = paragraph(3, words);
        // One ordinal per link, both on the component's row (y_offset 3).
        assert_eq!(root.link_index_and_height(), vec![(0, 3), (1, 3)]);
    }

    #[test]
    fn link_index_and_height_spans_wrapped_link_across_rows() {
        // A link wrapped onto two rows stays one ordinal, anchored at its
        // first row; a following link is ordinal 1 on the second row.
        let rows = vec![
            vec![Word::new("click".to_owned(), WordType::Link)],
            vec![
                Word::new("here".to_owned(), WordType::Link),
                Word::new(" ".to_owned(), WordType::Normal),
                Word::new("[b]".to_owned(), WordType::Link),
            ],
        ];
        let root = multiline(10, rows);
        assert_eq!(root.link_index_and_height(), vec![(0, 10), (1, 11)]);
    }

    #[test]
    fn link_anchor_at_returns_nth_url_else_none() {
        let root = ComponentRoot::new(
            None,
            vec![
                para_comp(0, link_words("first", "https://a.example")),
                para_comp(5, link_words("second", "https://b.example")),
            ],
        );
        assert_eq!(root.link_anchor_at(0), Some("https://a.example"));
        assert_eq!(root.link_anchor_at(1), Some("https://b.example"));
        assert_eq!(root.link_anchor_at(2), None);
    }

    #[test]
    fn select_returns_component_offset_and_errors_out_of_bounds() {
        let mut root = ComponentRoot::new(
            None,
            vec![
                para_comp(0, link_words("first", "https://a.example")),
                para_comp(7, link_words("second", "https://b.example")),
            ],
        );
        assert_eq!(root.select(0), Ok(0));
        assert_eq!(root.select(1), Ok(7));
        assert!(root.select(2).is_err());
    }

    #[test]
    fn heading_offset_sums_preceding_heights_else_errors() {
        // A two-row paragraph (height 2) precedes the heading.
        let filler = TextComponent::new_formatted(
            TextNode::Paragraph,
            vec![
                vec![Word::new("a".to_owned(), WordType::Normal)],
                vec![Word::new("b".to_owned(), WordType::Normal)],
            ],
        );
        let heading = TextComponent::new_formatted(
            TextNode::Heading,
            vec![vec![Word::new("Title".to_owned(), WordType::Normal)]],
        );
        let root = ComponentRoot::new(
            None,
            vec![
                Component::TextComponent(filler),
                Component::TextComponent(heading),
            ],
        );
        assert_eq!(root.heading_offset("#title"), Ok(2));
        assert!(root.heading_offset("#missing").is_err());
    }

    #[test]
    fn find_footnote_returns_body_for_matching_ref_else_message() {
        let words = vec![
            Word::new("1".to_owned(), WordType::FootnoteData), // meta-info ref key
            Word::new("the body".to_owned(), WordType::Footnote),
        ];
        let comp = TextComponent::new(TextNode::Footnote, words);
        let root = ComponentRoot::new(None, vec![Component::TextComponent(comp)]);
        assert_eq!(root.find_footnote("1"), "the body");
        assert_eq!(root.find_footnote("2"), "Footnote not found");
    }

    #[test]
    fn details_index_and_height_lists_visible_summaries_only() {
        let mk = |id, y| {
            let mut c = TextComponent::new(
                TextNode::DetailsSummary {
                    id,
                    folded: false,
                    body_len: 0,
                },
                vec![Word::new("Summary".to_owned(), WordType::Normal)],
            );
            c.set_y_offset(y);
            c
        };
        let mut hidden = mk(3, 20);
        hidden.set_hidden(true);
        let root = ComponentRoot::new(
            None,
            vec![
                Component::TextComponent(mk(1, 2)),
                Component::TextComponent(mk(2, 8)),
                Component::TextComponent(hidden),
            ],
        );
        // Hidden summary excluded; indices are sequential over visible ones.
        assert_eq!(root.details_index_and_height(), vec![(0, 2), (1, 8)]);
    }
}
