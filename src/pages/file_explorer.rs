use std::cmp;

use crate::{
    search::find_files,
    util::{App, Boxes, colors::color_config, general::GENERAL_CONFIG},
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph},
};

#[derive(Debug, Clone)]
pub struct MdFile {
    pub(crate) path: String,
    pub(crate) name: String,
}

impl MdFile {
    #[must_use]
    pub fn new(path: String, name: String) -> Self {
        Self { path, name }
    }

    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        std::path::Path::new(&self.path)
    }

    #[must_use]
    pub fn path_str(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn sort_path(&self) -> String {
        self.path()
            .to_str()
            .unwrap_or("")
            .trim_start_matches("./")
            .trim_start_matches(char::is_alphabetic)
            .to_string()
    }
}

#[derive(Debug, Clone)]
pub enum MdFileComponent {
    File(MdFile),
    Spacer,
}

#[derive(Debug, Clone, Default)]
pub struct FileTree {
    all_files: Vec<MdFile>,
    files: Vec<MdFileComponent>,
    state: ListState,
    page: usize,
    search: Option<String>,
    /// Selection snapshot taken when the search box opens, so we can
    /// restore the cursor row if the user dismisses search without
    /// committing (Esc, or Backspace down to empty).
    pre_search_selection: Option<usize>,
}

impl FileTree {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, file: MdFile) {
        self.all_files.push(file);
    }

    pub fn finish(&mut self) {
        self.files = self
            .all_files
            .iter()
            .cloned()
            .flat_map(|f| vec![MdFileComponent::File(f), MdFileComponent::Spacer])
            .collect();

        self.sort_name();
    }

    #[must_use]
    pub fn loaded(&self) -> bool {
        !self.all_files.is_empty()
    }

    #[must_use]
    pub fn selected(&self) -> Option<&MdFile> {
        let selected = self.state.selected()?;
        match self.files.get(selected)? {
            MdFileComponent::File(f) => Some(f),
            MdFileComponent::Spacer => None,
        }
    }

    pub fn next(&mut self, height: u16) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.files.len().saturating_sub(2) {
                    0
                } else {
                    i + 2
                }
            }
            None => 0,
        };
        self.page = i / self.partition(height);
        self.state.select(Some(i));
    }

    pub fn previous(&mut self, height: u16) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.files.len().saturating_sub(2)
                } else {
                    i - 2
                }
            }
            None => 0,
        };
        self.page = i / self.partition(height);
        self.state.select(Some(i));
    }

    pub fn sort_name(&mut self) {
        // Separate files and spacers into two vectors
        let (mut files, mut spacers): (Vec<_>, Vec<_>) = self
            .files
            .drain(..)
            .partition(|c| matches!(c, MdFileComponent::File(_)));

        // Sort the files in-place by name
        files.sort_unstable_by(|a, b| match (a, b) {
            (MdFileComponent::File(fa), MdFileComponent::File(fb)) => {
                let a = fa.sort_path();
                let b = fb.sort_path();

                b.to_lowercase().cmp(&a.to_lowercase())
            }
            _ => unreachable!(),
        });

        let mut result = Vec::with_capacity(files.len() + spacers.len());
        while let (Some(file), Some(spacer)) = (files.pop(), spacers.pop()) {
            result.push(file);
            result.push(spacer);
        }

        // Update self.files with the sorted and interleaved result
        self.files = result;
    }

    pub fn search(&mut self, query: Option<&str>) {
        self.state_mut().select(None);
        self.page = 0;
        self.search = query.map(ToOwned::to_owned);
        match query {
            Some(query) => {
                self.files = find_files(&self.all_files, query)
                    .into_iter()
                    .map(MdFileComponent::File)
                    .flat_map(|f| vec![f, MdFileComponent::Spacer])
                    .collect();
            }
            None => {
                self.files = self
                    .all_files
                    .iter()
                    .cloned()
                    .flat_map(|f| vec![MdFileComponent::File(f), MdFileComponent::Spacer])
                    .collect();
                self.sort_name();
            }
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    /// Remember the current selection so a later `restore_pre_search` can
    /// put the cursor back if the user dismisses search without committing.
    pub fn snapshot_pre_search(&mut self) {
        self.pre_search_selection = self.state.selected();
    }

    /// Restore the selection saved by `snapshot_pre_search`. Falls back to
    /// the first row if no snapshot exists or the index is now stale.
    pub fn restore_pre_search(&mut self) {
        let restore = self
            .pre_search_selection
            .take()
            .filter(|&i| i < self.files.len());
        match restore {
            Some(i) => {
                self.state.select(Some(i));
            }
            None => self.first(),
        }
    }

    pub fn first(&mut self) {
        self.state.select(Some(0));
        self.page = 0;
    }

    pub fn last(&mut self, height: u16) {
        let i = self.files.len().saturating_sub(2);
        self.state.select(Some(i));
        self.page = i / self.partition(height);
    }

    pub fn next_page(&mut self, height: u16) {
        let partition = self.partition(height);
        let i = match self.state.selected() {
            Some(i) => cmp::min(i + partition, self.files.len().saturating_sub(2)),
            None => 0,
        };
        self.state.select(Some(i));
        self.page = i / partition;
    }

    pub fn previous_page(&mut self, height: u16) {
        let partition = self.partition(height);
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(partition),
            None => 0,
        };
        self.state.select(Some(i));
        self.page = i / partition;
    }

    /// Number of `files` entries (files + interleaved spacers) shown per page.
    /// Rounded up to an even number so a page never ends between a file and
    /// its trailing spacer. Kept in sync with the render area height.
    fn partition(&self, height: u16) -> usize {
        let partition_size = usize::midpoint(height as usize, 2);
        if partition_size.is_multiple_of(2) {
            partition_size
        } else {
            partition_size + 1
        }
    }

    pub fn height(&self, height: u16) -> usize {
        cmp::min(self.files.len(), self.partition(height))
    }

    pub fn state(&self) -> &ListState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }
}

