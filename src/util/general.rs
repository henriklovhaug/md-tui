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
    pub heading_style: HeadingStyle,
    pub scrollbar: bool,
    pub image_max_height: u16,
    pub remember_position: bool,
    pub position_cache_ttl_minutes: u64,
    #[cfg(feature = "mermaid")]
    pub mermaid: bool,
    #[cfg(feature = "mermaid")]
    pub mermaid_style: crate::mermaid::MermaidStyle,
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
    GeneralConfig::from_settings(&settings)
});

impl GeneralConfig {
    fn from_settings(settings: &Config) -> Self {
        let width = settings.get::<u16>("width").unwrap_or(100);
        GeneralConfig {
            // width = 0 means "use full terminal width"
            width: if width == 0 { u16::MAX } else { width },
            gitignore: settings.get::<bool>("gitignore").unwrap_or(false),
            centering: settings
                .get::<Centering>("alignment")
                .unwrap_or(Centering::Left),
            help_menu: settings.get::<bool>("help_menu").unwrap_or(true),
            document_header: settings.get::<bool>("document_header").unwrap_or(false),
            file_tree_directory_header: settings
                .get::<bool>("file_tree_directory_header")
                .unwrap_or(false),
            heading_style: settings
                .get::<HeadingStyle>("heading_style")
                .unwrap_or(HeadingStyle::Hashes),
            scrollbar: settings.get::<bool>("scrollbar").unwrap_or(true),
            image_max_height: settings.get::<u16>("image_max_height").unwrap_or(20).max(1),
            remember_position: settings.get::<bool>("remember_position").unwrap_or(true),
            position_cache_ttl_minutes: settings
                .get::<u64>("position_cache_ttl_minutes")
                .unwrap_or(60),
            #[cfg(feature = "mermaid")]
            mermaid: settings.get::<bool>("mermaid").unwrap_or(true),
            #[cfg(feature = "mermaid")]
            mermaid_style: mermaid_style(settings),
        }
    }
}

#[cfg(feature = "mermaid")]
fn mermaid_style(settings: &Config) -> crate::mermaid::MermaidStyle {
    // An empty colour keeps the theme's own colour.
    let colour = |key: &str| {
        settings
            .get::<String>(key)
            .ok()
            .filter(|value| !value.is_empty())
    };
    let defaults = crate::mermaid::MermaidStyle::default();
    crate::mermaid::MermaidStyle {
        theme: settings
            .get::<String>("mermaid_theme")
            .unwrap_or(defaults.theme),
        background_color: colour("mermaid_background_color"),
        node_color: colour("mermaid_node_color"),
        border_color: colour("mermaid_border_color"),
        text_color: colour("mermaid_text_color"),
        line_color: colour("mermaid_line_color"),
    }
}

#[cfg(all(test, feature = "mermaid"))]
mod tests {
    use config::{Config, File, FileFormat};

    use super::GeneralConfig;

    fn config_from(toml: &str) -> GeneralConfig {
        let settings = Config::builder()
            .add_source(File::from_str(toml, FileFormat::Toml))
            .build()
            .unwrap();
        GeneralConfig::from_settings(&settings)
    }

    #[test]
    fn mermaid_is_on_by_default_with_the_modern_theme() {
        let config = config_from("");

        assert!(config.mermaid);
        assert_eq!(
            config.mermaid_style,
            crate::mermaid::MermaidStyle::default()
        );
    }

    #[test]
    fn mermaid_settings_are_read_and_empty_colours_keep_the_theme() {
        let config = config_from(
            r##"
            mermaid = false
            mermaid_theme = "dark"
            mermaid_background_color = "#1e1e2e"
            mermaid_text_color = ""
            "##,
        );

        assert!(!config.mermaid);
        assert_eq!(config.mermaid_style.theme, "dark");
        assert_eq!(
            config.mermaid_style.background_color.as_deref(),
            Some("#1e1e2e")
        );
        assert_eq!(config.mermaid_style.text_color, None);
    }
}
