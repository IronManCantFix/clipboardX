use serde::{Deserialize, Serialize};

/// 当前 Unix 毫秒时间戳。条目排序/去重统一使用毫秒，避免同一秒内多条记录顺序不稳定。
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    Text,
    Rtf,
    Image,
    File,
    Url,
}

impl EntryType {
    /// 从数据库存储的 type 字符串解析。
    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "text" => Self::Text,
            "rtf" => Self::Rtf,
            "image" => Self::Image,
            "file" => Self::File,
            "url" => Self::Url,
            _ => return None,
        })
    }
}

/// 窗口尺寸模式："auto"（默认/自适应）| "custom"（用户指定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SizeMode {
    #[default]
    Auto,
    Custom,
}

/// 搜索模式：模糊（按字符顺序）或精准子串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Fuzzy,
    Exact,
}

/// 弹窗出现位置：跟随鼠标或屏幕居中。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PositionMode {
    #[default]
    Cursor,
    Center,
}

// ---- 宽松反序列化：旧配置文件里这些字段是裸字符串，遇到非法值时回退默认，
//      而不是让整个 settings.json 解析失败导致启动报错 ----

fn lenient_size_mode<'de, D>(d: D) -> Result<SizeMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(d)?;
    Ok(match raw.as_str() {
        "custom" => SizeMode::Custom,
        _ => SizeMode::Auto,
    })
}

fn lenient_search_mode<'de, D>(d: D) -> Result<SearchMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(d)?;
    Ok(match raw.as_str() {
        "exact" => SearchMode::Exact,
        _ => SearchMode::Fuzzy,
    })
}

fn lenient_position_mode<'de, D>(d: D) -> Result<PositionMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(d)?;
    Ok(match raw.as_str() {
        "center" => PositionMode::Center,
        _ => PositionMode::Cursor,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: EntryType,
    pub content: String,
    /// 内容 hash：图片为 TIFF/PNG 字节哈希、RTF 为 RTF 数据哈希；其余类型 None
    #[serde(default)]
    pub content_hash: Option<i64>,
    pub rtf_data: Option<Vec<u8>>,
    /// 图片原图路径（相对 data_dir 存储，如 "images/{hash}.png"；旧版数据可能为绝对路径）
    pub image_path: Option<String>,
    /// 缩略图路径（同上）
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
/// 液态玻璃强度默认值：0.7（比中线略强，突出磨砂/提活效果）
fn default_glass_strength() -> f64 {
    0.7
}
fn default_image_preview_height() -> i64 {
    60
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub history_limit: i64,
    pub shortcut: String,
    /// 弹窗背景透明度 0.3..=1.0
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    /// 液态玻璃强度 0.2..=1.0（背景提活幅度 + 顶部光池强度，0=轻 1=强）
    #[serde(default = "default_glass_strength")]
    pub glass_strength: f64,
    /// 宽度模式
    #[serde(default, deserialize_with = "lenient_size_mode")]
    pub width_mode: SizeMode,
    /// 高度模式
    #[serde(default, deserialize_with = "lenient_size_mode")]
    pub height_mode: SizeMode,
    #[serde(default = "default_window_width")]
    pub window_width: i64,
    #[serde(default = "default_window_height")]
    pub window_height: i64,
    /// 是否启用搜索框
    #[serde(default = "default_true")]
    pub search_enabled: bool,
    /// 搜索模式
    #[serde(default, deserialize_with = "lenient_search_mode")]
    pub search_mode: SearchMode,
    /// 弹窗位置
    #[serde(default, deserialize_with = "lenient_position_mode")]
    pub position_mode: PositionMode,
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

impl Default for Settings {
    fn default() -> Self {
        Settings {
            history_limit: 15,
            shortcut: "Command+Shift+V".into(),
            opacity: 0.9,
            glass_strength: 0.7,
            width_mode: SizeMode::Auto,
            height_mode: SizeMode::Auto,
            window_width: 360,
            window_height: 480,
            search_enabled: true,
            search_mode: SearchMode::Fuzzy,
            position_mode: PositionMode::Cursor,
            image_preview_height: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_string_modes_still_parse() {
        let json = r#"{"history_limit":15,"shortcut":"Command+Shift+V","width_mode":"custom","height_mode":"auto","search_mode":"exact","position_mode":"center"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.width_mode, SizeMode::Custom);
        assert_eq!(s.height_mode, SizeMode::Auto);
        assert_eq!(s.search_mode, SearchMode::Exact);
        assert_eq!(s.position_mode, PositionMode::Center);
    }

    #[test]
    fn invalid_mode_falls_back_instead_of_failing() {
        let json = r#"{"history_limit":15,"shortcut":"Command+Shift+V","width_mode":"bogus","position_mode":"nope"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.width_mode, SizeMode::Auto);
        assert_eq!(s.position_mode, PositionMode::Cursor);
    }

    #[test]
    fn settings_roundtrip_serializes_enums_as_lowercase() {
        let mut s = Settings::default();
        s.width_mode = SizeMode::Custom;
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["width_mode"], "custom");
        let back: Settings = serde_json::from_value(v).unwrap();
        assert_eq!(back.width_mode, SizeMode::Custom);
    }

    #[test]
    fn missing_optional_fields_use_defaults() {
        let json = r#"{"history_limit":5,"shortcut":"X"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.opacity, 0.9);
        assert!(s.search_enabled);
        assert_eq!(s.search_mode, SearchMode::Fuzzy);
        assert_eq!(s.image_preview_height, 60);
    }
}
