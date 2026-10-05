fn main() {
    // build script はホスト用にコンパイルされるため #[cfg(windows)] では
    // ターゲット判定できない。Cargo が渡す CARGO_CFG_WINDOWS で判定する。
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = embed_manifest::new_manifest("StreamShougiBoard").to_string();
        // set_manifest() は行ごとに空白で包むため、UTF-8 BOM と XML 宣言が
        // 先頭バイトから押し出され、SxS の活性化コンテキストが不正になる
        // (「サイドバイサイド構成が正しくありません」で起動できない)。
        // ファイルへ書き出して、1 バイトも変えずに埋め込む。
        let manifest_path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("stream-shougi-board.manifest");
        std::fs::write(&manifest_path, manifest.as_bytes())
            .expect("failed to write Windows manifest");
        let mut resource = winresource::WindowsResource::new();
        resource
            .set("ProductName", "StreamShougiBoard")
            .set("CompanyName", "yuniruyuni")
            .set("LegalCopyright", "Copyright (c) 2026 Yuniruyuni")
            .set("FileDescription", "OBS 連携ローカル将棋盤")
            .set("InternalName", "stream-shougi-board.exe")
            .set("OriginalFilename", "stream-shougi-board.exe")
            .set_icon("assets/stream-shougi-board.ico")
            .set_manifest_file(manifest_path.to_str().expect("manifest path must be UTF-8"));
        resource
            .compile()
            .expect("failed to embed Windows resources");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/stream-shougi-board.ico");
}
