//! 設定の読み書き。OBS の見た目は配信をまたいで保つ必要があるので、
//! 終了時に同じ場所へ書き戻す。盤面そのものは保存しない (毎回まっさらから始める)。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::view::ViewSettings;

pub const DEFAULT_PORT: u16 = 16874;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub port: u16,
    /// 起動時に操作画面を既定のブラウザで開く。
    pub auto_open_control: bool,
    pub view: ViewSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            auto_open_control: true,
            view: ViewSettings::default(),
        }
    }
}

#[cfg(windows)]
fn config_dir() -> Option<PathBuf> {
    known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
        .map(|base| base.join("StreamShougiBoard"))
}

#[cfg(not(windows))]
fn config_dir() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(base).join("StreamShougiBoard"));
    }
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("StreamShougiBoard")
        })
}

pub fn config_path() -> Option<PathBuf> {
    Some(config_dir()?.join("config.json"))
}

fn coerce_port(value: Option<&serde_json::Value>) -> u16 {
    let Some(number) = value.and_then(serde_json::Value::as_i64) else {
        return DEFAULT_PORT;
    };
    // 0 は「OS に空きポートを選ばせる」なので許す。
    if number == 0 {
        return 0;
    }
    if (1024..=65535).contains(&number) {
        number as u16
    } else {
        DEFAULT_PORT
    }
}

/// 壊れた設定ファイルで起動できなくなるより、既定値へ落として起動する方が良い。
pub fn parse_config(raw: &str) -> AppConfig {
    let Ok(serde_json::Value::Object(record)) = serde_json::from_str::<serde_json::Value>(raw)
    else {
        return AppConfig::default();
    };

    let defaults = AppConfig::default();
    AppConfig {
        port: coerce_port(record.get("port")),
        auto_open_control: record
            .get("autoOpenControl")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(defaults.auto_open_control),
        view: defaults
            .view
            .merged(record.get("view").unwrap_or(&serde_json::Value::Null)),
    }
}

pub fn load() -> AppConfig {
    let Some(path) = config_path() else {
        return AppConfig::default();
    };
    // 初回起動でファイルが無いのは普通のこと。
    match std::fs::read_to_string(path) {
        Ok(raw) => parse_config(&raw),
        Err(_) => AppConfig::default(),
    }
}

pub fn save(config: &AppConfig) {
    let Some(path) = config_path() else { return };
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(config).unwrap_or_default();
        std::fs::write(path, format!("{text}\n"))
    })();
    if let Err(error) = result {
        // 保存に失敗しても配信は続けられるので、落とさず知らせるだけにする。
        eprintln!("StreamShougiBoard: 設定を保存できませんでした: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{BackgroundColor, HandLayout};

    #[test]
    fn 壊れた設定は既定値へ落とす() {
        for raw in ["{", "[]", "null", "\"text\""] {
            assert_eq!(parse_config(raw), AppConfig::default(), "{raw}");
        }
    }

    #[test]
    fn 保存した見た目を読み戻す() {
        let raw = r#"{"port":20000,"autoOpenControl":false,
            "view":{"backgroundColor":"white","backgroundOpacity":40,
                "handLayout":"stacked","flipped":true}}"#;
        let config = parse_config(raw);

        assert_eq!(config.port, 20000);
        assert!(!config.auto_open_control);
        assert_eq!(config.view.background_color, BackgroundColor::White);
        assert_eq!(config.view.background_opacity, 40);
        assert_eq!(config.view.hand_layout, HandLayout::Stacked);
        assert!(config.view.flipped);
    }

    #[test]
    fn 範囲外のポートは既定値へ落とす() {
        for raw in [r#"{"port":80}"#, r#"{"port":70000}"#, r#"{"port":"16874"}"#] {
            assert_eq!(parse_config(raw).port, DEFAULT_PORT, "{raw}");
        }
        // 0 は OS に空きポートを選ばせる指定として通す。
        assert_eq!(parse_config(r#"{"port":0}"#).port, 0);
    }

    #[test]
    fn 見た目の一部だけでも残りを既定値で埋める() {
        let config = parse_config(r#"{"view":{"backgroundOpacity":55}}"#);
        assert_eq!(config.view.background_opacity, 55);
        assert_eq!(config.view.margin, ViewSettings::default().margin);
    }
}
