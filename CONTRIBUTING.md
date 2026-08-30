# Contributing to StreamShougiBoard

バグ報告、改善提案、ドキュメント修正、コード変更を歓迎します。大きな仕様変更は、実装前に
GitHub Issue で目的と製品境界への影響を相談してください。

## 製品境界

StreamShougiBoard は、配信 PC の中だけで動くローカル完結型アプリケーションです。公開 Web サービス、
ログイン、外部データベース、LAN 向け listener を追加する変更は、通常の機能追加とは分けて
設計・レビューします。

また、これは**盤面編集ツール**であって対局エンジンではありません。合法手の判定、王手の検出、
手番の強制は入れません。任意の局面を並べられることが主な用途なので、それらを足すと本来の
使い方を壊します。

配布する exe に入る依存は、すべて許諾系ライセンスに限ります。コピーレフトを含むものは
入れません。詳しくは [docs/licensing.md](docs/licensing.md) を参照してください。

## 開発と検証

Rust はリポジトリ直下の `rust-toolchain.toml` で 1.97.1、Bun は `package.json` の
`packageManager` で 1.4.0 に固定しています。

```bash
bun install --frozen-lockfile
bash scripts/install-cargo-about.sh   # 配布バイナリを取って入れる (cargo install でも可)

bun run check        # 版の一致 → prepare:assets → 型 → lint → client のテスト
bun run watch:run    # http://127.0.0.1:16874/ が立つ

cd app
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo clippy --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings
cargo test --locked
```

`bun run check` は最初に `prepare:assets` を実行し、第三者ライセンスページと OBS 用ページを
生成します。release ビルドはこれを `rust-embed` で exe へ埋め込むので、生成前にビルドすると
ページの無い exe ができてしまいます。生成物は Git 管理しないので、変更へ含めないでください。

Windows 実行ファイルは次で作れます。

```bash
cargo build --release --locked --manifest-path app/Cargo.toml   # 今のプラットフォーム向け
bun run build:exe                                               # WSL/Linux からクロスビルド
```

クロスビルド (`x86_64-pc-windows-gnu`) は動作確認用です。配布物は Release workflow が
`windows-latest` の MSVC ターゲットで作ったものを使ってください。mingw のランタイムを
挟まないためです。

`app/build.rs` が Windows のアイコンと版情報を埋め込みます。
`app/assets/stream-shougi-board.ico` は Git 管理しているので、アイコンを変えるときだけ
作り直してください。

`bun.lock` と `app/Cargo.lock` は Pull Request 時に `bun audit` と RustSec の `cargo audit` で
検査します。同じ監査は既知の脆弱性データの更新を拾うため毎週自動実行します。

## プロトコルを変えるとき

`app/src/protocol.rs`、`client/src/protocol.ts`、`docs/protocol.md` を同時に更新し、
`PROTOCOL_VERSION` を両側で上げてから、
`UPDATE_PROTOCOL_FIXTURES=1 cargo test` で `protocol-fixtures/snapshot.json` を作り直します。
Rust 側と client 側のテストが同じ fixture を見ているので、片方だけ直すと必ず落ちます。

## 依存を足すとき

配布する exe に入る依存 (`app/Cargo.toml` の `[dependencies]` と `client` の `dependencies`) を
足したら、`bun run generate:licenses` が通ることを確認してください。検討済みのライセンス一覧の
外にあるものは、生成時にエラーになります。許諾系なら `about.toml` の `accepted` か
`scripts/generate-third-party-notices.ts` の `allowedNpmLicenseIdentifiers` へ足してください。
コピーレフトを含むものは入れません ([docs/licensing.md](docs/licensing.md))。

## リリース（メンテナー向け）

1. `app/Cargo.toml` と `package.json` のバージョン (両方同じ値) と `CHANGELOG.md` を更新して
   main へ反映する。`bun run check:version` が一致を確かめます。
2. ローカルの main を origin/main へ fast-forward し、main へ反映済みの同じ commit に
   `vX.Y.Z` タグを作成して origin へ push する。未マージ branch の commit には release tag を付けない。
3. Release workflow が検証、Windows ビルド、SHA-256 生成、GitHub Release 公開を
   完了したことを確認する。

Release workflow は完全な Git 履歴を取得し、タグが指す commit が origin/main に含まれることを
ビルド前に検証します。含まれない場合、タグと `app/Cargo.toml` のバージョンが一致しない場合、
検証・ビルド・asset upload のいずれかが失敗した場合は Release を公開しません。公開後の同じタグ・
asset の上書きは行わず、新しい修正版としてバージョンを上げてください。

配布する exe にコード署名はしていません。利用者が同一性を確かめられるよう、SHA-256 を
必ず同じ Release へ添付します。

## ライセンス

コントリビューションは、リポジトリの [MIT License](LICENSE) の下で提供されます。第三者の
コード、画像、データなどを追加する場合は、出典と再配布条件を Pull Request に記載してください。
