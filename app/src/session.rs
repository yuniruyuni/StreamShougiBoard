//! 操作コマンドを受けて盤面・選択・履歴を進める reducer。
//!
//! クリックの意味 (選択なのか移動なのか打ち込みなのか) をここへ集約しているので、
//! 操作ページを 2 枚開いても選択状態が割れず、挙動をそのまま cargo test で確かめられる。
//!
//! 将棋と囲碁は盤も履歴も別々に持ち、モードはどちらを表に出すかだけを決める。
//! 配信中に切り替えて戻っても、それぞれの局面と履歴はそのまま残っている。

use crate::board::{is_square, BoardState, Square};
use crate::go::{GoBoard, MAX_GO_SIZE, MIN_GO_SIZE};
use crate::piece::{Kind, Side};
use crate::protocol::{ClientMessage, GameMode, GameState, HistoryInfo, Selection, ServerMessage};
use crate::sfen::{format_sfen, initial_board, parse_sfen, preset_board};
use crate::sgf::{format_sgf, parse_sgf};
use crate::view::ViewSettings;

/// 履歴の上限。超えた分は古い側から捨てる。
pub const MAX_HISTORY: usize = 512;

#[cfg(test)]
mod visibility_tests {
    use super::*;
    use crate::board::square_at;

    #[test]
    fn 表示の切り替えは盤面と履歴と選択を変えない() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        for (file, rank) in [(7, 7), (7, 6), (3, 3)] {
            assert_eq!(
                session.apply(ClientMessage::TapSquare {
                    square: square_at(file, rank) as i64,
                }),
                None
            );
        }
        let board = session.shogi_board().clone();
        let history = session.shogi.clone();
        let index = session.history_index();
        let selection = session.selection().cloned();
        assert!(selection.is_some());
        assert!(history.len() > 1);

        for visible in [false, true, false] {
            assert_eq!(
                session.apply(ClientMessage::SetView {
                    view: serde_json::json!({"visible": visible}),
                }),
                None
            );
            assert_eq!(session.view().visible, visible);
            assert_eq!(session.shogi_board(), &board);
            assert_eq!(session.shogi, history);
            assert_eq!(session.history_index(), index);
            assert_eq!(session.selection(), selection.as_ref());
        }
        session.apply(ClientMessage::SetView {
            view: serde_json::json!({"visible": "true", "margin": 40}),
        });
        assert!(!session.view().visible);
        assert_eq!(session.view().margin, 40);
    }
}

/// 局面の列と、いま表示している位置。長さは必ず 1 以上で、末尾が最新の局面。
#[derive(Debug, Clone, PartialEq, Eq)]
struct History<T> {
    entries: Vec<T>,
    index: usize,
}

impl<T> History<T> {
    fn new(first: T) -> Self {
        Self {
            entries: vec![first],
            index: 0,
        }
    }

