use image::Rgb;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{StatefulWidget, Widget},
};
use ratatui_image::picker::Picker;

use crate::search::{compare_heading, find_and_mark};

use super::{
    image::ImageComponent,
    textcomponent::{TextComponent, TextNode},
    word::{Word, WordType},
};

#[derive(Clone)]
pub struct ComponentRoot {
    file_name: Option<String>,
    components: Vec<Component>,
    is_focused: bool,
    picker: Picker,
}

impl ComponentRoot {
    pub fn new(file_name: Option<String>, components: Vec<Component>) -> Self {
        let mut picker = Picker::from_termios().expect("Failed to create picker");
        picker.guess_protocol();
        picker.background_color = Some(Rgb::<u8>([255, 0, 255]));

        Self {
            file_name,
            components,
            is_focused: false,
            picker,
        }
    }

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

    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

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
        find_and_mark(search, &mut words)
    }

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
            if index - count < comp.num_links() {
                comp.visually_select(index - count)?;
                return Ok(comp.y_offset());
            }
            count += comp.num_links();
        }
        Err(format!("Index out of bounds: {} >= {}", index, count))
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

    pub fn link_index_and_height(&self) -> Vec<(usize, u16)> {
        let mut indexes = Vec::new();
        let mut count = 0;
        self.components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .for_each(|comp| {
                let height = comp.y_offset();
                comp.content().iter().enumerate().for_each(|(index, row)| {
                    row.iter().for_each(|c| {
                        if c.kind() == WordType::Link || c.kind() == WordType::Selected {
                            indexes.push((count, height + index as u16));
                            count += 1;
                        }
                    })
                });
            });

        indexes
    }

    /// Sets the y offset of the components
    pub fn set_scroll(&mut self, scroll: u16) {
        let mut y_offset = 0;
        for component in self.components.iter_mut() {
            component.set_y_offset(y_offset);
            component.set_scroll_offset(scroll);
            y_offset += component.height();
        }
    }

    pub fn heading_offset(&self, heading: &str) -> Result<u16, String> {
        let mut y_offset = 0;
        for component in self.components.iter() {
            match component {
                Component::TextComponent(comp) => {
                    if comp.kind() == TextNode::Heading
                        && compare_heading(&heading[1..], comp.content())
                    {
                        return Ok(y_offset);
                    }
                    y_offset += comp.height();
                }
                _ => todo!("Add height offset"),
            }
        }
        Err(format!("Heading not found: {}", heading))
    }

    /// Return the content of the components, where each element a line
    pub fn content(&self) -> Vec<String> {
        self.components()
            .iter()
            .flat_map(|c| c.content_as_lines())
            .collect()
    }

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

    /// Transforms the content of the components to fit the given width
    pub fn transform(&mut self, width: u16) {
        for component in self.components_mut() {
            component.transform(width);
        }
    }

    /// Because of the parsing, every table has a missing newline at the end
    pub fn add_missing_components(self) -> Self {
        let mut components = Vec::new();
        let mut iter = self
            .components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .peekable();
        while let Some(component) = iter.next() {
            components.push(Component::TextComponent(component.to_owned()));
            if let Some(next) = iter.peek() {
                if component.kind() != TextNode::LineBreak && next.kind() != TextNode::LineBreak {
                    components.push(Component::TextComponent(TextComponent::new(
                        TextNode::LineBreak,
                        Vec::new(),
                    )));
                }
            }
        }
        Self {
            file_name: self.file_name,
            components,
            is_focused: self.is_focused,
            picker: self.picker,
        }
    }

    pub fn height(&self) -> u16 {
        self.components.iter().map(|c| c.height()).sum()
    }

    pub fn num_links(&self) -> usize {
        self.components
            .iter()
            .filter_map(|f| match f {
                Component::TextComponent(comp) => Some(comp),
                Component::Image(_) => None,
            })
            .map(|c| c.num_links())
            .sum()
    }
}

impl StatefulWidget for ComponentRoot {
    type State = Picker;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        for component in self.components {
            component.render(area, buf, state);
        }
    }
}

pub trait ComponentProps {
    fn y_offset(&self) -> u16;
    fn height(&self) -> u16;
    fn set_y_offset(&mut self, y_offset: u16);
    fn set_scroll_offset(&mut self, scroll: u16);
}

#[derive(Debug, Clone)]
pub enum Component {
    TextComponent(TextComponent),
    Image(ImageComponent),
}

impl From<TextComponent> for Component {
    fn from(comp: TextComponent) -> Self {
        Component::TextComponent(comp)
    }
}

impl StatefulWidget for Component {
    type State = Picker;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        match self {
            Component::TextComponent(comp) => comp.render(area, buf),
            Component::Image(comp) => comp.render(area, buf, state),
        }
    }
}

impl ComponentProps for Component {
    fn y_offset(&self) -> u16 {
        match self {
            Component::TextComponent(comp) => comp.y_offset(),
            Component::Image(comp) => comp.y_offset(),
        }
    }

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
}
