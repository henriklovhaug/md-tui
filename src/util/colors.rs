use std::{
    str::FromStr,
    sync::{Arc, LazyLock, RwLock},
};

use config::{Config, ConfigBuilder, Environment, File, builder::DefaultState};
use ratatui::style::Color;

use crate::highlight::{DEFAULT_COLOR_MAP, HIGHLIGHT_NAMES};

/// Every colour reader below reads the same `~/.config/mdt/config.toml`; this
/// keeps the path and file source in one place.
fn config_builder() -> ConfigBuilder<DefaultState> {
    let config_dir = dirs::home_dir().unwrap();
    let config_file = config_dir.join(".config").join("mdt").join("config.toml");
    Config::builder().add_source(File::with_name(config_file.to_str().unwrap()).required(false))
}

#[derive(Debug, Clone, Copy)]
pub struct ColorConfig {
    // Inline styles
    pub italic_color: Color,
    pub bold_color: Color,
    pub striketrough_color: Color,
    pub bold_italic_color: Color,
    pub code_fg_color: Color,
    pub code_bg_color: Color,
    pub link_color: Color,
    pub link_selected_fg_color: Color,
    pub link_selected_bg_color: Color,

    // Block styles
    pub code_block_bg_color: Color,
    pub heading_fg_color: Color,
    pub heading_bg_color: Color,
    pub table_header_fg_color: Color,
    pub table_header_bg_color: Color,
    pub quote_bg_color: Color,

    // File tree
    pub file_tree_selected_fg_color: Color,
    pub file_tree_page_count_color: Color,
    pub file_tree_name_color: Color,
    pub file_tree_path_color: Color,

    // Quote markings
    pub quote_important: Color,
    pub quote_warning: Color,
    pub quote_tip: Color,
    pub quote_note: Color,
    pub quote_caution: Color,
    pub quote_default: Color,
}

