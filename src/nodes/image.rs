use std::sync::LazyLock;

use image::DynamicImage;
use ratatui::layout::Size;
use ratatui_image::{FilterType, Resize, picker::Picker, sliced::SlicedProtocol};

use super::{root::ComponentProps, textcomponent::TextNode};
use crate::util::general::GENERAL_CONFIG;

static IMAGE_PICKER: LazyLock<Picker> =
    LazyLock::new(|| Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()));

pub struct ImageComponent {
    _alt_text: String,
    y_offset: u16,
    height: u16,
    scroll_offset: u16,
    source: Option<DynamicImage>,
    image: Option<SlicedProtocol>,
    size: Size,
}

impl ImageComponent {
    pub fn new<T: ToString>(image: DynamicImage, width: u16, alt_text: T) -> Self {
        let resize = Resize::Fit(Some(FilterType::CatmullRom));
        // `parse_markdown` receives the text-wrap width, while the document
        // viewport reserves one additional column for its layout.
        let available = Size::new(width.saturating_sub(1), GENERAL_CONFIG.image_max_height);
        let size = resize.size_for(&image, IMAGE_PICKER.font_size(), available);

        Self {
            height: size.height,
            source: Some(image),
            image: None,
            size,
            _alt_text: alt_text.to_string(),
            scroll_offset: 0,
            y_offset: 0,
        }
    }

    pub fn image(&mut self) -> Option<&SlicedProtocol> {
        if self.image.is_none() {
            let source = self.source.take()?;
            let resize = Resize::Fit(Some(FilterType::CatmullRom));
            self.image =
                SlicedProtocol::new_with_resize(&IMAGE_PICKER, source, self.size, resize).ok();
        }
        self.image.as_ref()
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

#[cfg(test)]
mod tests {
    use image::DynamicImage;

    use super::ImageComponent;

    #[test]
    fn image_protocol_is_initialized_once_at_a_stable_size() {
        let source = DynamicImage::new_rgba8(1800, 1200);
        let mut component = ImageComponent::new(source, 120, "synthetic plot");
        let height = component.height;

        assert!(height > 0);
        assert!(component.source.is_some());
        assert!(component.image.is_none());

        let first = component.image().unwrap() as *const _;
        let second = component.image().unwrap() as *const _;

        assert_eq!(first, second);
        assert_eq!(component.height, height);
        assert!(component.source.is_none());
    }
}
