//! SGF の読み書き。囲碁モードでの表示・コピー・貼り付けの唯一の窓口。
//!
//! 読むのは本譜 (各分岐の最初の枝) だけで、配置 (AB/AW/AE) と着手 (B/W) から
//! 局面の列を作る。1 手ごとに履歴へ積めるので、読み込んだ棋譜を ←→ でそのまま並べられる。
//! コメントや対局情報は読み捨てる。
//!
//! 書き出すのは現在局面の配置だけ (SFEN 欄と同じく「この局面」を渡すためのもの)。
//! アゲハマの数と手数は SGF の配置に表せないので、書き出して読み直すと 0 と 1 手目に戻る。

use crate::go::{GoBoard, Point, DEFAULT_GO_SIZE};
use crate::piece::Side;

type Property = (String, Vec<String>);
type Node = Vec<Property>;

struct Reader<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl Reader<'_> {
    fn skip_space(&mut self) {
        while self.chars.next_if(|c| c.is_whitespace()).is_some() {}
    }

    /// `[` の直後から `]` までを読む。`\` は次の 1 文字をそのまま通す。
    fn value(&mut self) -> Option<String> {
        let mut text = String::new();
        loop {
            match self.chars.next()? {
                ']' => return Some(text),
                '\\' => text.push(self.chars.next()?),
                c => text.push(c),
            }
        }
    }

    fn node(&mut self) -> Option<Node> {
        let mut properties = Vec::new();
        loop {
            self.skip_space();
            if !self.chars.peek().is_some_and(char::is_ascii_alphabetic) {
                return Some(properties);
            }
            let mut name = String::new();
            while let Some(c) = self.chars.next_if(char::is_ascii_alphabetic) {
                // FF[3] 以前は AddBlack のような小文字混じりも許していた。大文字だけが識別子。
                if c.is_ascii_uppercase() {
                    name.push(c);
                }
            }
            let mut values = Vec::new();
            loop {
                self.skip_space();
                if self.chars.next_if_eq(&'[').is_none() {
                    break;
                }
                values.push(self.value()?);
            }
            if values.is_empty() {
                return None;
            }
            properties.push((name, values));
        }
    }

    /// `(` の直後から本譜を読む。最初の子へ潜ったら、残りの枝は読まずに終える。
    fn main_line(&mut self, nodes: &mut Vec<Node>) -> Option<()> {
        loop {
            self.skip_space();
            match self.chars.next()? {
                ';' => nodes.push(self.node()?),
                '(' => return self.main_line(nodes),
                ')' => return Some(()),
                _ => return None,
            }
        }
    }
}

fn read_main_line(text: &str) -> Option<Vec<Node>> {
    let mut reader = Reader {
        chars: text.chars().peekable(),
    };
    reader.skip_space();
    if reader.chars.next()? != '(' {
        return None;
    }
    let mut nodes = Vec::new();
    reader.main_line(&mut nodes)?;
    if nodes.is_empty() {
        return None;
    }
    Some(nodes)
}

fn values<'a>(node: &'a Node, name: &'a str) -> impl Iterator<Item = &'a str> {
    node.iter()
        .filter(move |(key, _)| key == name)
        .flat_map(|(_, values)| values.iter().map(String::as_str))
}

fn coordinate(letter: u8, size: usize) -> Option<usize> {
    let index = letter.checked_sub(b'a')? as usize;
    (index < size).then_some(index)
}

/// `pd` のような 2 文字の座標。範囲外なら None。
fn parse_point(text: &str, size: usize) -> Option<(usize, usize)> {
    let &[column, row] = text.as_bytes() else {
        return None;
    };
    Some((coordinate(column, size)?, coordinate(row, size)?))
}

/// 着手の座標。空値と、19 路以下での `tt` はパスを表すので Some(None) を返す。
fn parse_move(text: &str, size: usize) -> Option<Option<Point>> {
    if text.is_empty() || (text == "tt" && size <= 19) {
        return Some(None);
    }
    let (column, row) = parse_point(text, size)?;
    Some(Some(row * size + column))
}

