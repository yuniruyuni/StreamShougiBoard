//! 盤面の正規モデルと、その上で許される編集操作。
//!
//! このアプリは盤面編集だけを持ち、合法手の判定はしない。配信で任意の局面を並べるのが主な用途で、
//! 合法手を強いると詰将棋も途中図も作れなくなるため。
//! 禁止するのは編集として意味をなさない操作 (玉を取る、玉を駒台へ送る、自駒の上へ動かす) だけ。
//! 操作関数はすべて元の state を書き換えず、成功したら新しい state を、拒否したら None を返す。

use serde::{Deserialize, Serialize};

use crate::piece::{Kind, Piece, Side, HAND_ORDER};

pub const FILES: usize = 9;
pub const RANKS: usize = 9;
pub const SQUARE_COUNT: usize = FILES * RANKS;

/// 盤上のマス番号 0..80。
/// SFEN の走査順に合わせ、0 が 9一 (左上)、8 が 1一、80 が 1九 (右下)。
pub type Square = usize;

/// 筋 (1..9、右から数える) と段 (1..9、上から数える) からマス番号を作る。
pub fn square_at(file: usize, rank: usize) -> Square {
    (rank - 1) * FILES + (FILES - file)
}

// square_at と対になる読み出し。盤面ロジックからは使わないが、
// SFEN とテストが位置を語るときの共通語彙として置いておく。
#[allow(dead_code)]
pub fn file_of(square: Square) -> usize {
    FILES - (square % FILES)
}

#[allow(dead_code)]
pub fn rank_of(square: Square) -> usize {
    square / FILES + 1
}

pub fn is_square(value: i64) -> bool {
    (0..SQUARE_COUNT as i64).contains(&value)
}

/// 片側の持ち駒。同じ駒種でも 1 枚ずつ Piece を保持し、id を保って表示側の追跡を助ける。
/// 7 種すべてを必ず持つので、表示側は欠けた駒種を気にしなくてよい。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hand {
    #[serde(rename = "R")]
    pub rook: Vec<Piece>,
    #[serde(rename = "B")]
    pub bishop: Vec<Piece>,
    #[serde(rename = "G")]
    pub gold: Vec<Piece>,
    #[serde(rename = "S")]
    pub silver: Vec<Piece>,
    #[serde(rename = "N")]
    pub knight: Vec<Piece>,
    #[serde(rename = "L")]
    pub lance: Vec<Piece>,
    #[serde(rename = "P")]
    pub pawn: Vec<Piece>,
}

impl Hand {
    /// 玉は持ち駒にならないので None を返す。
    pub fn get(&self, kind: Kind) -> Option<&Vec<Piece>> {
        Some(match kind {
            Kind::R => &self.rook,
            Kind::B => &self.bishop,
            Kind::G => &self.gold,
            Kind::S => &self.silver,
            Kind::N => &self.knight,
            Kind::L => &self.lance,
            Kind::P => &self.pawn,
            Kind::K => return None,
        })
    }

    pub fn get_mut(&mut self, kind: Kind) -> Option<&mut Vec<Piece>> {
        Some(match kind {
            Kind::R => &mut self.rook,
            Kind::B => &mut self.bishop,
            Kind::G => &mut self.gold,
            Kind::S => &mut self.silver,
            Kind::N => &mut self.knight,
            Kind::L => &mut self.lance,
            Kind::P => &mut self.pawn,
            Kind::K => return None,
        })
    }

    pub fn count(&self, kind: Kind) -> usize {
        self.get(kind).map_or(0, Vec::len)
    }
}

/// 両陣営の持ち駒。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hands {
    #[serde(rename = "b")]
    pub black: Hand,
    #[serde(rename = "w")]
    pub white: Hand,
}

impl Hands {
    pub fn get(&self, side: Side) -> &Hand {
        match side {
            Side::Black => &self.black,
            Side::White => &self.white,
        }
    }

    pub fn get_mut(&mut self, side: Side) -> &mut Hand {
        match side {
            Side::Black => &mut self.black,
            Side::White => &mut self.white,
        }
    }
}