#[must_use]
pub fn read_color_config_from_file() -> ColorConfig {
    let settings = config_builder()
        .add_source(Environment::with_prefix("MDT").separator("_"))
        .build()
        .unwrap_or_default();

    ColorConfig {
        heading_bg_color: Color::from_str(
            &settings.get::<String>("h_bg_color").unwrap_or_default(),
        )
        .unwrap_or(Color::Blue),
        heading_fg_color: Color::from_str(
            &settings.get::<String>("h_fg_color").unwrap_or_default(),
        )
        .unwrap_or(Color::Black),
        italic_color: Color::from_str(&settings.get::<String>("italic_color").unwrap_or_default())
            .unwrap_or(Color::Reset),
        bold_color: Color::from_str(&settings.get::<String>("bold_color").unwrap_or_default())
            .unwrap_or(Color::Reset),
        striketrough_color: Color::from_str(
            &settings
                .get_string("striketrough_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::Reset),
        quote_bg_color: Color::from_str(
            &settings.get::<String>("quote_bg_color").unwrap_or_default(),
        )
        .unwrap_or(Color::Reset),
        code_fg_color: Color::from_str(
            &settings.get::<String>("code_fg_color").unwrap_or_default(),
        )
        .unwrap_or(Color::Red),
        code_bg_color: Color::from_str(
            &settings.get::<String>("code_bg_color").unwrap_or_default(),
        )
        .unwrap_or(Color::Rgb(48, 48, 48)),
        code_block_bg_color: Color::from_str(
            &settings
                .get::<String>("code_block_bg_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::Rgb(48, 48, 48)),
        link_color: Color::from_str(&settings.get::<String>("link_color").unwrap_or_default())
            .unwrap_or(Color::Blue),
        link_selected_fg_color: Color::from_str(
            &settings
                .get::<String>("link_selected_fg_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::Green),
        link_selected_bg_color: Color::from_str(
            &settings
                .get::<String>("link_selected_bg_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::DarkGray),
        table_header_fg_color: Color::from_str(
            &settings
                .get::<String>("table_header_fg_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::Yellow),
        table_header_bg_color: Color::from_str(
            &settings
                .get::<String>("table_header_bg_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::Reset),
        file_tree_selected_fg_color: Color::from_str(
            &settings
                .get::<String>("file_tree_selected_fg_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::LightGreen),
        file_tree_page_count_color: Color::from_str(
            &settings
                .get::<String>("file_tree_page_count_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::LightGreen),
        file_tree_name_color: Color::from_str(
            &settings
                .get::<String>("file_tree_name_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::Blue),
        file_tree_path_color: Color::from_str(
            &settings
                .get::<String>("file_tree_path_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::DarkGray),
        bold_italic_color: Color::from_str(
            &settings
                .get::<String>("bold_italic_color")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::Reset),
        quote_important: Color::from_str(
            &settings
                .get::<String>("quote_important")
                .unwrap_or_default(),
        )
        .unwrap_or(Color::LightRed),
        quote_warning: Color::from_str(
            &settings.get::<String>("quote_warning").unwrap_or_default(),
        )
        .unwrap_or(Color::LightYellow),

        quote_tip: Color::from_str(&settings.get::<String>("quote_tip").unwrap_or_default())
            .unwrap_or(Color::LightGreen),

        quote_note: Color::from_str(&settings.get::<String>("quote_note").unwrap_or_default())
            .unwrap_or(Color::LightBlue),

        quote_caution: Color::from_str(
            &settings.get::<String>("quote_caution").unwrap_or_default(),
        )
        .unwrap_or(Color::LightMagenta),

        quote_default: Color::from_str(
            &settings.get::<String>("quote_default").unwrap_or_default(),
        )
        .unwrap_or(Color::White),
    }
}

static COLOR_CONFIG_INTERNAL: LazyLock<Arc<RwLock<ColorConfig>>> =
    LazyLock::new(|| Arc::new(RwLock::new(read_color_config_from_file())));

pub fn set_color_config(config: ColorConfig) {
    let mut color_config_internal = COLOR_CONFIG_INTERNAL.write().unwrap();
    *color_config_internal = config;
}

#[must_use]
pub fn color_config() -> ColorConfig {
    *COLOR_CONFIG_INTERNAL.read().unwrap()
}

/// Colours for the tree-sitter highlight captures in [`HIGHLIGHT_NAMES`],
/// indexed the same way tree-sitter reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightColors([Color; HIGHLIGHT_NAMES.len()]);

impl std::ops::Index<usize> for HighlightColors {
    type Output = Color;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

/// Each capture is overridable with a `code_hl_`-prefixed key, dots replaced by
/// underscores: `function.builtin` becomes `code_hl_function_builtin`. Missing
/// or unparseable values keep the built-in colour.
#[must_use]
pub fn highlight_colors_from_settings(settings: &Config) -> HighlightColors {
    let mut colors = DEFAULT_COLOR_MAP;

    for (color, name) in colors.iter_mut().zip(HIGHLIGHT_NAMES) {
        let key = format!("code_hl_{}", name.replace('.', "_"));
        if let Ok(value) = settings.get::<String>(&key)
            && let Ok(parsed) = Color::from_str(&value)
        {
            *color = parsed;
        }
    }

    HighlightColors(colors)
}

#[must_use]
pub fn read_highlight_colors_from_file() -> HighlightColors {
    let settings = config_builder()
        .add_source(Environment::with_prefix("MDT").separator("_"))
        .build()
        .unwrap_or_default();

    highlight_colors_from_settings(&settings)
}

static HIGHLIGHT_COLORS_INTERNAL: LazyLock<Arc<RwLock<HighlightColors>>> =
    LazyLock::new(|| Arc::new(RwLock::new(read_highlight_colors_from_file())));

pub fn set_highlight_colors(colors: HighlightColors) {
    let mut highlight_colors_internal = HIGHLIGHT_COLORS_INTERNAL.write().unwrap();
    *highlight_colors_internal = colors;
}

#[must_use]
pub fn highlight_colors() -> HighlightColors {
    *HIGHLIGHT_COLORS_INTERNAL.read().unwrap()
}

#[derive(Clone, Copy)]
pub struct HeadingColors {
    pub level_2: Color,
    pub level_3: Color,
    pub level_4: Color,
    pub level_5: Color,
    pub level_6: Color,
}

#[must_use]
pub fn read_heading_colors_from_file() -> HeadingColors {
    let settings = config_builder().build().unwrap_or_default();

    HeadingColors {
        level_2: settings
            .get::<String>("h2_fg_color")
            .map(|s| Color::from_str(&s).unwrap_or(Color::Green))
            .unwrap_or(Color::Green),
        level_3: settings
            .get_string("h3_fg_color")
            .map(|s| Color::from_str(&s).unwrap_or(Color::Magenta))
            .unwrap_or(Color::Magenta),
        level_4: settings
            .get_string("h4_fg_color")
            .map(|s| Color::from_str(&s).unwrap_or(Color::Cyan))
            .unwrap_or(Color::Cyan),
        level_5: settings
            .get_string("h5_fg_color")
            .map(|s| Color::from_str(&s).unwrap_or(Color::Yellow))
            .unwrap_or(Color::Yellow),
        level_6: settings
            .get_string("h6_fg_color")
            .map(|s| Color::from_str(&s).unwrap_or(Color::LightRed))
            .unwrap_or(Color::LightRed),
    }
}

static HEADING_COLORS_INTERNAL: LazyLock<Arc<RwLock<HeadingColors>>> =
    LazyLock::new(|| Arc::new(RwLock::new(read_heading_colors_from_file())));

pub fn set_heading_colors(config: HeadingColors) {
    let mut heading_colors_internal = HEADING_COLORS_INTERNAL.write().unwrap();
    *heading_colors_internal = config;
}

#[must_use]
pub fn heading_colors() -> HeadingColors {
    *HEADING_COLORS_INTERNAL.read().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::FileFormat;

    fn settings_from(toml: &str) -> Config {
        Config::builder()
            .add_source(File::from_str(toml, FileFormat::Toml))
            .build()
            .unwrap()
    }

    fn index_of(name: &str) -> usize {
        HIGHLIGHT_NAMES
            .iter()
            .position(|n| *n == name)
            .expect("unknown highlight name")
    }

    #[test]
    fn defaults_match_the_built_in_palette() {
        let colors = highlight_colors_from_settings(&settings_from(""));

        for (i, default) in DEFAULT_COLOR_MAP.iter().enumerate() {
            assert_eq!(colors[i], *default);
        }
    }

    #[test]
    fn a_key_overrides_a_single_capture() {
        let colors =
            highlight_colors_from_settings(&settings_from(r##"code_hl_keyword = "#123123""##));

        assert_eq!(colors[index_of("keyword")], Color::Rgb(0x12, 0x31, 0x23));
        assert_eq!(
            colors[index_of("string")],
            DEFAULT_COLOR_MAP[index_of("string")]
        );
    }

    #[test]
    fn dotted_captures_are_keyed_with_underscores() {
        let colors = highlight_colors_from_settings(&settings_from(
            r##"
            code_hl_function_builtin = "green"
            code_hl_punctuation_bracket = "reset"
            code_hl_variable_parameter = "#ABCDEF"
            "##,
        ));

        assert_eq!(colors[index_of("function.builtin")], Color::Green);
        assert_eq!(colors[index_of("punctuation.bracket")], Color::Reset);
        assert_eq!(
            colors[index_of("variable.parameter")],
            Color::Rgb(0xAB, 0xCD, 0xEF)
        );
    }

    #[test]
    fn unparseable_values_fall_back_to_the_default() {
        let colors = highlight_colors_from_settings(&settings_from(
            r#"
            code_hl_keyword = "not-a-color"
            code_hl_string = ""
            "#,
        ));

        assert_eq!(
            colors[index_of("keyword")],
            DEFAULT_COLOR_MAP[index_of("keyword")]
        );
        assert_eq!(
            colors[index_of("string")],
            DEFAULT_COLOR_MAP[index_of("string")]
        );
    }
}
