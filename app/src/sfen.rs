//! SFEN の読み書きと、よく使う初期局面。
//!
//! 表示・コピー・貼り付けの唯一の窓口。盤面の内部モデルは駒の id を持つが、
//! SFEN は id を表現しないので、読み込みのたびに走査順で振り直す。

use crate::board::{square_at, BoardState, FILES, RANKS};
use crate::piece::{format_piece_letter, parse_piece_letter, Piece, Side, HAND_ORDER};

/// 平手の初期配置。
pub const HIRATE_SFEN: &str = "lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1";

/// 「全駒」。玉だけを盤に残し、残りをすべて両者の駒台へ置く。
/// 空の盤へ 1 枚ずつ足すより、要る駒だけ駒台から戻す方が並べるのが速いため。
pub const ALL_PIECES_SFEN: &str = "4k4/9/9/9/9/9/9/9/4K4 b RB2G2S2N2L9Prb2g2s2n2l9p 1";

/// 盤上に何も置かない空局面。詰将棋などを一から並べるときに使う。
pub const EMPTY_SFEN: &str = "9/9/9/9/9/9/9/9/9 b - 1";

/// 操作ページのプリセットボタン。
pub fn preset_sfen(name: &str) -> Option<&'static str> {
    Some(match name {
        "hirate" => HIRATE_SFEN,
        "allPieces" => ALL_PIECES_SFEN,
        "empty" => EMPTY_SFEN,
        _ => return None,
    })
}

struct IdSource {
    next: u32,
}

impl IdSource {
    fn new() -> Self {
        Self { next: 1 }
    }

    fn take(&mut self) -> String {
        let id = format!("p{}", self.next);
        self.next += 1;
        id
    }
}

fn parse_board_field(field: &str, state: &mut BoardState, ids: &mut IdSource) -> bool {
    let rows: Vec<&str> = field.split('/').collect();
    if rows.len() != RANKS {
        return false;
    }

    for (index, row) in rows.iter().enumerate() {
        let rank = index + 1;
        let mut file: i32 = FILES as i32;
        let chars: Vec<char> = row.chars().collect();
        let mut at = 0;

        while at < chars.len() {
            let ch = chars[at];

            if ch.is_ascii_digit() && ch != '0' {
                file -= ch.to_digit(10).unwrap_or(0) as i32;
                at += 1;
                if file < 0 {
                    return false;
                }
                continue;
            }

            let (promoted, letter) = if ch == '+' {
                match chars.get(at + 1) {
                    Some(&next) => {
                        at += 2;
                        (true, next)
                    }
                    None => return false,
                }
            } else {
                at += 1;
                (false, ch)
            };

            let Some((kind, side)) = parse_piece_letter(letter) else {
                return false;
            };
            if file < 1 {
                return false;
            }

            let square = square_at(file as usize, rank);
            state.squares[square] = Some(Piece::new(ids.take(), kind, promoted, side));
            file -= 1;
        }

        if file != 0 {
            return false;
        }
    }

    true
}

fn parse_hands_field(field: &str, state: &mut BoardState, ids: &mut IdSource) -> bool {
    if field == "-" {
        return true;
    }

    let chars: Vec<char> = field.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        let mut count: u32 = 0;
        while at < chars.len() && chars[at].is_ascii_digit() {
            count = count * 10 + chars[at].to_digit(10).unwrap_or(0);
            at += 1;
            if count > 81 {
                return false;
            }
        }
        if count == 0 {
            count = 1;
        }

        let Some(&letter) = chars.get(at) else {
            return false;
        };
        at += 1;

        let Some((kind, side)) = parse_piece_letter(letter) else {
            return false;
        };
        if !kind.is_hand_kind() {
            return false;
        }

        for _ in 0..count {
            let piece = Piece::new(ids.take(), kind, false, side);
            let Some(stack) = state.hands.get_mut(side).get_mut(kind) else {
                return false;
            };
            stack.push(piece);
        }
    }

    true
}

