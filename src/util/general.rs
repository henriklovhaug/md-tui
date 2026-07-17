use std::sync::LazyLock;

use serde::Deserialize;

#[derive(Debug)]
pub struct GeneralConfig {
    pub width: u16,
    pub gitignore: bool,
    pub centering: Centering,
    pub help_menu: bool,
    pub footer: bool,
}

#[derive(Debug, Deserialize)]
pub enum Centering {
    Left,
    Center,
    Right,
}

pub static GENERAL_CONFIG: LazyLock<GeneralConfig> = LazyLock::new(|| {
    let settings = super::load_user_config();

    let width = settings.get::<u16>("width").unwrap_or(100);
    GeneralConfig {
        // width = 0 means "use full terminal width"
        width: if width == 0 { u16::MAX } else { width },
        gitignore: settings.get::<bool>("gitignore").unwrap_or(false),
        centering: settings
            .get::<Centering>("alignment")
            .unwrap_or(Centering::Left),
        help_menu: settings.get::<bool>("help_menu").unwrap_or(true),
        footer: settings.get::<bool>("footer").unwrap_or(true),
    }
});
