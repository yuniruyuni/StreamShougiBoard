import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { Snapshot } from "~/protocol";
import fixture from "../../../protocol-fixtures/snapshot.json";
import { ShogiBoard } from "./component";
import { BoardStage } from "./Stage";

const snapshot = fixture as Snapshot;

describe("OBSへの表示切り替え", () => {
  test("非表示では不透明な背景・盤・持ち駒をすべて描画しない", () => {
    const hidden = {
      ...snapshot,
      view: { ...snapshot.view, visible: false, backgroundOpacity: 100 },
    };
    expect(
      renderToStaticMarkup(
        <BoardStage
          snapshot={hidden}
          width={1920}
          height={1080}
          gone={false}
        />,
      ),
    ).toBe("");
  });

  test("再表示すると同じ盤面と背景を描画する", () => {
    const shown = {
      ...snapshot,
      view: { ...snapshot.view, visible: true, backgroundOpacity: 100 },
    };
    const html = renderToStaticMarkup(
      <BoardStage snapshot={shown} width={1920} height={1080} gone={false} />,
    );
    expect(html).toContain('class="stage"');
    expect(html).toContain("background:");
    expect(html).toContain("<svg");
  });

  test("OBS非表示中でも操作画面の盤と駒を描画する", () => {
    const html = renderToStaticMarkup(
      <ShogiBoard
        board={snapshot.board}
        view={{ ...snapshot.view, visible: false }}
        boardSize={540}
        selection={snapshot.selection}
        showSelection={true}
      />,
    );
    expect(html).toContain("<svg");
    expect(html).toContain("馬");
  });
});
