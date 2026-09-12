/**
 * 盤と駒台の配置計算。SVG の座標だけを返し、描画も React も知らない。
 *
 * 盤面ページと操作ページが同じ関数を使うので、手元で見ている位置関係と
 * OBS に出ている位置関係が必ず一致する。
 */

import {
  FILES,
  HAND_ORDER,
  type HandKind,
  type Side,
  SQUARE_COUNT,
  type Square,
  type ViewSettings,
} from "~/shogi";

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface HandSlot {
  side: Side;
  kind: HandKind;
  rect: Rect;
}

/** 駒台の板。スロット 7 つをまとめて囲む一枚。 */
export interface HandPlate {
  side: Side;
  rect: Rect;
}

export interface BoardLayout {
  /** SVG の viewBox 寸法。 */
  width: number;
  height: number;
  /** 9x9 の枠。 */
  board: Rect;
  /** マスの幅。線の太さや文字の大きさは、すべてこれを基準に決める。 */
  cellWidth: number;
  /** マスの高さ。実物の盤に合わせて幅より少し高い。 */
  cellHeight: number;
  /** index は Square。盤の反転はここで吸収済み。 */
  squares: Rect[];
  hands: HandSlot[];
  /** 駒台は板 1 枚として描き、当たり判定だけを hands の升で取る。 */
  handPlates: HandPlate[];
}

/**
 * マスの縦横比 (高さ ÷ 幅)。実物の将棋盤のマスは正方形ではなく、わずかに縦長。
 * 一般的な寸法の 3.03cm x 3.33cm から取っている。
 */
export const CELL_ASPECT = 1.1;

/** 駒台の枠と盤との隙間を、マス目に対する比率で決める。 */
const HAND_GAP_RATIO = 0.4;
const HAND_PAD_RATIO = 0.18;
const HAND_SLOTS = HAND_ORDER.length;
/**
 * 筋・段の番号を置く帯の幅。盤の上と右へ確保する。
 * 番号を viewBox の外へはみ出させると、余白 0 の設定で端が切れてしまう。
 */
const COORD_GUTTER_RATIO = 0.45;

function coordGutter(view: ViewSettings, cellWidth: number): number {
  return view.showCoordinates ? cellWidth * COORD_GUTTER_RATIO : 0;
}

/**
 * 盤を反転したときは駒を回すのではなく、マスの並びを逆順にする。
 * こうすると駒に書かれた漢字は常に読める向きのまま、盤だけが後手視点になる。
 */
export function displayIndex(square: Square, flipped: boolean): number {
  return flipped ? SQUARE_COUNT - 1 - square : square;
}

/** 駒の向き。自分側 (盤の手前) を向いている駒が 0 度。 */
export function pieceRotation(side: Side, flipped: boolean): number {
  const pointsUp = (side === "b") !== flipped;
  return pointsUp ? 0 : 180;
}

function squareRects(
  board: Rect,
  cellWidth: number,
  cellHeight: number,
  flipped: boolean,
): Rect[] {
  const rects = new Array<Rect>(SQUARE_COUNT);
  for (let square = 0; square < SQUARE_COUNT; square += 1) {
    const index = displayIndex(square, flipped);
    rects[square] = {
      x: board.x + (index % FILES) * cellWidth,
      y: board.y + Math.floor(index / FILES) * cellHeight,
      width: cellWidth,
      height: cellHeight,
    };
  }
  return rects;
}

/** 駒台の板。スロットの原点から余白のぶんだけ外へ広げた矩形。 */
function handPlate(
  side: Side,
  origin: Rect,
  pad: number,
  width: number,
  height: number,
): HandPlate {
  return {
    side,
    rect: { x: origin.x - pad, y: origin.y - pad, width, height },
  };
}

/** 駒台 1 枚分の 7 スロットを、縦一列 (sides) か横一列 (stacked) に並べる。 */
function handSlots(
  side: Side,
  origin: Rect,
  cellWidth: number,
  cellHeight: number,
  vertical: boolean,
): HandSlot[] {
  return HAND_ORDER.map((kind, index) => ({
    side,
    kind,
    rect: {
      x: origin.x + (vertical ? 0 : index * cellWidth),
      y: origin.y + (vertical ? index * cellHeight : 0),
      width: cellWidth,
      height: cellHeight,
    },
  }));
}

function layoutSides(
  view: ViewSettings,
  boardSize: number,
  cellWidth: number,
  cellHeight: number,
): BoardLayout {
  const { margin, flipped } = view;
  const gap = cellWidth * HAND_GAP_RATIO;
  const pad = cellWidth * HAND_PAD_RATIO;
  const gutter = coordGutter(view, cellWidth);
  const handWidth = cellWidth + pad * 2;
  const handHeight = HAND_SLOTS * cellHeight + pad * 2;
  const boardHeight = cellHeight * FILES;

  const board: Rect = {
    x: margin + handWidth + gap,
    y: margin + gutter,
    width: boardSize,
    height: boardHeight,
  };

  // 手前 (盤の下側) の陣営の駒台を右下へ、相手の駒台を左上へ置く。
  const near: Side = flipped ? "w" : "b";
  const far: Side = flipped ? "b" : "w";

  const farOrigin = {
    x: margin + pad,
    y: board.y + pad,
    width: cellWidth,
    height: cellHeight,
  };
  const nearOrigin = {
    x: board.x + boardSize + gutter + gap + pad,
    y: board.y + boardHeight - handHeight + pad,
    width: cellWidth,
    height: cellHeight,
  };

  return {
    width: margin * 2 + handWidth * 2 + gap * 2 + gutter + boardSize,
    height: margin * 2 + gutter + boardHeight,
    board,
    cellWidth,
    cellHeight,
    squares: squareRects(board, cellWidth, cellHeight, flipped),
    hands: [
      ...handSlots(far, farOrigin, cellWidth, cellHeight, true),
      ...handSlots(near, nearOrigin, cellWidth, cellHeight, true),
    ],
    handPlates: [
      handPlate(far, farOrigin, pad, handWidth, handHeight),
      handPlate(near, nearOrigin, pad, handWidth, handHeight),
    ],
  };
}

