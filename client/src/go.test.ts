import { describe, expect, test } from "bun:test";
import goFixture from "../../protocol-fixtures/go-snapshot.json";
import {
  columnLabel,
  pointAt,
  pointName,
  rowLabel,
  starPoints,
  stoneAt,
} from "./go";
import type { Snapshot } from "./protocol";

const snapshot = goFixture as unknown as Snapshot;

describe("Rust が書き出した囲碁の snapshot", () => {
  test("client の型として受け取れる", () => {
    expect(snapshot.type).toBe("snapshot");
    expect(snapshot.game.mode).toBe("go");
    if (snapshot.game.mode !== "go") return;
    const { board, sgf } = snapshot.game;

    expect(typeof sgf).toBe("string");
    expect(board.size).toBe(9);
    expect(board.points).toHaveLength(81);
    // 黒が白 e5 を取った直後に、白を 1 つ手番を動かさずに並べた局面。
    expect(stoneAt(board, pointAt(9, 4, 4))).toBeNull();
    expect(stoneAt(board, pointAt(9, 4, 5))).toBe("b");
    expect(board.captures).toEqual({ b: 1, w: 0 });
    expect(board.lastMove).toBe(80);
    expect(board.turn).toBe("w");
    expect(typeof board.moveNumber).toBe("number");
    expect(snapshot.history.index).toBeLessThan(snapshot.history.length);
  });
});

describe("座標", () => {
  test("横は I を飛ばした英字、縦は下から数える", () => {
    expect(columnLabel(0)).toBe("A");
    expect(columnLabel(8)).toBe("J");
    expect(columnLabel(18)).toBe("T");
    expect(rowLabel(19, 0)).toBe("19");
    expect(rowLabel(19, 18)).toBe("1");
    expect(pointName(19, pointAt(19, 3, 15))).toBe("D4");
  });
});

describe("starPoints", () => {
  test("路数ごとの星の数", () => {
    expect(starPoints(19)).toHaveLength(9);
    expect(starPoints(13)).toHaveLength(5);
    expect(starPoints(9)).toHaveLength(5);
    expect(starPoints(7)).toEqual([[3, 3]]);
    expect(starPoints(6)).toEqual([]);
  });

  test("19 路は四隅の星が 4 の 4 にある", () => {
    expect(starPoints(19)).toContainEqual([3, 3]);
    expect(starPoints(19)).toContainEqual([15, 15]);
    expect(starPoints(19)).toContainEqual([9, 9]);
    expect(starPoints(9)).toContainEqual([2, 6]);
  });
});
