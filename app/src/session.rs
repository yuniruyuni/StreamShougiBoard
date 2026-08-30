//! 操作コマンドを受けて盤面・選択・履歴を進める reducer。
//!
//! クリックの意味 (選択なのか移動なのか打ち込みなのか) をここへ集約しているので、
//! 操作ページを 2 枚開いても選択状態が割れず、挙動をそのまま cargo test で確かめられる。

use crate::board::{is_square, BoardState, Square};
use crate::piece::{Kind, Side};
use crate::protocol::{ClientMessage, Selection};
use crate::sfen::{initial_board, parse_sfen, preset_board};
use crate::view::ViewSettings;

/// 履歴の上限。超えた分は古い側から捨てる。
pub const MAX_HISTORY: usize = 512;

#[derive(Debug, Clone)]
pub struct Session {
    /// 長さは必ず 1 以上。末尾が最新の局面。
    history: Vec<BoardState>,
    history_index: usize,
    view: ViewSettings,
    selection: Option<Selection>,
}

impl Session {
    pub fn new(view: ViewSettings) -> Self {
        Self {
            history: vec![initial_board()],
            history_index: 0,
            view,
            selection: None,
        }
    }

    /// 履歴は常に 1 件以上あるので、この関数は必ず盤面を返す。
    pub fn board(&self) -> &BoardState {
        self.history
            .get(self.history_index)
            .or_else(|| self.history.last())
            .expect("session history must not be empty")
    }

    pub fn view(&self) -> ViewSettings {
        self.view
    }

    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    pub fn history_index(&self) -> usize {
        self.history_index
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// 新しい局面を履歴へ積む。巻き戻した先で操作したときは、その先の手順を捨ててから積む。
    /// 分岐を持たせても、配信中に枝を選ぶ手立てが無いため。
    fn push_history(&mut self, board: BoardState) {
        self.history.truncate(self.history_index + 1);
        self.history.push(board);
        if self.history.len() > MAX_HISTORY {
            let excess = self.history.len() - MAX_HISTORY;
            self.history.drain(..excess);
        }
        self.history_index = self.history.len() - 1;
        self.selection = None;
    }

    /// 局面を丸ごと入れ替える。SFEN 入力とプリセットは履歴も作り直す。
    fn replace_history(&mut self, board: BoardState) {
        self.history = vec![board];
        self.history_index = 0;
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
            .board()
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
                match self.board().drop_piece(side, piece_kind, square) {
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
                match self.board().move_piece(from, square) {
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
            return match self.board().move_to_hand(square, side) {
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

        self.selection = if self.board().hand_count(side, piece_kind) == 0 {
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
        match edit(self.board(), raw as Square) {
            Some(edited) => {
                self.push_history(edited);
                None
            }
            None => Some(reason.to_owned()),
        }
    }

    fn history_go(&mut self, raw: i64) -> Option<String> {
        if raw < 0 || raw as usize >= self.history.len() {
            return Some("その手数はありません".to_owned());
        }
        self.history_index = raw as usize;
        self.selection = None;
        None
    }

    /// コマンドを適用し、編集として成立しなかった場合はその理由を返す。
    /// Ping は呼び出し側で処理するため、ここへは渡さない。
    pub fn apply(&mut self, command: ClientMessage) -> Option<String> {
        match command {
            ClientMessage::Ping { .. } => None,

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

            ClientMessage::SetTurn { side } => {
                if self.board().turn == side {
                    return None;
                }
                let mut board = self.board().clone();
                board.turn = side;
                self.push_history(board);
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
        let mut session = Session::new(ViewSettings::default());
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

        let board = session.board();
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
            session.board().piece_at(square_at(5, 6)).map(|p| p.kind),
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
            session.board().piece_at(square_at(5, 5)).map(|p| p.kind),
            Some(Kind::K)
        );
    }

    #[test]
    fn 相手駒を取ると持ち駒に入る() {
        let mut session = session_from("9/9/9/9/4s4/4R4/9/9/9 b - 1");
        tap(&mut session, 5, 6);
        tap(&mut session, 5, 5);
        assert_eq!(session.board().hand_count(Side::Black, Kind::S), 1);
    }

    #[test]
    fn 持ち駒を選んで盤を叩くと打てる() {
        let mut session = session_from("9/9/9/9/9/9/9/9/9 b 2P 1");
        tap_hand(&mut session, Side::Black, Kind::P);
        tap(&mut session, 5, 5);

        let board = session.board();
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

        let board = session.board();
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
            session.board().piece_at(square_at(5, 5)).map(|p| p.kind),
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
            .board()
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
            session.board().piece_at(square_at(4, 4)).map(|p| p.kind),
            Some(Kind::P)
        );
    }

    #[test]
    fn 範囲外の手数は拒否する() {
        let mut session = Session::new(ViewSettings::default());
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
        assert_eq!(format_sfen(session.board()), HIRATE_SFEN);
    }

    #[test]
    fn 解釈できない_sfen_は拒否し盤面を壊さない() {
        let mut session = Session::new(ViewSettings::default());
        let rejected = session.apply(ClientMessage::SetSfen {
            sfen: "これは sfen ではない".to_owned(),
        });
        assert_eq!(rejected.as_deref(), Some("sfen を解釈できません"));
        assert_eq!(format_sfen(session.board()), HIRATE_SFEN);
    }

    #[test]
    fn 未知のプリセット名は拒否する() {
        let mut session = Session::new(ViewSettings::default());
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
        let mut session = Session::new(ViewSettings::default());
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
        let mut session = Session::new(ViewSettings::default());
        assert_eq!(
            session.apply(ClientMessage::SetTurn { side: Side::Black }),
            None
        );
        assert_eq!(session.history_len(), 1);

        assert_eq!(
            session.apply(ClientMessage::SetTurn { side: Side::White }),
            None
        );
        assert_eq!(session.board().turn, Side::White);
        assert_eq!(session.history_len(), 2);
    }
}
