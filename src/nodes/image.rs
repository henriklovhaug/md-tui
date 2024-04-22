use image::{DynamicImage, Rgb};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};

use super::{root::ComponentProps, textcomponent::TextNode};

pub struct ImageComponent {
    _alt_text: String,
    y_offset: u16,
    height: u16,
    scroll_offset: u16,
    image: Box<dyn StatefulProtocol>,
}

impl ImageComponent {
    pub fn new<T: ToString>(image: DynamicImage, height: u16, alt_text: T) -> Self {
        let mut picker = Picker::from_termios().unwrap();
        picker.guess_protocol();
        picker.background_color = Some(Rgb::<u8>([0, 0, 0]));

        let image = picker.new_resize_protocol(image);

        Self {
            height,
            image,
            _alt_text: alt_text.to_string(),
            scroll_offset: 0,
            y_offset: 0,
        }
    }

    pub fn image_mut(&mut self) -> &mut Box<dyn StatefulProtocol> {
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

    fn kind(&self) -> TextNode {
        TextNode::Image
    }
}
