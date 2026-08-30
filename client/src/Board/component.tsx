/**
 * 盤・駒台・駒をまとめて描く SVG。操作ページと盤面ページが同じものを使う。
 *
 * 駒は盤上と駒台をまたいで 1 つのグループへ id 順に並べる。親が変わらないので、
 * 駒を取って駒台へ移す動きまで CSS transition ひとつで繋がる。
 */

import type { KeyboardEvent } from "react";
import {
  PIECE_GRADIENT_ID,
  PIECE_SIZE_SCALE,
  PieceGlyph,
} from "~/Piece/component";
import type { Selection } from "~/protocol";
import {
  type BoardState,
  fileOf,
  type HandKind,
  type Piece,
  pieceAt,
  pieceKanji,
  rankOf,
  type Side,
  SQUARE_COUNT,
  type Square,
  type ViewSettings,
} from "~/shogi";
import {
  type BoardLayout,
  computeLayout,
  fileLabels,
  pieceRotation,
  RANK_LABELS,
  type Rect,
  rankLabels,
} from "./layout";
import { BOARD_COLORS, type BoardColors, backgroundColor } from "./theme";

/** 盤と駒台の面を塗るグラデーションの id。 */
const BOARD_GRADIENT_ID = "ssb-board-face";
const HAND_GRADIENT_ID = "ssb-hand-face";
/** 駒の影。駒 1 枚ずつではなく、駒全体のグループへ 1 回だけ掛ける。 */
const PIECE_SHADOW_ID = "ssb-piece-shadow";
/**
 * 盤の木目。乱数で引くと再描画のたびに模様が変わって配信でチラつくので、
 * 固定の種から一度だけ作り、以降は同じ線を使い回す。座標は盤に対する比率。
 */
interface GrainLine {
  x: number;
  bend: number;
  drift: number;
  width: number;
  opacity: number;
}

function grainLines(): GrainLine[] {
  let seed = 0x9e3779b9;
  const next = () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  return Array.from({ length: 26 }, () => ({
    x: next(),
    bend: next() * 0.024 - 0.012,
    drift: next() * 0.032 - 0.016,
    width: 0.0016 + next() * 0.0028,
    opacity: 0.5 + next() * 0.8,
  }));
}

const GRAIN = grainLines();

const COORD_FONT =
  '"Yu Gothic", YuGothic, "Hiragino Sans", "Noto Sans JP", sans-serif';

export interface BoardHandlers {
  onSquare(square: Square): void;
  onHand(side: Side, kind: HandKind): void;
  onSquareDoubleClick(square: Square): void;
  onSquareContextMenu(square: Square): void;
}

export interface ShogiBoardProps {
  board: BoardState;
  view: ViewSettings;
  /** 盤 (9x9 の枠) の一辺の px。呼び出し側が置き場所に合わせて決める。 */
  boardSize: number;
  selection: Selection | null;
  /** 選択枠を描くか。盤面ページは view.showSelection に従い、操作ページは常に描く。 */
  showSelection: boolean;
  /** 省略すると表示専用になり、クリック判定用の矩形を出さない。 */
  handlers?: BoardHandlers;
  /**
   * 半透明の地を SVG の中に敷くか。既定は敷く。
   * 盤面ページは領域いっぱいへ伸縮するので、地はページ側で敷いて false を渡す。
   */
  paintBackground?: boolean;
}

interface PlacedPiece {
  piece: Piece;
  x: number;
  y: number;
  size: number;
  rotation: number;
}

function centeredIn(rect: Rect, size: number): { x: number; y: number } {
  return {
    x: rect.x + (rect.width - size) / 2,
    y: rect.y + (rect.height - size) / 2,
  };
}

/**
 * 盤上と駒台の駒を 1 つのリストへ集める。
 * 駒台は最上段の 1 枚だけを描き、残りの枚数は数字で表す。
 */
