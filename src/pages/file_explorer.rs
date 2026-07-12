use std::cmp;

use crate::{
    event_handler::viewport_height,
    search::find_files,
    util::{App, general::GENERAL_CONFIG},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState},
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
        self.state.select(Some(i));

        let vh = viewport_height(height) as usize;
        if i >= (self.page + 1) * vh {
            self.page += 1;
        } else if i < self.page * vh {
            self.page = i / vh;
        }
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
        self.state.select(Some(i));

        let vh = viewport_height(height) as usize;
        if i < self.page * vh {
            self.page = self.page.saturating_sub(1);
        } else if i >= (self.page + 1) * vh {
            self.page = i / vh;
        }
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
        let vh = viewport_height(height) as usize;
        self.page = i / vh;
    }

    pub fn next_page(&mut self, height: u16) {
        let vh = viewport_height(height) as usize;
        let i = match self.state.selected() {
            Some(i) => cmp::min(i + vh, self.files.len().saturating_sub(2)),
            None => 0,
        };
        self.state.select(Some(i));
        self.page = i / vh;
    }

    pub fn previous_page(&mut self, height: u16) {
        let vh = viewport_height(height) as usize;
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(vh),
            None => 0,
        };
        self.state.select(Some(i));
        self.page = i / vh;
    }

    pub fn height(&self, height: u16) -> usize {
        let vh = viewport_height(height) as usize;
        cmp::min(self.files.len(), vh)
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

    let vh = viewport_height(size.height);

    let area = Rect::new(x, 2, app.width(), vh);

    let items: Vec<ListItem> = file_tree
        .files
        .iter()
        .skip(file_tree.page * vh as usize)
        .take(vh as usize)
        .map(|f| match f {
            MdFileComponent::File(file) => ListItem::new(file.name()),
            MdFileComponent::Spacer => ListItem::new(""),
        })
        .collect();

    let list = List::new(items)
        .block(Block::default())
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ");

    let mut state = *file_tree.state();
    let selected = state.selected().map(|i| i % vh as usize);
    state.select(selected);

    f.render_stateful_widget(list, area, &mut state);
}
