/**
 * 盤の描画。中身の見た目そのものは目で見るしかないが、「描かれるべきものが
 * 描かれている」までは機械で確かめられる。
 *
 * 対象は SVG の文字列そのもの。DOM も描画エンジンも要らず、Rust 側が書き出した
 * fixture をそのまま食わせられるので、盤面ページと同じ入力で試せる。
 */

import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { Snapshot } from "~/protocol";
import { DEFAULT_VIEW, SQUARE_COUNT, type ViewSettings } from "~/shogi";
import fixture from "../../../protocol-fixtures/snapshot.json";
import { ShogiBoard } from "./component";
import { boardSizeToFit } from "./layout";
import { BOARD_COLORS, backgroundColor } from "./theme";

const snapshot = fixture as unknown as Snapshot;

const BOARD_SIZE = 540;

function render(
  patch: Partial<ViewSettings> = {},
  props: { showSelection?: boolean; paintBackground?: boolean } = {},
): string {
  const view = { ...DEFAULT_VIEW, ...patch };
  return renderToStaticMarkup(
    <ShogiBoard
      board={snapshot.board}
      view={view}
      boardSize={BOARD_SIZE}
      selection={snapshot.selection}
      showSelection={props.showSelection ?? false}
      paintBackground={props.paintBackground}
    />,
  );
}

/** 駒 1 枚につき 1 つ付く読み上げ用の名前を数える。 */
function pieceCount(svg: string): number {
  return [...svg.matchAll(/aria-label="(先手|後手)の/g)].length;
}

describe("ShogiBoard", () => {
  test("盤上の駒と、駒台の一番上の駒を描く", () => {
    const svg = render();

    // fixture は ▲7六歩 △3四歩 ▲2二角成 まで進み、先手の駒台に角が 1 枚ある局面。
    const onBoard = snapshot.board.squares.filter(
      (piece) => piece !== null,
    ).length;
    const handKinds = (["b", "w"] as const).flatMap((side) =>
      Object.values(snapshot.board.hands[side]).filter(
        (pieces) => pieces.length > 0,
      ),
    ).length;

    expect(onBoard).toBeGreaterThan(0);
    expect(pieceCount(svg)).toBe(onBoard + handKinds);
    expect(svg).toContain("馬");
  });

  test("空のマスには駒を描かない", () => {
    const svg = render();
    expect(pieceCount(svg)).toBeLessThan(SQUARE_COUNT);
  });

  test("直前の手のマスに枠を出し、切ると消える", () => {
    const shown = render({ showLastMove: true });
    const hidden = render({ showLastMove: false });

    // 枠は直前の手の from と to の 2 マスぶん。色は配色から引くので、
    // 見た目を調整してもこのテストは壊れない。
    const edge = BOARD_COLORS.lastMoveEdge;
    expect(shown.split(edge)).toHaveLength(3);
    expect(hidden).not.toContain(edge);
  });

  test("選択枠は showSelection のときだけ出る", () => {
    const on = render({}, { showSelection: true });
    const off = render({}, { showSelection: false });

    expect(on).toContain(BOARD_COLORS.selectionFill);
    expect(off).not.toContain(BOARD_COLORS.selectionFill);
  });

  test("筋と段の番号は showCoordinates で切り替わる", () => {
    const on = render({ showCoordinates: true });
    const off = render({ showCoordinates: false });

    expect(on).toContain("九");
    expect(off).not.toContain("九");
  });

  test("持ち駒が 2 枚以上あるときだけ枚数チップを出す", () => {
    const withOne = render();
    // fixture の持ち駒は角 1 枚だけなので、チップは出ない。
    expect(withOne).not.toContain(BOARD_COLORS.countChipFill);

    const stocked = structuredClone(snapshot.board);
    stocked.hands.b.P = [
      { id: "x1", kind: "P", promoted: false, side: "b" },
      { id: "x2", kind: "P", promoted: false, side: "b" },
    ];
    const svg = renderToStaticMarkup(
      <ShogiBoard
        board={stocked}
        view={DEFAULT_VIEW}
        boardSize={BOARD_SIZE}
        selection={null}
        showSelection={false}
      />,
    );
    expect(svg).toContain(BOARD_COLORS.countChipFill);
  });

  test("半透明の地は paintBackground で切り替わる", () => {
    const painted = render({ backgroundOpacity: 50 });
    const page = render({ backgroundOpacity: 50 }, { paintBackground: false });
    const none = render({ backgroundOpacity: 0 });

    // 盤面ページはページ側で敷くので、SVG の中には出さない。
    const fill = backgroundColor({ ...DEFAULT_VIEW, backgroundOpacity: 50 });
    expect(fill).not.toBeNull();
    expect(painted).toContain(fill ?? "");
    expect(page).not.toContain(fill ?? "");
    expect(none).not.toContain(fill ?? "");
  });

  test("駒台の位置を変えても描く駒の数は変わらない", () => {
    const sides = render({ handLayout: "sides" });
    const stacked = render({ handLayout: "stacked" });
    expect(pieceCount(stacked)).toBe(pieceCount(sides));
  });

  test("領域から決めた大きさでも viewBox がその中に収まる", () => {
    const view = { ...DEFAULT_VIEW };
    const size = boardSizeToFit(view, 1280, 720);
    const svg = renderToStaticMarkup(
      <ShogiBoard
        board={snapshot.board}
        view={view}
        boardSize={size}
        selection={null}
        showSelection={false}
      />,
    );

    const viewBox = /viewBox="0 0 ([\d.]+) ([\d.]+)"/.exec(svg);
    expect(viewBox).not.toBeNull();
    expect(Number(viewBox?.[1])).toBeLessThanOrEqual(1280);
    expect(Number(viewBox?.[2])).toBeLessThanOrEqual(720);
  });
});
