use crate::models::Settings;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SettingsStore {
    path: PathBuf,
    pub settings: Settings,
}

impl SettingsStore {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let settings = if path.exists() {
            let raw = fs::read_to_string(path)?;
            serde_json::from_str::<Settings>(&raw)?
        } else {
            Settings::default()
        };
        let mut s = Self {
            path: path.to_path_buf(),
            settings,
        };
        s.clamp();
        Ok(s)
    }

    fn clamp(&mut self) {
        self.settings.history_limit = self.settings.history_limit.clamp(1, 100);
        if self.settings.shortcut.trim().is_empty() {
            self.settings.shortcut = "Command+Shift+V".into();
        }
        self.settings.opacity = self.settings.opacity.clamp(0.3, 1.0);
        self.settings.window_width = self.settings.window_width.clamp(220, 800);
        self.settings.window_height = self.settings.window_height.clamp(200, 1200);
        self.settings.image_preview_height = self.settings.image_preview_height.clamp(24, 200);
        if !matches!(self.settings.window_mode.as_str(), "auto" | "custom") {
            self.settings.window_mode = "auto".into();
        }
        if !matches!(self.settings.search_mode.as_str(), "fuzzy" | "exact") {
            self.settings.search_mode = "fuzzy".into();
        }
        if !matches!(self.settings.position_mode.as_str(), "cursor" | "center") {
            self.settings.position_mode = "cursor".into();
        }
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.clamp();
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&self.settings)?)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_settings_path() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "clipboardx-settings-{}-{}",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn defaults_when_no_file() {
        let dir = tmp_settings_path();
        std::fs::create_dir_all(&dir).unwrap();
        let s = SettingsStore::load(&dir.join("settings.json")).unwrap();
        assert_eq!(s.settings.history_limit, 15);
        assert_eq!(s.settings.shortcut, "Command+Shift+V");
    }

    #[test]
    fn save_then_reload_roundtrip() {
        let dir = tmp_settings_path();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let mut s = SettingsStore::load(&path).unwrap();
        s.settings.history_limit = 30;
        s.settings.shortcut = "Command+Shift+1".into();
        s.save().unwrap();
        let loaded = SettingsStore::load(&path).unwrap();
        assert_eq!(loaded.settings.history_limit, 30);
        assert_eq!(loaded.settings.shortcut, "Command+Shift+1");
    }

    #[test]
    fn limit_is_clamped_to_100() {
        let dir = tmp_settings_path();
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = SettingsStore::load(&dir.join("settings.json")).unwrap();
        s.settings.history_limit = 9999;
        s.save().unwrap();
        assert_eq!(s.settings.history_limit, 100);
    }
}
