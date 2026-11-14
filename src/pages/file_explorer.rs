use std::borrow::ToOwned;
use std::cmp;
use std::path::Path;

use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{HighlightSpacing, Widget};
use ratatui::{
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};
use unicode_width::UnicodeWidthStr;

use super::file_details::{DETAILS_WIDTH, file_details};
use crate::search::find_files;
use crate::util::colors::color_config;
use crate::util::general::GENERAL_CONFIG;

/// Columns before each row, taken by the selection marker.
const MARKER_WIDTH: usize = 2;
/// Columns between a file name and its details. A top-level path is the name
/// plus `./`, so three leaves a column between it and the details.
const DETAILS_GAP: usize = 3;

#[derive(Debug, Clone)]
enum MdFileComponent {
    File(MdFile),
    Spacer,
}

impl MdFileComponent {
    fn list_item(&self, name_column: usize) -> ListItem<'static> {
        match self {
            Self::File(file) => file.list_item(name_column),
            Self::Spacer => ListItem::new(Text::raw("")),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MdFile {
    pub path: String,
    pub name: String,
    pub(super) details: Option<String>,
}

impl MdFile {
    #[must_use]
    pub fn new(path: String, name: String) -> Self {
        let details = GENERAL_CONFIG
            .file_tree_details
            .then(|| file_details(Path::new(&path)))
            .flatten();
        Self {
            path,
            name,
            details,
        }
    }

    #[must_use]
    pub fn path_str(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        Path::new(&self.path)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn list_item(&self, name_column: usize) -> ListItem<'static> {
        let colors = color_config();
        let mut name_row = vec![self.name.clone().fg(colors.file_tree_name_color)];
        if let Some(details) = &self.details {
            let padding = name_column.saturating_sub(self.name.width()) + DETAILS_GAP;
            name_row
                .push(format!("{}{details}", " ".repeat(padding)).fg(colors.file_tree_path_color));
        }
        ListItem::new(vec![
            Line::from(name_row),
            Line::from(self.path.clone().italic().fg(colors.file_tree_path_color)),
        ])
    }
}

/// Where the details start on a page, so they line up. A longer name pushes its own details over.
fn name_column(items: &[MdFileComponent], width: u16) -> usize {
    items
        .iter()
        .filter_map(|item| match item {
            MdFileComponent::File(file) if file.details.is_some() => Some(file.name.width()),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .min(usize::from(width).saturating_sub(MARKER_WIDTH + DETAILS_GAP + DETAILS_WIDTH))
}

#[derive(Debug, Clone, Default)]
pub struct FileTree {
    all_files: Vec<MdFile>,
    files: Vec<MdFileComponent>,
    page: u32,
    list_state: ListState,
    search: Option<String>,
    loaded: bool,
}

impl FileTree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            all_files: Vec::new(),
            files: Vec::new(),
            list_state: ListState::default(),
            page: 0,
            search: None,
            loaded: false,
        }
    }

    #[must_use]
    pub fn loaded(&self) -> bool {
        self.loaded
    }

    #[must_use]
    pub fn finish(self) -> Self {
        let mut this = self;
        this.loaded = true;
        this
    }

    pub fn sort(&mut self) {
        let filtered: Vec<&MdFile> = self
            .files
            .iter()
            .filter_map(|c| match c {
                MdFileComponent::File(f) => Some(f),
                MdFileComponent::Spacer => None,
            })
            .sorted_unstable_by(|a, b| a.name.cmp(&b.name))
            .collect();

        let spacers = vec![MdFileComponent::Spacer; filtered.len()];

        self.files = filtered
            .into_iter()
            .zip(spacers)
            .flat_map(|(f, s)| vec![MdFileComponent::File(f.to_owned()), s])
            .collect::<Vec<_>>();
    }

    pub fn sort_name(&mut self) {
        // Separate files and spacers into two vectors
        let (mut files, mut spacers): (Vec<_>, Vec<_>) = self
            .files
            .drain(..)
            .partition(|c| matches!(c, MdFileComponent::File(_)));

        // Sort the files in-place by name
        files.sort_unstable_by(|a, b| match (a, b) {
            (MdFileComponent::File(fa), MdFileComponent::File(fb)) => fb
                .name
                .to_lowercase()
                .cmp(&fa.name.to_lowercase())
                .then_with(|| fb.path.cmp(&fa.path)),
            _ => unreachable!(), // This case should not happen
        });

        // Interleave files and spacers
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
                    .collect();
            }
            None => {
                self.files = self
                    .all_files
                    .iter()
                    .cloned()
                    .map(MdFileComponent::File)
                    .collect();
            }
        }
        self.fill_spacers();
    }

    fn fill_spacers(&mut self) {
        let spacers = vec![MdFileComponent::Spacer; self.files.len()];
        self.files = self
            .files
            .iter()
            .cloned()
            .zip(spacers)
            .flat_map(|(f, s)| vec![f, s])
            .collect::<Vec<_>>();
    }

    pub fn next(&mut self, height: u16) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.files.len() - 2 {
                    0
                } else {
                    i + 2
                }
            }
            None => 0,
        };
        self.page = (i / self.partition(height)) as u32;
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self, height: u16) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.files.len() - 2
                } else {
                    i.saturating_sub(2)
                }
            }
            None => 0,
        };
        self.page = (i / self.partition(height)) as u32;
        self.list_state.select(Some(i));
    }

    pub fn next_page(&mut self, height: u16) {
        let partition = self.partition(height);
        let i = match self.list_state.selected() {
            Some(i) => {
                if i + partition >= self.files.len() {
                    0
                } else {
                    i + partition
                }
            }
            None => 0,
        };
        self.page = (i / partition) as u32;
        self.list_state.select(Some(i));
    }

    pub fn previous_page(&mut self, height: u16) {
        let partition = self.partition(height);
        let i = match self.list_state.selected() {
            Some(i) => {
                if i < partition {
                    self.files.len().saturating_sub(partition)
                } else {
                    i.saturating_sub(partition)
                }
            }
            None => 0,
        };
        self.page = (i / partition) as u32;
        self.list_state.select(Some(i));
    }

    pub fn first(&mut self) {
        self.list_state.select(Some(0));
        self.page = 0;
    }

    pub fn last(&mut self, height: u16) {
        let partition = self.partition(height);
        let i = self.files.len() - 2;
        self.list_state.select(Some(i));
        self.page = (i / partition) as u32;
    }

    pub fn unselect(&mut self) {
        self.list_state.select(None);
    }

    #[must_use]
    pub fn selected(&self) -> Option<&MdFile> {
        match self.list_state.selected() {
            Some(i) => self.files.get(i).and_then(|f| match f {
                MdFileComponent::File(f) => Some(f),
                MdFileComponent::Spacer => None,
            }),
            None => None,
        }
    }

    pub fn add_file(&mut self, file: MdFile) {
        self.all_files.push(file.clone());
        self.files.push(MdFileComponent::File(file));
        self.files.push(MdFileComponent::Spacer);
    }

    #[must_use]
    pub fn files(&self) -> Vec<&MdFile> {
        self.files
            .iter()
            .filter_map(|f| match f {
                MdFileComponent::File(f) => Some(f),
                MdFileComponent::Spacer => None,
            })
            .collect::<Vec<&MdFile>>()
    }

    #[must_use]
    pub fn all_files(&self) -> &Vec<MdFile> {
        &self.all_files
    }

    fn partition(&self, height: u16) -> usize {
        let partition_size = usize::midpoint(height as usize, 2);

        if partition_size.is_multiple_of(2) {
            partition_size
        } else {
            partition_size + 1
        }
    }

    #[must_use]
    pub fn state(&self) -> &ListState {
        &self.list_state
    }

    #[must_use]
    pub fn height(&self, height: u16) -> usize {
        cmp::min(
            self.partition(height) / 2 * 3,
            self.files
                .iter()
                .filter(|f| matches!(f, MdFileComponent::File(_)))
                .count()
                * 3,
        )
    }

    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.list_state
    }
}

impl Widget for FileTree {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = self.state().to_owned();
        let file_len = self.files.len();
        let partition = self.partition(area.height);

        let items = if let Some(iter) = self
            .files
            .chunks(self.partition(area.height))
            .nth(self.page as usize)
        {
            iter.to_owned()
        } else {
            self.files
        };

        state.select(state.selected().map(|i| i % partition));

        let y_height = items.len() / 2 * 3;
        let name_column = name_column(&items, area.width);

        let items = List::new(items.iter().map(|item| item.list_item(name_column)))
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

        StatefulWidget::render(items, area, buf, &mut state);

        let area = Rect {
            y: area.y + y_height as u16 + 2,
            ..area
        };

        let total_pages = usize::div_ceil(file_len, partition);

        let page_count_str = format!("  {}/{}", self.page + 1, total_pages);

        let page_count = Text::styled(
            page_count_str,
            Style::default().fg(color_config().file_tree_page_count_color),
        );

        page_count.render(area, buf);
    }
}
