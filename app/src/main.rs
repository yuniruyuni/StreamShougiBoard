//! StreamShougiBoard 本体のエントリ。
//!
//! ローカル完結型で、127.0.0.1 のサーバーと、それを操作するタスクトレイだけを持つ。
//! 外向きの listener もクラウドもアカウントも持たない (docs/security.md)。

// トレイ常駐なので、Windows では起動時にコンソール窓を出さない。
#![cfg_attr(windows, windows_subsystem = "windows")]

mod board;
mod config;
mod fixtures;
mod net;
mod piece;
mod platform;
mod protocol;
mod session;
mod sfen;
mod view;

#[cfg(windows)]
mod win;

use std::sync::Arc;

use anyhow::Result;

use crate::net::Hub;

/// 開発中はファイルを保存するたびに再起動するので、そのたびにタブが増えないようにする。
const NO_OPEN_ENV: &str = "STREAM_SHOUGI_BOARD_NO_OPEN";

pub struct Urls {
    pub control: String,
    pub board: String,
    pub licenses: String,
}

fn main() -> Result<()> {
    let mut config = config::load();

    // 実際のポートを先に知る必要があるので、bind だけ同期で済ませる。
    let (listener, started) = net::server::bind(config.port)?;
    let port = started.port;
    let hub = Arc::new(Hub::new(config.view));

    let urls = Urls {
        control: format!("http://{}:{port}", net::server::LOOPBACK_HOST),
        board: format!("http://{}:{port}/board", net::server::LOOPBACK_HOST),
        licenses: format!("http://{}:{port}/licenses", net::server::LOOPBACK_HOST),
    };

    println!("StreamShougiBoard: 操作画面 {}", urls.control);
    println!("StreamShougiBoard: OBS ブラウザソース {}", urls.board);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server_hub = hub.clone();
    let server = std::thread::spawn(move || -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(net::serve(listener, server_hub, port, shutdown_rx))
    });

    if config.auto_open_control && std::env::var_os(NO_OPEN_ENV).is_none() {
        platform::open_in_browser(&urls.control);
    }

    // 送信端は join を抜けるまで生かしておく。ここで drop すると受信側が即座に解決してしまい、
    // 何もしていないのに graceful shutdown が走る。
    let shutdown_tx = {
        #[cfg(windows)]
        {
            // トレイのメッセージループが「終了」まで main スレッドを占有する。
            win::run(&hub, &urls)?;
            let _ = shutdown_tx.send(());
            None::<tokio::sync::oneshot::Sender<()>>
        }
        #[cfg(not(windows))]
        {
            // Ctrl+C はサーバー側で受ける。
            eprintln!("StreamShougiBoard: 終了するには Ctrl+C を押してください。");
            Some(shutdown_tx)
        }
    };

    let result = server.join();
    drop(shutdown_tx);

    match result {
        Ok(result) => result?,
        Err(_) => anyhow::bail!("サーバースレッドが異常終了しました"),
    }

    // 見た目の設定は配信をまたいで残す。終了時に一度だけ書き戻す。
    let view = hub.view();
    if view != config.view {
        config.view = view;
        config::save(&config);
    }

    Ok(())
}
