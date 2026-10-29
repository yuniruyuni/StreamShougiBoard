# Local WebSocket protocol

`app/src/protocol.rs` と `client/src/protocol.ts` の 3 者で対になっています。
どれかを変えたら残りも直し、`UPDATE_PROTOCOL_FIXTURES=1 cargo test` で
`protocol-fixtures/snapshot.json` (将棋) と `protocol-fixtures/go-snapshot.json` (囲碁) を作り直してください。Rust 側と client 側のテストが
同じ fixture を見ているので、片方だけ直すと必ずどちらかが落ちます。

## OBSへの表示・非表示

`view.visible` はOBS用の盤面・持ち駒・背景の表示を切り替える真偽値です。
`{"type":"set_view","view":{"visible":false}}` で全体を隠し、`true` で再表示します。
操作画面の盤、局面、選択、Undo/Redoの履歴は変えません。
設定は終了時に保存され、既存の `visible:false` も更新後に維持されます。
この項目のない従来設定では `true` を既定値にします。

## Transport

- HTTP origin: `http://127.0.0.1:<port>` (既定 16874)
- WebSocket: `ws://127.0.0.1:<port>/ws`
- JSON テキストフレーム
- 外部公開・TLS・認証トークンなし

TLS を使わないのは通信が同じ PC の中で閉じているためです。代わりにサーバーは bind 先、Host、
Origin を検証します ([security.md](security.md))。

## 座標

将棋のマスは 0..80 の整数です。SFEN の走査順に合わせ、`0` が 9一 (左上)、`8` が 1一、`80` が 1九です。
筋 `f` (1..9、右から) と段 `r` (1..9、上から) からは `(r - 1) * 9 + (9 - f)` で求まります。

囲碁の交点は 0..size*size-1 の整数です。SGF の座標と同じ向きで、`0` が左上、行ごとに左から右へ
数えます。列 `c` (0 始まり、左から) と行 `r` (0 始まり、上から) からは `r * size + c` で求まります。
画面に出す「D4」のような座標 (I を飛ばした英字と、下から数える数字) は表示側だけで作ります。

盤の反転は表示だけの話なので、プロトコルには現れません。

## Connection lifecycle

接続直後、サーバーは必ず現在状態を送ります。以降も、状態が変わるたびに同じ形のものを送ります。
増分イベントはありません。

```json
{
  "type": "snapshot",
  "appVersion": "0.1.0",
  "rev": 12,
  "view": { "backgroundColor": "black", "backgroundOpacity": 0, "margin": 16, "…": "" },
  "history": { "index": 2, "length": 3 },
  "game": {
    "mode": "shogi",
    "board": {
      "squares": [null, { "id": "p3", "kind": "N", "promoted": false, "side": "w" }, "…81 要素"],
      "hands": { "b": { "R": [], "B": [], "G": [], "S": [], "N": [], "L": [], "P": [] }, "w": {} },
      "turn": "b",
      "moveNumber": 3,
      "lastMove": { "from": 60, "to": 51 }
    },
    "selection": { "kind": "square", "square": 60 },
    "sfen": "lnsgkgsnl/1r5b1/ppppppppp/9/9/2P6/PP1PPPPPP/1B5R1/LNSGKGSNL w - 2"
  }
}
```

囲碁モードのときは `game` が次の形になります。

```json
{
  "mode": "go",
  "board": {
    "size": 19,
    "points": [null, "b", "w", "…361 要素"],
    "captures": { "b": 1, "w": 0 },
    "turn": "w",
    "moveNumber": 8,
    "lastMove": 49
  },
  "sgf": "(;GM[1]FF[4]CA[UTF-8]SZ[19]AB[ba]AW[ca]PL[W])"
}
```

`game.mode` は `shogi` か `go` です。サーバーは将棋と囲碁の局面・履歴を別々に持っていて、
snapshot には表に出ているほうだけを載せます。`history` もそのモードの履歴です。

