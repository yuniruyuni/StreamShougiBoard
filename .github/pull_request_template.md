## 変更内容

<!-- 何を、なぜ変更したかを簡潔に説明してください。 -->

## 検証

<!-- 実行したコマンドと、手動で確認した環境・操作を記載してください。 -->

- [ ] `bun run check`
- [ ] `cd app && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- [ ] OBS 側の表示に関わる変更では、実際に `/board` をブラウザソースへ入れて確認した

## 確認事項

- [ ] 公開 Web サービス、ログイン、外部 DB、LAN listener を意図せず追加していません。
- [ ] 合法手の判定・王手の検出・手番の強制を追加していません（盤面編集ツールとしての境界）。
- [ ] プロトコルを変えた場合、`app/src/protocol.rs` と `client/src/protocol.ts` と
      `docs/protocol.md` を同時に更新し、`UPDATE_PROTOCOL_FIXTURES=1 cargo test` で
      fixture も作り直しました。
- [ ] 配布物に入る依存を足した場合、`bun run generate:licenses` が通ることを確認しました。
- [ ] ユーザーデータや秘密情報をコミットしていません。
