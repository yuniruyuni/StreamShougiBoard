/**
 * 碁盤・碁石・アゲハマをまとめて描く SVG。操作ページと盤面ページが同じものを使う。
 *
 * 将棋盤 (component.tsx) と同じ木の面と配色を使い、同じ配信の中で切り替えても
 * 盤の質感が揃うようにしている。
 */

import type { KeyboardEvent, MouseEvent } from "react";
import {
  columnLabel,
  type GoBoard,
  type Point,
  pointName,
  rowLabel,
  starPoints,
  stoneAt,
} from "~/go";
import type { Side, ViewSettings } from "~/shogi";
import { COORD_FONT, GRAIN } from "./component";
import { type CaptureBowl, computeGoLayout, type GoLayout } from "./goLayout";
import { BOARD_COLORS, type BoardColors, backgroundColor } from "./theme";

const BOARD_GRADIENT_ID = "ssb-go-board-face";
const BOWL_GRADIENT_ID = "ssb-go-bowl-face";
const STONE_GRADIENT_ID: Record<Side, string> = {
  b: "ssb-go-stone-black",
  w: "ssb-go-stone-white",
};
const STONE_SHADOW_ID = "ssb-go-stone-shadow";

/**
 * クリックが何を意味するか。修飾キーで決まり、サーバーへは色の指定として届く。
 * turn は手番の石を置いて手番を渡す、black / white は手番を動かさずにその色を並べる。
 */
export type GoTool = "turn" | "black" | "white" | "erase";

/**
 * 修飾キーから道具を選ぶ。Alt が消去、Ctrl (Mac は ⌘) が白、Shift が黒。
 * 複数押されていたら、取り返しのつきにくい消去を優先する。
 */
export function toolFromModifiers(event: {
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}): GoTool {
  if (event.altKey) return "erase";
  if (event.ctrlKey || event.metaKey) return "white";
  if (event.shiftKey) return "black";
  return "turn";
}

export interface GoBoardHandlers {
  onPoint(point: Point, tool: GoTool): void;
}

export interface GoBoardViewProps {
  board: GoBoard;
  view: ViewSettings;
  /** 碁盤 (木の面) の一辺の px。 */
  boardSize: number;
  /** 省略すると表示専用になり、クリック判定用の矩形を出さない。 */
  handlers?: GoBoardHandlers;
  /** 半透明の地を SVG の中に敷くか。盤面ページはページ側で敷くので false を渡す。 */
  paintBackground?: boolean;
}

