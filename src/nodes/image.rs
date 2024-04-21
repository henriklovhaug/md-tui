use image::DynamicImage;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{StatefulWidget, Widget},
};
use ratatui_image::{picker::Picker, Image, Resize};

use crate::util::CONFIG;

use super::root::ComponentProps;

#[derive(Debug, Clone, Default)]
pub struct ImageComponent {
    alt_text: String,
    y_offset: u16,
    height: u16,
    scroll_offset: u16,
    image: DynamicImage,
}

impl ImageComponent {
    pub fn new(image: DynamicImage, height: u16) -> Self {
        Self {
            height,
            image,
            ..Default::default()
        }
    }

    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.scroll_offset = offset;
    }

    pub fn get_scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub fn get_image(&self) -> &DynamicImage {
        &self.image
    }

    pub fn get_y_offset(&self) -> u16 {
        self.y_offset
    }

    pub fn get_height(&self) -> u16 {
        self.height
    }
}

impl ComponentProps for ImageComponent {
    fn y_offset(&self) -> u16 {
        self.y_offset
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn set_y_offset(&mut self, y_offset: u16) {
        self.y_offset = y_offset;
    }

    fn set_scroll_offset(&mut self, scroll: u16) {
        self.scroll_offset = scroll;
    }
}

impl StatefulWidget for ImageComponent {
    type State = Picker;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let image = if let Ok(img) = state.new_protocol(
            self.image,
            Rect::new(0, 0, CONFIG.width, 15),
            Resize::Fit(None),
        ) {
            img
        } else {
            return;
        };

        let image = Image::new(image.as_ref());

        image.render(area, buf)
    }
}