    /// 履歴は常に 1 件以上あるので、この関数は必ず局面を返す。
    fn current(&self) -> &T {
        self.entries
            .get(self.index)
            .or_else(|| self.entries.last())
            .expect("session history must not be empty")
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    /// 新しい局面を積む。巻き戻した先で操作したときは、その先の手順を捨ててから積む。
    /// 分岐を持たせても、配信中に枝を選ぶ手立てが無いため。
    fn push(&mut self, entry: T) {
        self.entries.truncate(self.index + 1);
        self.entries.push(entry);
        self.trim();
        self.index = self.entries.len() - 1;
    }

    /// 局面の列を丸ごと入れ替え、最後の局面を表示する。空の列は渡さない。
    fn replace(&mut self, entries: Vec<T>) {
        debug_assert!(!entries.is_empty());
        if entries.is_empty() {
            return;
        }
        self.entries = entries;
        self.trim();
        self.index = self.entries.len() - 1;
    }

    fn trim(&mut self) {
        if self.entries.len() > MAX_HISTORY {
            let excess = self.entries.len() - MAX_HISTORY;
            self.entries.drain(..excess);
        }
    }

    fn go(&mut self, raw: i64) -> bool {
        if raw < 0 || raw as usize >= self.entries.len() {
            return false;
        }
        self.index = raw as usize;
        true
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    mode: GameMode,
    shogi: History<BoardState>,
    go: History<GoBoard>,
    view: ViewSettings,
    /// 将棋の盤でつまんでいる駒。囲碁には選択が無い。
    selection: Option<Selection>,
}

const SHOGI_ONLY: &str = "将棋モードでだけ使えます";
const GO_ONLY: &str = "囲碁モードでだけ使えます";

impl Session {
    pub fn new(view: ViewSettings, mode: GameMode) -> Self {
        Self {
            mode,
            shogi: History::new(initial_board()),
            go: History::new(GoBoard::initial()),
            view,
            selection: None,
        }
    }

    pub fn mode(&self) -> GameMode {
        self.mode
    }

    /// 将棋の現在局面。囲碁モードの間も、戻ったときに出す局面として保たれている。
    pub fn shogi_board(&self) -> &BoardState {
        self.shogi.current()
    }

    pub fn go_board(&self) -> &GoBoard {
        self.go.current()
    }

    pub fn view(&self) -> ViewSettings {
        self.view
    }

    // 配信の形は snapshot() が組む。選択だけを覗くのはテストだけ。
    #[cfg(test)]
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// 表に出ているモードの履歴の位置。
    pub fn history_index(&self) -> usize {
        match self.mode {
            GameMode::Shogi => self.shogi.index,
            GameMode::Go => self.go.index,
        }
    }

    pub fn history_len(&self) -> usize {
        match self.mode {
            GameMode::Shogi => self.shogi.len(),
            GameMode::Go => self.go.len(),
        }
    }

    /// 購読者へ配る形。盤面ページと操作ページはこれを丸ごと受け取って全置換する。
    pub fn snapshot(&self, app_version: &'static str, rev: u64) -> ServerMessage {
        let game = match self.mode {
            GameMode::Shogi => GameState::Shogi {
                board: self.shogi_board().clone(),
                selection: self.selection.clone(),
                sfen: format_sfen(self.shogi_board()),
            },
            GameMode::Go => GameState::Go {
                board: self.go_board().clone(),
                sgf: format_sgf(self.go_board()),
            },
        };
        ServerMessage::Snapshot {
            app_version,
            rev,
            view: self.view,
            history: HistoryInfo {
                index: self.history_index(),
                length: self.history_len(),
            },
            game,
        }
    }

    fn push_history(&mut self, board: BoardState) {
        self.shogi.push(board);
        self.selection = None;
    }

    /// 局面を丸ごと入れ替える。SFEN 入力とプリセットは履歴も作り直す。
    fn replace_history(&mut self, board: BoardState) {
        self.shogi.replace(vec![board]);
        self.selection = None;
    }

    /// 駒を進めた手だけ手数を数え、手番を相手へ渡す。並べ替えの操作では動かさない。
    fn advance_turn(mut board: BoardState) -> BoardState {
        board.turn = board.turn.opponent();
        board.move_number += 1;
        board
    }

    /// 駒があれば選ぶ、無ければ選択を外す。移動に失敗したときの落とし所として使う。
    fn select_square_or_clear(&mut self, square: Square) {
        self.selection = self
            .shogi_board()
            .piece_at(square)
            .map(|_| Selection::Square { square });
    }

    fn tap_square(&mut self, raw: i64) -> Option<String> {
        if !is_square(raw) {
            return Some("盤の外です".to_owned());
        }
        let square = raw as Square;

        match self.selection.clone() {
            None => {
                self.select_square_or_clear(square);
                None
            }

            Some(Selection::Hand { side, piece_kind }) => {
                match self.shogi_board().drop_piece(side, piece_kind, square) {
                    Some(dropped) => {
                        self.push_history(Self::advance_turn(dropped));
                        None
                    }
                    None => {
                        self.select_square_or_clear(square);
                        Some("そこへは打てません".to_owned())
                    }
                }
            }

            Some(Selection::Square { square: from }) => {
                // 同じマスをもう一度叩いたら選択解除。掴み直しに操作を増やさない。
                if from == square {
                    self.selection = None;
                    return None;
                }
                match self.shogi_board().move_piece(from, square) {
                    Some(moved) => {
                        self.push_history(Self::advance_turn(moved));
                        None
                    }
                    None => {
                        // 自駒の上や玉を取ろうとした場合は、移動ではなく選択の付け替えとして扱う。
                        self.select_square_or_clear(square);
                        None
                    }
                }
            }
        }
    }

    fn tap_hand(&mut self, side: Side, piece_kind: Kind) -> Option<String> {
        if !piece_kind.is_hand_kind() {
            return Some("駒台に置けない駒です".to_owned());
        }

        if let Some(Selection::Square { square }) = self.selection.clone() {
            return match self.shogi_board().move_to_hand(square, side) {
                Some(sent) => {
                    self.push_history(sent);
                    None
                }
                None => {
                    self.selection = None;
                    Some("その駒は駒台へ送れません".to_owned())
                }
            };
        }

        if let Some(Selection::Hand {
            side: selected_side,
            piece_kind: selected_kind,
        }) = self.selection.clone()
        {
            if selected_side == side && selected_kind == piece_kind {
                self.selection = None;
                return None;
            }
        }

        self.selection = if self.shogi_board().hand_count(side, piece_kind) == 0 {
            None
        } else {
            Some(Selection::Hand { side, piece_kind })
        };
        None
    }

    fn edit_square(
        &mut self,
        raw: i64,
        edit: impl Fn(&BoardState, Square) -> Option<BoardState>,
        reason: &str,
    ) -> Option<String> {
        if !is_square(raw) {
            return Some("盤の外です".to_owned());
        }
        match edit(self.shogi_board(), raw as Square) {
            Some(edited) => {
                self.push_history(edited);
                None
            }
            None => Some(reason.to_owned()),
        }
    }

    /// 色の指定が無ければ手番の石を置いて手番を渡す。指定があれば並べ替えとして手番を動かさない。
    fn go_place(&mut self, raw: i64, color: Option<Side>) -> Option<String> {
        let board = self.go_board();
        if !board.is_point(raw) {
            return Some("盤の外です".to_owned());
        }
        let point = raw as usize;
        match color {
            None => match board.place(point, board.turn) {
                Some(mut placed) => {
                    placed.turn = placed.turn.opponent();
                    placed.move_number += 1;
                    self.go.push(placed);
                    None
                }
                None => Some("そこには石があります".to_owned()),
            },
            Some(color) => {
                // 同じ色の石がすでにある点は、置き直しても何も変わらないので黙って流す。
                if let Some(edited) = board.set_stone(point, color) {
                    self.go.push(edited);
                }
                None
            }
        }
    }

    fn go_remove(&mut self, raw: i64) -> Option<String> {
        let board = self.go_board();
        if !board.is_point(raw) {
            return Some("盤の外です".to_owned());
        }
        // 空点を消しても何も変わらない。消しゴムでなぞる操作を拒否で埋めない。
        if let Some(removed) = board.remove(raw as usize) {
            self.go.push(removed);
        }
        None
    }

    fn set_turn(&mut self, side: Side) {
        match self.mode {
            GameMode::Shogi => {
                if self.shogi_board().turn == side {
                    return;
                }
                let mut board = self.shogi_board().clone();
                board.turn = side;
                self.push_history(board);
            }
            GameMode::Go => {
                if self.go_board().turn == side {
                    return;
                }
                let mut board = self.go_board().clone();
                board.turn = side;
                self.go.push(board);
            }
        }
    }

    fn history_go(&mut self, raw: i64) -> Option<String> {
        let moved = match self.mode {
            GameMode::Shogi => self.shogi.go(raw),
            GameMode::Go => self.go.go(raw),
        };
        if !moved {
            return Some("その手数はありません".to_owned());
        }
        self.selection = None;
        None
    }

    /// モードの違うコマンドは、その場の盤に意味を持たないので拒否する。
    fn require(&self, mode: GameMode) -> Option<String> {
        (self.mode != mode).then(|| {
            match mode {
                GameMode::Shogi => SHOGI_ONLY,
                GameMode::Go => GO_ONLY,
            }
            .to_owned()
        })
    }

    /// コマンドを適用し、編集として成立しなかった場合はその理由を返す。
    /// Ping は呼び出し側で処理するため、ここへは渡さない。
    pub fn apply(&mut self, command: ClientMessage) -> Option<String> {
        if let Some(mode) = command.required_mode() {
            if let Some(reason) = self.require(mode) {
                return Some(reason);
            }
        }

        match command {
            ClientMessage::Ping { .. } => None,

            ClientMessage::SetMode { mode } => {
                self.mode = mode;
                // 将棋の選択は盤を離れると意味を失う。戻ったときに古い選択枠が残らないよう外す。
                self.selection = None;
                None
            }

            ClientMessage::TapSquare { square } => self.tap_square(square),

            ClientMessage::TapHand { side, piece_kind } => self.tap_hand(side, piece_kind),

            ClientMessage::TogglePromote { square } => {
                self.edit_square(square, BoardState::toggle_promote, "その駒は成れません")
            }

            ClientMessage::FlipPiece { square } => self.edit_square(
                square,
                BoardState::flip_piece_side,
                "そのマスに駒がありません",
            ),

            ClientMessage::ClearSelection => {
                self.selection = None;
                None
            }

            ClientMessage::SetSfen { sfen } => match parse_sfen(&sfen) {
                Some(board) => {
                    self.replace_history(board);
                    None
                }
                None => Some("sfen を解釈できません".to_owned()),
            },

            ClientMessage::Preset { name } => match preset_board(&name) {
                Some(board) => {
                    self.replace_history(board);
                    None
                }
                None => Some("未知のプリセットです".to_owned()),
            },

            ClientMessage::GoPlace { point, color } => self.go_place(point, color),

            ClientMessage::GoRemove { point } => self.go_remove(point),

            ClientMessage::GoNew { size } => {
                let board = usize::try_from(size).ok().and_then(GoBoard::empty);
                match board {
                    Some(board) => {
                        self.go.replace(vec![board]);
                        None
                    }
                    None => Some(format!("{MIN_GO_SIZE}〜{MAX_GO_SIZE} 路の盤だけを作れます")),
                }
            }

            ClientMessage::SetSgf { sgf } => match parse_sgf(&sgf) {
                Some(positions) => {
                    self.go.replace(positions);
                    None
                }
                None => Some("sgf を解釈できません".to_owned()),
            },

            ClientMessage::SetTurn { side } => {
                self.set_turn(side);
                None
            }

            ClientMessage::HistoryGo { index } => self.history_go(index),

            ClientMessage::SetView { view } => {
                self.view = self.view.merged(&view);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square_at;
    use crate::sfen::{format_sfen, HIRATE_SFEN};

    fn session_from(sfen: &str) -> Session {
        let board = parse_sfen(sfen).expect("invalid test sfen");
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        session.replace_history(board);
        session
    }

    fn tap(session: &mut Session, file: usize, rank: usize) {
        let rejected = session.apply(ClientMessage::TapSquare {
            square: square_at(file, rank) as i64,
        });
        assert_eq!(rejected, None, "{file}{rank} で拒否された");
    }

    fn tap_hand(session: &mut Session, side: Side, kind: Kind) {
        let rejected = session.apply(ClientMessage::TapHand {
            side,
            piece_kind: kind,
        });
        assert_eq!(rejected, None, "駒台 {side:?} {kind:?} で拒否された");
    }

    #[test]
    fn 駒のあるマスは選択と解除ができる() {
        let mut session = session_from("9/9/9/9/4P4/9/9/9/9 b - 1");
        tap(&mut session, 5, 5);
        assert_eq!(
            session.selection(),
            Some(&Selection::Square {
                square: square_at(5, 5)
            })
        );

        tap(&mut session, 5, 5);
        assert_eq!(session.selection(), None);
    }

    #[test]
    fn 空きマスを叩いても何も選ばれない() {
        let mut session = session_from("9/9/9/9/9/9/9/9/9 b - 1");
        tap(&mut session, 5, 5);
        assert_eq!(session.selection(), None);
    }

    #[test]
    fn 選択してから別マスを叩くと動き手番と手数が進む() {
        let mut session = session_from("9/9/9/9/4P4/9/9/9/9 b - 1");
        tap(&mut session, 5, 5);
        tap(&mut session, 5, 4);

        let board = session.shogi_board();
        assert_eq!(
            board.piece_at(square_at(5, 4)).map(|p| p.kind),
            Some(Kind::P)
        );
        assert_eq!(board.turn, Side::White);
        assert_eq!(board.move_number, 2);
        assert_eq!(session.selection(), None);
        assert_eq!(session.history_len(), 2);
    }

    #[test]
    fn 自駒の上を叩いたときは移動せず選択が移る() {
        let mut session = session_from("9/9/9/9/4P4/4S4/9/9/9 b - 1");
        tap(&mut session, 5, 6);
        tap(&mut session, 5, 5);

        assert_eq!(
            session.selection(),
            Some(&Selection::Square {
                square: square_at(5, 5)
            })
        );
        assert_eq!(session.history_len(), 1);
        assert_eq!(
            session
                .shogi_board()
                .piece_at(square_at(5, 6))
                .map(|p| p.kind),
            Some(Kind::S)
        );
    }

    #[test]
    fn 玉を取ろうとしたときも移動せず選択が移る() {
        let mut session = session_from("9/9/9/9/4k4/4R4/9/9/9 b - 1");
        tap(&mut session, 5, 6);
        tap(&mut session, 5, 5);

        assert_eq!(
            session.selection(),
            Some(&Selection::Square {
                square: square_at(5, 5)
            })
        );
        assert_eq!(
            session
                .shogi_board()
                .piece_at(square_at(5, 5))
                .map(|p| p.kind),
            Some(Kind::K)
        );
    }

    #[test]
    fn 相手駒を取ると持ち駒に入る() {
        let mut session = session_from("9/9/9/9/4s4/4R4/9/9/9 b - 1");
        tap(&mut session, 5, 6);
        tap(&mut session, 5, 5);
        assert_eq!(session.shogi_board().hand_count(Side::Black, Kind::S), 1);
    }

    #[test]
    fn 持ち駒を選んで盤を叩くと打てる() {
        let mut session = session_from("9/9/9/9/9/9/9/9/9 b 2P 1");
        tap_hand(&mut session, Side::Black, Kind::P);
        tap(&mut session, 5, 5);

        let board = session.shogi_board();
        assert_eq!(
            board.piece_at(square_at(5, 5)).map(|p| p.kind),
            Some(Kind::P)
        );
        assert_eq!(board.hand_count(Side::Black, Kind::P), 1);
        assert_eq!(session.selection(), None);
    }

    #[test]
    fn 持っていない駒台を叩いても選択されない() {
        let mut session = session_from("9/9/9/9/9/9/9/9/9 b - 1");
        tap_hand(&mut session, Side::Black, Kind::P);
        assert_eq!(session.selection(), None);
    }

    #[test]
    fn 同じ持ち駒をもう一度叩くと選択が外れる() {
        let mut session = session_from("9/9/9/9/9/9/9/9/9 b P 1");
        tap_hand(&mut session, Side::Black, Kind::P);
        tap_hand(&mut session, Side::Black, Kind::P);
        assert_eq!(session.selection(), None);
    }

    #[test]
    fn 盤上の駒を選んでから駒台を叩くと送れる() {
        let mut session = session_from("9/9/9/9/4+s4/9/9/9/9 b - 1");
        tap(&mut session, 5, 5);
        tap_hand(&mut session, Side::White, Kind::S);

        let board = session.shogi_board();
        assert!(board.piece_at(square_at(5, 5)).is_none());
        assert_eq!(board.hand_count(Side::White, Kind::S), 1);
        // 駒台へ送るのは並べ替えなので手番は動かさない。
        assert_eq!(board.turn, Side::Black);
        assert_eq!(board.move_number, 1);
    }

    #[test]
    fn 玉を選んで駒台を叩くと拒否され選択だけ外れる() {
        let mut session = session_from("9/9/9/9/4K4/9/9/9/9 b - 1");
        tap(&mut session, 5, 5);
        let rejected = session.apply(ClientMessage::TapHand {
            side: Side::Black,
            piece_kind: Kind::P,
        });

        assert_eq!(rejected.as_deref(), Some("その駒は駒台へ送れません"));
        assert_eq!(session.selection(), None);
        assert_eq!(
            session
                .shogi_board()
                .piece_at(square_at(5, 5))
                .map(|p| p.kind),
            Some(Kind::K)
        );
    }

    #[test]
    fn 成れない駒への成りは拒否する() {
        let mut session = session_from("9/9/9/9/4G4/9/9/9/9 b - 1");
        let rejected = session.apply(ClientMessage::TogglePromote {
            square: square_at(5, 5) as i64,
        });
        assert_eq!(rejected.as_deref(), Some("その駒は成れません"));
        assert_eq!(session.history_len(), 1);
    }

    #[test]
    fn 成りと反転は別操作なので歩へ戻せる() {
        let mut session = session_from("9/9/9/9/4P4/9/9/9/9 b - 1");
        let square = square_at(5, 5) as i64;
        for command in [
            ClientMessage::TogglePromote { square },
            ClientMessage::FlipPiece { square },
            ClientMessage::FlipPiece { square },
            ClientMessage::TogglePromote { square },
        ] {
            assert_eq!(session.apply(command), None);
        }

        let piece = session
            .shogi_board()
            .piece_at(square_at(5, 5))
            .expect("piece")
            .clone();
        assert_eq!(
            (piece.kind, piece.promoted, piece.side),
            (Kind::P, false, Side::Black)
        );
    }

    #[test]
    fn 盤外のマスは拒否する() {
        let mut session = session_from("9/9/9/9/9/9/9/9/9 b - 1");
        assert_eq!(
            session
                .apply(ClientMessage::TapSquare { square: 81 })
                .as_deref(),
            Some("盤の外です")
        );
        assert_eq!(
            session
                .apply(ClientMessage::TapSquare { square: -1 })
                .as_deref(),
            Some("盤の外です")
        );
    }

    #[test]
    fn 戻ってから指すと先の履歴を捨てる() {
        let mut session = session_from("9/9/9/9/4P4/9/9/9/9 b - 1");
        tap(&mut session, 5, 5);
        tap(&mut session, 5, 4);
        tap(&mut session, 5, 4);
        tap(&mut session, 5, 3);
        assert_eq!(session.history_len(), 3);

        assert_eq!(session.apply(ClientMessage::HistoryGo { index: 1 }), None);
        assert_eq!(session.history_index(), 1);

        tap(&mut session, 5, 4);
        tap(&mut session, 4, 4);
        assert_eq!(session.history_len(), 3);
        assert_eq!(session.history_index(), 2);
        assert_eq!(
            session
                .shogi_board()
                .piece_at(square_at(4, 4))
                .map(|p| p.kind),
            Some(Kind::P)
        );
    }

    #[test]
    fn 範囲外の手数は拒否する() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        assert!(session
            .apply(ClientMessage::HistoryGo { index: 5 })
            .is_some());
        assert!(session
            .apply(ClientMessage::HistoryGo { index: -1 })
            .is_some());
    }

    #[test]
    fn 履歴は上限で古い側から捨てる() {
        let mut session = session_from("9/9/9/9/4P4/9/9/9/9 b - 1");
        let square = square_at(5, 5) as i64;
        for _ in 0..(MAX_HISTORY + 10) {
            assert_eq!(session.apply(ClientMessage::TogglePromote { square }), None);
        }
        assert_eq!(session.history_len(), MAX_HISTORY);
        assert_eq!(session.history_index(), MAX_HISTORY - 1);
    }

    #[test]
    fn sfen_読み込みとプリセットは履歴を作り直す() {
        let mut session = session_from("9/9/9/9/4P4/9/9/9/9 b - 1");
        tap(&mut session, 5, 5);
        tap(&mut session, 5, 4);
        assert_eq!(session.history_len(), 2);

        assert_eq!(
            session.apply(ClientMessage::Preset {
                name: "hirate".to_owned()
            }),
            None
        );
        assert_eq!(session.history_len(), 1);
        assert_eq!(session.history_index(), 0);
        assert_eq!(format_sfen(session.shogi_board()), HIRATE_SFEN);
    }

    #[test]
    fn 解釈できない_sfen_は拒否し盤面を壊さない() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        let rejected = session.apply(ClientMessage::SetSfen {
            sfen: "これは sfen ではない".to_owned(),
        });
        assert_eq!(rejected.as_deref(), Some("sfen を解釈できません"));
        assert_eq!(format_sfen(session.shogi_board()), HIRATE_SFEN);
    }

    #[test]
    fn 未知のプリセット名は拒否する() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        assert_eq!(
            session
                .apply(ClientMessage::Preset {
                    name: "kaku".to_owned()
                })
                .as_deref(),
            Some("未知のプリセットです")
        );
    }

    #[test]
    fn set_view_は範囲外の値を丸めて取り込む() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        assert_eq!(
            session.apply(ClientMessage::SetView {
                view: serde_json::json!({ "margin": 99999, "backgroundOpacity": 45 })
            }),
            None
        );
        assert_eq!(session.view().background_opacity, 45);
        assert_eq!(session.view().margin, crate::view::MAX_MARGIN);
        assert_eq!(session.history_len(), 1);
    }

    #[test]
    fn set_turn_は同じ手番なら履歴を増やさない() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        assert_eq!(
            session.apply(ClientMessage::SetTurn { side: Side::Black }),
            None
        );
        assert_eq!(session.history_len(), 1);

        assert_eq!(
            session.apply(ClientMessage::SetTurn { side: Side::White }),
            None
        );
        assert_eq!(session.shogi_board().turn, Side::White);
        assert_eq!(session.history_len(), 2);
    }