function layoutStacked(
  view: ViewSettings,
  boardSize: number,
  cellWidth: number,
  cellHeight: number,
): BoardLayout {
  const { margin, flipped } = view;
  const gap = cellWidth * HAND_GAP_RATIO;
  const pad = cellWidth * HAND_PAD_RATIO;
  const gutter = coordGutter(view, cellWidth);
  const handHeight = cellHeight + pad * 2;
  const handWidth = HAND_SLOTS * cellWidth + pad * 2;
  const boardHeight = cellHeight * FILES;
  const handX = margin + (boardSize - handWidth) / 2 + pad;

  const board: Rect = {
    x: margin,
    y: margin + handHeight + gap + gutter,
    width: boardSize,
    height: boardHeight,
  };

  const near: Side = flipped ? "w" : "b";
  const far: Side = flipped ? "b" : "w";

  const farOrigin = {
    x: handX,
    y: margin + pad,
    width: cellWidth,
    height: cellHeight,
  };
  const nearOrigin = {
    x: handX,
    y: board.y + boardHeight + gap + pad,
    width: cellWidth,
    height: cellHeight,
  };

  return {
    width: margin * 2 + boardSize + gutter,
    height: margin * 2 + boardHeight + gutter + (handHeight + gap) * 2,
    board,
    cellWidth,
    cellHeight,
    squares: squareRects(board, cellWidth, cellHeight, flipped),
    hands: [
      ...handSlots(far, farOrigin, cellWidth, cellHeight, false),
      ...handSlots(near, nearOrigin, cellWidth, cellHeight, false),
    ],
    handPlates: [
      handPlate(far, farOrigin, pad, handWidth, handHeight),
      handPlate(near, nearOrigin, pad, handWidth, handHeight),
    ],
  };
}

/**
 * 盤の一辺 px を渡してレイアウトを組む。大きさを設定として持たないのは、
 * 盤面ページがブラウザソースの領域から決め、操作ページはプレビューの都合で決めるため。
 */
export function computeLayout(
  view: ViewSettings,
  boardSize: number,
): BoardLayout {
  const cellWidth = boardSize / FILES;
  const cellHeight = cellWidth * CELL_ASPECT;
  return view.handLayout === "stacked"
    ? layoutStacked(view, boardSize, cellWidth, cellHeight)
    : layoutSides(view, boardSize, cellWidth, cellHeight);
}

/** これより小さくは描かない。潰れた盤を出すより、はみ出す方がまだ気づける。 */
export const MIN_FITTED_BOARD_SIZE = 90;

/** 比率を測るためだけに一度組んでみる大きさ。値そのものに意味は無い。 */
const PROBE_BOARD_SIZE = 900;

/**
 * 幅 x 高さの領域へ、外周の余白を残して収まる最大の盤の一辺を返す。
 *
 * 余白以外の寸法 (駒台・隙間・座標の帯) はすべて盤に比例するので、
 * 余白 0 で一度組んでみて、その縦横が盤の何倍になるかを測ってから割る。
 * 比率を式で持たずに実際のレイアウトへ聞くことで、配置を変えても追従する。
 */
export function boardSizeToFit(
  view: ViewSettings,
  width: number,
  height: number,
): number {
  const probe = computeLayout({ ...view, margin: 0 }, PROBE_BOARD_SIZE);
  const widthPerBoard = probe.width / PROBE_BOARD_SIZE;
  const heightPerBoard = probe.height / PROBE_BOARD_SIZE;

  // 余白が領域より大きいときは、余白の方を諦める。
  const inset = Math.min(view.margin, Math.min(width, height) / 4);
  const fitted = Math.min(
    (width - inset * 2) / widthPerBoard,
    (height - inset * 2) / heightPerBoard,
  );

  // 割ってから掛け直すと浮動小数の誤差で領域をわずかに超えることがある。
  // 千分の一 px 単位へ切り捨てて、必ず内側に収まるようにする。
  const trimmed = Math.floor(fitted * 1000) / 1000;
  return Math.max(MIN_FITTED_BOARD_SIZE, trimmed);
}

/** 筋 (9..1) と段 (一..九) のラベル。反転時は並びも逆になる。 */
export const FILE_LABELS = [
  "9",
  "8",
  "7",
  "6",
  "5",
  "4",
  "3",
  "2",
  "1",
] as const;
export const RANK_LABELS = [
  "一",
  "二",
  "三",
  "四",
  "五",
  "六",
  "七",
  "八",
  "九",
] as const;

export function fileLabels(flipped: boolean): readonly string[] {
  return flipped ? [...FILE_LABELS].reverse() : FILE_LABELS;
}

export function rankLabels(flipped: boolean): readonly string[] {
  return flipped ? [...RANK_LABELS].reverse() : RANK_LABELS;
}
