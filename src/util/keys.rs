use config::{Config, Environment, File};
use crossterm::event::KeyCode;
use lazy_static::lazy_static;

pub enum Action {
    Up,
    Down,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    Search,
    SelectLink,
    SelectLinkAlt,
    SearchNext,
    SearchPrevious,
    Edit,
    Hover,
    Enter,
    Escape,
    ToTop,
    ToBottom,
    Help,
    Back,
    ToFileTree,
    None,
}

#[derive(Debug)]
pub struct KeyConfig {
    pub up: char,
    pub down: char,
    pub page_up: char,
    pub page_down: char,
    pub half_page_up: char,
    pub half_page_down: char,
    pub search: char,
    pub search_next: char,
    pub search_previous: char,
    pub select_link: char,
    pub select_link_alt: char,
    pub edit: char,
    pub hover: char,
    pub top: char,
    pub bottom: char,
    pub back: char,
    pub file_tree: char,
}

pub fn key_to_action(key: KeyCode) -> Action {
    match key {
        KeyCode::Char(c) => {
            if c == KEY_CONFIG.up {
                return Action::Up;
            }

            if c == KEY_CONFIG.down {
                return Action::Down;
            }

            if c == KEY_CONFIG.page_up {
                return Action::PageUp;
            }

            if c == KEY_CONFIG.page_down {
                return Action::PageDown;
            }

            if c == KEY_CONFIG.half_page_up {
                return Action::HalfPageUp;
            }

            if c == KEY_CONFIG.half_page_down {
                return Action::HalfPageDown;
            }

            if c == KEY_CONFIG.search || c == '/' {
                return Action::Search;
            }

            if c == KEY_CONFIG.select_link {
                return Action::SelectLink;
            }

            if c == KEY_CONFIG.select_link_alt {
                return Action::SelectLinkAlt;
            }

            if c == KEY_CONFIG.search_next {
                return Action::SearchNext;
            }

            if c == KEY_CONFIG.search_previous {
                return Action::SearchPrevious;
            }

            if c == KEY_CONFIG.edit {
                return Action::Edit;
            }

            if c == KEY_CONFIG.hover {
                return Action::Hover;
            }

            if c == KEY_CONFIG.top {
                return Action::ToTop;
            }

            if c == KEY_CONFIG.bottom {
                return Action::ToBottom;
            }

            if c == KEY_CONFIG.back {
                return Action::Back;
            }

            if c == KEY_CONFIG.file_tree {
                return Action::ToFileTree;
            }

            if c == '?' {
                return Action::Help;
            }

            Action::None
        }
        KeyCode::Up => Action::Up,
        KeyCode::Down => Action::Down,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Right => Action::PageDown,
        KeyCode::Left => Action::PageUp,
        KeyCode::Enter => Action::Enter,
        KeyCode::Esc => Action::Escape,
        _ => Action::None,
    }
}

lazy_static! {
    pub static ref KEY_CONFIG: KeyConfig = {
        let config_dir = dirs::home_dir().unwrap();
        let config_file = config_dir.join(".config").join("mdt").join("config.toml");
        let settings = Config::builder()
            .add_source(File::with_name(config_file.to_str().unwrap()).required(false))
            .add_source(Environment::with_prefix("MDT").separator("_"))
            .build()
            .unwrap();

        KeyConfig {
            up: settings.get::<char>("up").unwrap_or('k'),
            down: settings.get::<char>("down").unwrap_or('j'),
            page_up: settings.get::<char>("page_up").unwrap_or('u'),
            page_down: settings.get::<char>("page_down").unwrap_or('d'),
            half_page_up: settings.get::<char>("half_page_up").unwrap_or('h'),
            half_page_down: settings.get::<char>("half_page_down").unwrap_or('l'),
            search: settings.get::<char>("search").unwrap_or('f'),
            select_link: settings.get::<char>("select_link").unwrap_or('s'),
            select_link_alt: settings.get::<char>("select_link_alt").unwrap_or('S'),
            search_next: settings.get::<char>("search_next").unwrap_or('n'),
            search_previous: settings.get::<char>("search_previous").unwrap_or('N'),
            edit: settings.get::<char>("edit").unwrap_or('e'),
            hover: settings.get::<char>("hover").unwrap_or('K'),
            top: settings.get::<char>("top").unwrap_or('g'),
            bottom: settings.get::<char>("bottom").unwrap_or('G'),
            back: settings.get::<char>("back").unwrap_or('b'),
            file_tree: settings.get::<char>("file_tree").unwrap_or('t'),
        }
    };
}
