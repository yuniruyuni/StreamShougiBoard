//! OS ごとに違う小さな仕事だけをまとめる。失敗しても本体の動作は止めない。

use std::process::{Command, Stdio};

fn spawn_detached(program: &str, args: &[&str]) {
    let result = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Err(error) = result {
        eprintln!("StreamShougiBoard: {program} を実行できませんでした: {error}");
    }
}

/// 操作画面を既定のブラウザで開く。
pub fn open_in_browser(url: &str) {
    #[cfg(windows)]
    {
        // start の第 1 引数はウィンドウ題名として解釈されるので、空文字を挟む。
        spawn_detached("cmd", &["/c", "start", "", url]);
    }
    #[cfg(target_os = "macos")]
    {
        spawn_detached("open", &[url]);
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        spawn_detached("xdg-open", &[url]);
    }
}

/// OBS 用 URL をクリップボードへ入れる。使える手段が無い環境では何もしない。
/// 呼び出しはトレイからだけなので、Windows 以外では未使用になる。
#[cfg_attr(not(windows), allow(dead_code))]
pub fn copy_to_clipboard(text: &str) {
    #[cfg(windows)]
    {
        if let Err(error) = crate::win::clipboard::set_text(text) {
            eprintln!("StreamShougiBoard: クリップボードへコピーできませんでした: {error}");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = text;
    }
}
