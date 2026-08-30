//! OBS 側の見た目設定。操作ページから変更し、盤面ページが即座に反映する。
//!
//! 盤面そのものと違って利用者の好みでしかないので、不正な値は拒否ではなく既定値へ丸める。
//! 配信中に設定を触って盤が消えるより、無害な値に落ちる方が良い。

use serde::{Deserialize, Serialize};

/// 盤と駒台を囲む地に敷く色。濃さは background_opacity が決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundColor {
    White,
    Black,
}

/// 駒台の置き場所。sides は盤の左右、stacked は盤の上下。
/// 配信レイアウトの空きが横長か縦長かで選べるよう、両方持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandLayout {
    Sides,
    Stacked,
}

pub const MIN_MARGIN: i64 = 0;
pub const MAX_MARGIN: i64 = 200;
pub const MIN_BACKGROUND_OPACITY: i64 = 0;
pub const MAX_BACKGROUND_OPACITY: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewSettings {
    pub background_color: BackgroundColor,
    /// 地の濃さ (%)。0 なら地を塗らず、盤と駒台の外はそのまま映像が透ける。
    pub background_opacity: i64,
    /// 盤と駒台を含めた全体の外周余白 px。盤の大きさはブラウザソースの領域から決まるので、
    /// これは領域の縁から何 px 空けるかを表す。
    pub margin: i64,
    pub hand_layout: HandLayout,
    /// 直前の指し手のマスを光らせる。
    pub show_last_move: bool,
    /// 操作中の選択枠を OBS 側にも出す。既定は off で、選択は手元だけに見せる。
    pub show_selection: bool,
    /// 筋 (9..1) と段 (一..九) の番号を出す。
    pub show_coordinates: bool,
    /// 後手視点へ盤を 180 度回す。
    pub flipped: bool,
    /// 駒の移動を CSS transition で補間する。
    pub animate: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            background_color: BackgroundColor::Black,
            background_opacity: 0,
            margin: 16,
            hand_layout: HandLayout::Sides,
            show_last_move: true,
            show_selection: false,
            show_coordinates: true,
            flipped: false,
            animate: true,
        }
    }
}

fn clamp_int(value: Option<&serde_json::Value>, min: i64, max: i64, fallback: i64) -> i64 {
    match value.and_then(serde_json::Value::as_f64) {
        Some(number) if number.is_finite() => (number.round() as i64).clamp(min, max),
        _ => fallback,
    }
}

fn pick_bool(value: Option<&serde_json::Value>, fallback: bool) -> bool {
    value
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(fallback)
}

fn pick_enum<T: for<'de> Deserialize<'de>>(value: Option<&serde_json::Value>, fallback: T) -> T {
    value
        .and_then(|raw| serde_json::from_value(raw.clone()).ok())
        .unwrap_or(fallback)
}

impl ViewSettings {
    /// 受け取った部分更新を現在の設定へ重ねる。未知のキーは落とし、
    /// 範囲外の数値は clamp して、常に完全で表示可能な設定を返す。
    pub fn merged(self, patch: &serde_json::Value) -> Self {
        let Some(input) = patch.as_object() else {
            return self;
        };

        Self {
            background_color: pick_enum(input.get("backgroundColor"), self.background_color),
            background_opacity: clamp_int(
                input.get("backgroundOpacity"),
                MIN_BACKGROUND_OPACITY,
                MAX_BACKGROUND_OPACITY,
                self.background_opacity,
            ),
            margin: clamp_int(input.get("margin"), MIN_MARGIN, MAX_MARGIN, self.margin),
            hand_layout: pick_enum(input.get("handLayout"), self.hand_layout),
            show_last_move: pick_bool(input.get("showLastMove"), self.show_last_move),
            show_selection: pick_bool(input.get("showSelection"), self.show_selection),
            show_coordinates: pick_bool(input.get("showCoordinates"), self.show_coordinates),
            flipped: pick_bool(input.get("flipped"), self.flipped),
            animate: pick_bool(input.get("animate"), self.animate),
        }
    }
}
