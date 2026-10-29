//! Rust と TypeScript でプロトコルの形がずれていないかを見張るための固定データ。
//!
//! Rust 側がここで書き出した JSON を、client 側のテスト
//! (`client/src/protocol.test.ts`) が読んで、表示に必要なフィールドを取り出せるか確かめる。
//! 片側だけ直すと必ずどちらかのテストが落ちる。
//!
//! 将棋と囲碁で盤の形が違うので、モードごとに 1 つずつ書き出す。
//!
//! 形を意図的に変えたときは `UPDATE_PROTOCOL_FIXTURES=1 cargo test` で更新する。

#![cfg(test)]

use std::path::PathBuf;

use crate::protocol::GameMode;
use crate::session::Session;
use crate::view::ViewSettings;
use crate::{board::square_at, piece::Side, protocol::ClientMessage};

/// fixture に書く版。実際の版とは無関係で、形が変わっていないことだけを見る。
const FIXTURE_APP_VERSION: &str = "0.0.0-fixture";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("protocol-fixtures")
        .join(name)
}

/// 持ち駒・成駒・直前の手・選択が全部入った局面を作る。
/// ここが空の初期局面だと、抜けたフィールドに気づけない。
fn representative_session() -> Session {
    let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
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

/// 9 路で、黒が白 1 子を取った直後の局面。アゲハマ・直前の手・手番が全部入る。
fn representative_go_session() -> Session {
    let mut session = Session::new(ViewSettings::default(), GameMode::Shogi);
    for command in [
        ClientMessage::SetMode { mode: GameMode::Go },
        ClientMessage::GoNew { size: 9 },
    ] {
        assert_eq!(session.apply(command), None);
    }
    // 黒 d5 → 白 e5 → 黒 e4 … と交互に打ち、最後の黒 e6 で白 e5 を取る。
    for point in [31, 40, 39, 0, 41, 8, 49] {
        assert_eq!(
            session.apply(ClientMessage::GoPlace { point, color: None }),
            None
        );
    }
    // 手番を動かさない配置も 1 つ混ぜる。
    assert_eq!(
        session.apply(ClientMessage::GoPlace {
            point: 80,
            color: Some(Side::White),
        }),
        None
    );
    session
}

fn to_json(session: &Session, rev: u64) -> String {
    // 実際の版を書くと、版を上げるたびに fixture が古くなる。ここは形だけを写す。
    let message = session.snapshot(FIXTURE_APP_VERSION, rev);
    let mut json = serde_json::to_string_pretty(&message).expect("serialize snapshot");
    json.push('\n');
    json
}

fn snapshot_json() -> String {
    to_json(&representative_session(), 6)
}

fn go_snapshot_json() -> String {
    to_json(&representative_go_session(), 10)
}

fn check_fixture(name: &str, expected: &str) {
    let path = fixture_path(name);

    if std::env::var_os("UPDATE_PROTOCOL_FIXTURES").is_some() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture dir");
        }
        std::fs::write(&path, expected).expect("write fixture");
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
        "protocol-fixtures/{name} が古くなっています。\
         意図した変更なら UPDATE_PROTOCOL_FIXTURES=1 cargo test で更新し、\
         client/src/protocol.ts も合わせて直してください。"
    );
}

#[test]
fn snapshot_の_fixture_が最新の形と一致する() {
    check_fixture("snapshot.json", &snapshot_json());
}

#[test]
fn 囲碁の_snapshot_の_fixture_が最新の形と一致する() {
    check_fixture("go-snapshot.json", &go_snapshot_json());
}

#[test]
fn 代表局面に必要な要素が揃っている() {
    let value: serde_json::Value = serde_json::from_str(&snapshot_json()).expect("json");

    let game = &value["game"];

    // 持ち駒 (取った角)、成駒 (馬)、直前の手、選択の 4 つが同時に写っていること。
    assert_eq!(game["mode"], "shogi");
    assert_eq!(
        game["board"]["hands"]["b"]["B"].as_array().map(Vec::len),
        Some(1)
    );
    assert!(game["sfen"]
        .as_str()
        .is_some_and(|sfen| sfen.contains("+B")));
    assert!(game["board"]["lastMove"]["from"].is_number());
    assert_eq!(game["selection"]["kind"], "hand");
}

#[test]
fn 囲碁の代表局面に必要な要素が揃っている() {
    let value: serde_json::Value = serde_json::from_str(&go_snapshot_json()).expect("json");
    let game = &value["game"];

    // 取られた白 e5 (40) は空点に戻り、黒のアゲハマが 1。手番は白、直前の手は色指定の白。
    assert_eq!(game["mode"], "go");
    assert_eq!(game["board"]["size"], 9);
    assert_eq!(game["board"]["points"][40], serde_json::Value::Null);
    assert_eq!(game["board"]["points"][49], "b");
    assert_eq!(game["board"]["captures"]["b"], 1);
    assert_eq!(game["board"]["turn"], "w");
    assert_eq!(game["board"]["lastMove"], 80);
    assert!(game["sgf"]
        .as_str()
        .is_some_and(|sgf| sgf.contains("SZ[9]")));
}