/// 配置の座標。FF[4] の `aa:cc` (矩形の圧縮表記) も展開する。
fn parse_points(text: &str, size: usize) -> Option<Vec<Point>> {
    let (from, to) = text.split_once(':').unwrap_or((text, text));
    let (c1, r1) = parse_point(from, size)?;
    let (c2, r2) = parse_point(to, size)?;
    let mut points = Vec::new();
    for row in r1.min(r2)..=r1.max(r2) {
        for column in c1.min(c2)..=c1.max(c2) {
            points.push(row * size + column);
        }
    }
    Some(points)
}

fn parse_size(root: &Node) -> Option<usize> {
    match values(root, "SZ").next() {
        None => Some(DEFAULT_GO_SIZE),
        // 長方形の盤 (SZ[19:13]) は扱わない。
        Some(text) => text.trim().parse().ok(),
    }
}

fn parse_side(text: &str) -> Option<Side> {
    match text.trim() {
        "B" | "b" => Some(Side::Black),
        "W" | "w" => Some(Side::White),
        _ => None,
    }
}

/// 1 つの節点を局面へ当てる。配置を先に、着手を後に処理する (SGF の定め)。
fn apply_node(board: &GoBoard, node: &Node) -> Option<GoBoard> {
    let size = board.size;
    let mut next = board.clone();

    for (name, color) in [
        ("AE", None),
        ("AB", Some(Side::Black)),
        ("AW", Some(Side::White)),
    ] {
        for value in values(node, name) {
            for point in parse_points(value, size)? {
                next.points[point] = color;
            }
        }
    }

    for (name, color) in [("B", Side::Black), ("W", Side::White)] {
        for value in values(node, name) {
            if let Some(point) = parse_move(value, size)? {
                // 石のある点への着手は壊れた棋譜。黙って飛ばすと以降の局面がずれるので拒む。
                next = next.place(point, color)?;
            }
            next.turn = color.opponent();
            next.move_number += 1;
        }
    }

    if let Some(side) = values(node, "PL").next() {
        next.turn = parse_side(side)?;
    }
    Some(next)
}

/// 本譜の局面列を返す。先頭は根の節点 (配置) を当てた局面で、以降は節点ごとに 1 局面。
/// 囲碁以外の棋譜 (GM が 1 でない) と、範囲外の路数は拒む。
pub fn parse_sgf(text: &str) -> Option<Vec<GoBoard>> {
    let nodes = read_main_line(text)?;
    let root = &nodes[0];
    if let Some(game) = values(root, "GM").next() {
        if game.trim() != "1" {
            return None;
        }
    }
    let empty = GoBoard::empty(parse_size(root)?)?;

    let mut positions = Vec::with_capacity(nodes.len());
    let mut current = apply_node(&empty, root)?;
    // 配置だけの局面に直前の手は無い。根に着手が書かれていた場合だけ残る。
    positions.push(current.clone());
    for node in &nodes[1..] {
        current = apply_node(&current, node)?;
        positions.push(current.clone());
    }
    Some(positions)
}

fn point_text(board: &GoBoard, point: Point) -> String {
    let column = (b'a' + (point % board.size) as u8) as char;
    let row = (b'a' + (point / board.size) as u8) as char;
    format!("[{column}{row}]")
}

