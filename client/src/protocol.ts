/**
 * ローカル WebSocket プロトコル。`app/src/protocol.rs` と docs/protocol.md に対応する。
 *
 * サーバーは状態が変わるたびに snapshot を丸ごと送るので、こちら側は受け取ったものを
 * 全置換するだけでよい。増分イベントも再同期手順も無い。
 */

import type { GoBoard, Point } from "./go";
import type { BoardState, HandKind, Side, Square, ViewSettings } from "./shogi";

declare const __APP_VERSION__: string | undefined;

/**
 * ビルド時に埋め込まれる、このページを焼いた exe の版 (client/bin/build.ts が差し込む)。
 * 差し込まれていない場合 (テストや型検査) は "dev" になる。
 */
export const APP_VERSION =
  typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : "dev";

/** 操作ページが今つまんでいる駒。盤面ページは view.showSelection のときだけ描く。 */
export type Selection =
  | { kind: "square"; square: Square }
  | { kind: "hand"; side: Side; pieceKind: HandKind };

export interface HistoryInfo {
  /** 現在表示している履歴の位置 (0 始まり)。 */
  index: number;
  /** 履歴の総数。 */
  length: number;
}

/** 盤に出すゲーム。将棋と囲碁は局面も履歴も別々にサーバーが持つ。 */
export type GameMode = "shogi" | "go";

export interface ShogiGame {
  mode: "shogi";
  board: BoardState;
  selection: Selection | null;
  /** 現在局面の SFEN。表示欄をサーバー側の正規表記で揃える。 */
  sfen: string;
}

export interface GoGame {
  mode: "go";
  board: GoBoard;
  /** 現在局面を配置だけで表した SGF。 */
  sgf: string;
}

export type GameState = ShogiGame | GoGame;

export interface Snapshot {
  type: "snapshot";
  /** snapshot を送ってきた exe の版。ページ側の版と食い違ったら読み直す。 */
  appVersion: string;
  rev: number;
  view: ViewSettings;
  /** 表に出ているモードの履歴。 */
  history: HistoryInfo;
  game: GameState;
}

export interface Pong {
  type: "pong";
  t: number;
}

/** 直前のコマンドが編集として成立しなかったことだけを伝える。状態は snapshot が運ぶ。 */
export interface Rejected {
  type: "rejected";
  reason: string;
}

export type ServerMessage = Snapshot | Pong | Rejected;

export interface Ping {
  type: "ping";
  t: number;
}

/**
 * 盤のマスを叩いた。選択・移動・打ち込みのどれになるかはサーバー側の選択状態で決まる。
 * 判定をサーバーへ寄せているので、操作ページを複数開いても選択が割れない。
 */
export type Command =
  | { type: "set_mode"; mode: GameMode }
  | { type: "tap_square"; square: Square }
  | { type: "tap_hand"; side: Side; pieceKind: HandKind }
  | { type: "toggle_promote"; square: Square }
  | { type: "flip_piece"; square: Square }
  | { type: "clear_selection" }
  | { type: "set_sfen"; sfen: string }
  | { type: "preset"; name: string }
  /** color を省くと手番の石を置いて手番を渡し、指定すると手番を動かさずにその色を置く。 */
  | { type: "go_place"; point: Point; color?: Side }
  | { type: "go_remove"; point: Point }
  | { type: "go_new"; size: number }
  | { type: "set_sgf"; sgf: string }
  | { type: "set_turn"; side: Side }
  | { type: "history_go"; index: number }
  | { type: "set_view"; view: Partial<ViewSettings> };

export type ClientMessage = Ping | Command;
