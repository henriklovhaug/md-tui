use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

use config::{Config, Environment, File};
use lazy_static::lazy_static;
use ratatui::style::Color;

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

pub fn read_color_config_from_file() -> ColorConfig {
    let config_dir = dirs::home_dir().unwrap();
    let config_file = config_dir.join(".config").join("mdt").join("config.toml");
    let settings = Config::builder()
        .add_source(File::with_name(config_file.to_str().unwrap()).required(false))
        .add_source(Environment::with_prefix("MDT").separator("_"))
        .build()
        .unwrap();

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

lazy_static! {
    static ref COLOR_CONFIG_INTERNAL: Arc<RwLock<ColorConfig>> =
        RwLock::new(read_color_config_from_file()).into();
}

pub fn set_color_config(config: ColorConfig) {
    let mut color_config_internal = COLOR_CONFIG_INTERNAL.write().unwrap();
    *color_config_internal = config;
}

#[must_use]
pub fn color_config() -> ColorConfig {
    *COLOR_CONFIG_INTERNAL.read().unwrap()
}

#[derive(Clone, Copy)]
pub struct HeadingColors {
    pub level_2: Color,
    pub level_3: Color,
    pub level_4: Color,
    pub level_5: Color,
    pub level_6: Color,
}

pub fn read_heading_colors_from_file() -> HeadingColors {
    let config_dir = dirs::home_dir().unwrap();
    let config_file = config_dir.join(".config").join("mdt").join("config.toml");
    let settings = Config::builder()
        .add_source(File::with_name(config_file.to_str().unwrap()).required(false))
        .build()
        .unwrap();

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

lazy_static! {
    static ref HEADING_COLORS_INTERNAL: Arc<RwLock<HeadingColors>> =
        RwLock::new(read_heading_colors_from_file()).into();
}

pub fn set_heading_colors(config: HeadingColors) {
    let mut heading_colors_internal = HEADING_COLORS_INTERNAL.write().unwrap();
    *heading_colors_internal = config;
}

#[must_use]
pub fn heading_colors() -> HeadingColors {
    *HEADING_COLORS_INTERNAL.read().unwrap()
}
