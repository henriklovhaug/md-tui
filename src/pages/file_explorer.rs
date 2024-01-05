use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::Text;
use ratatui::widgets::{HighlightSpacing, Widget};
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

#[derive(Debug, Clone, Default)]
pub struct MdFile {
    pub path: String,
    pub name: String,
    pub selected: bool,
}

impl MdFile {
    pub fn new(path: String, name: String) -> Self {
        Self {
            path,
            name,
            selected: false,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Into<ListItem<'_>> for MdFile {
    fn into(self) -> ListItem<'static> {
        let mut text = Text::default();
        text.extend([
            self.name.to_owned().blue(),
            self.path.to_owned().italic().gray(),
            "\n".into(),
        ]);
        ListItem::new(text)
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileTree {
    files: Vec<MdFile>,
    list_state: ListState,
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            list_state: ListState::default(),
        }
    }

    pub fn with_items(files: Vec<MdFile>) -> Self {
        Self {
            files,
            list_state: ListState::default(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.files.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.files.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.list_state.select(None);
    }

    pub fn selected(&self) -> Option<&MdFile> {
        match self.list_state.selected() {
            Some(i) => self.files.get(i),
            None => None,
        }
    }

    pub fn add_file(&mut self, file: MdFile) {
        self.files.push(file);
    }

    pub fn files(&self) -> &Vec<MdFile> {
        &self.files
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

        let items = self
            .files
            .into_iter()
            .map(|f| f.into())
            .collect::<Vec<ListItem>>();

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
    }
}
