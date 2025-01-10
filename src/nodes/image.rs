use std::cmp;

use image::{DynamicImage, Rgb};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};

use super::{root::ComponentProps, textcomponent::TextNode};

pub struct ImageComponent {
    _alt_text: String,
    y_offset: u16,
    height: u16,
    scroll_offset: u16,
    image: StatefulProtocol,
}

impl ImageComponent {
    pub fn new<T: ToString>(image: DynamicImage, height: u32, alt_text: T) -> Option<Self> {
        let picker = Picker::from_query_stdio().ok()?;

        let image = picker.new_resize_protocol(image);

        let (_, f_height) = picker.font_size();

        let height = cmp::min(height / f_height as u32, 20) as u16;

        Some(Self {
            height,
            image,
            _alt_text: alt_text.to_string(),
            scroll_offset: 0,
            y_offset: 0,
        })
    }

    pub fn image_mut(&mut self) -> &mut StatefulProtocol {
        &mut self.image
    }

    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.scroll_offset = offset;
    }

    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub fn y_offset(&self) -> u16 {
        self.y_offset
    }

    pub fn height(&self) -> u16 {
        self.height
    }
}

impl ComponentProps for ImageComponent {
    fn height(&self) -> u16 {
        self.height
    }

    fn set_y_offset(&mut self, y_offset: u16) {
        self.y_offset = y_offset;
    }

    fn set_scroll_offset(&mut self, scroll: u16) {
        self.scroll_offset = scroll;
    }

    fn kind(&self) -> TextNode {
        TextNode::Image
    }
}
