# Licensing

StreamShougiBoard 本体は [MIT License](../LICENSE) です。ここでは、配布する Windows 実行ファイルに
**実際に入っているもの**をどう洗い出して表示しているかを記録します。

## 方針

配布物に入るものは、すべて許諾系ライセンス (MIT / Apache-2.0 / BSD / ISC など) に限ります。
コピーレフトを入れません。単一 exe の配布と、LGPL のような再リンク可能な形での提供の要求は
相性が悪く、利用者にとっても曖昧さが残るためです。

この方針のために本体を Rust にしました。JavaScript ランタイムを静的リンクする方式では、
LGPL-2.1 の JavaScriptCore が必ず付いてきます。

## 配布物に入る第三者コンポーネント

| コンポーネント | 含まれ方 | ライセンス |
| --- | --- | --- |
| Cargo の実行時依存 (axum / tokio / serde / rust-embed / windows など) | 実行ファイル本体へ静的リンク | MIT / Apache-2.0 / BSD / Unicode-3.0 など |
| react / react-dom / scheduler | JS バンドル (`control.js` / `board.js`) | MIT |

`build-dependencies` と `dev-dependencies`、npm の `devDependencies` はビルドにしか使わないので
対象外です。

Windows の MSVC ターゲットで作るので、mingw のランタイムも入りません。

## 生成のしくみ

`scripts/generate-third-party-notices.ts` が
`app/assets/third-party-licenses.generated.html` を書き出し、client のビルドが `static/licenses.html`
として取り込み、`rust-embed` が exe へ埋め込みます。実行中は次から開けます。

- タスクトレイの「第三者ライセンス...」
- 操作画面のフッターのリンク
- `http://127.0.0.1:<port>/licenses`

Cargo 側は `cargo-about 0.9.1` に `about.toml` の許諾リストで検証させ、npm 側は
`client/package.json` の実行時依存を推移的にたどります。ライセンス本文は、どちらも実際に
同梱されている `LICENSE` などのファイルから読み取ります。

検討済みのライセンス一覧の外にあるものが混ざると、生成は**エラーで止まります**。黙って
掲載を落とすより、依存を足した人がその場で気づく方が安全なためです。

ページの先頭には `Cargo.lock` と `bun.lock` の SHA-256 を載せます。表示されている一覧が、
どのロック状態から生成されたものかを後から照合できます。

## 依存を足すとき

1. `cargo add` または `bun add` する。
2. `bun run generate:licenses` が通ることを確認する。
3. 通らない場合は、そのライセンス本文を読む。許諾系なら `about.toml` の `accepted` か
   `scripts/generate-third-party-notices.ts` の `allowedNpmLicenseIdentifiers` へ足す。
   コピーレフトなら**足さず、依存自体を見直してください**。
