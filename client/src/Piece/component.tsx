/**
 * 駒 1 枚の SVG。五角形の駒形に漢字を載せるだけで、画像アセットを使わない。
 *
 * 形も文字も 0..1 の正規座標で書き、外側の scale でマス目の大きさへ合わせる。
 * どの盤サイズでも輪郭が崩れず、OBS 側で拡大しても綺麗に出る。
 *
 * 面は塗り分けず、先端が明るく根元が暗いグラデーション 1 枚で木の丸みを出す。
 * グラデーションは駒ごと回るので、後手の駒でも「その駒の先端」が明るい。
 * 盤へ落ちる影は駒ごとに持たず、駒をまとめたグループへフィルタ 1 つで掛ける。
 */

import type { BoardColors } from "~/Board/theme";
import { type Kind, pieceKanji, type Side } from "~/shogi";

/**
 * 駒形の五角形。上が尖った向き (自分から見て前) を 0 度とする。
 * 頂点・肩・底の比率は変えずに、マス目をほぼ埋める大きさまで広げてある。
 */
const KOMA_PATH =
  "M0.5 0.005 L0.824 0.144 L0.965 0.995 L0.035 0.995 L0.176 0.144 Z";

/** グラデーションの id。盤の <defs> で定義したものを参照する。 */
export const PIECE_GRADIENT_ID = "ssb-piece-face";

/**
 * 駒種ごとの大きさ。実物の駒も王が一番大きく、歩が一番小さい。
 * この寸法差が「本物の駒らしさ」のかなりの部分を作る。
 */
export const PIECE_SIZE_SCALE: Record<Kind, number> = {
  K: 1.0,
  R: 0.97,
  B: 0.97,
  G: 0.94,
  S: 0.94,
  N: 0.9,
  L: 0.88,
  P: 0.88,
};

const PIECE_FONT =
  '"Yu Mincho", YuMincho, "Hiragino Mincho ProN", "Noto Serif JP", "MS Mincho", serif';

export interface PieceGlyphProps {
  kind: Kind;
  promoted: boolean;
  side: Side;
  colors: BoardColors;
  /** 駒を描く正方形の左上と一辺。 */
  x: number;
  y: number;
  size: number;
  /** 0 か 180。後手の駒を上下反転させる。 */
  rotation: number;
  opacity?: number;
}

export function PieceGlyph({
  kind,
  promoted,
  side,
  colors,
  x,
  y,
  size,
  rotation,
  opacity,
}: PieceGlyphProps) {
  const kanji = pieceKanji(kind, promoted);
  const showPromoted = promoted && kanji !== pieceKanji(kind, false);

  return (
    <g
      transform={`translate(${x} ${y}) rotate(${rotation} ${size / 2} ${size / 2}) scale(${size})`}
      opacity={opacity}
      aria-label={`${side === "b" ? "先手" : "後手"}の${kanji}`}
    >
      <path
        d={KOMA_PATH}
        fill={`url(#${PIECE_GRADIENT_ID})`}
        stroke={colors.pieceStroke}
        strokeWidth={0.013}
        strokeLinejoin="round"
      />
      <text
        x={0.5}
        y={0.569}
        textAnchor="middle"
        dominantBaseline="central"
        fontFamily={PIECE_FONT}
        fontSize={0.594}
        fill={showPromoted ? colors.piecePromotedText : colors.pieceText}
        // 同じ色の細い縁で太らせる。合成ボールドより環境ごとの差が出ない。
        stroke={showPromoted ? colors.piecePromotedText : colors.pieceText}
        strokeWidth={0.012}
        paintOrder="stroke"
      >
        {kanji}
      </text>
    </g>
  );
}
