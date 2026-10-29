/**
 * 碁盤とアゲハマ置き場の配置計算。SVG の座標だけを返し、描画も React も知らない。
 *
 * 将棋盤の配置 (layout.ts) と同じく、盤面ページと操作ページが同じ関数を使う。
 * 駒台の位置設定 (handLayout) はここではアゲハマ置き場の位置として読む。
 */

import type { Point } from "~/go";
import type { Side, ViewSettings } from "~/shogi";
import type { Rect } from "./layout";

/** 片側のアゲハマ置き場。中には持ち主の石と、取った数を描く。 */
export interface CaptureBowl {
  side: Side;
  rect: Rect;
  /** 横長 (盤の上下に置くとき) か、縦長 (左右に置くとき) か。 */
  horizontal: boolean;
}

export interface GoLayout {
  /** SVG の viewBox 寸法。 */
  width: number;
  height: number;
  /** 碁盤の木の面。線はこの内側に引く。 */
  board: Rect;
  size: number;
  /** 線と線の間隔。石の大きさや線の太さは、すべてこれを基準に決める。 */
  spacing: number;
  /** index は Point。盤の反転はここで吸収済み。 */
  points: { x: number; y: number }[];
  bowls: CaptureBowl[];
  /** 座標の文字の大きさ。路数によらず読める大きさにするので、盤の一辺から決める。 */
  coordFontSize: number;
}

/** アゲハマ置き場の寸法と盤との隙間を、盤の一辺に対する比率で決める。 */
const BOWL_LONG_RATIO = 0.2;
const BOWL_SHORT_RATIO = 0.13;
const BOWL_GAP_RATIO = 0.035;
const COORD_FONT_RATIO = 0.028;
const COORD_GUTTER_RATIO = 1.6;

/** 盤を反転したときは、交点の並びを逆順にして後手 (白) 視点にする。 */
export function displayPoint(
  point: Point,
  size: number,
  flipped: boolean,
): number {
  return flipped ? size * size - 1 - point : point;
}

function pointCenters(
  board: Rect,
  size: number,
  spacing: number,
  flipped: boolean,
): { x: number; y: number }[] {
  return Array.from({ length: size * size }, (_, point) => {
    const index = displayPoint(point, size, flipped);
    return {
      x: board.x + ((index % size) + 0.5) * spacing,
      y: board.y + (Math.floor(index / size) + 0.5) * spacing,
    };
  });
}

/**
 * 盤の一辺 px と路数を渡してレイアウトを組む。盤の外周には線の間隔の半分の縁を残す。
 */
export function computeGoLayout(
  view: ViewSettings,
  boardSize: number,
  size: number,
): GoLayout {
  const { margin, flipped } = view;
  const spacing = boardSize / size;
  const coordFontSize = boardSize * COORD_FONT_RATIO;
  const gutter = view.showCoordinates ? coordFontSize * COORD_GUTTER_RATIO : 0;
  const gap = boardSize * BOWL_GAP_RATIO;
  const long = boardSize * BOWL_LONG_RATIO;
  const short = boardSize * BOWL_SHORT_RATIO;

  // 手前 (盤の下側) の陣営を右下へ、相手を左上へ置く。将棋の駒台と同じ並び。
  const near: Side = flipped ? "w" : "b";
  const far: Side = flipped ? "b" : "w";

  if (view.handLayout === "stacked") {
    const board: Rect = {
      x: margin,
      y: margin + short + gap + gutter,
      width: boardSize,
      height: boardSize,
    };
    const bowlX = margin + (boardSize - long) / 2;
    return {
      width: margin * 2 + boardSize + gutter,
      height: margin * 2 + boardSize + gutter + (short + gap) * 2,
      board,
      size,
      spacing,
      points: pointCenters(board, size, spacing, flipped),
      bowls: [
        {
          side: far,
          rect: { x: bowlX, y: margin, width: long, height: short },
          horizontal: true,
        },
        {
          side: near,
          rect: {
            x: bowlX,
            y: board.y + boardSize + gap,
            width: long,
            height: short,
          },
          horizontal: true,
        },
      ],
      coordFontSize,
    };
  }

  const board: Rect = {
    x: margin + short + gap,
    y: margin + gutter,
    width: boardSize,
    height: boardSize,
  };
  return {
    width: margin * 2 + (short + gap) * 2 + gutter + boardSize,
    height: margin * 2 + gutter + boardSize,
    board,
    size,
    spacing,
    points: pointCenters(board, size, spacing, flipped),
    bowls: [
      {
        side: far,
        rect: { x: margin, y: board.y, width: short, height: long },
        horizontal: false,
      },
      {
        side: near,
        rect: {
          x: board.x + boardSize + gutter + gap,
          y: board.y + boardSize - long,
          width: short,
          height: long,
        },
        horizontal: false,
      },
    ],
    coordFontSize,
  };
}
