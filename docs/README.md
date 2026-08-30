# Documentation

- [architecture.md](architecture.md): 状態の持ち主、プロセス構成、各ワークスペースの責務
- [protocol.md](protocol.md): ローカル WebSocket プロトコル
- [security.md](security.md): loopback サービスの脅威モデルと防御
- [licensing.md](licensing.md): 配布物に入る第三者コンポーネントと、その表示

本体は Rust、OBS へ出すページと操作画面は TypeScript + React です。
配布物に入るものをすべて許諾系ライセンスに保つための選択で、経緯は
[licensing.md](licensing.md) に書いています。

製品境界は「ローカル完結・`127.0.0.1` の HTTP/WebSocket・OBS ブラウザソースが同一 origin から
ページと WS を読む・接続時 snapshot で全置換」に絞っています。配信 PC の中だけで閉じれば、
アカウントも外部通信も要らず、配信が止まる原因をひとつ減らせるためです。

盤の操作をどう決めたかは [architecture.md](architecture.md) に理由を書いています。