/// 直前の指し手。盤面ページのハイライトにだけ使う。
/// 盤上から駒台への移動では to が None、持ち駒を打ったときは from が None になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastMove {
    pub from: Option<Square>,
    pub to: Option<Square>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardState {
    /// 長さ 81。空きマスは None。
    pub squares: Vec<Option<Piece>>,
    pub hands: Hands,
    pub turn: Side,
    pub move_number: u32,
    pub last_move: Option<LastMove>,
}

impl BoardState {
    pub fn empty() -> Self {
        Self {
            squares: vec![None; SQUARE_COUNT],
            hands: Hands::default(),
            turn: Side::Black,
            move_number: 1,
            last_move: None,
        }
    }

    /// 盤外を一様に None として扱う。
    pub fn piece_at(&self, square: Square) -> Option<&Piece> {
        self.squares.get(square).and_then(Option::as_ref)
    }

    pub fn hand_count(&self, side: Side, kind: Kind) -> usize {
        self.hands.get(side).count(kind)
    }

    /// 取った駒を持ち駒へ入れる。成りは戻し、陣営は取った側へ移す。
    fn add_to_hand(&mut self, side: Side, piece: Piece) {
        if let Some(stack) = self.hands.get_mut(side).get_mut(piece.kind) {
            stack.push(Piece::new(piece.id, piece.kind, false, side));
        }
    }

    /// 盤上の駒を別のマスへ動かす。
    /// 移動先に相手駒があれば取って持ち駒へ入れる。玉は取れず、自駒の上へは動かせない。
    pub fn move_piece(&self, from: Square, to: Square) -> Option<Self> {
        if from == to {
            return None;
        }
        let moving = self.piece_at(from)?.clone();

        let target = self.piece_at(to).cloned();
        if let Some(target) = &target {
            if target.side == moving.side || target.kind == Kind::K {
                return None;
            }
        }

        let mut next = self.clone();
        if let Some(target) = target {
            next.add_to_hand(moving.side, target);
        }
        next.squares[from] = None;
        next.squares[to] = Some(moving);
        next.last_move = Some(LastMove {
            from: Some(from),
            to: Some(to),
        });
        Some(next)
    }

    /// 持ち駒を打つ。編集盤なので二歩や行き所のない駒は許す一方、
    /// 相手駒の上へ打った場合は盤上と同じくその駒を取る。
    pub fn drop_piece(&self, side: Side, kind: Kind, to: Square) -> Option<Self> {
        if to >= SQUARE_COUNT {
            return None;
        }
        let dropping = self.hands.get(side).get(kind)?.last()?.clone();

        let target = self.piece_at(to).cloned();
        if let Some(target) = &target {
            if target.side == side || target.kind == Kind::K {
                return None;
            }
        }

        let mut next = self.clone();
        next.hands.get_mut(side).get_mut(kind)?.pop();
        if let Some(target) = target {
            next.add_to_hand(side, target);
        }
        next.squares[to] = Some(Piece::new(dropping.id, kind, false, side));
        next.last_move = Some(LastMove {
            from: None,
            to: Some(to),
        });
        Some(next)
    }

    /// 盤上の駒を駒台へ送る。玉は駒台に置けない。
    pub fn move_to_hand(&self, from: Square, side: Side) -> Option<Self> {
        let piece = self.piece_at(from)?.clone();
        if !piece.kind.is_hand_kind() {
            return None;
        }

        let mut next = self.clone();
        next.squares[from] = None;
        next.add_to_hand(side, piece);
        next.last_move = Some(LastMove {
            from: Some(from),
            to: None,
        });
        Some(next)
    }

    /// 成 / 不成 を切り替える。金と玉は対象外。
    pub fn toggle_promote(&self, square: Square) -> Option<Self> {
        let piece = self.piece_at(square)?;
        if !piece.kind.is_promotable() {
            return None;
        }

        let mut next = self.clone();
        let promoted = !piece.promoted;
        next.squares[square] = Some(Piece::new(
            piece.id.clone(),
            piece.kind,
            promoted,
            piece.side,
        ));
        Some(next)
    }

