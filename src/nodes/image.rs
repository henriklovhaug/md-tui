use std::cmp;

use image::DynamicImage;
use ratatui::layout::Size;
use ratatui_image::{FilterType, Resize, picker::Picker, sliced::SlicedProtocol};

use crate::util::general::GENERAL_CONFIG;

use super::{root::ComponentProps, textcomponent::TextNode};

pub struct ImageComponent {
    _alt_text: String,
    y_offset: u16,
    scroll_offset: u16,
    image: SlicedProtocol,
}

impl ImageComponent {
    pub fn new<T: ToString>(image: DynamicImage, height: u32, alt_text: T) -> Option<Self> {
        let picker = Picker::from_query_stdio().ok()?;

        let font = picker.font_size();

        let max_height = cmp::min(height / u32::from(font.height), 20) as u16;

        let size = Size::new(GENERAL_CONFIG.width, max_height);
        let image = SlicedProtocol::new_with_resize(
            &picker,
            image,
            size,
            Resize::Fit(Some(FilterType::Nearest)),
        )
        .ok()?;

        Some(Self {
            image,
            _alt_text: alt_text.to_string(),
            scroll_offset: 0,
            y_offset: 0,
        })
    }

    #[must_use]
    pub fn image(&self) -> &SlicedProtocol {
        &self.image
    }

    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.scroll_offset = offset;
    }

    #[must_use]
    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    #[must_use]
    pub fn y_offset(&self) -> u16 {
        self.y_offset
    }

    #[must_use]
    pub fn height(&self) -> u16 {
        self.image.size().height
    }
}

impl ComponentProps for ImageComponent {
    fn height(&self) -> u16 {
        self.image.size().height
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
