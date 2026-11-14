use std::sync::LazyLock;

use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug)]
pub struct GeneralConfig {
    pub width: u16,
    pub gitignore: bool,
    pub centering: Centering,
    pub help_menu: bool,
    pub document_header: bool,
    pub file_tree_directory_header: bool,
    pub document_header_full_path: bool,
    pub heading_style: HeadingStyle,
    pub file_tree_details: bool,
    pub scrollbar: bool,
    pub image_max_height: u16,
    pub remember_position: bool,
    pub position_cache_ttl_minutes: u64,
    pub escape_quits: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Centering {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeadingStyle {
    Hashes,
    Bars,
}

pub static GENERAL_CONFIG: LazyLock<GeneralConfig> = LazyLock::new(|| {
    let config_dir = dirs::home_dir().unwrap();
    let config_file = config_dir.join(".config").join("mdt").join("config.toml");
    let settings = Config::builder()
        .add_source(File::with_name(config_file.to_str().unwrap()).required(false))
        .add_source(Environment::with_prefix("MDT").separator("_"))
        .build()
        .unwrap_or_default();

    let width = settings.get::<u16>("width").unwrap_or(100);
    GeneralConfig {
        // width = 0 means "use full terminal width"
        width: if width == 0 { u16::MAX } else { width },
        gitignore: settings.get::<bool>("gitignore").unwrap_or(false),
        centering: settings
            .get::<Centering>("alignment")
            .unwrap_or(Centering::Left),
        help_menu: settings.get::<bool>("help_menu").unwrap_or(true),
        document_header: shows_document_header(&settings),
        file_tree_directory_header: settings
            .get::<bool>("file_tree_directory_header")
            .unwrap_or(false),
        document_header_full_path: settings
            .get::<bool>("document_header_full_path")
            .unwrap_or(false),
        heading_style: settings
            .get::<HeadingStyle>("heading_style")
            .unwrap_or(HeadingStyle::Hashes),
        file_tree_details: settings.get::<bool>("file_tree_details").unwrap_or(false),
        scrollbar: settings.get::<bool>("scrollbar").unwrap_or(true),
        image_max_height: settings.get::<u16>("image_max_height").unwrap_or(20).max(1),
        remember_position: settings.get::<bool>("remember_position").unwrap_or(true),
        position_cache_ttl_minutes: settings
            .get::<u64>("position_cache_ttl_minutes")
            .unwrap_or(60),
        // The "_" env separator delivers MDT_ESCAPE_QUITS as `escape.quits`.
        escape_quits: settings
            .get::<bool>("escape.quits")
            .or_else(|_| settings.get::<bool>("escape_quits"))
            .unwrap_or(false),
    }
});

/// The full path option is the more specific of the two, so it shows the header by itself.
fn shows_document_header(settings: &Config) -> bool {
    settings.get::<bool>("document_header").unwrap_or(false)
        || settings
            .get::<bool>("document_header_full_path")
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shows_header_with(document_header: bool, full_path: bool) -> bool {
        let settings = Config::builder()
            .set_override("document_header", document_header)
            .and_then(|builder| builder.set_override("document_header_full_path", full_path))
            .and_then(|builder| builder.build())
            .unwrap();
        shows_document_header(&settings)
    }

    #[test]
    fn document_header_is_off_by_default() {
        assert!(!shows_document_header(&Config::default()));
    }

    #[test]
    fn either_option_shows_the_document_header() {
        assert!(!shows_header_with(false, false));
        assert!(shows_header_with(true, false));
        assert!(shows_header_with(false, true));
        assert!(shows_header_with(true, true));
    }
}