function placePieces(
  board: BoardState,
  layout: BoardLayout,
  view: ViewSettings,
): PlacedPiece[] {
  const placed: PlacedPiece[] = [];

  for (let square = 0; square < SQUARE_COUNT; square += 1) {
    const piece = pieceAt(board, square);
    const rect = layout.squares[square];
    if (piece === null || rect === undefined) continue;
    const size = layout.cell * PIECE_SIZE_SCALE[piece.kind];
    const { x, y } = centeredIn(rect, size);
    placed.push({
      piece,
      x,
      y,
      size,
      rotation: pieceRotation(piece.side, view.flipped),
    });
  }

  for (const slot of layout.hands) {
    const pieces = board.hands[slot.side][slot.kind];
    const top = pieces[pieces.length - 1];
    if (top === undefined) continue;
    const size = layout.cell * PIECE_SIZE_SCALE[top.kind];
    const { x, y } = centeredIn(slot.rect, size);
    placed.push({
      piece: top,
      x,
      y,
      size,
      rotation: pieceRotation(slot.side, view.flipped),
    });
  }

  // id 順に並べておくと、駒が盤と駒台を行き来しても DOM 上の順序が変わらない。
  return placed.sort((a, b) => (a.piece.id < b.piece.id ? -1 : 1));
}

function BoardFace({
  layout,
  colors,
}: {
  layout: BoardLayout;
  colors: BoardColors;
}) {
  const { board } = layout;

  return (
    <g>
      <rect
        x={board.x}
        y={board.y}
        width={board.width}
        height={board.height}
        fill={`url(#${BOARD_GRADIENT_ID})`}
      />
      <g stroke={colors.grainColor} fill="none">
        {GRAIN.map((grain) => (
          <path
            key={`grain-${grain.x}`}
            d={`M${board.x + grain.x * board.width} ${board.y} q ${grain.bend * board.width} ${board.height / 2} ${grain.drift * board.width} ${board.height}`}
            strokeWidth={grain.width * board.width}
            strokeOpacity={grain.opacity}
          />
        ))}
      </g>
    </g>
  );
}

function HandPlates({
  layout,
  colors,
}: {
  layout: BoardLayout;
  colors: BoardColors;
}) {
  const inset = layout.cell * 0.05;

  return (
    <g>
      {layout.handPlates.map((plate) => (
        <g key={`plate-${plate.side}`}>
          <rect
            x={plate.rect.x}
            y={plate.rect.y}
            width={plate.rect.width}
            height={plate.rect.height}
            rx={layout.cell * 0.1}
            fill={`url(#${HAND_GRADIENT_ID})`}
            stroke={colors.handPlateEdge}
            strokeWidth={Math.max(1, layout.cell * 0.024)}
          />
          <rect
            x={plate.rect.x + inset}
            y={plate.rect.y + inset}
            width={plate.rect.width - inset * 2}
            height={plate.rect.height - inset * 2}
            rx={layout.cell * 0.07}
            fill="none"
            stroke={colors.handPlateInner}
            strokeWidth={Math.max(1, layout.cell * 0.016)}
          />
        </g>
      ))}
    </g>
  );
}

function GridLines({
  layout,
  colors,
}: {
  layout: BoardLayout;
  colors: BoardColors;
}) {
  const { board, cell } = layout;
  const width = Math.max(1, cell * 0.018);
  const lines: { x1: number; y1: number; x2: number; y2: number }[] = [];

  for (let i = 0; i <= 9; i += 1) {
    lines.push({
      x1: board.x + i * cell,
      y1: board.y,
      x2: board.x + i * cell,
      y2: board.y + board.height,
    });
    lines.push({
      x1: board.x,
      y1: board.y + i * cell,
      x2: board.x + board.width,
      y2: board.y + i * cell,
    });
  }

  const stars = [3, 6].flatMap((row) =>
    [3, 6].map((col) => ({
      cx: board.x + col * cell,
      cy: board.y + row * cell,
    })),
  );

  return (
    <g>
      <g stroke={colors.lineColor} strokeWidth={width} strokeLinecap="square">
        {lines.map((line) => (
          <line key={`line-${line.x1}-${line.y1}-${line.x2}`} {...line} />
        ))}
      </g>
      {/* 外周だけ太くすると、盤の輪郭が締まって駒が上に乗って見える。 */}
      <rect
        x={board.x}
        y={board.y}
        width={board.width}
        height={board.height}
        fill="none"
        stroke={colors.lineColor}
        strokeWidth={width * 2.5}
      />
      {stars.map((star) => (
        <circle
          key={`star-${star.cx}-${star.cy}`}
          cx={star.cx}
          cy={star.cy}
          r={cell * 0.045}
          fill={colors.starColor}
        />
      ))}
    </g>
  );
}

