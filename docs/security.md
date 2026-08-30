# Security

## 脅威モデル

ローカル Web サービスとして、主に次の 2 つを想定します。

1. LAN 上の別端末が盤面や WebSocket へ到達すること
2. ブラウザで開いた悪意あるページが localhost の WebSocket へ繋ぐこと

同じ Windows ユーザー権限で動くネイティブなマルウェアからの防御は対象外です。

## 防御

- listener は IPv4 loopback `127.0.0.1` にだけ bind する
- `0.0.0.0` や LAN アドレスへ変えられる設定を提供しない
- HTTP の `Host` は `127.0.0.1:<port>` と `localhost:<port>` だけを許可する
- WebSocket の `Origin` は同じ HTTP origin だけを許可する
- `Origin` の無い WebSocket upgrade も拒否する。ブラウザは必ず付けてくるので、
  無いものは非ブラウザからの接続とみなす
- HTML に厳しい Content-Security-Policy を付ける (`default-src 'none'`、`script-src 'self'`、
  `connect-src` はこの loopback の WebSocket だけ)。インラインの script も style も使わない
- 外部 CDN へ一切接続しない。React も含めて全て自前でバンドルし、exe へ埋め込む
- 配るのは生成済みアセット名と完全一致するパスだけで、任意のパスをファイルシステムへ渡さない
- 第三者ライセンスページはアプリ本体より厳しい CSP で配る。生成物なので style をインラインで
  持つ代わりに、script も外部接続も一切許さない
- 受信フレームは 16 KiB までに制限する (SFEN の貼り付けを通すため 4 KiB より広い)
- 受け付けるメッセージは `ping` と既知のコマンドだけ。未知の `type` は黙って捨てる
- 遅い購読者は送信待ちが 1 MiB を超えた時点で切り、再接続時の snapshot で追いつかせる
- HTTP は `GET` と `HEAD` だけを受ける
- レスポンスに `no-store`、`nosniff`、`no-referrer` を付ける

Bearer token は loopback では秘密になりにくく、配布と更新も複雑にするので使いません。
LAN 経由は bind 先と Host 検証で、ブラウザ経由は Origin 検証で塞ぎます。

## 外向きの通信

通常動作では一切ありません。更新確認も送信も行いません。

タスクトレイは Win32 API を直接呼ぶだけで、子プロセスも一時ファイルも作りません。

## 保存するもの

- `%APPDATA%\StreamShougiBoard\config.json`: ポート、起動時に操作画面を開くか、OBS 側の見た目の設定
- 操作画面の `localStorage`: 利用者が明示的に保存した局面の SFEN

盤面そのものは保存しません。毎回まっさらな平手から始まります。

## 運用上の注意

Windows ファイアウォールで外部からの受信規則を作る必要はありません。もし実行時に公開ネットワーク
向けの許可を求められたら、拒否して構いません。

別の PC から盤面を見る用途が必要になった場合は、この listener を公開するのではなく、認証・TLS・
レート制限を備えた別のリレーとして設計してください。
