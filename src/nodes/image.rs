use std::cmp;

use image::{DynamicImage, Rgb};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{StatefulWidgetRef, Widget},
};
use ratatui_image::{picker::Picker, protocol::Protocol, Image, Resize};

use crate::util::CONFIG;

use super::{root::ComponentProps, textcomponent::TextNode};

pub struct ImageComponent {
    _alt_text: String,
    y_offset: u16,
    height: u16,
    scroll_offset: u16,
    image: Box<dyn Protocol>,
}

impl ImageComponent {
    pub fn new<T: ToString>(image: DynamicImage, height: u16, alt_text: T) -> Self {
        let mut picker = Picker::from_termios().unwrap();
        picker.guess_protocol();
        picker.background_color = Some(Rgb::<u8>([0, 0, 0]));

        let image = picker
            .new_protocol(image, Rect::new(0, 0, CONFIG.width, 20), Resize::Fit(None))
            .unwrap();

        Self {
            height,
            image,
            _alt_text: alt_text.to_string(),
            scroll_offset: 0,
            y_offset: 0,
        }
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

impl StatefulWidgetRef for ImageComponent {
    type State = Picker;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        if self.y_offset().saturating_sub(self.scroll_offset()) + self.height() > area.height
            || self.y_offset().saturating_sub(self.scroll_offset()) + self.height() == 0
        {
            return;
        }

        let image = Image::new(self.image.as_ref());

        let height = cmp::min(
            self.height,
            (self.y_offset() + self.height).saturating_sub(self.scroll_offset()),
        );

        let area = Rect::new(
            area.x,
            self.y_offset().saturating_sub(self.scroll_offset()),
            area.width,
            height,
        );

        image.render(area, buf)
    }
}