function Coordinates({
  layout,
  colors,
  view,
}: {
  layout: BoardLayout;
  colors: BoardColors;
  view: ViewSettings;
}) {
  const { board, cell } = layout;
  const fontSize = cell * 0.3;

  const labels = [
    ...fileLabels(view.flipped).map((label, index) => ({
      key: `file-${label}`,
      text: label,
      x: board.x + (index + 0.5) * cell,
      y: board.y - fontSize * 0.5,
      baseline: "middle" as const,
    })),
    ...rankLabels(view.flipped).map((label, index) => ({
      key: `rank-${label}`,
      text: label,
      x: board.x + board.width + fontSize * 0.7,
      y: board.y + (index + 0.5) * cell,
      baseline: "central" as const,
    })),
  ];

  return (
    <g fontFamily={COORD_FONT} fontSize={fontSize} textAnchor="middle">
      {labels.map((label) => (
        <g key={label.key}>
          {/* 番号は盤の外に出るので、先に縁を描いてから塗って映像から浮かせる。 */}
          <text
            x={label.x}
            y={label.y}
            dominantBaseline={label.baseline}
            stroke={colors.coordHalo}
            strokeWidth={fontSize * 0.28}
            strokeLinejoin="round"
            fill="none"
          >
            {label.text}
          </text>
          <text
            x={label.x}
            y={label.y}
            dominantBaseline={label.baseline}
            fill={colors.coordColor}
          >
            {label.text}
          </text>
        </g>
      ))}
    </g>
  );
}

function HighlightRect({ rect, fill }: { rect: Rect; fill: string }) {
  return (
    <rect
      x={rect.x}
      y={rect.y}
      width={rect.width}
      height={rect.height}
      fill={fill}
    />
  );
}

function LastMoveMark({
  rect,
  colors,
  cell,
}: {
  rect: Rect;
  colors: BoardColors;
  cell: number;
}) {
  const inset = Math.max(1, cell * 0.03);

  return (
    <g>
      <HighlightRect rect={rect} fill={colors.lastMoveFill} />
      <rect
        x={rect.x + inset}
        y={rect.y + inset}
        width={rect.width - inset * 2}
        height={rect.height - inset * 2}
        fill="none"
        stroke={colors.lastMoveEdge}
        strokeWidth={inset * 1.4}
      />
    </g>
  );
}

function HandCounts({
  board,
  layout,
  colors,
}: {
  board: BoardState;
  layout: BoardLayout;
  colors: BoardColors;
}) {
  const radius = layout.cell * 0.16;
  const fontSize = layout.cell * 0.22;

  return (
    <g
      fontFamily={COORD_FONT}
      fontSize={fontSize}
      fontWeight="bold"
      textAnchor="middle"
    >
      {layout.hands.map((slot) => {
        const count = board.hands[slot.side][slot.kind].length;
        if (count < 2) return null;
        const x = slot.rect.x + slot.rect.width * 0.84;
        const y = slot.rect.y + slot.rect.height * 0.8;
        return (
          <g key={`count-${slot.side}-${slot.kind}`}>
            <circle
              cx={x}
              cy={y}
              r={radius}
              fill={colors.countChipFill}
              stroke={colors.countChipEdge}
              strokeWidth={Math.max(1, layout.cell * 0.024)}
            />
            <text
              x={x}
              y={y}
              dominantBaseline="central"
              fill={colors.countText}
            >
              {count}
            </text>
          </g>
        );
      })}
    </g>
  );
}

