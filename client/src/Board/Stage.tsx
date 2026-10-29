import type { Snapshot } from "~/protocol";
import { ShogiBoard } from "./component";
import { GoBoardView } from "./GoBoard";
import { computeGoLayout } from "./goLayout";
import { boardSizeToFit } from "./layout";
import { backgroundColor } from "./theme";

interface BoardStageProps {
  snapshot: Snapshot;
  width: number;
  height: number;
  gone: boolean;
}

/** OBS用の表示境界。操作画面の盤は非表示設定の影響を受けない。 */
export function BoardStage({ snapshot, width, height, gone }: BoardStageProps) {
  const { view, game } = snapshot;
  if (!view.visible) return null;

  // 背景も含めてOBSの領域全体を表示・非表示にする。
  const background = backgroundColor(view);
  return (
    <div
      className={gone ? "stage stage--gone" : "stage"}
      style={background === null ? undefined : { background }}
    >
      {game.mode === "shogi" ? (
        <ShogiBoard
          board={game.board}
          view={view}
          boardSize={boardSizeToFit(view, width, height)}
          selection={game.selection}
          showSelection={view.showSelection}
          paintBackground={false}
        />
      ) : (
        <GoBoardView
          board={game.board}
          view={view}
          boardSize={boardSizeToFit(view, width, height, (v, size) =>
            computeGoLayout(v, size, game.board.size),
          )}
          paintBackground={false}
        />
      )}
    </div>
  );
}