/// SFEN 文字列を盤面へ変換する。手番と手数は省略を許し、それぞれ先手番・1 手目として扱う。
/// 解釈できない文字列には None を返す (利用者の貼り付けミスを黙って無視しないため)。
pub fn parse_sfen(text: &str) -> Option<BoardState> {
    let fields: Vec<&str> = text.split_whitespace().collect();
    let board_field = *fields.first()?;

    let mut state = BoardState::empty();
    let mut ids = IdSource::new();
    if !parse_board_field(board_field, &mut state, &mut ids) {
        return None;
    }

    state.turn = match fields.get(1).copied().unwrap_or("b") {
        "b" => Side::Black,
        "w" => Side::White,
        _ => return None,
    };

    if !parse_hands_field(fields.get(2).copied().unwrap_or("-"), &mut state, &mut ids) {
        return None;
    }

    let move_number: u32 = fields.get(3).copied().unwrap_or("1").parse().ok()?;
    if move_number < 1 {
        return None;
    }
    state.move_number = move_number;

    Some(state)
}

fn format_board_field(state: &BoardState) -> String {
    let mut rows: Vec<String> = Vec::with_capacity(RANKS);

    for rank in 1..=RANKS {
        let mut row = String::new();
        let mut empty = 0;

        for file in (1..=FILES).rev() {
            match state.piece_at(square_at(file, rank)) {
                None => empty += 1,
                Some(piece) => {
                    if empty > 0 {
                        row.push_str(&empty.to_string());
                        empty = 0;
                    }
                    row.push_str(&format_piece_letter(piece.kind, piece.promoted, piece.side));
                }
            }
        }

        if empty > 0 {
            row.push_str(&empty.to_string());
        }
        rows.push(row);
    }

    rows.join("/")
}

fn format_hands_field(state: &BoardState) -> String {
    let mut field = String::new();

    for side in [Side::Black, Side::White] {
        for kind in HAND_ORDER {
            let count = state.hand_count(side, kind);
            if count == 0 {
                continue;
            }
            if count > 1 {
                field.push_str(&count.to_string());
            }
            field.push_str(&format_piece_letter(kind, false, side));
        }
    }

    if field.is_empty() {
        "-".to_owned()
    } else {
        field
    }
}

pub fn format_sfen(state: &BoardState) -> String {
    format!(
        "{} {} {} {}",
        format_board_field(state),
        state.turn.letter(),
        format_hands_field(state),
        state.move_number
    )
}

/// プリセットは実行時に必ず解釈できるので、失敗を呼び出し側へ伝播させない。
pub fn preset_board(name: &str) -> Option<BoardState> {
    parse_sfen(preset_sfen(name)?)
}

