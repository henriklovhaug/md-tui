use std::sync::LazyLock;

use image::{DynamicImage, RgbaImage};
use resvg::{tiny_skia, usvg};

/// Rasterise at twice the SVG's nominal size so diagrams stay sharp after
/// being scaled to fit the terminal.
const SCALE: f32 = 2.0;

static SVG_OPTIONS: LazyLock<usvg::Options<'static>> = LazyLock::new(|| {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    options
});

/// Render Mermaid source to an image, or `None` if it cannot be rendered.
#[must_use]
pub fn render(source: &str) -> Option<DynamicImage> {
    let svg = mermaid_rs_renderer::render(source).ok()?;
    let tree = usvg::Tree::from_str(&svg, &SVG_OPTIONS).ok()?;
    let size = tree.size().to_int_size().scale_by(SCALE)?;

    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())?;
    // An opaque background means the premultiplied pixels equal straight RGBA.
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(SCALE, SCALE),
        &mut pixmap.as_mut(),
    );

    RgbaImage::from_raw(size.width(), size.height(), pixmap.take()).map(DynamicImage::ImageRgba8)
}
