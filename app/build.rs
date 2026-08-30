fn main() {
    // build script はホスト用にコンパイルされるため #[cfg(windows)] では
    // ターゲット判定できない。Cargo が渡す CARGO_CFG_WINDOWS で判定する。
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = embed_manifest::new_manifest("StreamShougiBoard").to_string();
        let mut resource = winresource::WindowsResource::new();
        resource
            .set("ProductName", "StreamShougiBoard")
            .set("CompanyName", "yuniruyuni")
            .set("LegalCopyright", "Copyright (c) 2026 Yuniruyuni")
            .set("FileDescription", "OBS 連携ローカル将棋盤")
            .set("InternalName", "stream-shougi-board.exe")
            .set("OriginalFilename", "stream-shougi-board.exe")
            .set_icon("assets/stream-shougi-board.ico")
            .set_manifest(&manifest);
        resource
            .compile()
            .expect("failed to embed Windows resources");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/stream-shougi-board.ico");
}
