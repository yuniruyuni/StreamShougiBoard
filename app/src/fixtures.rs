//! Rust と TypeScript でプロトコルの形がずれていないかを見張るための固定データ。
//!
//! Rust 側がここで書き出した JSON を、client 側のテスト
//! (`client/src/protocol.test.ts`) が読んで、表示に必要なフィールドを取り出せるか確かめる。
//! 片側だけ直すと必ずどちらかのテストが落ちる。
//!
//! 形を意図的に変えたときは `UPDATE_PROTOCOL_FIXTURES=1 cargo test` で更新する。

#![cfg(test)]

use std::path::PathBuf;

use crate::protocol::{HistoryInfo, ServerMessage};
use crate::session::Session;
use crate::sfen::format_sfen;
use crate::view::ViewSettings;
use crate::{board::square_at, piece::Side, protocol::ClientMessage};

/// fixture に書く版。実際の版とは無関係で、形が変わっていないことだけを見る。
const FIXTURE_APP_VERSION: &str = "0.0.0-fixture";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("protocol-fixtures")
        .join("snapshot.json")
}

/// 持ち駒・成駒・直前の手・選択が全部入った局面を作る。
/// ここが空の初期局面だと、抜けたフィールドに気づけない。
fn representative_session() -> Session {
    let mut session = Session::new(ViewSettings::default());
    // ▲7六歩 △3四歩 ▲2二角成
    for (from, to) in [((7, 7), (7, 6)), ((3, 3), (3, 4)), ((8, 8), (2, 2))] {
        for (file, rank) in [from, to] {
            assert_eq!(
                session.apply(ClientMessage::TapSquare {
                    square: square_at(file, rank) as i64
                }),
                None
            );
        }
    }
    // 取った直後の駒を成らせて、成駒も写し取る。
    assert_eq!(
        session.apply(ClientMessage::TogglePromote {
            square: square_at(2, 2) as i64
        }),
        None
    );
    // 打つ駒を選んだ状態も一緒に写し取る。
    assert_eq!(
        session.apply(ClientMessage::TapHand {
            side: Side::Black,
            piece_kind: crate::piece::Kind::B,
        }),
        None
    );
    session
}

fn snapshot_json() -> String {
    let session = representative_session();
    let message = ServerMessage::Snapshot {
        // 実際の版を書くと、版を上げるたびに fixture が古くなる。ここは形だけを写す。
        app_version: FIXTURE_APP_VERSION,
        rev: 6,
        board: session.board().clone(),
        view: session.view(),
        selection: session.selection().cloned(),
        history: HistoryInfo {
            index: session.history_index(),
            length: session.history_len(),
        },
        sfen: format_sfen(session.board()),
    };
    let mut json = serde_json::to_string_pretty(&message).expect("serialize snapshot");
    json.push('\n');
    json
}

#[test]
fn snapshot_の_fixture_が最新の形と一致する() {
    let expected = snapshot_json();
    let path = fixture_path();

    if std::env::var_os("UPDATE_PROTOCOL_FIXTURES").is_some() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture dir");
        }
        std::fs::write(&path, &expected).expect("write fixture");
        return;
    }

    let actual = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{} を読めません ({error})。UPDATE_PROTOCOL_FIXTURES=1 cargo test で作り直してください。",
            path.display()
        )
    });

    assert_eq!(
        actual, expected,
        "protocol-fixtures/snapshot.json が古くなっています。\
         意図した変更なら UPDATE_PROTOCOL_FIXTURES=1 cargo test で更新し、\
         client/src/protocol.ts も合わせて直してください。"
    );
}

#[test]
fn 代表局面に必要な要素が揃っている() {
    let value: serde_json::Value = serde_json::from_str(&snapshot_json()).expect("json");

    // 持ち駒 (取った角)、成駒 (馬)、直前の手、選択の 4 つが同時に写っていること。
    assert_eq!(
        value["board"]["hands"]["b"]["B"].as_array().map(Vec::len),
        Some(1)
    );
    assert!(value["sfen"]
        .as_str()
        .is_some_and(|sfen| sfen.contains("+B")));
    assert!(value["board"]["lastMove"]["from"].is_number());
    assert_eq!(value["selection"]["kind"], "hand");
}
