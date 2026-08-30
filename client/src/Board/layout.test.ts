import { describe, expect, test } from "bun:test";
import { DEFAULT_VIEW, squareAt, type ViewSettings } from "~/shogi";
import {
  boardSizeToFit,
  computeLayout,
  displayIndex,
  MIN_FITTED_BOARD_SIZE,
  pieceRotation,
} from "./layout";

const BOARD_SIZE = 540;

const view = (patch: Partial<ViewSettings> = {}): ViewSettings => ({
  ...DEFAULT_VIEW,
  margin: 10,
  ...patch,
});

describe("displayIndex", () => {
  test("反転しなければマスの並びはそのまま", () => {
    expect(displayIndex(0, false)).toBe(0);
    expect(displayIndex(80, false)).toBe(80);
  });

  test("反転すると 9一 と 1九 が入れ替わる", () => {
    expect(displayIndex(squareAt(9, 1), true)).toBe(80);
    expect(displayIndex(squareAt(1, 9), true)).toBe(0);
  });
});

describe("pieceRotation", () => {
  test("通常は先手が上向き、後手が下向き", () => {
    expect(pieceRotation("b", false)).toBe(0);
    expect(pieceRotation("w", false)).toBe(180);
  });

  test("盤を反転すると向きも入れ替わる", () => {
    expect(pieceRotation("b", true)).toBe(180);
    expect(pieceRotation("w", true)).toBe(0);
  });
});

describe("座標の帯", () => {
  test.each(["sides", "stacked"] as const)(
    "%s では盤の上と右へ場所を空ける",
    (handLayout) => {
      const on = computeLayout(
        view({ handLayout, showCoordinates: true }),
        BOARD_SIZE,
      );
      const off = computeLayout(
        view({ handLayout, showCoordinates: false }),
        BOARD_SIZE,
      );

      // 番号を viewBox の外へ出すと、余白 0 の設定で端が切れてしまう。
      expect(on.board.y).toBeGreaterThan(off.board.y);
      expect(on.width - on.board.x - on.board.width).toBeGreaterThan(
        off.width - off.board.x - off.board.width,
      );
    },
  );

  test("余白 0 でも盤の上に座標のぶんの余地が残る", () => {
    const layout = computeLayout(
      view({ margin: 0, showCoordinates: true }),
      BOARD_SIZE,
    );
    expect(layout.board.y).toBeGreaterThan(0);
  });
});

describe("computeLayout (左右の駒台)", () => {
  const layout = computeLayout(view({ handLayout: "sides" }), BOARD_SIZE);

  test("マスは正方形で 81 個ある", () => {
    expect(layout.squares).toHaveLength(81);
    expect(layout.cell).toBe(60);
  });

  test("9一 が盤の左上、1九 が右下", () => {
    const topLeft = layout.squares[squareAt(9, 1)];
    const bottomRight = layout.squares[squareAt(1, 9)];
    expect(topLeft).toMatchObject({ x: layout.board.x, y: layout.board.y });
    expect(bottomRight).toMatchObject({
      x: layout.board.x + 8 * layout.cell,
      y: layout.board.y + 8 * layout.cell,
    });
  });

  test("駒台は両陣営分で 14 スロット", () => {
    expect(layout.hands).toHaveLength(14);
    expect(new Set(layout.hands.map((slot) => slot.side))).toEqual(
      new Set(["b", "w"]),
    );
  });

  test("先手の駒台が右、後手の駒台が左に来る", () => {
    const black = layout.hands.filter((slot) => slot.side === "b");
    const white = layout.hands.filter((slot) => slot.side === "w");
    expect(Math.min(...black.map((slot) => slot.rect.x))).toBeGreaterThan(
      layout.board.x,
    );
    expect(Math.max(...white.map((slot) => slot.rect.x))).toBeLessThan(
      layout.board.x,
    );
  });

  test("盤を反転すると駒台の左右も入れ替わる", () => {
    const flipped = computeLayout(
      view({ handLayout: "sides", flipped: true }),
      BOARD_SIZE,
    );
    const black = flipped.hands.filter((slot) => slot.side === "b");
    expect(Math.max(...black.map((slot) => slot.rect.x))).toBeLessThan(
      flipped.board.x,
    );
  });

  test("駒台の板が両陣営分あり、その陣営の 7 升をすべて含む", () => {
    expect(layout.handPlates).toHaveLength(2);
    for (const plate of layout.handPlates) {
      const slots = layout.hands.filter((slot) => slot.side === plate.side);
      expect(slots).toHaveLength(7);
      for (const slot of slots) {
        expect(slot.rect.x).toBeGreaterThanOrEqual(plate.rect.x);
        expect(slot.rect.y).toBeGreaterThanOrEqual(plate.rect.y);
        expect(slot.rect.x + slot.rect.width).toBeLessThanOrEqual(
          plate.rect.x + plate.rect.width,
        );
        expect(slot.rect.y + slot.rect.height).toBeLessThanOrEqual(
          plate.rect.y + plate.rect.height,
        );
      }
    }
  });

  test("盤と駒台が余白の内側へ収まる", () => {
    const items = [
      layout.board,
      ...layout.hands.map((slot) => slot.rect),
      ...layout.handPlates.map((plate) => plate.rect),
    ];
    for (const rect of items) {
      expect(rect.x).toBeGreaterThanOrEqual(0);
      expect(rect.y).toBeGreaterThanOrEqual(0);
      expect(rect.x + rect.width).toBeLessThanOrEqual(layout.width);
      expect(rect.y + rect.height).toBeLessThanOrEqual(layout.height);
    }
  });
});