    fn go_session(size: i64) -> Session {
        let mut session = Session::new(ViewSettings::default(), GameMode::Go);
        assert_eq!(session.apply(ClientMessage::GoNew { size }), None);
        session
    }

    fn place(session: &mut Session, point: i64, color: Option<Side>) -> Option<String> {
        session.apply(ClientMessage::GoPlace { point, color })
    }

    #[test]
    fn 手番の石を交互に置き手数が進む() {
        let mut session = go_session(9);
        assert_eq!(place(&mut session, 40, None), None);
        assert_eq!(place(&mut session, 41, None), None);

        let board = session.go_board();
        assert_eq!(board.stone_at(40), Some(Side::Black));
        assert_eq!(board.stone_at(41), Some(Side::White));
        assert_eq!(board.turn, Side::Black);
        assert_eq!(board.move_number, 3);
        assert_eq!(session.history_len(), 3);
    }

    #[test]
    fn 石のある点へ交互に置こうとすると拒否する() {
        let mut session = go_session(9);
        place(&mut session, 40, None);
        assert!(place(&mut session, 40, None).is_some());
        assert_eq!(session.history_len(), 2);
    }

    #[test]
    fn 色を指定すると手番を動かさずに並べる() {
        let mut session = go_session(9);
        for point in [0, 1, 2] {
            assert_eq!(place(&mut session, point, Some(Side::White)), None);
        }
        let board = session.go_board();
        assert_eq!(board.points.iter().flatten().count(), 3);
        assert_eq!(board.turn, Side::Black);
        assert_eq!(board.move_number, 1);

        // 同じ色を重ねても履歴は増えない。
        let length = session.history_len();
        assert_eq!(place(&mut session, 0, Some(Side::White)), None);
        assert_eq!(session.history_len(), length);
    }

