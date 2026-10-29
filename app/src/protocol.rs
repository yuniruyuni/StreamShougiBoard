//! ローカル WebSocket プロトコル。docs/protocol.md および client/src/protocol.ts と対で更新する。
//!
//! 盤面と設定を合わせても JSON で数 KB にしかならないので、増分イベントは持たず
//! 状態が変わるたびに snapshot を丸ごと配る。盤面ページは snapshot を全置換するだけでよく、
//! 取りこぼし検出も再同期手順も要らない。

use serde::{Deserialize, Serialize};

use crate::board::{BoardState, Square};
use crate::go::GoBoard;
use crate::piece::{Kind, Side};
use crate::view::ViewSettings;

/// exe に焼かれた版。ページも同じ exe から配られるので、これが食い違うのは
/// 「アプリを更新する前から開きっぱなしのページが、そのまま再接続してきた」ときだけ。
/// 別立てのプロトコル版を手で維持するより、ビルドを一意に指すこの値を配る方が確実。
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// クライアントから受け付けるフレームの上限。コメント入りの SGF 棋譜を貼り付けても通るよう広めに取る。
/// listener は 127.0.0.1 にしか開かないので、大きくしても外から使われる窓にはならない。
pub const MAX_CLIENT_FRAME_BYTES: usize = 256 * 1024;

/// 盤に出すゲーム。将棋と囲碁は局面も履歴も別々に持ち、これはどちらを表に出すかだけを決める。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameMode {
    #[default]
    Shogi,
    Go,
}

/// 操作ページが今つまんでいる駒。盤面ページは view.showSelection のときだけ描く。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Selection {
    Square {
        square: Square,
    },
    Hand {
        side: Side,
        #[serde(rename = "pieceKind")]
        piece_kind: Kind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HistoryInfo {
    /// 現在表示している履歴の位置 (0 始まり)。
    pub index: usize,
    /// 履歴の総数。
    pub length: usize,
}

/// 表に出ているゲームの局面。モードごとに盤の形も文字表記も違うので、mode で見分ける。
// 将棋の盤が囲碁より大きいのは承知のうえ。snapshot を組むたびに作って即座に直列化するだけなので、
// Box に包んでも確保が 1 つ増えるだけで得るものが無い。
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum GameState {
    Shogi {
        board: BoardState,
        selection: Option<Selection>,
        /// 現在局面の SFEN。操作ページの表示欄をサーバー側の正規表記で揃える。
        sfen: String,
    },
    Go {
        board: GoBoard,
        /// 現在局面を配置だけで表した SGF。
        sgf: String,
    },
}

// Snapshot が他の variant より大きいのは承知のうえ。実際に送るものの大半が Snapshot で、
// Box に包むと配信のたびに余計な確保が入るため、そのまま持つ。
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    Snapshot {
        app_version: &'static str,
        rev: u64,
        view: ViewSettings,
        /// 表に出ているモードの履歴。
        history: HistoryInfo,
        game: GameState,
    },
    Pong {
        t: i64,
    },
    /// 直前のコマンドが編集として成立しなかったことだけを伝える。状態は snapshot が運ぶ。
    Rejected {
        reason: String,
    },
}

/// 盤面ページが送るのは Ping だけ。操作ページはこれに加えてコマンドを送る。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping {
        t: i64,
    },
    SetMode {
        mode: GameMode,
    },
    /// 盤のマスを叩いた。選択・移動・打ち込みのどれになるかはサーバー側の選択状態で決まる。
    /// 判定をサーバーへ寄せることで、操作ページを複数開いても選択が割れない。
    TapSquare {
        square: i64,
    },
    /// 駒台を叩いた。選択中の盤上の駒があればそこへ送り、無ければ持ち駒を選ぶ。
    TapHand {
        side: Side,
        #[serde(rename = "pieceKind")]
        piece_kind: Kind,
    },
    TogglePromote {
        square: i64,
    },
    FlipPiece {
        square: i64,
    },
    ClearSelection,
    SetSfen {
        sfen: String,
    },
    Preset {
        name: String,
    },
    /// 交点へ石を置く。color を省くと手番の石を置いて手番を渡し、
    /// 指定すると手番を動かさずにその色を置く (相手の石があれば置き換える)。
    GoPlace {
        point: i64,
        #[serde(default)]
        color: Option<Side>,
    },
    GoRemove {
        point: i64,
    },
    /// 指定した路数の空の碁盤で始め直す。
    GoNew {
        size: i64,
    },
    SetSgf {
        sgf: String,
    },
    SetTurn {
        side: Side,
    },
    HistoryGo {
        index: i64,
    },
    SetView {
        view: serde_json::Value,
    },
}

impl ClientMessage {
    /// そのモードの盤でしか意味を持たないコマンドなら、そのモードを返す。
    pub fn required_mode(&self) -> Option<GameMode> {
        match self {
            ClientMessage::TapSquare { .. }
            | ClientMessage::TapHand { .. }
            | ClientMessage::TogglePromote { .. }
            | ClientMessage::FlipPiece { .. }
            | ClientMessage::SetSfen { .. }
            | ClientMessage::Preset { .. } => Some(GameMode::Shogi),
            ClientMessage::GoPlace { .. }
            | ClientMessage::GoRemove { .. }
            | ClientMessage::GoNew { .. }
            | ClientMessage::SetSgf { .. } => Some(GameMode::Go),
            ClientMessage::Ping { .. }
            | ClientMessage::SetMode { .. }
            | ClientMessage::ClearSelection
            | ClientMessage::SetTurn { .. }
            | ClientMessage::HistoryGo { .. }
            | ClientMessage::SetView { .. } => None,
        }
    }
}

