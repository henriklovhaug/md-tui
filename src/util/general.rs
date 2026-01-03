use std::sync::LazyLock;

use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug)]
pub struct GeneralConfig {
    pub width: u16,
    pub gitignore: bool,
    pub centering: Centering,
    pub help_menu: bool,
}

#[derive(Debug, Deserialize)]
pub enum Centering {
    Left,
    Center,
    Right,
}

pub static GENERAL_CONFIG: LazyLock<GeneralConfig> = LazyLock::new(|| {
    let config_dir = dirs::home_dir().unwrap();
    let config_file = config_dir.join(".config").join("mdt").join("config.toml");
    let settings = Config::builder()
        .add_source(File::with_name(config_file.to_str().unwrap()).required(false))
        .add_source(Environment::with_prefix("MDT").separator("_"))
        .build()
        .unwrap();

    let width = settings.get::<u16>("width").unwrap_or(100);
    GeneralConfig {
        // width = 0 means "use full terminal width"
        width: if width == 0 { u16::MAX } else { width },
        gitignore: settings.get::<bool>("gitignore").unwrap_or(false),
        centering: settings
            .get::<Centering>("alignment")
            .unwrap_or(Centering::Left),
        help_menu: settings.get::<bool>("help_menu").unwrap_or(true),
    }
});