    #[test]
    fn 取り除きは空点なら何もしない() {
        let mut session = go_session(9);
        place(&mut session, 40, None);
        assert_eq!(session.apply(ClientMessage::GoRemove { point: 40 }), None);
        assert_eq!(session.go_board().stone_at(40), None);
        let length = session.history_len();
        assert_eq!(session.apply(ClientMessage::GoRemove { point: 40 }), None);
        assert_eq!(session.history_len(), length);
    }

    #[test]
    fn 盤外の交点と範囲外の路数は拒否する() {
        let mut session = go_session(9);
        assert!(place(&mut session, 81, None).is_some());
        assert!(place(&mut session, -1, None).is_some());
        assert!(session
            .apply(ClientMessage::GoRemove { point: 81 })
            .is_some());
        assert!(session.apply(ClientMessage::GoNew { size: 1 }).is_some());
        assert!(session.apply(ClientMessage::GoNew { size: 25 }).is_some());
        assert_eq!(session.go_board().size, 9);
    }

    #[test]
    fn 路数を変えると履歴を作り直す() {
        let mut session = go_session(9);
        place(&mut session, 40, None);
        assert_eq!(session.apply(ClientMessage::GoNew { size: 13 }), None);
        assert_eq!(session.go_board().size, 13);
        assert_eq!(session.history_len(), 1);
    }