/** クリック判定用の矩形へ付ける読み上げ用の名前。「7七 先手の歩」のように読める。 */
function squareLabel(board: BoardState, square: Square): string {
  const rank = RANK_LABELS[rankOf(square) - 1] ?? "";
  const place = `${fileOf(square)}${rank}`;
  const piece = pieceAt(board, square);
  if (piece === null) return `${place} 空きマス`;
  const side = piece.side === "b" ? "先手" : "後手";
  return `${place} ${side}の${pieceKanji(piece.kind, piece.promoted)}`;
}

/**
 * 盤と駒台の上に敷く透明な当たり判定。表示と入力を分けておくと、
 * 駒がアニメーションで動いている最中でもクリック位置がぶれない。
 */
function HitTargets({
  board,
  layout,
  handlers,
}: {
  board: BoardState;
  layout: BoardLayout;
  handlers: BoardHandlers;
}) {
  const onKeyDown = (event: KeyboardEvent<SVGRectElement>, square: Square) => {
    // マウスのダブルクリックと右クリックに当たる操作をキーからも届かせる。
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      handlers.onSquare(square);
    } else if (event.key === "p") {
      handlers.onSquareDoubleClick(square);
    } else if (event.key === "f") {
      handlers.onSquareContextMenu(square);
    }
  };

  return (
    <g fill="transparent">
      {layout.squares.map((rect, square) => (
        // SVG の中に <button> は置けないので、role で同じ意味を持たせる。
        // biome-ignore lint/a11y/useSemanticElements: SVG 内に button 要素は置けない
        <rect
          // 盤のマスは 81 個で並びも変わらないので、index がそのままマスの識別子になる。
          // biome-ignore lint/suspicious/noArrayIndexKey: index はマス番号そのもの
          key={`hit-square-${square}`}
          x={rect.x}
          y={rect.y}
          width={rect.width}
          height={rect.height}
          role="button"
          tabIndex={0}
          aria-label={squareLabel(board, square)}
          onClick={() => handlers.onSquare(square)}
          onDoubleClick={() => handlers.onSquareDoubleClick(square)}
          onContextMenu={(event) => {
            event.preventDefault();
            handlers.onSquareContextMenu(square);
          }}
          onKeyDown={(event) => onKeyDown(event, square)}
        />
      ))}
      {layout.hands.map((slot) => (
        // biome-ignore lint/a11y/useSemanticElements: SVG 内に button 要素は置けない
        <rect
          key={`hit-hand-${slot.side}-${slot.kind}`}
          x={slot.rect.x}
          y={slot.rect.y}
          width={slot.rect.width}
          height={slot.rect.height}
          role="button"
          tabIndex={0}
          aria-label={`${slot.side === "b" ? "先手" : "後手"}の駒台 ${pieceKanji(slot.kind, false)} ${board.hands[slot.side][slot.kind].length}枚`}
          onClick={() => handlers.onHand(slot.side, slot.kind)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            handlers.onHand(slot.side, slot.kind);
          }}
        />
      ))}
    </g>
  );
}

