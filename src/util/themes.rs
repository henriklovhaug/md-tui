//! Small, self-contained color presets. The user's config.toml is never rewritten.
use std::{fs, io::Write, path::PathBuf};

use crossterm::event::KeyCode;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::colors::{
    ColorConfig, HeadingColors, HighlightColors, color_config, heading_colors, highlight_colors,
    read_color_config_from_file, read_heading_colors_from_file, read_highlight_colors_from_file,
    set_color_config, set_heading_colors, set_highlight_colors,
};

const BUILTINS: [&str; 6] = [
    "Custom",
    "Match terminal",
    "Light",
    "Dark",
    "Warm light",
    "High contrast",
];

fn config_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".config/mdt")
}

fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

fn selected_path() -> PathBuf {
    config_dir().join("selected-theme")
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThemePalette {
    pub colors: ColorConfig,
    pub headings: HeadingColors,
    pub syntax: HighlightColors,
}

impl ThemePalette {
    fn from_config() -> Self {
        Self {
            colors: read_color_config_from_file(),
            headings: read_heading_colors_from_file(),
            syntax: read_highlight_colors_from_file(),
        }
    }

    fn current() -> Self {
        Self {
            colors: color_config(),
            headings: heading_colors(),
            syntax: highlight_colors(),
        }
    }

    fn apply(self) {
        set_color_config(self.colors);
        set_heading_colors(self.headings);
        set_highlight_colors(self.syntax);
    }

    fn terminal() -> Self {
        let r = Color::Reset;
        Self {
            colors: ColorConfig {
                italic_color: r,
                bold_color: r,
                striketrough_color: r,
                bold_italic_color: r,
                code_fg_color: r,
                code_bg_color: r,
                link_color: r,
                link_selected_fg_color: r,
                link_selected_bg_color: r,
                scrollbar_color: r,
                code_block_bg_color: r,
                heading_fg_color: r,
                heading_bg_color: r,
                table_header_fg_color: r,
                table_header_bg_color: r,
                quote_bg_color: r,
                file_tree_selected_fg_color: r,
                file_tree_page_count_color: r,
                file_tree_name_color: r,
                file_tree_path_color: r,
                quote_important: r,
                quote_warning: r,
                quote_tip: r,
                quote_note: r,
                quote_caution: r,
                quote_default: r,
                help_bg_color: r,
                help_fg_color: r,
                help_title_color: r,
            },
            headings: HeadingColors {
                level_2: r,
                level_3: r,
                level_4: r,
                level_5: r,
                level_6: r,
            },
            syntax: HighlightColors::reset(),
        }
    }

    fn builtin(name: &str) -> Option<Self> {
        let mut theme = Self::terminal();
        let c = &mut theme.colors;
        let h = &mut theme.headings;
        match name {
            "Match terminal" => {}
            "Light" | "Warm light" => {
                theme.syntax = HighlightColors::light();
                let warm = name == "Warm light";
                let accent = if warm {
                    Color::Rgb(115, 66, 30)
                } else {
                    Color::Blue
                };
                c.heading_fg_color = accent;
                c.link_color = accent;
                c.code_fg_color = if warm {
                    Color::Rgb(118, 52, 25)
                } else {
                    Color::Red
                };
                c.code_bg_color = if warm {
                    Color::Rgb(247, 239, 221)
                } else {
                    Color::Rgb(239, 241, 245)
                };
                c.code_block_bg_color = c.code_bg_color;
                c.link_selected_bg_color = accent;
                c.link_selected_fg_color = Color::White;
                c.scrollbar_color = accent;
                c.file_tree_name_color = accent;
                c.file_tree_path_color = Color::DarkGray;
                c.file_tree_selected_fg_color = accent;
                c.table_header_fg_color = accent;
                c.help_fg_color = Color::DarkGray;
                c.help_title_color = accent;
                h.level_2 = accent;
                h.level_3 = if warm {
                    Color::Rgb(140, 82, 32)
                } else {
                    Color::Magenta
                };
                h.level_4 = accent;
                h.level_5 = Color::DarkGray;
                h.level_6 = accent;
            }
            "Dark" | "High contrast" => {
                theme.syntax = HighlightColors::dark();
                let contrast = name == "High contrast";
                let accent = if contrast { Color::Yellow } else { Color::Cyan };
                c.heading_fg_color = accent;
                c.link_color = accent;
                c.code_fg_color = if contrast {
                    Color::White
                } else {
                    Color::LightRed
                };
                c.code_bg_color = Color::Rgb(40, 40, 40);
                c.code_block_bg_color = c.code_bg_color;
                c.link_selected_bg_color = if contrast { Color::Yellow } else { Color::Blue };
                c.link_selected_fg_color = if contrast { Color::Black } else { Color::White };
                c.scrollbar_color = accent;
                c.file_tree_name_color = accent;
                c.file_tree_path_color = Color::Gray;
                c.file_tree_selected_fg_color = accent;
                c.table_header_fg_color = accent;
                c.help_fg_color = Color::Gray;
                c.help_title_color = accent;
                h.level_2 = accent;
                h.level_3 = if contrast {
                    Color::LightCyan
                } else {
                    Color::Magenta
                };
                h.level_4 = Color::LightGreen;
                h.level_5 = Color::LightYellow;
                h.level_6 = Color::LightRed;
            }
            _ => return None,
        }
        Some(theme)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Browse,
    SaveName,
    ConfirmDelete,
}

#[derive(Debug, Clone)]
pub struct ThemeChooser {
    pub open: bool,
    pub mode: ThemeMode,
    pub input: String,
    pub status: String,
    pub selected: usize,
    pub active: String,
    pub saved: Vec<String>,
}

impl Default for ThemeChooser {
    fn default() -> Self {
        let mut chooser = Self {
            open: false,
            mode: ThemeMode::Browse,
            input: String::new(),
            status: String::new(),
            selected: 0,
            active: "Custom".into(),
            saved: saved_names(),
        };
        if let Ok(name) = fs::read_to_string(selected_path()) {
            let name = name.trim();
            if let Ok(palette) = chooser.palette(name) {
                palette.apply();
                chooser.active = name.to_string();
                chooser.selected = chooser
                    .names()
                    .iter()
                    .position(|item| item == name)
                    .unwrap_or(0);
            }
        }
        chooser
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !BUILTINS
            .iter()
            .any(|builtin| builtin.eq_ignore_ascii_case(name))
}

fn saved_names() -> Vec<String> {
    let Ok(entries) = fs::read_dir(themes_dir()) else {
        return Vec::new();
    };
    let mut names: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_stem()?.to_str()?;
            (path.extension()?.to_str()? == "json" && valid_name(name)).then(|| name.to_string())
        })
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names
}

impl ThemeChooser {
    pub fn names(&self) -> Vec<String> {
        BUILTINS
            .iter()
            .map(|name| (*name).to_string())
            .chain(self.saved.iter().cloned())
            .collect()
    }

    fn palette(&self, name: &str) -> Result<ThemePalette, String> {
        if name == "Custom" {
            return Ok(ThemePalette::from_config());
        }
        if let Some(palette) = ThemePalette::builtin(name) {
            return Ok(palette);
        }
        if !valid_name(name) || !self.saved.iter().any(|saved| saved == name) {
            return Err("Unknown theme".into());
        }
        let path = themes_dir().join(format!("{name}.json"));
        let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        serde_json::from_str(&text).map_err(|err| format!("Invalid theme: {err}"))
    }

    fn activate(&mut self, name: &str) -> Result<bool, String> {
        let palette = self.palette(name)?;
        fs::create_dir_all(config_dir()).map_err(|err| err.to_string())?;
        fs::write(selected_path(), format!("{name}\n")).map_err(|err| err.to_string())?;
        palette.apply();
        self.active = name.to_string();
        Ok(true)
    }

    fn save(&mut self) -> Result<(), String> {
        let name = self.input.trim();
        if !valid_name(name) {
            return Err("Use 1–32 letters, digits, _ or -; no built-in names".into());
        }
        if self
            .saved
            .iter()
            .any(|saved| saved.eq_ignore_ascii_case(name))
        {
            return Err("Theme already exists; choose another name".into());
        }
        fs::create_dir_all(themes_dir()).map_err(|err| err.to_string())?;
        let path = themes_dir().join(format!("{name}.json"));
        let text = serde_json::to_string_pretty(&ThemePalette::current())
            .map_err(|err| err.to_string())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|err| err.to_string())?;
        file.write_all(text.as_bytes())
            .map_err(|err| err.to_string())?;
        let name = name.to_string();
        self.saved = saved_names();
        self.selected = self
            .names()
            .iter()
            .position(|item| item == &name)
            .unwrap_or(0);
        self.activate(&name)?;
        self.input.clear();
        self.mode = ThemeMode::Browse;
        self.status = format!("Saved {name}");
        Ok(())
    }

    fn delete_selected(&mut self) -> Result<(), String> {
        let name = self
            .names()
            .get(self.selected)
            .cloned()
            .ok_or("No theme selected")?;
        if !self.saved.iter().any(|saved| saved == &name) {
            return Err("Built-in themes cannot be deleted".into());
        }
        if self.active == name {
            return Err("Switch themes before deleting this one".into());
        }
        fs::remove_file(themes_dir().join(format!("{name}.json")))
            .map_err(|err| err.to_string())?;
        self.saved = saved_names();
        self.selected = self.selected.min(self.names().len().saturating_sub(1));
        self.mode = ThemeMode::Browse;
        self.status = format!("Deleted {name}");
        Ok(())
    }

    /// Returns true when the displayed document must be re-parsed for syntax colors.
    pub fn key(&mut self, key: KeyCode) -> bool {
        self.status.clear();
        match self.mode {
            ThemeMode::Browse => match key {
                KeyCode::Esc | KeyCode::Char('q') => self.open = false,
                KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected = (self.selected + 1).min(self.names().len() - 1)
                }
                KeyCode::Char('s') => {
                    self.input.clear();
                    self.mode = ThemeMode::SaveName;
                }
                KeyCode::Char('x') => {
                    if self.selected < BUILTINS.len() {
                        self.status = "Built-in themes cannot be deleted".into();
                    } else {
                        self.mode = ThemeMode::ConfirmDelete;
                    }
                }
                KeyCode::Enter => {
                    if let Some(name) = self.names().get(self.selected).cloned() {
                        match self.activate(&name) {
                            Ok(changed) => {
                                self.open = false;
                                return changed;
                            }
                            Err(err) => self.status = err,
                        }
                    }
                }
                _ => {}
            },
            ThemeMode::SaveName => match key {
                KeyCode::Esc => self.mode = ThemeMode::Browse,
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Enter => {
                    if let Err(err) = self.save() {
                        self.status = err;
                    }
                }
                KeyCode::Char(c)
                    if (c.is_ascii_alphanumeric() || c == '-' || c == '_')
                        && self.input.len() < 32 =>
                {
                    self.input.push(c);
                }
                _ => {}
            },
            ThemeMode::ConfirmDelete => match key {
                KeyCode::Char('y') => {
                    if let Err(err) = self.delete_selected() {
                        self.status = err;
                        self.mode = ThemeMode::Browse;
                    }
                }
                _ => self.mode = ThemeMode::Browse,
            },
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_visible_selections() {
        for name in BUILTINS.iter().skip(1) {
            let palette = ThemePalette::builtin(name).unwrap();
            assert!(
                palette.colors.link_selected_bg_color != palette.colors.link_selected_fg_color
                    || palette.colors.link_selected_bg_color == Color::Reset
            );
        }
    }

    #[test]
    fn palette_round_trips_as_json() {
        let palette = ThemePalette::builtin("Dark").unwrap();
        let text = serde_json::to_string_pretty(&palette).unwrap();
        let restored: ThemePalette = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.colors.link_color, palette.colors.link_color);
    }

    #[test]
    fn names_cannot_escape_theme_directory() {
        assert!(!valid_name("../outside"));
        assert!(!valid_name("light"));
        assert!(valid_name("my_dark"));
    }
}
