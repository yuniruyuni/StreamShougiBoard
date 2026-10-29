import { describe, expect, test } from "bun:test";
import fixture from "../../protocol-fixtures/snapshot.json";
import type { Snapshot } from "./protocol";
import {
  HAND_ORDER,
  pieceAt,
  pieceKanji,
  SQUARE_COUNT,
  squareAt,
} from "./shogi";

/**
 * Rust 側 (`app/src/fixtures.rs`) が書き出した snapshot を、この型のまま読めるか確かめる。
 * 片側だけプロトコルを変えると、Rust 側かここのどちらかが必ず落ちる。
 */
const snapshot = fixture as unknown as Snapshot;

if (snapshot.game.mode !== "shogi") {
  throw new Error("snapshot.json は将棋の局面のはず");
}
const game = snapshot.game;

describe("Rust が書き出した snapshot", () => {
  test("client の型として受け取れる", () => {
    expect(snapshot.type).toBe("snapshot");
    // 版そのものはリリースのたびに変わるので、形だけを見る。
    expect(typeof snapshot.appVersion).toBe("string");
    expect(typeof snapshot.rev).toBe("number");
    expect(typeof game.sfen).toBe("string");
  });

  test("盤は 81 マスで、駒は表示に必要な形で入っている", () => {
    expect(game.board.squares).toHaveLength(SQUARE_COUNT);

    // 2二 は成った角 (馬)。表示側はこの 3 つだけを見て駒を描く。
    const uma = pieceAt(game.board, squareAt(2, 2));
    expect(uma).not.toBeNull();
    if (uma === null) return;
    expect(uma.kind).toBe("B");
    expect(uma.promoted).toBe(true);
    expect(uma.side).toBe("b");
    expect(pieceKanji(uma.kind, uma.promoted)).toBe("馬");
    expect(typeof uma.id).toBe("string");
  });

  test("持ち駒は 7 種すべてが揃っている", () => {
    for (const side of ["b", "w"] as const) {
      const hand = game.board.hands[side];
      for (const kind of HAND_ORDER) {
        expect(Array.isArray(hand[kind])).toBe(true);
      }
    }
    // 取った角が先手の駒台に 1 枚。
    expect(game.board.hands.b.B).toHaveLength(1);
  });

  test("直前の手と選択を描くための情報が入っている", () => {
    expect(game.board.lastMove).toEqual({
      from: squareAt(8, 8),
      to: squareAt(2, 2),
    });
    expect(game.selection).toEqual({
      kind: "hand",
      side: "b",
      pieceKind: "B",
    });
  });

  test("手番・手数・履歴を読める", () => {
    expect(game.board.turn).toBe("w");
    expect(typeof game.board.moveNumber).toBe("number");
    expect(snapshot.history.index).toBeLessThan(snapshot.history.length);
  });

  test("表示設定は client が使うキーをすべて持つ", () => {
    const view = snapshot.view;
    expect(view.backgroundColor).toBe("black");
    expect(typeof view.backgroundOpacity).toBe("number");
    expect(typeof view.margin).toBe("number");
    expect(view.handLayout).toBe("sides");
    for (const key of [
      "visible",
      "showLastMove",
      "showSelection",
      "showCoordinates",
      "flipped",
      "animate",
    ] as const) {
      expect(typeof view[key]).toBe("boolean");
    }
  });
});
