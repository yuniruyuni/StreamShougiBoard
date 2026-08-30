//! ローカル WebSocket プロトコル。docs/protocol.md および client/src/protocol.ts と対で更新する。
//!
//! 盤面と設定を合わせても JSON で数 KB にしかならないので、増分イベントは持たず
//! 状態が変わるたびに snapshot を丸ごと配る。盤面ページは snapshot を全置換するだけでよく、
//! 取りこぼし検出も再同期手順も要らない。

use serde::{Deserialize, Serialize};

use crate::board::{BoardState, Square};
use crate::piece::{Kind, Side};
use crate::view::ViewSettings;

/// exe に焼かれた版。ページも同じ exe から配られるので、これが食い違うのは
/// 「アプリを更新する前から開きっぱなしのページが、そのまま再接続してきた」ときだけ。
/// 別立てのプロトコル版を手で維持するより、ビルドを一意に指すこの値を配る方が確実。
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// クライアントから受け付けるフレームの上限。SFEN の貼り付けを通すため広めに取る。
pub const MAX_CLIENT_FRAME_BYTES: usize = 16 * 1024;

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
        board: BoardState,
        view: ViewSettings,
        selection: Option<Selection>,
        history: HistoryInfo,
        /// 現在局面の SFEN。操作ページの表示欄をサーバー側の正規表記で揃える。
        sfen: String,
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
            board: crate::sfen::initial_board(),
            view: ViewSettings::default(),
            selection: Some(Selection::Hand {
                side: Side::Black,
                piece_kind: Kind::P,
            }),
            history: HistoryInfo {
                index: 0,
                length: 1,
            },
            sfen: crate::sfen::HIRATE_SFEN.to_owned(),
        };
        let json = serde_json::to_value(&message).expect("serialize");

        assert_eq!(json["type"], "snapshot");
        assert_eq!(json["appVersion"], APP_VERSION);
        assert_eq!(json["selection"]["kind"], "hand");
        assert_eq!(json["selection"]["pieceKind"], "P");
        assert_eq!(json["board"]["moveNumber"], 1);
        assert_eq!(json["board"]["lastMove"], serde_json::Value::Null);
        // 持ち駒は 7 種すべてを必ず持つ。表示側が欠けた駒種を気にしなくてよい。
        assert_eq!(
            json["board"]["hands"]["b"].as_object().map(|o| o.len()),
            Some(7)
        );
        assert_eq!(json["board"]["squares"].as_array().map(Vec::len), Some(81));
        assert_eq!(json["view"]["margin"], 16);
        assert_eq!(json["view"]["showSelection"], false);
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
