import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { Snapshot } from "~/protocol";
import { DEFAULT_VIEW, type ViewSettings } from "~/shogi";
import goFixture from "../../../protocol-fixtures/go-snapshot.json";
import { GoBoardView, toolFromModifiers } from "./GoBoard";
import { BoardStage } from "./Stage";
import { BOARD_COLORS } from "./theme";

const snapshot = goFixture as unknown as Snapshot;
if (snapshot.game.mode !== "go") {
  throw new Error("go-snapshot.json は囲碁の局面のはず");
}
const game = snapshot.game;

function render(patch: Partial<ViewSettings> = {}): string {
  return renderToStaticMarkup(
    <GoBoardView
      board={game.board}
      view={{ ...DEFAULT_VIEW, ...patch }}
      boardSize={540}
    />,
  );
}

const stoneCount = (svg: string) => [...svg.matchAll(/class="stone"/g)].length;

describe("GoBoardView", () => {
  test("盤上の石と、両者のアゲハマ置き場の石を描く", () => {
    const onBoard = game.board.points.filter((p) => p !== null).length;
    expect(onBoard).toBeGreaterThan(0);
    expect(stoneCount(render())).toBe(onBoard + 2);
  });

  test("アゲハマの数を出す", () => {
    const svg = render();
    expect(svg).toContain("黒のアゲハマ 1");
    expect(svg).toContain("白のアゲハマ 0");
  });

  test("直前の手の印は showLastMove で切り替わる", () => {
    expect(render({ showLastMove: true })).toContain(
      BOARD_COLORS.lastStoneMark,
    );
    expect(render({ showLastMove: false })).not.toContain(
      BOARD_COLORS.lastStoneMark,
    );
  });

  test("座標は showCoordinates で切り替わる", () => {
    expect(render({ showCoordinates: true })).toContain(">J<");
    expect(render({ showCoordinates: false })).not.toContain(">J<");
  });

  test("操作ページでは交点ごとに当たり判定を置く", () => {
    const svg = renderToStaticMarkup(
      <GoBoardView
        board={game.board}
        view={DEFAULT_VIEW}
        boardSize={540}
        handlers={{ onPoint: () => {} }}
      />,
    );
    expect([...svg.matchAll(/role="button"/g)]).toHaveLength(81);
    expect(svg).toContain('aria-label="E4 黒石"');
  });
});

describe("toolFromModifiers", () => {
  const keys = {
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
  };

  test("修飾キーで置く石と消去を選ぶ", () => {
    expect(toolFromModifiers(keys)).toBe("turn");
    expect(toolFromModifiers({ ...keys, shiftKey: true })).toBe("black");
    expect(toolFromModifiers({ ...keys, ctrlKey: true })).toBe("white");
    expect(toolFromModifiers({ ...keys, metaKey: true })).toBe("white");
    expect(toolFromModifiers({ ...keys, altKey: true })).toBe("erase");
  });

  test("重ねて押したときは消去、白、黒の順に優先する", () => {
    expect(toolFromModifiers({ ...keys, altKey: true, shiftKey: true })).toBe(
      "erase",
    );
    expect(toolFromModifiers({ ...keys, ctrlKey: true, shiftKey: true })).toBe(
      "white",
    );
  });
});

describe("OBS の盤面ページ", () => {
  test("囲碁モードでは碁盤を描く", () => {
    const html = renderToStaticMarkup(
      <BoardStage
        snapshot={snapshot}
        width={1920}
        height={1080}
        gone={false}
      />,
    );
    expect(html).toContain("9路の碁盤");
  });
});