/// 現在局面を配置だけの SGF にする。
pub fn format_sgf(board: &GoBoard) -> String {
    let mut text = format!("(;GM[1]FF[4]CA[UTF-8]SZ[{}]", board.size);
    for (name, color) in [("AB", Side::Black), ("AW", Side::White)] {
        let stones: String = (0..board.points.len())
            .filter(|&point| board.stone_at(point) == Some(color))
            .map(|point| point_text(board, point))
            .collect();
        if !stones.is_empty() {
            text.push_str(name);
            text.push_str(&stones);
        }
    }
    text.push_str(match board.turn {
        Side::Black => "PL[B]",
        Side::White => "PL[W]",
    });
    text.push(')');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last(text: &str) -> GoBoard {
        parse_sgf(text).expect("parse").pop().expect("position")
    }

    #[test]
    fn 配置と手番を読む() {
        let board = last("(;GM[1]FF[4]SZ[9]AB[cc][gg]AW[cg]PL[W])");
        assert_eq!(board.size, 9);
        assert_eq!(board.stone_at(board.point_at(2, 2)), Some(Side::Black));
        assert_eq!(board.stone_at(board.point_at(6, 6)), Some(Side::Black));
        assert_eq!(board.stone_at(board.point_at(2, 6)), Some(Side::White));
        assert_eq!(board.turn, Side::White);
        assert_eq!(board.last_move, None);
    }

    #[test]
    fn 路数の指定が無ければ_19_路() {
        assert_eq!(last("(;GM[1])").size, 19);
    }

    #[test]
    fn 着手は_1_手ごとに局面になり取りも処理する() {
        // 黒が隅の白 1 子を取る。
        let positions = parse_sgf("(;SZ[9]AW[aa];B[ba];W[ee];B[ab])").expect("parse");
        assert_eq!(positions.len(), 4);
        let end = positions.last().expect("end");
        assert_eq!(end.stone_at(0), None);
        assert_eq!(end.captures.black, 1);
        assert_eq!(end.last_move, Some(end.point_at(0, 1)));
        assert_eq!(end.turn, Side::White);
        assert_eq!(end.move_number, 4);
    }

    #[test]
    fn パスは手番と手数だけ進める() {
        let end = last("(;SZ[19];B[pd];W[];B[tt])");
        assert_eq!(end.points.iter().flatten().count(), 1);
        assert_eq!(end.turn, Side::White);
        assert_eq!(end.move_number, 4);
    }

    #[test]
    fn 分岐は最初の枝だけを読む() {
        let positions = parse_sgf("(;SZ[9];B[ee](;W[cc];B[gg])(;W[gc]))").expect("parse");
        assert_eq!(positions.len(), 4);
        let end = positions.last().expect("end");
        assert_eq!(end.stone_at(end.point_at(2, 2)), Some(Side::White));
        assert_eq!(end.stone_at(end.point_at(6, 2)), None);
    }

    #[test]
    fn 圧縮表記とエスケープと空白を受け付ける() {
        // 同じ節点で同じ点を配置し直すのは SGF の定めに反するので、取り除きは次の節点で行う。
        let board = last("(\n ;GM [1] SZ[5] C[a \\] b] AB[aa:bb]\n ;AE[aa] )");
        assert_eq!(board.points.iter().flatten().count(), 3);
        assert_eq!(board.stone_at(0), None);
    }

    #[test]
    fn 小文字混じりの古い識別子も読む() {
        let board = last("(;GaMe[1]SiZe[9]AddBlack[ee])");
        assert_eq!(board.size, 9);
        assert_eq!(board.stone_at(board.point_at(4, 4)), Some(Side::Black));
    }

    #[test]
    fn 囲碁でない棋譜や壊れた棋譜は拒む() {
        for text in [
            "",
            "(;GM[2])",
            "(;SZ[25])",
            "(;SZ[19:13])",
            "(;SZ[9];B[zz])",
            "(;SZ[9];B[ee];W[ee])",
            "(;SZ[9]C[閉じていない)",
            "(;PL[X])",
            "()",
            "sfen 9/9/9 b - 1",
        ] {
            assert!(parse_sgf(text).is_none(), "{text}");
        }
    }

    #[test]
    fn 書き出した配置は読み直すと同じ局面になる() {
        let source = "(;GM[1]FF[4]CA[UTF-8]SZ[13]AB[dd][jj]AW[dj]PL[W])";
        let board = last(source);
        assert_eq!(format_sgf(&board), source);
        assert_eq!(last(&format_sgf(&board)), board);
    }

    #[test]
    fn 空の盤は配置を書かない() {
        assert_eq!(
            format_sgf(&GoBoard::initial()),
            "(;GM[1]FF[4]CA[UTF-8]SZ[19]PL[B])"
        );
    }
}