`appVersion` は snapshot を送ってきた exe の版です。ページも同じ exe から配られるので、
本来これがページ側の版と食い違うことはありません。食い違うのは**アプリを更新する前から
開きっぱなしのページが、そのまま再接続してきたとき**だけです (切断は日常として黙って繋ぎ直す
作りなので、古い JS のまま動き続けてしまいます)。そのときクライアントはページを 1 回だけ
自動で読み直し、新しいページに入れ替わります。読み直しても揃わなければ、繰り返さずに
コンソールへ出して止まります。

プロトコル版を別に持たないのは、ページと exe が必ず同じビルドから出てくるためです。
番号を手で維持する代わりに、ビルドを一意に指すアプリの版をそのまま配っています。

`game` と `view` は 1 つの文書として同時に全置換します。

`selection` (将棋だけ) は操作中に選んでいる駒です。盤面ページは `view.showSelection` が真のときだけ描きます。
既定は偽で、手元の選択枠は配信に出ません。

`lastMove` の `to` は、盤上から駒台へ送ったときに `null` になります。`from` は持ち駒を打ったときに
`null` になります。

`captures` は取った石の数 (アゲハマ) で、`b` は黒が取った白石の数です。`lastMove` は直前に置いた石の
交点で、その石が取り除かれると `null` になります。`sgf` は現在局面を配置 (`AB` / `AW` / `PL`) だけで
表したもので、アゲハマの数と手数は含みません。

`board.squares` の各駒が持つ `id` は、その駒が盤と駒台を行き来しても変わりません。表示側は
これを手掛かりに同じ駒として追い、移動を補間します。SFEN には現れない値なので、SFEN を
読み込むと振り直されます。

## Client → server

盤面ページが送るのは `ping` だけです。操作画面はこれに加えてコマンドを送ります。

```json
{"type":"ping","t":1785380000000}

{"type":"set_mode","mode":"go"}

{"type":"tap_square","square":60}
{"type":"tap_hand","side":"b","pieceKind":"P"}
{"type":"toggle_promote","square":19}
{"type":"flip_piece","square":19}
{"type":"clear_selection"}
{"type":"set_sfen","sfen":"lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1"}
{"type":"preset","name":"hirate"}
{"type":"go_place","point":60}
{"type":"go_place","point":60,"color":"w"}
{"type":"go_remove","point":60}
{"type":"go_new","size":13}
{"type":"set_sgf","sgf":"(;GM[1]SZ[19];B[pd];W[dp])"}

{"type":"set_turn","side":"w"}
{"type":"history_go","index":1}
{"type":"set_view","view":{"backgroundColor":"white","backgroundOpacity":40,"margin":40}}
```

`tap_square` と `tap_hand` は「盤や駒台を叩いた」という事実だけを伝えます。それが選択なのか
移動なのか打ち込みなのかは、サーバー側の選択状態を見て `app/src/session.rs` が決めます。
判定をサーバーへ寄せているので、操作画面を 2 枚開いても解釈が割れません。

`preset` の `name` は `hirate` (平手)、`allPieces` (全駒)、`empty` (空の盤) です。

`go_place` は `color` を省くと手番の石を置いて手番を渡し、手数を 1 つ進めます。`color` を付けると
手番も手数も動かさずにその色を置きます (置石や詰碁を並べる用)。相手の石の上なら置き換え、同じ色の
石の上なら何もしません。どちらの場合も、呼吸点の無くなった相手の連は取ってアゲハマに数えます。
`go_remove` は石を取り除くだけで、アゲハマには数えません。

`go_new` は 2〜19 路の空の盤で履歴を作り直します。`set_sgf` は本譜 (各分岐の最初の枝) を読み、
根の節点の配置と 1 手ごとの局面を履歴へ並べて、最後の局面を表示します。

`tap_square` / `tap_hand` / `toggle_promote` / `flip_piece` / `set_sfen` / `preset` は将棋モード、
`go_*` / `set_sgf` は囲碁モードでだけ受け付け、それ以外のときは `rejected` を返します。
`set_turn` / `history_go` / `clear_selection` / `set_view` は表に出ているモードへ効きます。
`set_mode` は表に出すモードを切り替えるだけで、どちらの局面も消しません。

クライアントから受け付けるフレームは 256 KiB までです (SGF の棋譜を貼り付けるため)。

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