describe("boardSizeToFit", () => {
  test.each(["sides", "stacked"] as const)(
    "%s では領域へちょうど収まる大きさを返す",
    (handLayout) => {
      const settings = view({ handLayout, margin: 20 });
      const size = boardSizeToFit(settings, 1280, 720);
      const layout = computeLayout(settings, size);

      expect(layout.width).toBeLessThanOrEqual(1280);
      expect(layout.height).toBeLessThanOrEqual(720);
      // 余白は layout の寸法に含まれるので、埋まっている側は領域そのものと一致する。
      const filled =
        Math.abs(layout.width - 1280) < 1 || Math.abs(layout.height - 720) < 1;
      expect(filled).toBe(true);
    },
  );

  test("領域が広いほど盤も大きくなる", () => {
    const settings = view();
    expect(boardSizeToFit(settings, 1920, 1080)).toBeGreaterThan(
      boardSizeToFit(settings, 960, 540),
    );
  });

  test("余白が領域より大きくても潰れない", () => {
    const size = boardSizeToFit(view({ margin: 200 }), 320, 240);
    expect(size).toBeGreaterThanOrEqual(MIN_FITTED_BOARD_SIZE);
  });
});

describe("computeLayout (上下の駒台)", () => {
  const layout = computeLayout(view({ handLayout: "stacked" }), BOARD_SIZE);

  test("幅は盤と余白と座標の帯で決まる", () => {
    const withoutCoordinates = computeLayout(
      view({ handLayout: "stacked", showCoordinates: false }),
      BOARD_SIZE,
    );
    expect(withoutCoordinates.width).toBe(BOARD_SIZE + 10 * 2);
    // 座標を出すぶんだけ右へ広がる。
    expect(layout.width).toBeGreaterThan(withoutCoordinates.width);
    expect(layout.height).toBeGreaterThan(layout.width);
  });

  test("先手の駒台が盤の下、後手の駒台が盤の上に来る", () => {
    const black = layout.hands.filter((slot) => slot.side === "b");
    const white = layout.hands.filter((slot) => slot.side === "w");
    expect(Math.min(...black.map((slot) => slot.rect.y))).toBeGreaterThan(
      layout.board.y,
    );
    expect(Math.max(...white.map((slot) => slot.rect.y))).toBeLessThan(
      layout.board.y,
    );
  });

  test("駒台の板が両陣営分あり、その陣営の 7 升をすべて含む", () => {
    expect(layout.handPlates).toHaveLength(2);
    for (const plate of layout.handPlates) {
      const slots = layout.hands.filter((slot) => slot.side === plate.side);
      expect(slots).toHaveLength(7);
      for (const slot of slots) {
        expect(slot.rect.x).toBeGreaterThanOrEqual(plate.rect.x);
        expect(slot.rect.y).toBeGreaterThanOrEqual(plate.rect.y);
        expect(slot.rect.x + slot.rect.width).toBeLessThanOrEqual(
          plate.rect.x + plate.rect.width,
        );
        expect(slot.rect.y + slot.rect.height).toBeLessThanOrEqual(
          plate.rect.y + plate.rect.height,
        );
      }
    }
  });

  test("盤と駒台が余白の内側へ収まる", () => {
    const items = [
      layout.board,
      ...layout.hands.map((slot) => slot.rect),
      ...layout.handPlates.map((plate) => plate.rect),
    ];
    for (const rect of items) {
      expect(rect.x).toBeGreaterThanOrEqual(0);
      expect(rect.x + rect.width).toBeLessThanOrEqual(layout.width);
      expect(rect.y + rect.height).toBeLessThanOrEqual(layout.height);
    }
  });
});
