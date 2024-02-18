use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::Text;
use ratatui::widgets::{HighlightSpacing, Widget};
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

#[derive(Debug, Clone)]
enum MdFileComponent {
    File(MdFile),
    Spacer,
}

#[derive(Debug, Clone, Default)]
pub struct MdFile {
    pub path: String,
    pub name: String,
}

impl MdFile {
    pub fn new(path: String, name: String) -> Self {
        Self { path, name }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl From<MdFile> for ListItem<'_> {
    fn from(val: MdFile) -> Self {
        let mut text = Text::default();
        text.extend([
            val.name.to_owned().blue(),
            val.path.to_owned().italic().gray(),
        ]);
        ListItem::new(text)
    }
}

impl From<MdFileComponent> for ListItem<'_> {
    fn from(value: MdFileComponent) -> Self {
        match value {
            MdFileComponent::File(f) => f.into(),
            MdFileComponent::Spacer => ListItem::new(Text::raw("")),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileTree {
    files: Vec<MdFileComponent>,
    page: u32,
    list_state: ListState,
    search: Option<String>,
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            list_state: ListState::default(),
            page: 0,
            search: None,
        }
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

    pub fn sort_2(&mut self) {
        // Separate files and spacers into two vectors
        let (mut files, mut spacers): (Vec<_>, Vec<_>) = self
            .files
            .drain(..)
            .partition(|c| matches!(c, MdFileComponent::File(_)));

        // Sort the files in-place by name
        files.sort_unstable_by(|a, b| match (a, b) {
            (MdFileComponent::File(fa), MdFileComponent::File(fb)) => fb.name.cmp(&fa.name),
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

    pub fn search(&mut self, query: &str) {
        self.search = Some(query.to_string());
    }

    pub fn next(&mut self, height: u16) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.files.len() {
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
                    self.files.len()
                } else {
                    i.saturating_sub(2)
                }
            }
            None => 0,
        };
        self.page = (i / self.partition(height)) as u32;
        self.list_state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.list_state.select(None);
    }

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
        self.files.push(MdFileComponent::File(file));
        self.files.push(MdFileComponent::Spacer);
    }

    pub fn files(&self) -> Vec<&MdFile> {
        self.files
            .iter()
            .filter_map(|f| match f {
                MdFileComponent::File(f) => Some(f),
                MdFileComponent::Spacer => None,
            })
            .collect::<Vec<&MdFile>>()
    }

    fn partition(&self, height: u16) -> usize {
        let partition_size = (height as usize + 2) / 2;

        if partition_size % 2 == 0 {
            partition_size
        } else {
            partition_size + 1
        }
    }

    pub fn state(&self) -> &ListState {
        &self.list_state
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

        let page_count = format!("  {}/{}", self.page + 1, file_len / partition + 1);

        let paragraph = Text::styled(page_count, Style::default().fg(Color::LightGreen));

        let items = List::new(items)
            .block(
                Block::default()
                    .title("MD-CLI")
                    .add_modifier(Modifier::BOLD)
                    .title_alignment(Alignment::Center),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::LightGreen)
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

        paragraph.render(area, buf);
    }
}
