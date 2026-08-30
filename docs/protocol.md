# Local WebSocket protocol

`app/src/protocol.rs` と `client/src/protocol.ts` の 3 者で対になっています。
どれかを変えたら残りも直し、`UPDATE_PROTOCOL_FIXTURES=1 cargo test` で
`protocol-fixtures/snapshot.json` を作り直してください。Rust 側と client 側のテストが
同じ fixture を見ているので、片方だけ直すと必ずどちらかが落ちます。

## Transport

- HTTP origin: `http://127.0.0.1:<port>` (既定 16874)
- WebSocket: `ws://127.0.0.1:<port>/ws`
- JSON テキストフレーム
- 外部公開・TLS・認証トークンなし

TLS を使わないのは通信が同じ PC の中で閉じているためです。代わりにサーバーは bind 先、Host、
Origin を検証します ([security.md](security.md))。

## 座標

マスは 0..80 の整数です。SFEN の走査順に合わせ、`0` が 9一 (左上)、`8` が 1一、`80` が 1九です。
筋 `f` (1..9、右から) と段 `r` (1..9、上から) からは `(r - 1) * 9 + (9 - f)` で求まります。

盤の反転は表示だけの話なので、プロトコルには現れません。

## Connection lifecycle

接続直後、サーバーは必ず現在状態を送ります。以降も、状態が変わるたびに同じ形のものを送ります。
増分イベントはありません。

```json
{
  "type": "snapshot",
  "appVersion": "0.1.0",
  "rev": 12,
  "board": {
    "squares": [null, { "id": "p3", "kind": "N", "promoted": false, "side": "w" }, "…81 要素"],
    "hands": { "b": { "R": [], "B": [], "G": [], "S": [], "N": [], "L": [], "P": [] }, "w": {} },
    "turn": "b",
    "moveNumber": 3,
    "lastMove": { "from": 60, "to": 51 }
  },
  "view": { "backgroundColor": "black", "backgroundOpacity": 0, "margin": 16, "…": "" },
  "selection": { "kind": "square", "square": 60 },
  "history": { "index": 2, "length": 3 },
  "sfen": "lnsgkgsnl/1r5b1/ppppppppp/9/9/2P6/PP1PPPPPP/1B5R1/LNSGKGSNL w - 2"
}
```

`appVersion` は snapshot を送ってきた exe の版です。ページも同じ exe から配られるので、
本来これがページ側の版と食い違うことはありません。食い違うのは**アプリを更新する前から
開きっぱなしのページが、そのまま再接続してきたとき**だけです (切断は日常として黙って繋ぎ直す
作りなので、古い JS のまま動き続けてしまいます)。そのときクライアントはページを 1 回だけ
自動で読み直し、新しいページに入れ替わります。読み直しても揃わなければ、繰り返さずに
コンソールへ出して止まります。

プロトコル版を別に持たないのは、ページと exe が必ず同じビルドから出てくるためです。
番号を手で維持する代わりに、ビルドを一意に指すアプリの版をそのまま配っています。

`board` と `view` は 1 つの文書として同時に全置換します。

`selection` は操作中に選んでいる駒です。盤面ページは `view.showSelection` が真のときだけ描きます。
既定は偽で、手元の選択枠は配信に出ません。

`lastMove` の `to` は、盤上から駒台へ送ったときに `null` になります。`from` は持ち駒を打ったときに
`null` になります。

`board.squares` の各駒が持つ `id` は、その駒が盤と駒台を行き来しても変わりません。表示側は
これを手掛かりに同じ駒として追い、移動を補間します。SFEN には現れない値なので、SFEN を
読み込むと振り直されます。

## Client → server

盤面ページが送るのは `ping` だけです。操作画面はこれに加えてコマンドを送ります。

```json
{"type":"ping","t":1785380000000}

{"type":"tap_square","square":60}
{"type":"tap_hand","side":"b","pieceKind":"P"}
{"type":"toggle_promote","square":19}
{"type":"flip_piece","square":19}
{"type":"clear_selection"}
{"type":"set_sfen","sfen":"lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1"}
{"type":"preset","name":"hirate"}
{"type":"set_turn","side":"w"}
{"type":"history_go","index":1}
{"type":"set_view","view":{"backgroundColor":"white","backgroundOpacity":40,"margin":40}}
```

`tap_square` と `tap_hand` は「盤や駒台を叩いた」という事実だけを伝えます。それが選択なのか
移動なのか打ち込みなのかは、サーバー側の選択状態を見て `app/src/session.rs` が決めます。
判定をサーバーへ寄せているので、操作画面を 2 枚開いても解釈が割れません。

`preset` の `name` は `hirate` (平手)、`allPieces` (全駒)、`empty` (空の盤) です。

`set_view` は部分更新です。知らないキーは落とし、範囲外の数値は丸めます。配信中に設定を触って
盤が消えるより、無害な値へ落ちる方が良いためです。

`backgroundColor` は `white` か `black`、`backgroundOpacity` は 0〜100 (%) です。盤と駒台と駒は
常に不透明で、この地が敷かれるのはそれらを囲む外側だけです。0 なら地を塗らないので、OBS では
盤と駒台だけが映像の上に乗ります。

盤の大きさは設定に含みません。盤面ページがブラウザソースの領域から決めて中央へ置き、
`margin` はその領域の縁から何 px 空けるかを表します。

未知の `type` は黙って捨てます。

## Server → client

```json
{"type":"pong","t":1785380000000}
{"type":"rejected","reason":"その駒は成れません"}
```

`rejected` は、編集として成立しなかったことを送り主にだけ返します。状態そのものは常に snapshot が
運ぶので、拒否されたときも直後に snapshot が届きます (中身は変わりません)。

## Liveness

クライアントは 15 秒ごとに `ping` を送り、30 秒 何も届かなければ接続が死んだとみなして繋ぎ直します。
再接続は指数バックオフ (500ms 〜 15s、±20% のゆらぎ) で行い、**snapshot を受理した時点**で
バックオフを戻します。WebSocket が open しただけでは、直後に落ちる場合を成功と
誤認してしまうためです。

盤面ページは切断しても 3 秒は表示を保ち、それを超えたら盤を消します。アプリの再起動で一瞬切れる
たびに盤が明滅すると配信に出せない一方、アプリを終了したのに古い盤面が残り続けるのも困るためです。
