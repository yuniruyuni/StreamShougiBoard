//! 駒の種別と SFEN 表記の対応。盤面ロジックとは独立させ、ここだけが表記を知っている。

use serde::{Deserialize, Serialize};

/// 先手 (black) と後手 (white)。SFEN の手番表記に合わせた記号でやり取りする。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    #[serde(rename = "b")]
    Black,
    #[serde(rename = "w")]
    White,
}

impl Side {
    pub fn opponent(self) -> Self {
        match self {
            Side::Black => Side::White,
            Side::White => Side::Black,
        }
    }

    pub fn letter(self) -> char {
        match self {
            Side::Black => 'b',
            Side::White => 'w',
        }
    }
}

/// 成っていない状態の駒種。SFEN の大文字表記をそのまま識別子に使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    K,
    R,
    B,
    G,
    S,
    N,
    L,
    P,
}

/// 持ち駒欄の並び順。SFEN の慣例どおり価値の高い順に並べ、出力と UI 表示の両方でこの順を使う。
pub const HAND_ORDER: [Kind; 7] = [
    Kind::R,
    Kind::B,
    Kind::G,
    Kind::S,
    Kind::N,
    Kind::L,
    Kind::P,
];

impl Kind {
    /// 成れるかどうか。金と玉は成れない。
    pub fn is_promotable(self) -> bool {
        !matches!(self, Kind::K | Kind::G)
    }

    /// 持ち駒になりうるか。玉は取れないので持ち駒にならない。
    pub fn is_hand_kind(self) -> bool {
        self != Kind::K
    }

    pub fn letter(self) -> char {
        match self {
            Kind::K => 'K',
            Kind::R => 'R',
            Kind::B => 'B',
            Kind::G => 'G',
            Kind::S => 'S',
            Kind::N => 'N',
            Kind::L => 'L',
            Kind::P => 'P',
        }
    }

    pub fn from_letter(letter: char) -> Option<Self> {
        Some(match letter.to_ascii_lowercase() {
            'k' => Kind::K,
            'r' => Kind::R,
            'b' => Kind::B,
            'g' => Kind::G,
            's' => Kind::S,
            'n' => Kind::N,
            'l' => Kind::L,
            'p' => Kind::P,
            _ => return None,
        })
    }
}

/// 盤上の 1 枚の駒。`id` は駒が移動しても変わらず、表示側のアニメーションの手掛かりになる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub id: String,
    pub kind: Kind,
    pub promoted: bool,
    pub side: Side,
}

impl Piece {
    pub fn new(id: impl Into<String>, kind: Kind, promoted: bool, side: Side) -> Self {
        Self {
            id: id.into(),
            kind,
            promoted: promoted && kind.is_promotable(),
            side,
        }
    }
}

/// SFEN の 1 駒表記を解釈する。`+` は呼び出し側で取り除いた後の 1 文字を渡す。
/// 大文字が先手、小文字が後手。
pub fn parse_piece_letter(letter: char) -> Option<(Kind, Side)> {
    let kind = Kind::from_letter(letter)?;
    let side = if letter.is_ascii_uppercase() {
        Side::Black
    } else {
        Side::White
    };
    Some((kind, side))
}

/// SFEN の駒表記へ戻す。成駒には `+` を前置する。
pub fn format_piece_letter(kind: Kind, promoted: bool, side: Side) -> String {
    let letter = match side {
        Side::Black => kind.letter(),
        Side::White => kind.letter().to_ascii_lowercase(),
    };
    if promoted && kind.is_promotable() {
        format!("+{letter}")
    } else {
        letter.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 成れるのは金と玉以外() {
        assert!(!Kind::K.is_promotable());
        assert!(!Kind::G.is_promotable());
        for kind in [Kind::R, Kind::B, Kind::S, Kind::N, Kind::L, Kind::P] {
            assert!(kind.is_promotable(), "{kind:?}");
        }
    }

    #[test]
    fn 玉だけが持ち駒にならない() {
        assert!(!Kind::K.is_hand_kind());
        for kind in HAND_ORDER {
            assert!(kind.is_hand_kind(), "{kind:?}");
        }
    }

    #[test]
    fn 駒表記を往復できる() {
        for kind in [
            Kind::K,
            Kind::R,
            Kind::B,
            Kind::G,
            Kind::S,
            Kind::N,
            Kind::L,
            Kind::P,
        ] {
            for side in [Side::Black, Side::White] {
                let text = format_piece_letter(kind, false, side);
                let letter = text.chars().next().expect("letter");
                assert_eq!(parse_piece_letter(letter), Some((kind, side)));
            }
        }
    }

    #[test]
    fn 成れない駒に成りを付けない() {
        assert_eq!(format_piece_letter(Kind::G, true, Side::Black), "G");
        assert_eq!(format_piece_letter(Kind::P, true, Side::Black), "+P");
        assert!(!Piece::new("p1", Kind::G, true, Side::Black).promoted);
    }

    #[test]
    fn 未知の文字は解釈しない() {
        assert_eq!(parse_piece_letter('x'), None);
        assert_eq!(parse_piece_letter('1'), None);
    }
}
