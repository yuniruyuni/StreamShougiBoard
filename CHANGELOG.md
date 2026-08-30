# Changelog

このファイルは主な変更だけを記録します。書式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/)、
バージョンは [Semantic Versioning](https://semver.org/lang/ja/) に従います。

## [Unreleased]

## [0.1.0] - 2026-08-30

最初のリリース。

本体は Rust、OBS へ出すページと操作画面は TypeScript + React です。配布する exe に入るものを
すべて許諾系ライセンスに保つための構成で、経緯は [docs/licensing.md](docs/licensing.md) に
残しています。

### Added

- 配信 PC の中だけで動く将棋盤。`127.0.0.1` に HTTP/WebSocket サーバーを持ち、
  操作画面と OBS ブラウザソース用の盤面ページを同じ origin から配る。
- 盤面編集の操作一式。クリックで選択・移動、駒台への出し入れ、持ち駒を打つ、
  ダブルクリックで成／不成、右クリックで先後の反転。合法手の判定はしない。
- 平手・全駒・空の盤のプリセット、SFEN の表示・コピー・読み込み、局面の保存。
- 履歴の前後移動（← → キーとプルダウン）。
- OBS 側の見た目設定。盤と駒台を囲む地の色（白／黒）と濃さ、駒台の位置、外周の余白、
  直前の手のハイライト、筋・段の番号、盤の反転、駒の移動アニメーション。
  設定は次の起動へ引き継ぐ。
- 盤面ページはブラウザソースの領域いっぱいに盤を広げ、上下左右の中央へ置く。
  大きさは領域から決まるので設定に持たない。半透明の地は領域いっぱいに敷く。
- Windows のタスクトレイ常駐。操作画面を開く、OBS 用 URL をコピー、第三者ライセンスを開く、終了。
  tooltip に OBS 側の接続数を出す。
- 配布する exe に入る第三者コンポーネントのライセンスページ（`/licenses`）。

[Unreleased]: https://github.com/yuniruyuni/StreamShougiBoard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/yuniruyuni/StreamShougiBoard/releases/tag/v0.1.0