    /// 盤上の駒の先手 / 後手を入れ替える。局面を作るときの向き直しに使う。
    pub fn flip_piece_side(&self, square: Square) -> Option<Self> {
        let piece = self.piece_at(square)?;

        let mut next = self.clone();
        next.squares[square] = Some(Piece::new(
            piece.id.clone(),
            piece.kind,
            piece.promoted,
            piece.side.opponent(),
        ));
        Some(next)
    }

    /// 盤上と両駒台に存在する駒種ごとの枚数。局面を並べたときの員数確認に使う。
    #[allow(dead_code)]
    pub fn count_kinds(&self) -> [(Kind, usize); 8] {
        let mut counts = [
            (Kind::K, 0),
            (Kind::R, 0),
            (Kind::B, 0),
            (Kind::G, 0),
            (Kind::S, 0),
            (Kind::N, 0),
            (Kind::L, 0),
            (Kind::P, 0),
        ];
        let mut bump = |kind: Kind| {
            if let Some(entry) = counts.iter_mut().find(|(k, _)| *k == kind) {
                entry.1 += 1;
            }
        };
        for piece in self.squares.iter().flatten() {
            bump(piece.kind);
        }
        for side in [Side::Black, Side::White] {
            for kind in HAND_ORDER {
                for _ in 0..self.hands.get(side).count(kind) {
                    bump(kind);
                }
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sfen::parse_sfen;

    fn board(sfen: &str) -> BoardState {
        parse_sfen(sfen).expect("invalid test sfen")
    }

    #[test]
    fn マス番号は筋と段から引ける() {
        for (file, rank, index) in [(9, 1, 0), (1, 1, 8), (5, 5, 40), (1, 9, 80)] {
            assert_eq!(square_at(file, rank), index);
            assert_eq!(file_of(index), file);
            assert_eq!(rank_of(index), rank);
        }
    }

    #[test]
    fn 空きマスへ動かせる() {
        let before = board("9/9/9/9/4P4/9/9/9/9 b - 1");
        let from = square_at(5, 5);
        let to = square_at(5, 4);
        let after = before.move_piece(from, to).expect("move");

        assert!(after.piece_at(from).is_none());
        assert_eq!(after.piece_at(to).map(|p| p.kind), Some(Kind::P));
        assert_eq!(
            after.last_move,
            Some(LastMove {
                from: Some(from),
                to: Some(to)
            })
        );
        // 元の state は書き換えない。
        assert!(before.piece_at(from).is_some());
    }

    #[test]
    fn 相手駒を取ると成りを外して自分の持ち駒へ入る() {
        let before = board("9/9/9/9/4+r4/4P4/9/9/9 b - 1");
        let after = before
            .move_piece(square_at(5, 6), square_at(5, 5))
            .expect("capture");

        assert_eq!(after.hand_count(Side::Black, Kind::R), 1);
        let captured = &after.hands.black.rook[0];
        assert!(!captured.promoted);
        assert_eq!(captured.side, Side::Black);
    }

    #[test]
    fn 取った駒は_id_を保つ() {
        let before = board("9/9/9/9/4r4/4P4/9/9/9 b - 1");
        let captured_id = before.piece_at(square_at(5, 5)).expect("piece").id.clone();
        let after = before
            .move_piece(square_at(5, 6), square_at(5, 5))
            .expect("capture");
        assert_eq!(after.hands.black.rook[0].id, captured_id);
    }

    #[test]
    fn 自駒の上と玉へは動かせない() {
        assert!(board("9/9/9/9/4P4/4P4/9/9/9 b - 1")
            .move_piece(square_at(5, 6), square_at(5, 5))
            .is_none());
        assert!(board("9/9/9/9/4k4/4R4/9/9/9 b - 1")
            .move_piece(square_at(5, 6), square_at(5, 5))
            .is_none());
    }

    #[test]
    fn 空きマスからも同じマスへも動かせない() {
        let before = board("9/9/9/9/4P4/9/9/9/9 b - 1");
        assert!(before
            .move_piece(square_at(1, 1), square_at(5, 5))
            .is_none());
        assert!(before
            .move_piece(square_at(5, 5), square_at(5, 5))
            .is_none());
    }

    #[test]
    fn 持ち駒を打つと駒台から減る() {
        let before = board("9/9/9/9/9/9/9/9/9 b 2P 1");
        let to = square_at(5, 5);
        let after = before.drop_piece(Side::Black, Kind::P, to).expect("drop");

        assert_eq!(after.hand_count(Side::Black, Kind::P), 1);
        assert_eq!(after.piece_at(to).map(|p| p.kind), Some(Kind::P));
        assert_eq!(
            after.last_move,
            Some(LastMove {
                from: None,
                to: Some(to)
            })
        );
    }

    #[test]
    fn 編集盤なので二歩も行き所のない駒も許す() {
        let before = board("9/9/9/9/9/9/4P4/9/9 b P 1");
        assert!(before
            .drop_piece(Side::Black, Kind::P, square_at(5, 1))
            .is_some());
    }

    #[test]
    fn 相手駒の上へ打つとその駒を取る() {
        let before = board("9/9/9/9/4s4/9/9/9/9 b P 1");
        let after = before
            .drop_piece(Side::Black, Kind::P, square_at(5, 5))
            .expect("drop");
        assert_eq!(after.hand_count(Side::Black, Kind::S), 1);
    }

    #[test]
    fn 持っていない駒と自駒と玉の上へは打てない() {
        assert!(board("9/9/9/9/9/9/9/9/9 b - 1")
            .drop_piece(Side::Black, Kind::P, square_at(5, 5))
            .is_none());
        assert!(board("9/9/9/9/4P4/9/9/9/9 b P 1")
            .drop_piece(Side::Black, Kind::P, square_at(5, 5))
            .is_none());
        assert!(board("9/9/9/9/4k4/9/9/9/9 b P 1")
            .drop_piece(Side::Black, Kind::P, square_at(5, 5))
            .is_none());
    }

    #[test]
    fn 盤上の駒を指定した側の駒台へ送れる() {
        let before = board("9/9/9/9/4+p4/9/9/9/9 b - 1");
        let from = square_at(5, 5);
        let after = before.move_to_hand(from, Side::Black).expect("to hand");

        assert!(after.piece_at(from).is_none());
        assert_eq!(after.hand_count(Side::Black, Kind::P), 1);
        assert!(!after.hands.black.pawn[0].promoted);
        assert_eq!(
            after.last_move,
            Some(LastMove {
                from: Some(from),
                to: None
            })
        );
    }

    #[test]
    fn 玉は駒台へ送れない() {
        assert!(board("9/9/9/9/4K4/9/9/9/9 b - 1")
            .move_to_hand(square_at(5, 5), Side::Black)
            .is_none());
    }

    #[test]
    fn 成れる駒は成と不成を往復できる() {
        let before = board("9/9/9/9/4S4/9/9/9/9 b - 1");
        let square = square_at(5, 5);
        let promoted = before.toggle_promote(square).expect("promote");
        assert!(promoted.piece_at(square).expect("piece").promoted);
        let back = promoted.toggle_promote(square).expect("unpromote");
        assert!(!back.piece_at(square).expect("piece").promoted);
    }

    #[test]
    fn 金と玉は成れない() {
        assert!(board("9/9/9/9/4G4/9/9/9/9 b - 1")
            .toggle_promote(square_at(5, 5))
            .is_none());
        assert!(board("9/9/9/9/4K4/9/9/9/9 b - 1")
            .toggle_promote(square_at(5, 5))
            .is_none());
    }

    #[test]
    fn 陣営反転は駒種と成りを変えない() {
        let before = board("9/9/9/9/4+P4/9/9/9/9 b - 1");
        let square = square_at(5, 5);
        let after = before.flip_piece_side(square).expect("flip");
        let piece = after.piece_at(square).expect("piece");

        assert_eq!(piece.kind, Kind::P);
        assert!(piece.promoted);
        assert_eq!(piece.side, Side::White);
    }
}
