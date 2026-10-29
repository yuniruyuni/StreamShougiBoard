//! 碁盤の正規モデルと、その上で許される編集操作。
//!
//! 将棋側と同じく盤面編集だけを持ち、合法手の判定はしない。コウも着手禁止点も咎めない。
//! ただし石を置いたときの「取り」だけは自動で処理する。呼吸点の無くなった相手の石を
//! 手で 1 つずつ消すのは並べる作業として意味が無く、取り忘れた盤は配信で誤った局面になるため。
//! 自分の石が呼吸点を失う手 (自殺手) も禁止せず、そのまま盤に残す。
//! 操作関数はすべて元の state を書き換えず、成功したら新しい state を、拒否したら None を返す。

use serde::{Deserialize, Serialize};

use crate::piece::Side;

pub const DEFAULT_GO_SIZE: usize = 19;
/// SGF の座標表記 (a..s) で表せる範囲に揃える。
pub const MIN_GO_SIZE: usize = 2;
pub const MAX_GO_SIZE: usize = 19;

/// 交点の番号 0..size*size。0 が左上で、行ごとに左から右へ数える (SGF の座標と同じ向き)。
pub type Point = usize;

/// 取った石の数 (アゲハマ)。`black` は黒が取った白石の数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Captures {
    #[serde(rename = "b")]
    pub black: u32,
    #[serde(rename = "w")]
    pub white: u32,
}

