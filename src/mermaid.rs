use std::sync::LazyLock;

use image::{DynamicImage, RgbaImage};
use mermaid_rs_renderer::{RenderOptions, Theme};
use resvg::{tiny_skia, usvg};

/// Rasterise at twice the SVG's nominal size so diagrams stay sharp after
/// being scaled to fit the terminal.
const SCALE: f32 = 2.0;

static SVG_OPTIONS: LazyLock<usvg::Options<'static>> = LazyLock::new(|| {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    options
});

/// How Mermaid diagrams are colored. Each color is any SVG color, such as
/// `#1e1e2e` or `white`, and replaces the theme's color when set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MermaidStyle {
    /// A mermaid-rs-renderer theme name: modern, default, dark, forest or neutral.
    pub theme: String,
    pub background_color: Option<String>,
    pub node_color: Option<String>,
    pub border_color: Option<String>,
    pub text_color: Option<String>,
    pub line_color: Option<String>,
}

impl Default for MermaidStyle {
    fn default() -> Self {
        Self {
            theme: "modern".to_owned(),
            background_color: None,
            node_color: None,
            border_color: None,
            text_color: None,
            line_color: None,
        }
    }
}

impl MermaidStyle {
    fn theme(&self) -> Theme {
        let mut theme = Theme::from_name(&self.theme).unwrap_or_else(Theme::modern);
        let overrides = [
            (
                &self.background_color,
                vec![&mut theme.background, &mut theme.edge_label_background],
            ),
            (
                &self.node_color,
                vec![&mut theme.primary_color, &mut theme.sequence_actor_fill],
            ),
            (
                &self.border_color,
                vec![
                    &mut theme.primary_border_color,
                    &mut theme.sequence_actor_border,
                ],
            ),
            (
                &self.text_color,
                vec![&mut theme.primary_text_color, &mut theme.text_color],
            ),
            (
                &self.line_color,
                vec![&mut theme.line_color, &mut theme.sequence_actor_line],
            ),
        ];
        for (color, fields) in overrides {
            if let Some(color) = color {
                for field in fields {
                    color.clone_into(field);
                }
            }
        }
        theme
    }
}

/// Render Mermaid source to an image, or `None` if it cannot be rendered.
#[must_use]
pub fn render(source: &str, style: &MermaidStyle) -> Option<DynamicImage> {
    let options = RenderOptions {
        theme: style.theme(),
        ..RenderOptions::default()
    };
    let svg = mermaid_rs_renderer::render_with_options(source, options).ok()?;
    let tree = usvg::Tree::from_str(&svg, &SVG_OPTIONS).ok()?;
    let size = tree.size().to_int_size().scale_by(SCALE)?;

    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(SCALE, SCALE),
        &mut pixmap.as_mut(),
    );

    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let c = pixel.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    RgbaImage::from_raw(size.width(), size.height(), rgba).map(DynamicImage::ImageRgba8)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOWCHART: &str = "flowchart TD\n  A[Start] --> B[End]\n";

    fn corner_pixel(style: &MermaidStyle) -> [u8; 4] {
        render(FLOWCHART, style)
            .unwrap()
            .to_rgba8()
            .get_pixel(0, 0)
            .0
    }

    #[test]
    fn dark_theme_renders_a_dark_background() {
        let style = MermaidStyle {
            theme: "dark".to_owned(),
            ..MermaidStyle::default()
        };

        assert_eq!(corner_pixel(&style), [0x33, 0x33, 0x33, 0xff]);
    }

    #[test]
    fn background_color_overrides_the_theme() {
        let style = MermaidStyle {
            theme: "dark".to_owned(),
            background_color: Some("#1e1e2e".to_owned()),
            ..MermaidStyle::default()
        };

        assert_eq!(corner_pixel(&style), [0x1e, 0x1e, 0x2e, 0xff]);
    }
}
