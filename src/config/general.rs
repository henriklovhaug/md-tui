use config::{Config, Environment, File};
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Debug)]
pub struct GeneralConfig {
    pub width: u16,
    pub gitignore: bool,
    pub centering: Centering,
}

#[derive(Debug, Deserialize)]
pub enum Centering {
    Left,
    Center,
    Right,
}

lazy_static! {
    pub static ref GENERAL_CONFIG: GeneralConfig = {
        let config_dir = dirs::home_dir().unwrap();
        let config_file = config_dir.join(".config").join("mdt").join("config.toml");
        let settings = Config::builder()
            .add_source(File::with_name(config_file.to_str().unwrap()).required(false))
            .add_source(Environment::with_prefix("MDT").separator("_"))
            .build()
            .unwrap();

        GeneralConfig {
            width: settings.get::<u16>("width").unwrap_or(100),
            gitignore: settings.get::<bool>("gitignore").unwrap_or(false),
            centering: settings
                .get::<Centering>("alignment")
                .unwrap_or(Centering::Left),
        }
    };
}
