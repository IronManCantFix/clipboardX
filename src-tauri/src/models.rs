use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    Text,
    Rtf,
    Image,
    File,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: EntryType,
    pub content: String,
    pub rtf_data: Option<Vec<u8>>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub file_paths: Vec<String>,
    pub source_app: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

fn default_opacity() -> f64 {
    0.9
}
fn default_width_mode() -> String {
    "auto".into()
}
fn default_height_mode() -> String {
    "auto".into()
}
fn default_position_mode() -> String {
    "cursor".into()
}
fn default_search_mode() -> String {
    "fuzzy".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub history_limit: i64,
    pub shortcut: String,
    /// 弹窗背景透明度 0.3..=1.0
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    /// 宽度模式："auto"（默认宽度）| "custom"（指定宽度）
    #[serde(default = "default_width_mode")]
    pub width_mode: String,
    /// 高度模式："auto"（按条数自适应）| "custom"（指定高度）
    #[serde(default = "default_height_mode")]
    pub height_mode: String,
    #[serde(default = "default_window_width")]
    pub window_width: i64,
    #[serde(default = "default_window_height")]
    pub window_height: i64,
    /// 是否启用搜索框
    #[serde(default = "default_true")]
    pub search_enabled: bool,
    /// 搜索模式："fuzzy"（模糊）| "exact"（精准子串）
    #[serde(default = "default_search_mode")]
    pub search_mode: String,
    /// 弹窗位置："cursor"（跟随鼠标）| "center"（屏幕居中）
    #[serde(default = "default_position_mode")]
    pub position_mode: String,
    /// 图片条目缩略图高度（px）
    #[serde(default = "default_image_preview_height")]
    pub image_preview_height: i64,
}

fn default_window_width() -> i64 {
    360
}
fn default_window_height() -> i64 {
    480
}
fn default_image_preview_height() -> i64 {
    60
}
fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            history_limit: 15,
            shortcut: "Command+Shift+V".into(),
            opacity: 0.9,
            width_mode: "auto".into(),
            height_mode: "auto".into(),
            window_width: 360,
            window_height: 480,
            search_enabled: true,
            search_mode: "fuzzy".into(),
            position_mode: "cursor".into(),
            image_preview_height: 60,
        }
    }
}