export function ShogiBoard({
  board,
  view,
  boardSize,
  selection,
  showSelection,
  handlers,
  paintBackground = true,
}: ShogiBoardProps) {
  const layout = computeLayout(view, boardSize);
  const colors = BOARD_COLORS;
  const background = paintBackground ? backgroundColor(view) : null;
  const placed = placePieces(board, layout, view);

  const lastMoveSquares =
    view.showLastMove && board.lastMove !== null
      ? [board.lastMove.from, board.lastMove.to].filter(
          (square): square is Square => square !== null,
        )
      : [];

  const selectedSquare =
    showSelection && selection !== null && selection.kind === "square"
      ? selection.square
      : null;
  const selectedRect =
    selectedSquare === null ? null : (layout.squares[selectedSquare] ?? null);
  const selectedHand =
    showSelection && selection !== null && selection.kind === "hand"
      ? selection
      : null;

  return (
    <svg
      className={view.animate ? "board board--animated" : "board"}
      viewBox={`0 0 ${layout.width} ${layout.height}`}
      width={layout.width}
      height={layout.height}
      role="img"
      aria-label="将棋盤"
    >
      <defs>
        {/* 盤と駒台は左上が明るく、駒は先端が明るい。向きが違うので別々に持つ。 */}
        <linearGradient id={BOARD_GRADIENT_ID} x1="0" y1="0" x2="0.35" y2="1">
          <stop offset="0" stopColor={colors.boardRamp[0]} />
          <stop offset="0.5" stopColor={colors.boardRamp[1]} />
          <stop offset="1" stopColor={colors.boardRamp[2]} />
        </linearGradient>
        <linearGradient id={HAND_GRADIENT_ID} x1="0" y1="0" x2="0.3" y2="1">
          <stop offset="0" stopColor={colors.handRamp[0]} />
          <stop offset="1" stopColor={colors.handRamp[1]} />
        </linearGradient>
        <linearGradient id={PIECE_GRADIENT_ID} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={colors.pieceRamp[0]} />
          <stop offset="0.45" stopColor={colors.pieceRamp[1]} />
          <stop offset="1" stopColor={colors.pieceRamp[2]} />
        </linearGradient>
        {/*
          駒は重ならないので、グループにまとめて 1 回ぼかせば 1 枚ずつ掛けたのと同じ絵になる。
          駒の面は不透明なので、上に載る文字は影に効かない。
        */}
        <filter
          id={PIECE_SHADOW_ID}
          x="-10%"
          y="-10%"
          width="120%"
          height="120%"
        >
          <feDropShadow
            dx="0"
            dy={layout.cell * 0.035}
            stdDeviation={layout.cell * 0.022}
            floodColor={colors.pieceShadow}
            floodOpacity="1"
          />
        </filter>
      </defs>

      {background !== null && (
        // 盤と駒台の下へ全面に敷く。上に乗る盤・駒台・駒は不透明なので、
        // 半透明になるのは余白と盤の外側だけになる。
        <rect
          x={0}
          y={0}
          width={layout.width}
          height={layout.height}
          fill={background}
        />
      )}

      <HandPlates layout={layout} colors={colors} />
      <BoardFace layout={layout} colors={colors} />

      {lastMoveSquares.map((square) => {
        const rect = layout.squares[square];
        return rect === undefined ? null : (
          <LastMoveMark
            key={`last-${square}`}
            rect={rect}
            colors={colors}
            cell={layout.cell}
          />
        );
      })}

      {selectedRect !== null && (
        <HighlightRect rect={selectedRect} fill={colors.selectionFill} />
      )}

      {selectedHand !== null &&
        layout.hands
          .filter(
            (slot) =>
              slot.side === selectedHand.side &&
              slot.kind === selectedHand.pieceKind,
          )
          .map((slot) => (
            <HighlightRect
              key={`sel-${slot.side}-${slot.kind}`}
              rect={slot.rect}
              fill={colors.selectionFill}
            />
          ))}

      <GridLines layout={layout} colors={colors} />
      {view.showCoordinates && (
        <Coordinates layout={layout} colors={colors} view={view} />
      )}

      <g className="pieces" filter={`url(#${PIECE_SHADOW_ID})`}>
        {placed.map((item) => (
          <PieceGlyph
            key={item.piece.id}
            kind={item.piece.kind}
            promoted={item.piece.promoted}
            side={item.piece.side}
            colors={colors}
            x={item.x}
            y={item.y}
            size={item.size}
            rotation={item.rotation}
          />
        ))}
      </g>

      <HandCounts board={board} layout={layout} colors={colors} />
      {handlers !== undefined && (
        <HitTargets board={board} layout={layout} handlers={handlers} />
      )}
    </svg>
  );
}