impl Captures {
    fn get_mut(&mut self, side: Side) -> &mut u32 {
        match side {
            Side::Black => &mut self.black,
            Side::White => &mut self.white,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoBoard {
    pub size: usize,
    /// 長さ size*size。空点は None。
    pub points: Vec<Option<Side>>,
    pub captures: Captures,
    pub turn: Side,
    pub move_number: u32,
    /// 直前に置いた石。盤面ページのハイライトにだけ使う。
    pub last_move: Option<Point>,
}

impl GoBoard {
    /// 路数が範囲外なら None。
    pub fn empty(size: usize) -> Option<Self> {
        if !(MIN_GO_SIZE..=MAX_GO_SIZE).contains(&size) {
            return None;
        }
        Some(Self {
            size,
            points: vec![None; size * size],
            captures: Captures::default(),
            turn: Side::Black,
            move_number: 1,
            last_move: None,
        })
    }

    pub fn initial() -> Self {
        Self::empty(DEFAULT_GO_SIZE).expect("default go size is in range")
    }

    // 列と行で位置を語るのはテストだけ。盤面ロジックとプロトコルは交点番号で通す。
    #[cfg(test)]
    pub fn point_at(&self, column: usize, row: usize) -> Point {
        row * self.size + column
    }

    pub fn is_point(&self, value: i64) -> bool {
        (0..self.points.len() as i64).contains(&value)
    }

    /// 盤外を一様に None として扱う。
    pub fn stone_at(&self, point: Point) -> Option<Side> {
        self.points.get(point).copied().flatten()
    }

    fn neighbors(&self, point: Point) -> impl Iterator<Item = Point> {
        let size = self.size;
        let (row, column) = (point / size, point % size);
        [
            (row > 0).then(|| point - size),
            (row + 1 < size).then(|| point + size),
            (column > 0).then(|| point - 1),
            (column + 1 < size).then(|| point + 1),
        ]
        .into_iter()
        .flatten()
    }

    /// point を含む連と、その連に呼吸点があるかどうか。point が空点なら空の連を返す。
    fn group(&self, point: Point) -> (Vec<Point>, bool) {
        let Some(color) = self.stone_at(point) else {
            return (Vec::new(), true);
        };
        let mut seen = vec![false; self.points.len()];
        let mut stack = vec![point];
        let mut stones = Vec::new();
        let mut has_liberty = false;
        seen[point] = true;
        while let Some(current) = stack.pop() {
            stones.push(current);
            for next in self.neighbors(current) {
                match self.stone_at(next) {
                    None => has_liberty = true,
                    Some(stone) if stone == color && !seen[next] => {
                        seen[next] = true;
                        stack.push(next);
                    }
                    Some(_) => {}
                }
            }
        }
        (stones, has_liberty)
    }

    /// 空点へ color の石を置き、呼吸点の無くなった相手の連を取ってアゲハマへ足す。
    /// 手番と手数は動かさない (着手として数えるかは呼び出し側が決める)。
    pub fn place(&self, point: Point, color: Side) -> Option<Self> {
        if self.stone_at(point).is_some() || point >= self.points.len() {
            return None;
        }
        let mut next = self.clone();
        next.points[point] = Some(color);

        let mut captured = 0;
        for neighbor in self.neighbors(point) {
            if next.stone_at(neighbor) != Some(color.opponent()) {
                continue;
            }
            let (stones, has_liberty) = next.group(neighbor);
            if has_liberty {
                continue;
            }
            for stone in stones {
                next.points[stone] = None;
                captured += 1;
            }
        }
        *next.captures.get_mut(color) += captured;
        next.last_move = Some(point);
        Some(next)
    }

    /// 石を取り除く。取りではなく編集なので、アゲハマには数えない。
    pub fn remove(&self, point: Point) -> Option<Self> {
        self.stone_at(point)?;
        let mut next = self.clone();
        next.points[point] = None;
        if next.last_move == Some(point) {
            next.last_move = None;
        }
        Some(next)
    }

    /// 交点を color の石にする。相手の石があれば置き換え、空点なら置いて取りも処理する。
    /// 黒だけ・白だけを続けて並べる操作に使う。
    pub fn set_stone(&self, point: Point, color: Side) -> Option<Self> {
        match self.stone_at(point) {
            Some(stone) if stone == color => None,
            Some(_) => self.remove(point)?.place(point, color),
            None => self.place(point, color),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 図で盤を作る。`X` が黒、`O` が白、`.` が空点。
    fn board(rows: &[&str]) -> GoBoard {
        let mut board = GoBoard::empty(rows.len()).expect("size");
        for (row, line) in rows.iter().enumerate() {
            for (column, mark) in line.chars().enumerate() {
                let point = board.point_at(column, row);
                board.points[point] = match mark {
                    'X' => Some(Side::Black),
                    'O' => Some(Side::White),
                    _ => None,
                };
            }
        }
        board
    }

    fn picture(board: &GoBoard) -> Vec<String> {
        board
            .points
            .chunks(board.size)
            .map(|row| {
                row.iter()
                    .map(|stone| match stone {
                        Some(Side::Black) => 'X',
                        Some(Side::White) => 'O',
                        None => '.',
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn 路数は範囲内だけ作れる() {
        assert!(GoBoard::empty(1).is_none());
        assert!(GoBoard::empty(20).is_none());
        for size in [9, 13, 19] {
            let board = GoBoard::empty(size).expect("size");
            assert_eq!(board.points.len(), size * size);
            assert_eq!(board.turn, Side::Black);
        }
    }

    #[test]
    fn 置くと直前の手になり手番は動かない() {
        let placed = GoBoard::initial().place(60, Side::Black).expect("place");
        assert_eq!(placed.stone_at(60), Some(Side::Black));
        assert_eq!(placed.last_move, Some(60));
        assert_eq!(placed.turn, Side::Black);
        assert_eq!(placed.move_number, 1);
    }

    #[test]
    fn 石のある点には置けない() {
        let placed = GoBoard::initial().place(0, Side::Black).expect("place");
        assert!(placed.place(0, Side::White).is_none());
        assert!(placed.place(0, Side::Black).is_none());
    }

    #[test]
    fn 呼吸点を塞ぐと相手の石を取る() {
        let start = board(&["...", "XO.", ".X."]);
        let after = start.place(5, Side::Black).expect("place");
        // 上がまだ空いているので取れない。
        assert_eq!(picture(&after)[1], "XOX");
        assert_eq!(after.captures.black, 0);

        let start = board(&[".X.", "XO.", ".X."]);
        let after = start.place(5, Side::Black).expect("place");
        assert_eq!(picture(&after), [".X.", "X.X", ".X."]);
        assert_eq!(after.captures, Captures { black: 1, white: 0 });
    }

    #[test]
    fn 連はまとめて取り隅と辺も数える() {
        let start = board(&["OO.X", "XXX.", "....", "...."]);
        let after = start.place(2, Side::Black).expect("place");
        assert_eq!(picture(&after)[0], "..XX");
        assert_eq!(after.captures.black, 2);
    }

    #[test]
    fn 一手で複数の連を取る() {
        let start = board(&[".O.", "OXO", "X.X"]);
        // 白が (1,2) を埋めると、中央の黒と下隅の黒 2 子がまとめて呼吸点を失う。
        let after = start.place(7, Side::White).expect("place");
        assert_eq!(picture(&after), [".O.", "O.O", ".O."]);
        assert_eq!(after.captures.white, 3);
    }

    #[test]
    fn 取りが自殺手より先に効く() {
        // 黒 (2,1) は置いた瞬間は呼吸点が無いが、白 (1,1) を取って生きる。
        let start = board(&[".XO.", "XO.O", ".XO.", "...."]);
        let after = start.place(6, Side::Black).expect("place");
        assert_eq!(picture(&after)[1], "X.XO");
        assert_eq!(after.captures.black, 1);
    }

    #[test]
    fn 自殺手は禁止せずそのまま残す() {
        let start = board(&[".X", "X."]);
        let after = start.place(0, Side::White).expect("place");
        assert_eq!(after.stone_at(0), Some(Side::White));
        assert_eq!(after.captures.white, 0);
    }

    #[test]
    fn 取り除きはアゲハマに数えない() {
        let start = board(&["XO", ".."]);
        assert_eq!(picture(&start.remove(1).expect("remove"))[0], "X.");
        assert_eq!(
            start.remove(1).expect("remove").captures,
            Captures::default()
        );
        assert!(start.remove(2).is_none());
    }

    #[test]
    fn 取り除いた石が直前の手なら印も消す() {
        let placed = GoBoard::initial().place(10, Side::White).expect("place");
        assert_eq!(placed.remove(10).expect("remove").last_move, None);
    }

    #[test]
    fn 色指定は相手の石を置き換え同じ色なら何もしない() {
        let start = board(&["XO", ".."]);
        let replaced = start.set_stone(1, Side::Black).expect("replace");
        assert_eq!(picture(&replaced)[0], "XX");
        assert!(start.set_stone(0, Side::Black).is_none());
        assert_eq!(
            start.set_stone(2, Side::White).expect("place").stone_at(2),
            Some(Side::White)
        );
    }
}