/// 受信 JSON を ClientMessage として認めるかどうかだけを判定する。
/// 各コマンドの引数の妥当性 (マスが盤内か等) は適用時に見る。
/// 未知の type や壊れた JSON は None を返し、呼び出し側が黙って捨てる。
pub fn parse_client_message(text: &str) -> Option<ClientMessage> {
    serde_json::from_str(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 操作ページのコマンドを解釈できる() {
        assert!(matches!(
            parse_client_message(r#"{"type":"tap_square","square":60}"#),
            Some(ClientMessage::TapSquare { square: 60 })
        ));
        assert!(matches!(
            parse_client_message(r#"{"type":"tap_hand","side":"b","pieceKind":"P"}"#),
            Some(ClientMessage::TapHand {
                side: Side::Black,
                piece_kind: Kind::P
            })
        ));
        assert!(matches!(
            parse_client_message(r#"{"type":"clear_selection"}"#),
            Some(ClientMessage::ClearSelection)
        ));
        assert!(matches!(
            parse_client_message(r#"{"type":"ping","t":42}"#),
            Some(ClientMessage::Ping { t: 42 })
        ));
    }

    #[test]
    fn 囲碁とモード切替のコマンドを解釈できる() {
        assert!(matches!(
            parse_client_message(r#"{"type":"set_mode","mode":"go"}"#),
            Some(ClientMessage::SetMode { mode: GameMode::Go })
        ));
        assert!(matches!(
            parse_client_message(r#"{"type":"go_place","point":60}"#),
            Some(ClientMessage::GoPlace {
                point: 60,
                color: None
            })
        ));
        assert!(matches!(
            parse_client_message(r#"{"type":"go_place","point":60,"color":"w"}"#),
            Some(ClientMessage::GoPlace {
                point: 60,
                color: Some(Side::White)
            })
        ));
        assert!(matches!(
            parse_client_message(r#"{"type":"go_new","size":9}"#),
            Some(ClientMessage::GoNew { size: 9 })
        ));
        assert!(parse_client_message(r#"{"type":"set_mode","mode":"chess"}"#).is_none());
    }

    #[test]
    fn 未知の_type_と壊れた_json_は捨てる() {
        assert!(parse_client_message(r#"{"type":"drop_table"}"#).is_none());
        assert!(parse_client_message("{").is_none());
        assert!(parse_client_message("[]").is_none());
    }

    #[test]
    fn snapshot_は_client_が待つ形で出る() {
        let message = ServerMessage::Snapshot {
            app_version: APP_VERSION,
            rev: 3,
            view: ViewSettings::default(),
            history: HistoryInfo {
                index: 0,
                length: 1,
            },
            game: GameState::Shogi {
                board: crate::sfen::initial_board(),
                selection: Some(Selection::Hand {
                    side: Side::Black,
                    piece_kind: Kind::P,
                }),
                sfen: crate::sfen::HIRATE_SFEN.to_owned(),
            },
        };
        let json = serde_json::to_value(&message).expect("serialize");

        assert_eq!(json["type"], "snapshot");
        assert_eq!(json["appVersion"], APP_VERSION);
        let game = &json["game"];
        assert_eq!(game["mode"], "shogi");
        assert_eq!(game["selection"]["kind"], "hand");
        assert_eq!(game["selection"]["pieceKind"], "P");
        assert_eq!(game["board"]["moveNumber"], 1);
        assert_eq!(game["board"]["lastMove"], serde_json::Value::Null);
        // 持ち駒は 7 種すべてを必ず持つ。表示側が欠けた駒種を気にしなくてよい。
        assert_eq!(
            game["board"]["hands"]["b"].as_object().map(|o| o.len()),
            Some(7)
        );
        assert_eq!(game["board"]["squares"].as_array().map(Vec::len), Some(81));
        assert_eq!(json["view"]["margin"], 16);
        assert_eq!(json["view"]["showSelection"], false);
    }

    #[test]
    fn 囲碁の_snapshot_は盤と_sgf_を運ぶ() {
        let message = ServerMessage::Snapshot {
            app_version: APP_VERSION,
            rev: 1,
            view: ViewSettings::default(),
            history: HistoryInfo {
                index: 0,
                length: 1,
            },
            game: GameState::Go {
                board: GoBoard::initial(),
                sgf: crate::sgf::format_sgf(&GoBoard::initial()),
            },
        };
        let json = serde_json::to_value(&message).expect("serialize");
        let game = &json["game"];
        assert_eq!(game["mode"], "go");
        assert_eq!(game["board"]["size"], 19);
        assert_eq!(game["board"]["points"].as_array().map(Vec::len), Some(361));
        assert_eq!(
            game["board"]["captures"],
            serde_json::json!({"b": 0, "w": 0})
        );
        assert_eq!(game["board"]["lastMove"], serde_json::Value::Null);
        assert!(game.get("selection").is_none());
    }

    #[test]
    fn pong_と_rejected_も型どおりに出る() {
        let pong = serde_json::to_value(ServerMessage::Pong { t: 7 }).expect("serialize");
        assert_eq!(pong, serde_json::json!({"type": "pong", "t": 7}));

        let rejected = serde_json::to_value(ServerMessage::Rejected {
            reason: "だめ".to_owned(),
        })
        .expect("serialize");
        assert_eq!(
            rejected,
            serde_json::json!({"type": "rejected", "reason": "だめ"})
        );
    }
}
