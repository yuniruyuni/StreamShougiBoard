//! 盤面の正規状態を持ち、購読者へ配るハブ。
//!
//! 盤面と設定は合わせても数 KB なので、変化のたびに snapshot を丸ごと配る。
//! 購読者は受け取った snapshot で全置換すればよく、取りこぼしの検出も差分の再生も要らない。

use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::protocol::{ClientMessage, HistoryInfo, ServerMessage, APP_VERSION};
use crate::session::Session;
use crate::sfen::format_sfen;
use crate::view::ViewSettings;

/// 配信途中の snapshot を保持する上限。
/// 追いつけない購読者は最新の snapshot を 1 件受け取り直せばよいので、深く積む必要はない。
const BROADCAST_CAPACITY: usize = 32;

struct HubState {
    session: Session,
    rev: u64,
}

pub struct Hub {
    state: Mutex<HubState>,
    updates: broadcast::Sender<String>,
}

impl Hub {
    pub fn new(view: ViewSettings) -> Self {
        let (updates, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            state: Mutex::new(HubState {
                session: Session::new(view),
                rev: 0,
            }),
            updates,
        }
    }

    /// OBS 側と操作ページを合わせた現在の接続数。トレイの表示に使う。
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn subscriber_count(&self) -> usize {
        self.updates.receiver_count()
    }

    pub fn view(&self) -> ViewSettings {
        self.locked().session.view()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.updates.subscribe()
    }

    /// Mutex は同期処理しか挟まないので、poison しても最新の中身をそのまま使い続ける。
    fn locked(&self) -> std::sync::MutexGuard<'_, HubState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn snapshot_of(state: &HubState) -> ServerMessage {
        let board = state.session.board();
        ServerMessage::Snapshot {
            app_version: APP_VERSION,
            rev: state.rev,
            board: board.clone(),
            view: state.session.view(),
            selection: state.session.selection().cloned(),
            history: HistoryInfo {
                index: state.session.history_index(),
                length: state.session.history_len(),
            },
            sfen: format_sfen(board),
        }
    }

    /// 接続直後と、取りこぼした購読者の復帰に使う現在状態。
    pub fn snapshot_json(&self) -> String {
        encode(&Self::snapshot_of(&self.locked()))
    }

    /// コマンドを適用し、拒否された理由があれば返す。状態はどちらでも配り直す。
    pub fn apply(&self, command: ClientMessage) -> Option<String> {
        let payload;
        let rejected;
        {
            let mut state = self.locked();
            rejected = state.session.apply(command);
            state.rev += 1;
            payload = encode(&Self::snapshot_of(&state));
        }
        // 購読者がいなければ SendError になるが、状態はすでに更新済みなので無視してよい。
        let _ = self.updates.send(payload);
        rejected
    }
}

pub fn encode(message: &ServerMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|error| {
        // 盤面はすべて serde 由来の型なので、ここへは来ない想定。
        // 万一来ても接続を殺さず、クライアント側が捨てられる形を返す。
        eprintln!("StreamShougiBoard: snapshot を組み立てられませんでした: {error}");
        String::from(r#"{"type":"rejected","reason":"internal error"}"#)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square_at;
    use crate::piece::{Kind, Side};
    use crate::sfen::HIRATE_SFEN;
    use crate::view::BackgroundColor;

    fn snapshot_value(hub: &Hub) -> serde_json::Value {
        serde_json::from_str(&hub.snapshot_json()).expect("snapshot json")
    }

    #[test]
    fn 起動直後は平手を配る() {
        let hub = Hub::new(ViewSettings::default());
        let snapshot = snapshot_value(&hub);
        assert_eq!(snapshot["sfen"], HIRATE_SFEN);
        assert_eq!(snapshot["history"]["length"], 1);
        assert_eq!(snapshot["rev"], 0);
    }

    #[test]
    fn コマンドのたびに全購読者へ配り直す() {
        let hub = Hub::new(ViewSettings::default());
        let mut a = hub.subscribe();
        let mut b = hub.subscribe();

        assert_eq!(hub.subscriber_count(), 2);
        assert!(hub
            .apply(ClientMessage::TapSquare {
                square: square_at(7, 7) as i64
            })
            .is_none());

        for receiver in [&mut a, &mut b] {
            let payload = receiver.try_recv().expect("snapshot");
            let value: serde_json::Value = serde_json::from_str(&payload).expect("json");
            assert_eq!(value["selection"]["kind"], "square");
        }
    }

    #[test]
    fn 拒否された理由を返しつつ状態は配り直す() {
        let hub = Hub::new(ViewSettings::default());
        let mut receiver = hub.subscribe();

        let rejected = hub.apply(ClientMessage::Preset {
            name: "存在しない".to_owned(),
        });

        assert!(rejected.is_some());
        let payload = receiver.try_recv().expect("snapshot");
        let value: serde_json::Value = serde_json::from_str(&payload).expect("json");
        assert_eq!(value["sfen"], HIRATE_SFEN);
    }

    #[test]
    fn rev_はコマンドのたびに増える() {
        let hub = Hub::new(ViewSettings::default());
        assert_eq!(snapshot_value(&hub)["rev"], 0);
        hub.apply(ClientMessage::ClearSelection);
        assert_eq!(snapshot_value(&hub)["rev"], 1);
    }

    #[test]
    fn 起動時の見た目設定を引き継ぐ() {
        let view = ViewSettings {
            background_color: BackgroundColor::White,
            margin: 40,
            ..ViewSettings::default()
        };
        let hub = Hub::new(view);
        let snapshot = snapshot_value(&hub);
        assert_eq!(snapshot["view"]["backgroundColor"], "white");
        assert_eq!(snapshot["view"]["margin"], 40);
    }

    #[test]
    fn 溜め込んだ購読者は取りこぼしを検出できる() {
        let hub = Hub::new(ViewSettings::default());
        let mut slow = hub.subscribe();

        // 受け取らないまま上限を超えて配ると、古い分は落ちる。
        for _ in 0..(BROADCAST_CAPACITY + 4) {
            hub.apply(ClientMessage::ClearSelection);
        }

        // 呼び出し側はここで現在の snapshot を送り直して追いつかせる。
        assert!(matches!(
            slow.try_recv(),
            Err(broadcast::error::TryRecvError::Lagged(_))
        ));
        assert!(hub.subscriber_count() >= 1);
    }

    #[test]
    fn 見た目の変更は履歴を増やさず設定へ入る() {
        let hub = Hub::new(ViewSettings::default());
        hub.apply(ClientMessage::SetView {
            view: serde_json::json!({ "backgroundOpacity": 45, "margin": 99999 }),
        });

        let snapshot = snapshot_value(&hub);
        assert_eq!(snapshot["view"]["backgroundOpacity"], 45);
        // 範囲外は上限へ丸める。
        assert_eq!(snapshot["view"]["margin"], 200);
        assert_eq!(snapshot["history"]["length"], 1);
        assert_eq!(hub.view().background_opacity, 45);
    }

    #[test]
    fn 玉は駒台へ送れず理由が返る() {
        let hub = Hub::new(ViewSettings::default());
        hub.apply(ClientMessage::TapSquare {
            square: square_at(5, 9) as i64,
        });
        let rejected = hub.apply(ClientMessage::TapHand {
            side: Side::Black,
            piece_kind: Kind::P,
        });
        assert_eq!(rejected.as_deref(), Some("その駒は駒台へ送れません"));
    }
}