function BoardFace({
  layout,
  colors,
}: {
  layout: GoLayout;
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

function GridLines({
  layout,
  colors,
}: {
  layout: GoLayout;
  colors: BoardColors;
}) {
  const { board, size, spacing } = layout;
  const width = Math.max(1, board.width * 0.0018);
  const start = spacing / 2;
  const end = board.width - spacing / 2;
  const lines = Array.from({ length: size }, (_, i) => start + i * spacing);
  // 反転しても星の配置は点対称なので、表示位置へ写し直さなくてよい。
  const stars = starPoints(size);

  return (
    <g>
      <g stroke={colors.lineColor} strokeWidth={width} strokeLinecap="square">
        {lines.map((offset) => (
          <g key={`line-${offset}`}>
            <line
              x1={board.x + offset}
              y1={board.y + start}
              x2={board.x + offset}
              y2={board.y + end}
            />
            <line
              x1={board.x + start}
              y1={board.y + offset}
              x2={board.x + end}
              y2={board.y + offset}
            />
          </g>
        ))}
      </g>
      {/* 外周だけ太くすると、盤の輪郭が締まる。 */}
      <rect
        x={board.x + start}
        y={board.y + start}
        width={end - start}
        height={end - start}
        fill="none"
        stroke={colors.lineColor}
        strokeWidth={width * 2}
      />
      {stars.map(([column, row]) => (
        <circle
          key={`star-${column}-${row}`}
          cx={board.x + start + column * spacing}
          cy={board.y + start + row * spacing}
          r={Math.max(1.5, spacing * 0.1)}
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
  layout: GoLayout;
  colors: BoardColors;
  view: ViewSettings;
}) {
  const { board, size, spacing, coordFontSize: fontSize } = layout;
  const order = Array.from({ length: size }, (_, i) =>
    view.flipped ? size - 1 - i : i,
  );

  const labels = [
    ...order.map((column, index) => ({
      key: `column-${column}`,
      text: columnLabel(column),
      x: board.x + (index + 0.5) * spacing,
      y: board.y - fontSize * 0.75,
    })),
    ...order.map((row, index) => ({
      key: `row-${row}`,
      text: rowLabel(size, row),
      x: board.x + board.width + fontSize * 0.85,
      y: board.y + (index + 0.5) * spacing,
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
            dominantBaseline="central"
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
            dominantBaseline="central"
            fill={colors.coordColor}
          >
            {label.text}
          </text>
        </g>
      ))}
    </g>
  );
}

function Stone({
  side,
  cx,
  cy,
  r,
  colors,
}: {
  side: Side;
  cx: number;
  cy: number;
  r: number;
  colors: BoardColors;
}) {
  return (
    <circle
      className="stone"
      cx={cx}
      cy={cy}
      r={r}
      fill={`url(#${STONE_GRADIENT_ID[side]})`}
      stroke={side === "w" ? colors.whiteStoneEdge : "none"}
      strokeWidth={side === "w" ? Math.max(0.5, r * 0.04) : 0}
    />
  );
}

const SIDE_NAME: Record<Side, string> = { b: "黒", w: "白" };

/** アゲハマ置き場。持ち主の石を 1 つ描き、その横か下へ取った数を出す。 */
function Bowl({
  bowl,
  count,
  colors,
  unit,
}: {
  bowl: CaptureBowl;
  count: number;
  colors: BoardColors;
  /** 盤の一辺。中身の大きさを路数によらず揃えるための基準。 */
  unit: number;
}) {
  const { rect, side, horizontal } = bowl;
  const r = unit * 0.032;
  const stone = horizontal
    ? { cx: rect.x + rect.width * 0.25, cy: rect.y + rect.height / 2 }
    : { cx: rect.x + rect.width / 2, cy: rect.y + rect.height * 0.24 };
  const caption = horizontal
    ? { x: rect.x + rect.width * 0.62, y: rect.y + rect.height * 0.3 }
    : { x: rect.x + rect.width / 2, y: rect.y + rect.height * 0.5 };
  const number = horizontal
    ? { x: rect.x + rect.width * 0.62, y: rect.y + rect.height * 0.68 }
    : { x: rect.x + rect.width / 2, y: rect.y + rect.height * 0.74 };

  return (
    <g aria-label={`${SIDE_NAME[side]}のアゲハマ ${count}`}>
      <rect
        x={rect.x}
        y={rect.y}
        width={rect.width}
        height={rect.height}
        rx={unit * 0.012}
        fill={`url(#${BOWL_GRADIENT_ID})`}
        stroke={colors.handPlateEdge}
        strokeWidth={Math.max(1, unit * 0.0025)}
      />
      <Stone side={side} r={r} colors={colors} {...stone} />
      <g fontFamily={COORD_FONT} textAnchor="middle" fill={colors.pieceText}>
        <text
          x={caption.x}
          y={caption.y}
          fontSize={unit * 0.022}
          dominantBaseline="central"
        >
          アゲハマ
        </text>
        <text
          x={number.x}
          y={number.y}
          fontSize={unit * 0.045}
          fontWeight="bold"
          dominantBaseline="central"
        >
          {count}
        </text>
      </g>
    </g>
  );
}

function HitTargets({
  board,
  layout,
  handlers,
}: {
  board: GoBoard;
  layout: GoLayout;
  handlers: GoBoardHandlers;
}) {
  const { spacing, size } = layout;

  const onClick = (event: MouseEvent<SVGRectElement>, point: Point) => {
    handlers.onPoint(point, toolFromModifiers(event));
  };

  const onKeyDown = (event: KeyboardEvent<SVGRectElement>, point: Point) => {
    // マウスの修飾キーに当たる操作をキーからも届かせる。
    const tool: GoTool | null =
      event.key === "Enter" || event.key === " "
        ? "turn"
        : event.key === "b"
          ? "black"
          : event.key === "w"
            ? "white"
            : event.key === "Delete" || event.key === "Backspace"
              ? "erase"
              : null;
    if (tool === null) return;
    event.preventDefault();
    handlers.onPoint(point, tool);
  };

  return (
    <g fill="transparent">
      {layout.points.map((center, point) => {
        const stone = stoneAt(board, point);
        const name = pointName(size, point);
        return (
          // SVG の中に <button> は置けないので、role で同じ意味を持たせる。
          // biome-ignore lint/a11y/useSemanticElements: SVG 内に button 要素は置けない
          <rect
            // 交点の数も並びも変わらないので、index がそのまま交点の識別子になる。
            // biome-ignore lint/suspicious/noArrayIndexKey: index は交点番号そのもの
            key={`hit-point-${point}`}
            x={center.x - spacing / 2}
            y={center.y - spacing / 2}
            width={spacing}
            height={spacing}
            role="button"
            tabIndex={0}
            aria-label={`${name} ${stone === null ? "空点" : `${SIDE_NAME[stone]}石`}`}
            onClick={(event) => onClick(event, point)}
            onContextMenu={(event) => {
              event.preventDefault();
              handlers.onPoint(point, "erase");
            }}
            onKeyDown={(event) => onKeyDown(event, point)}
          />
        );
      })}
    </g>
  );
}

export function GoBoardView({
  board,
  view,
  boardSize,
  handlers,
  paintBackground = true,
}: GoBoardViewProps) {
  const layout = computeGoLayout(view, boardSize, board.size);
  const colors = BOARD_COLORS;
  const background = paintBackground ? backgroundColor(view) : null;
  const radius = layout.spacing * 0.48;
  const lastMove =
    view.showLastMove && board.lastMove !== null ? board.lastMove : null;
  const lastSide = lastMove === null ? null : stoneAt(board, lastMove);
  const lastCenter = lastMove === null ? undefined : layout.points[lastMove];

  return (
    <svg
      className={view.animate ? "board board--animated" : "board"}
      viewBox={`0 0 ${layout.width} ${layout.height}`}
      width={layout.width}
      height={layout.height}
      role="img"
      aria-label={`${board.size}路の碁盤`}
    >
      <defs>
        <linearGradient id={BOARD_GRADIENT_ID} x1="0" y1="0" x2="0.35" y2="1">
          <stop offset="0" stopColor={colors.boardRamp[0]} />
          <stop offset="0.5" stopColor={colors.boardRamp[1]} />
          <stop offset="1" stopColor={colors.boardRamp[2]} />
        </linearGradient>
        <linearGradient id={BOWL_GRADIENT_ID} x1="0" y1="0" x2="0.3" y2="1">
          <stop offset="0" stopColor={colors.handRamp[0]} />
          <stop offset="1" stopColor={colors.handRamp[1]} />
        </linearGradient>
        {(["b", "w"] as const).map((side) => {
          const ramp =
            side === "b" ? colors.blackStoneRamp : colors.whiteStoneRamp;
          return (
            // 光を左上から当てる。石ごとに同じ向きで光るので、盤全体の光源が揃って見える。
            <radialGradient
              key={side}
              id={STONE_GRADIENT_ID[side]}
              cx="0.36"
              cy="0.32"
              r="0.7"
            >
              <stop offset="0" stopColor={ramp[0]} />
              <stop offset="1" stopColor={ramp[1]} />
            </radialGradient>
          );
        })}
        <filter
          id={STONE_SHADOW_ID}
          x="-10%"
          y="-10%"
          width="120%"
          height="120%"
        >
          <feDropShadow
            dx={layout.spacing * 0.03}
            dy={layout.spacing * 0.05}
            stdDeviation={layout.spacing * 0.03}
            floodColor={colors.stoneShadow}
            floodOpacity="1"
          />
        </filter>
      </defs>

      {background !== null && (
        <rect
          x={0}
          y={0}
          width={layout.width}
          height={layout.height}
          fill={background}
        />
      )}

      <BoardFace layout={layout} colors={colors} />
      <GridLines layout={layout} colors={colors} />
      {view.showCoordinates && (
        <Coordinates layout={layout} colors={colors} view={view} />
      )}

      <g className="stones" filter={`url(#${STONE_SHADOW_ID})`}>
        {layout.points.map((center, point) => {
          const side = stoneAt(board, point);
          return side === null ? null : (
            <Stone
              // 色が変わったら別の石として描き直し、置いた瞬間の動きを付け直す。
              // biome-ignore lint/suspicious/noArrayIndexKey: index は交点番号そのもの
              key={`stone-${point}-${side}`}
              side={side}
              cx={center.x}
              cy={center.y}
              r={radius}
              colors={colors}
            />
          );
        })}
      </g>

      {lastSide !== null && lastCenter !== undefined && (
        <circle
          className="last-stone"
          cx={lastCenter.x}
          cy={lastCenter.y}
          r={radius * 0.42}
          fill="none"
          stroke={colors.lastStoneMark}
          strokeWidth={Math.max(1, radius * 0.16)}
        />
      )}

      {layout.bowls.map((bowl) => (
        <Bowl
          key={`bowl-${bowl.side}`}
          bowl={bowl}
          count={board.captures[bowl.side]}
          colors={colors}
          unit={boardSize}
        />
      ))}

      {handlers !== undefined && (
        <HitTargets board={board} layout={layout} handlers={handlers} />
      )}
    </svg>
  );
}