pub fn render_file_tree(f: &mut Frame, app: &App, file_tree: FileTree) {
    let size = f.area();
    let x = match GENERAL_CONFIG.centering {
        crate::util::general::Centering::Left => 2,
        crate::util::general::Centering::Center => {
            if size.width > GENERAL_CONFIG.width {
                (size.width - GENERAL_CONFIG.width) / 2
            } else {
                2
            }
        }
        crate::util::general::Centering::Right => size
            .width
            .saturating_sub(GENERAL_CONFIG.width)
            .saturating_sub(2),
    };

    // The list gets the full terminal height; paging is driven by `partition`,
    // which is derived from this same height so the visible page matches the
    // selection math in `next`/`previous`.
    let area = Rect {
        x,
        width: app.width().saturating_sub(3),
        ..size
    };

    let mut state = file_tree.state().to_owned();
    let file_len = file_tree.files.len();
    let partition = file_tree.partition(area.height);

    let page = file_tree
        .files
        .chunks(partition)
        .nth(file_tree.page)
        .unwrap_or(&file_tree.files);

    // Selection index is global; map it into the currently rendered page.
    state.select(state.selected().map(|i| i % partition));

    // Each file card is two lines (name + italic path) plus a spacer row.
    let y_height = page.len() / 2 * 3;

    let items: Vec<ListItem> = page
        .iter()
        .map(|c| match c {
            MdFileComponent::File(file) => ListItem::new(Text::from(vec![
                Line::from(file.name().fg(color_config().file_tree_name_color)),
                Line::from(
                    file.path_str()
                        .italic()
                        .fg(color_config().file_tree_path_color),
                ),
            ])),
            MdFileComponent::Spacer => ListItem::new(Text::raw("")),
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("MD-TUI")
                .add_modifier(Modifier::BOLD)
                .title_alignment(Alignment::Center),
        )
        .highlight_style(
            Style::default()
                .fg(color_config().file_tree_selected_fg_color)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{02503} ")
        .repeat_highlight_symbol(true)
        .highlight_spacing(HighlightSpacing::Always);

    f.render_stateful_widget(list, area, &mut state);

    let total_pages = usize::div_ceil(file_len, partition).max(1);
    let page_count = Paragraph::new(format!("  {}/{}", file_tree.page + 1, total_pages))
        .style(Style::default().fg(color_config().file_tree_page_count_color));
    let page_count_area = Rect {
        y: area.y + y_height as u16 + 2,
        ..area
    };
    f.render_widget(page_count, page_count_area);

    // Bottom-anchored help overlay, sized to the collapsed hint or the full
    // table. Mirrors the markdown view's help sizing so it never clobbers the
    // list on short terminals. Hidden while the search box is open.
    if GENERAL_CONFIG.help_menu && app.boxes != Boxes::Search {
        const HELP_BLOCK_HEIGHT: u16 = 30;
        const HELP_CONTENT_HEIGHT: u16 = 28;
        let (block_h, content_basis, content_h) = if app.help_box.expanded() {
            (HELP_BLOCK_HEIGHT, HELP_CONTENT_HEIGHT, HELP_CONTENT_HEIGHT)
        } else {
            (3, 1, 3)
        };
        let block_area = Rect {
            x: area.x,
            y: size.height.saturating_sub(block_h + 1),
            width: area.width.saturating_sub(1),
            height: cmp::min(block_h, size.height),
        };
        let help_area = Rect {
            x: area.x + 2,
            y: size.height.saturating_sub(content_basis + 2),
            width: app.width().saturating_sub(5),
            height: cmp::min(content_h, size.height),
        };
        f.render_widget(Clear, block_area);
        f.render_widget(app.help_box, help_area);
    }
}
