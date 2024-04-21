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
            .new_protocol(
                image,
                Rect::new(0, 100, CONFIG.width, 20),
                Resize::Fit(None),
            )
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

    pub fn get_scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub fn get_y_offset(&self) -> u16 {
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

    #[doc = r" Draws the current state of the widget in the given buffer. That is the only method required"]
    #[doc = r" to implement a custom stateful widget."]
    fn render_ref(&self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        if self.get_y_offset().saturating_sub(self.get_scroll_offset()) + self.height()
            >= area.height
            || self.get_y_offset().saturating_sub(self.get_scroll_offset()) == 0
        {
            return;
        }

        let image = Image::new(self.image.as_ref());

        let area = Rect::new(
            area.x,
            self.y_offset() - self.get_scroll_offset(),
            area.width,
            self.height,
        );

        image.render(area, buf)
    }
}
