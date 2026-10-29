import { describe, expect, test } from "bun:test";
import { DEFAULT_VIEW, type ViewSettings } from "~/shogi";
import { computeGoLayout, displayPoint } from "./goLayout";
import { boardSizeToFit } from "./layout";

const view = (patch: Partial<ViewSettings> = {}): ViewSettings => ({
  ...DEFAULT_VIEW,
  margin: 10,
  ...patch,
});

const within = (value: number, start: number, length: number): boolean =>
  value > start && value < start + length;

describe("computeGoLayout", () => {
  test.each([9, 13, 19])("%i 路の交点はすべて盤の内側にある", (size) => {
    const layout = computeGoLayout(view(), 540, size);
    expect(layout.points).toHaveLength(size * size);
    for (const point of layout.points) {
      expect(within(point.x, layout.board.x, layout.board.width)).toBe(true);
      expect(within(point.y, layout.board.y, layout.board.height)).toBe(true);
    }
    expect(layout.spacing).toBeCloseTo(540 / size);
  });

  test("反転すると左上と右下が入れ替わる", () => {
    expect(displayPoint(0, 19, true)).toBe(360);
    expect(displayPoint(0, 19, false)).toBe(0);
    const normal = computeGoLayout(view(), 540, 9);
    const flipped = computeGoLayout(view({ flipped: true }), 540, 9);
    expect(flipped.points[0]).toEqual(normal.points[80] ?? { x: 0, y: 0 });
  });

  test("手前のアゲハマは黒で、反転すると白になる", () => {
    const near = (patch: Partial<ViewSettings>) =>
      computeGoLayout(view(patch), 540, 19).bowls.reduce((a, b) =>
        a.rect.y > b.rect.y ? a : b,
      ).side;
    expect(near({})).toBe("b");
    expect(near({ flipped: true })).toBe("w");
    expect(near({ handLayout: "stacked" })).toBe("b");
  });

  test("アゲハマ置き場は盤と重ならない", () => {
    for (const handLayout of ["sides", "stacked"] as const) {
      const layout = computeGoLayout(view({ handLayout }), 540, 19);
      for (const { rect } of layout.bowls) {
        const overlaps =
          rect.x < layout.board.x + layout.board.width &&
          rect.x + rect.width > layout.board.x &&
          rect.y < layout.board.y + layout.board.height &&
          rect.y + rect.height > layout.board.y;
        expect(overlaps).toBe(false);
        expect(rect.x + rect.width).toBeLessThanOrEqual(layout.width);
        expect(rect.y + rect.height).toBeLessThanOrEqual(layout.height);
      }
    }
  });

  test("領域から決めた大きさで組むと領域に収まる", () => {
    for (const handLayout of ["sides", "stacked"] as const) {
      const v = view({ handLayout, margin: 24 });
      const fit = (vv: ViewSettings, size: number) =>
        computeGoLayout(vv, size, 19);
      const size = boardSizeToFit(v, 1280, 720, fit);
      const layout = fit(v, size);
      expect(layout.width).toBeLessThanOrEqual(1280);
      expect(layout.height).toBeLessThanOrEqual(720);
    }
  });
});