    #[test]
    fn sgf_を読むと手順が履歴になり最後の局面を出す() {
        let mut session = go_session(19);
        assert_eq!(
            session.apply(ClientMessage::SetSgf {
                sgf: "(;SZ[9];B[ee];W[ce];B[gg])".to_owned()
            }),
            None
        );
        assert_eq!(session.history_len(), 4);
        assert_eq!(session.history_index(), 3);
        assert_eq!(session.go_board().size, 9);

        assert_eq!(session.apply(ClientMessage::HistoryGo { index: 1 }), None);
        assert_eq!(session.go_board().points.iter().flatten().count(), 1);

        assert!(session
            .apply(ClientMessage::SetSgf {
                sgf: "壊れた".to_owned()
            })
            .is_some());
        assert_eq!(session.history_len(), 4);
    }

    #[test]
    fn 囲碁の手番の切り替えは履歴へ積む() {
        let mut session = go_session(9);
        session.apply(ClientMessage::SetTurn { side: Side::White });
        assert_eq!(session.go_board().turn, Side::White);
        assert_eq!(session.history_len(), 2);
        place(&mut session, 0, None);
        assert_eq!(session.go_board().stone_at(0), Some(Side::White));
    }

    #[test]
    fn モードの違うコマンドは拒否し盤を変えない() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        assert!(place(&mut session, 0, None).is_some());
        assert!(session.apply(ClientMessage::GoNew { size: 9 }).is_some());
        assert_eq!(session.go_board(), &GoBoard::initial());

