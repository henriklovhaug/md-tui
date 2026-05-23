use std::collections::HashSet;

use crate::search::{compare_heading, find_and_mark};

use super::{
    image::ImageComponent,
    textcomponent::{TextComponent, TextNode},
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

    #[must_use]
    pub fn link_index_and_height(&self) -> Vec<(usize, u16)> {
        let mut indexes = Vec::new();
        let mut count = 0;
        self.components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .filter(|comp| !comp.is_hidden())
            .for_each(|comp| {
                let height = comp.y_offset();
                comp.content().iter().enumerate().for_each(|(index, row)| {
                    row.iter().for_each(|c| {
                        if matches!(
                            c.kind(),
                            WordType::Link | WordType::Selected | WordType::FootnoteInline
                        ) {
                            indexes.push((count, height + index as u16));
                            count += 1;
                        }
                    });
                });
            });

        indexes
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
    pub fn selected(&self) -> &str {
        let block = self
            .components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .find(|c| c.is_focused())
            .unwrap();
        block.highlight_link().unwrap()
    }

    #[must_use]
    pub fn selected_underlying_type(&self) -> WordType {
        let selected = self
            .components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .find(|c| c.is_focused())
            .unwrap()
            .content()
            .iter()
            .flatten()
            .filter(|c| c.kind() == WordType::Selected)
            .collect::<Vec<_>>();

        selected.first().unwrap().previous_type()
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
