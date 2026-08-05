use crate::models::Settings;
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
            // 兼容旧版配置：将旧的 window_mode 迁移到 width_mode / height_mode
            let mut value: serde_json::Value = serde_json::from_str(&raw)?;
            if let Some(obj) = value.as_object_mut() {
                if !obj.contains_key("width_mode") || !obj.contains_key("height_mode") {
                    if let Some(mode) = obj.get("window_mode").and_then(|v| v.as_str()) {
                        let m = if mode == "custom" { "custom" } else { "auto" };
                        obj.entry("width_mode")
                            .or_insert_with(|| serde_json::Value::String(m.into()));
                        obj.entry("height_mode")
                            .or_insert_with(|| serde_json::Value::String(m.into()));
                    }
                }
            }
            serde_json::from_value::<Settings>(value)?
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
        if !matches!(self.settings.width_mode.as_str(), "auto" | "custom") {
            self.settings.width_mode = "auto".into();
        }
        if !matches!(self.settings.height_mode.as_str(), "auto" | "custom") {
            self.settings.height_mode = "auto".into();
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
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_settings_path() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "clipboardx-settings-{}-{}-{}",
            std::process::id(),
            nanos,
            n
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

    #[test]
    fn legacy_window_mode_migrates_to_width_and_height() {
        let dir = tmp_settings_path();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(
            &path,
            r#"{"history_limit":15,"shortcut":"Command+Shift+V","window_mode":"custom","window_width":500,"window_height":700}"#,
        )
        .unwrap();
        let s = SettingsStore::load(&path).unwrap();
        assert_eq!(s.settings.width_mode, "custom");
        assert_eq!(s.settings.height_mode, "custom");
        assert_eq!(s.settings.window_width, 500);
        assert_eq!(s.settings.window_height, 700);
    }
}
