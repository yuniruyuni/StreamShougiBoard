/**
 * 盤面を語るための型と、表示側だけで使う小さな関数。
 *
 * 盤面の正規状態を持つのはサーバー (Rust) なので、ここには編集ロジックを置かない。
 * 型の並びは `app/src/board.rs` / `app/src/piece.rs` / `app/src/view.rs` と対で、
 * どちらかを変えたら両方直す。ずれは protocol-fixtures のテストが検出する。
 */

/** 先手 (black) と後手 (white)。SFEN の手番表記に合わせた記号を使う。 */
export type Side = "b" | "w";

/** 成っていない状態の駒種。SFEN の大文字表記をそのまま識別子に使う。 */
export type Kind = "K" | "R" | "B" | "G" | "S" | "N" | "L" | "P";

/** 持ち駒になりうる駒種。玉は取れないので持ち駒にならない。 */
export type HandKind = Exclude<Kind, "K">;

/** 持ち駒欄の並び順。SFEN の慣例どおり価値の高い順に並べる。 */
export const HAND_ORDER: readonly HandKind[] = [
  "R",
  "B",
  "G",
  "S",
  "N",
  "L",
  "P",
];

/** 盤上の 1 枚の駒。`id` は駒が移動しても変わらず、アニメーションの手掛かりになる。 */
export interface Piece {
  id: string;
  kind: Kind;
  promoted: boolean;
  side: Side;
}

export const FILES = 9;
export const RANKS = 9;
export const SQUARE_COUNT = FILES * RANKS;

/**
 * 盤上のマス番号 0..80。
 * SFEN の走査順に合わせ、0 が 9一 (左上)、8 が 1一、80 が 1九 (右下)。
 */
export type Square = number;

/** 筋 (1..9、右から数える) と段 (1..9、上から数える) からマス番号を作る。 */
export function squareAt(file: number, rank: number): Square {
  return (rank - 1) * FILES + (FILES - file);
}

export function fileOf(square: Square): number {
  return FILES - (square % FILES);
}

export function rankOf(square: Square): number {
  return Math.floor(square / FILES) + 1;
}

/** 片側の持ち駒。7 種すべてが必ず入っている。 */
export type Hand = Record<HandKind, Piece[]>;

/** 直前の指し手。盤上から駒台へ送ったときは to、持ち駒を打ったときは from が null になる。 */
export interface LastMove {
  from: Square | null;
  to: Square | null;
}

export interface BoardState {
  /** 長さ 81。空きマスは null。 */
  squares: (Piece | null)[];
  hands: Record<Side, Hand>;
  turn: Side;
  moveNumber: number;
  lastMove: LastMove | null;
}

/** 盤外を一様に null として扱う。 */
export function pieceAt(board: BoardState, square: Square): Piece | null {
  return board.squares[square] ?? null;
}

const KANJI: Record<Kind, string> = {
  K: "玉",
  R: "飛",
  B: "角",
  G: "金",
  S: "銀",
  N: "桂",
  L: "香",
  P: "歩",
};

const PROMOTED_KANJI: Partial<Record<Kind, string>> = {
  R: "龍",
  B: "馬",
  S: "全",
  N: "圭",
  L: "杏",
  P: "と",
};

/** 駒に書かれる漢字。成銀・成桂・成香は駒に彫られる 1 文字表記を使う。 */
export function pieceKanji(kind: Kind, promoted: boolean): string {
  return (promoted ? PROMOTED_KANJI[kind] : undefined) ?? KANJI[kind];
}

/** 盤と駒台を囲む地に敷く色。濃さは backgroundOpacity が決める。 */
export type BackgroundColor = "white" | "black";

/** 駒台の置き場所。sides は盤の左右、stacked は盤の上下。 */
export type HandLayout = "sides" | "stacked";

export interface ViewSettings {
  backgroundColor: BackgroundColor;
  /** 地の濃さ (%)。0 なら地を塗らず、盤と駒台の外はそのまま映像が透ける。 */
  backgroundOpacity: number;
  /**
   * 盤と駒台を含めた全体の外周余白 px。盤の大きさはブラウザソースの領域から決まるので、
   * これは領域の縁から何 px 空けるかを表す。
   */
  margin: number;
  handLayout: HandLayout;
  showLastMove: boolean;
  showSelection: boolean;
  showCoordinates: boolean;
  flipped: boolean;
  animate: boolean;
}

export const MIN_MARGIN = 0;
export const MAX_MARGIN = 200;
export const MIN_BACKGROUND_OPACITY = 0;
export const MAX_BACKGROUND_OPACITY = 100;

/** サーバー側の既定値 (`app/src/view.rs`) と揃える。テストと初期表示にだけ使う。 */
export const DEFAULT_VIEW: ViewSettings = {
  backgroundColor: "black",
  backgroundOpacity: 0,
  margin: 16,
  handLayout: "sides",
  showLastMove: true,
  showSelection: false,
  showCoordinates: true,
  flipped: false,
  animate: true,
};
