/**
 * 盤・駒台・駒の配色。盤と駒台は常に不透明で、映像が透けるのはそれらを囲む地だけ。
 * その地に敷く色と濃さは view の backgroundColor / backgroundOpacity が決める。
 *
 * 木の面は単色ではなくグラデーションで塗る。駒は先端が明るく根元が暗く、
 * 盤と駒台は左上が明るい。実物の木の見え方に寄せると、OBS で拡大しても平板にならない。
 */

import type { BackgroundColor, ViewSettings } from "~/shogi";

/** 3 段のグラデーション。順に 0% / 中間 / 100% の色。 */
export type Ramp3 = readonly [string, string, string];
/** 2 段のグラデーション。 */
export type Ramp2 = readonly [string, string];

export interface BoardColors {
  /** 盤の面。左上から右下へ。 */
  boardRamp: Ramp3;
  /** 駒台の板。 */
  handRamp: Ramp2;
  /** 駒の面。駒の先端から根元へ。 */
  pieceRamp: Ramp3;
  /** 盤の木目。盤の面に薄く重ねる線の色。 */
  grainColor: string;
  lineColor: string;
  starColor: string;
  coordColor: string;
  /** 番号は盤の外 (透ける地の上) に出るので、縁取りで映像から浮かせる。 */
  coordHalo: string;
  /** 直前の手のマス。淡い塗りと、それを縁取る線。 */
  lastMoveFill: string;
  lastMoveEdge: string;
  selectionFill: string;
  handPlateEdge: string;
  /** 駒台の板の内側に引く細い線。板の厚みに見える。 */
  handPlateInner: string;
  /** 駒が盤に落とす影。 */
  pieceShadow: string;
  pieceStroke: string;
  pieceText: string;
  piecePromotedText: string;
  /** 持ち駒の枚数を出す丸チップ。 */
  countChipFill: string;
  countChipEdge: string;
  countText: string;
}

export const BOARD_COLORS: BoardColors = {
  boardRamp: ["#ffdda2", "#f8c87f", "#eeb265"],
  handRamp: ["#d99a45", "#bd7a2c"],
  pieceRamp: ["#ffe9b4", "#f7d693", "#dfb066"],
  grainColor: "rgba(176, 112, 28, 0.10)",
  lineColor: "#432a08",
  starColor: "#432a08",
  coordColor: "#3a2a12",
  coordHalo: "rgba(255, 255, 255, 0.8)",
  // 盤が橙なので、直前の手も橙にすると盤へ溶ける。朱で締める。
  lastMoveFill: "rgba(190, 45, 25, 0.20)",
  lastMoveEdge: "rgba(168, 40, 26, 0.9)",
  selectionFill: "rgba(0, 150, 80, 0.38)",
  handPlateEdge: "#8c6a3c",
  handPlateInner: "rgba(255, 255, 255, 0.18)",
  pieceShadow: "rgba(58, 32, 6, 0.30)",
  pieceStroke: "#75470f",
  pieceText: "#20170d",
  piecePromotedText: "#a8281a",
  countChipFill: "#c0392b",
  countChipEdge: "#ffffff",
  countText: "#ffffff",
};

const BACKGROUND_RGB: Record<BackgroundColor, string> = {
  white: "255, 255, 255",
  black: "0, 0, 0",
};

/**
 * 盤の背後に敷く地。濃さ 0 なら null を返し、OBS では映像がそのまま透ける。
 *
 * rgba で返すのは、盤面ページが SVG ではなくページ側の背景として敷くため。
 * 盤は領域に合わせて伸縮するので、SVG の中を塗ると余った上下左右が塗り残しになる。
 */
export function backgroundColor(view: ViewSettings): string | null {
  if (view.backgroundOpacity <= 0) return null;
  return `rgba(${BACKGROUND_RGB[view.backgroundColor]}, ${view.backgroundOpacity / 100})`;
}
