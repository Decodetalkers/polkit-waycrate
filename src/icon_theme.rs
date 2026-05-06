use std::path::PathBuf;

use iced::widget::svg;

const DEFAULT_SVG: &[u8] = include_bytes!("../misc/data-warning.svg");

static THEMES_LIST: &[&str] = &["breeze", "Adwaita"];

fn get_icon_path_from_xdgicon(icon_name: &str) -> Option<PathBuf> {
    for theme in THEMES_LIST {
        let iconpath = xdg::BaseDirectories::with_prefix(format!("icons/{theme}/state/24"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{icon_name}.svg")) {
            return Some(iconpath);
        }
        let iconpath = xdg::BaseDirectories::with_prefix(format!("icons/{theme}/state/32"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{icon_name}.svg")) {
            return Some(iconpath);
        }
        let iconpath = xdg::BaseDirectories::with_prefix(format!("icons/{theme}/state/64"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{icon_name}.svg")) {
            return Some(iconpath);
        }
    }
    None
}

pub trait IconTheme {
    fn from_icon(icon: &str) -> Self;
    fn default_icon() -> Self;
}

impl IconTheme for svg::Handle {
    fn from_icon(icon: &str) -> Self {
        match get_icon_path_from_xdgicon(icon) {
            Some(icon) => Self::from_path(icon),
            None => Self::from_memory(DEFAULT_SVG),
        }
    }
    fn default_icon() -> Self {
        Self::from_memory(DEFAULT_SVG)
    }
}