        session.apply(ClientMessage::SetMode { mode: GameMode::Go });
        assert!(session
            .apply(ClientMessage::TapSquare {
                square: square_at(7, 7) as i64
            })
            .is_some());
        assert!(session
            .apply(ClientMessage::Preset {
                name: "hirate".to_owned()
            })
            .is_some());
        assert_eq!(format_sfen(session.shogi_board()), HIRATE_SFEN);
    }

    #[test]
    fn モードを行き来しても両方の局面と履歴が残る() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        tap(&mut session, 7, 7);
        tap(&mut session, 7, 6);
        tap(&mut session, 3, 3);
        assert!(session.selection().is_some());
        let shogi = session.shogi_board().clone();

        session.apply(ClientMessage::SetMode { mode: GameMode::Go });
        assert_eq!(session.mode(), GameMode::Go);
        assert_eq!(session.history_len(), 1);
        place(&mut session, 60, None);

        session.apply(ClientMessage::SetMode {
            mode: GameMode::Shogi,
        });
        assert_eq!(session.shogi_board(), &shogi);
        assert_eq!(session.history_len(), 2);
        // 切り替えをまたいで選択は持ち越さない。
        assert_eq!(session.selection(), None);

        session.apply(ClientMessage::SetMode { mode: GameMode::Go });
        assert_eq!(session.go_board().stone_at(60), Some(Side::Black));
    }

    #[test]
    fn 履歴の移動は表のモードにだけ効く() {
        let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
        tap(&mut session, 7, 7);
        tap(&mut session, 7, 6);
        session.apply(ClientMessage::SetMode { mode: GameMode::Go });
        place(&mut session, 0, None);
        place(&mut session, 1, None);

        assert_eq!(session.apply(ClientMessage::HistoryGo { index: 0 }), None);
        assert_eq!(session.go_board(), &GoBoard::initial());
        session.apply(ClientMessage::SetMode {
            mode: GameMode::Shogi,
        });
        assert_eq!(session.history_index(), 1);
    }
}