pub fn initial_board() -> BoardState {
    parse_sfen(HIRATE_SFEN).expect("hirate sfen must parse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square_at;
    use crate::piece::Kind;

    #[test]
    fn 平手の駒を正しい向きと位置へ並べる() {
        let board = parse_sfen(HIRATE_SFEN).expect("hirate");

        // 5九 が先手玉、5一 が後手玉。
        let king = board.piece_at(square_at(5, 9)).expect("先手玉");
        assert_eq!((king.kind, king.side), (Kind::K, Side::Black));
        let enemy = board.piece_at(square_at(5, 1)).expect("後手玉");
        assert_eq!((enemy.kind, enemy.side), (Kind::K, Side::White));

        // 8八 が先手角、2八 が先手飛。
        assert_eq!(
            board.piece_at(square_at(8, 8)).map(|p| p.kind),
            Some(Kind::B)
        );
        assert_eq!(
            board.piece_at(square_at(2, 8)).map(|p| p.kind),
            Some(Kind::R)
        );
        assert_eq!(
            board.piece_at(square_at(7, 7)).map(|p| p.kind),
            Some(Kind::P)
        );
        assert!(board.piece_at(square_at(5, 5)).is_none());
        assert_eq!(board.turn, Side::Black);
        assert_eq!(board.move_number, 1);
    }

    #[test]
    fn 成駒と持ち駒を読む() {
        let board = parse_sfen("9/9/9/9/4+P4/9/9/9/9 w 2G3p 7").expect("sfen");

        let piece = board.piece_at(square_at(5, 5)).expect("piece");
        assert_eq!(
            (piece.kind, piece.promoted, piece.side),
            (Kind::P, true, Side::Black)
        );
        assert_eq!(board.hand_count(Side::Black, Kind::G), 2);
        assert_eq!(board.hand_count(Side::White, Kind::P), 3);
        assert_eq!(board.turn, Side::White);
        assert_eq!(board.move_number, 7);
    }

    #[test]
    fn 手番と持ち駒と手数は省略できる() {
        let board = parse_sfen("9/9/9/9/9/9/9/9/9").expect("sfen");
        assert_eq!(board.turn, Side::Black);
        assert_eq!(board.move_number, 1);
    }

    #[test]
    fn 駒に一意な_id_を振る() {
        let board = preset_board("hirate").expect("hirate");
        let ids: Vec<&str> = board
            .squares
            .iter()
            .flatten()
            .map(|piece| piece.id.as_str())
            .collect();
        assert_eq!(ids.len(), 40);
        let unique: std::collections::HashSet<&&str> = ids.iter().collect();
        assert_eq!(unique.len(), 40);
    }

    #[test]
    fn 解釈できない_sfen_は拒否する() {
        for sfen in [
            "9/9/9/9/9/9/9/9",             // 段が足りない
            "8/9/9/9/9/9/9/9/9",           // 筋の合計が 9 でない
            "ppppppppppp/9/9/9/9/9/9/9/9", // 筋が 9 を超える
            "x8/9/9/9/9/9/9/9/9",          // 未知の駒文字
            "9/9/9/9/9/9/9/9/9 x - 1",     // 手番が不正
            "9/9/9/9/9/9/9/9/9 b K 1",     // 持ち駒に玉
            "9/9/9/9/9/9/9/9/9 b - 0",     // 手数が 0
            "   ",                         // 空文字
        ] {
            assert!(parse_sfen(sfen).is_none(), "{sfen}");
        }
    }

    #[test]
    fn プリセットを往復できる() {
        for sfen in [HIRATE_SFEN, ALL_PIECES_SFEN, EMPTY_SFEN] {
            let board = parse_sfen(sfen).expect("preset");
            assert_eq!(format_sfen(&board), sfen);
        }
    }

    #[test]
    fn 持ち駒を先手後手の順で価値の高い順に並べる() {
        let board = parse_sfen("9/9/9/9/9/9/9/9/9 b p2lRG 1").expect("sfen");
        assert_eq!(format_sfen(&board), "9/9/9/9/9/9/9/9/9 b RG2lp 1");
    }

    #[test]
    fn 全駒は玉_2_枚だけを盤に残す() {
        let board = preset_board("allPieces").expect("allPieces");
        assert_eq!(board.squares.iter().flatten().count(), 2);
        assert_eq!(board.hand_count(Side::Black, Kind::P), 9);
        assert_eq!(board.hand_count(Side::White, Kind::P), 9);
    }

    #[test]
    fn プリセットの駒数が将棋の駒数と合う() {
        for name in ["hirate", "allPieces"] {
            let counts = preset_board(name).expect(name).count_kinds();
            assert_eq!(
                counts,
                [
                    (Kind::K, 2),
                    (Kind::R, 2),
                    (Kind::B, 2),
                    (Kind::G, 4),
                    (Kind::S, 4),
                    (Kind::N, 4),
                    (Kind::L, 4),
                    (Kind::P, 18),
                ],
                "{name}"
            );
        }
        let empty = preset_board("empty").expect("empty").count_kinds();
        assert!(empty.iter().all(|(_, count)| *count == 0));
    }

    #[test]
    fn 未知のプリセット名は無い() {
        assert!(preset_board("kaku").is_none());
    }
}
