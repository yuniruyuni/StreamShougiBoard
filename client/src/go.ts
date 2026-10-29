/**
 * 碁盤を語るための型と、表示側だけで使う小さな関数。
 *
 * 盤面の正規状態を持つのはサーバー (Rust) なので、ここには取りの判定などの編集ロジックを置かない。
 * 型の並びは `app/src/go.rs` と対で、どちらかを変えたら両方直す。
 */

import type { Side } from "./shogi";

/** 操作ページから選べる路数。SGF からはこれ以外の路数 (2〜19) も読み込める。 */
export const GO_SIZES = [19, 13, 9] as const;

/**
 * 交点の番号 0..size*size。0 が左上で、行ごとに左から右へ数える (SGF の座標と同じ向き)。
 */
export type Point = number;

/** 取った石の数 (アゲハマ)。`b` は黒が取った白石の数。 */
export type Captures = Record<Side, number>;

export interface GoBoard {
  size: number;
  /** 長さ size*size。空点は null。 */
  points: (Side | null)[];
  captures: Captures;
  turn: Side;
  moveNumber: number;
  /** 直前に置いた石。 */
  lastMove: Point | null;
}

export function pointAt(size: number, column: number, row: number): Point {
  return row * size + column;
}

/** 盤外を一様に null として扱う。 */
export function stoneAt(board: GoBoard, point: Point): Side | null {
  return board.points[point] ?? null;
}

/** 横の座標。紛らわしい I を飛ばす慣例に従う。 */
const COLUMN_LETTERS = "ABCDEFGHJKLMNOPQRST";

export function columnLabel(column: number): string {
  return COLUMN_LETTERS[column] ?? "";
}

/** 縦の座標。下端を 1 として上へ数える。 */
export function rowLabel(size: number, row: number): string {
  return String(size - row);
}

/** 「D4」のような、配信の解説で使う呼び方。 */
export function pointName(size: number, point: Point): string {
  return `${columnLabel(point % size)}${rowLabel(size, Math.floor(point / size))}`;
}

/**
 * 星の位置 (列, 行)。9・13・19 路は慣例の位置に、それ以外の路数は天元だけを打つ
 * (奇数路のとき)。
 */
export function starPoints(size: number): [number, number][] {
  const edge = size >= 13 ? 3 : 2;
  const far = size - 1 - edge;
  const mid = (size - 1) / 2;
  switch (size) {
    case 19:
      return [edge, mid, far].flatMap((row) =>
        [edge, mid, far].map((column): [number, number] => [column, row]),
      );
    case 13:
    case 9:
      return [
        [edge, edge],
        [far, edge],
        [mid, mid],
        [edge, far],
        [far, far],
      ];
    default:
      return size % 2 === 1 && size >= 5 ? [[mid, mid]] : [];
  }
}
