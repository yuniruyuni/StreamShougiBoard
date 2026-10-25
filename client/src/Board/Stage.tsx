import type { Snapshot } from "~/protocol";
import { ShogiBoard } from "./component";
import { boardSizeToFit } from "./layout";
import { backgroundColor } from "./theme";

interface BoardStageProps {
  snapshot: Snapshot;
  width: number;
  height: number;
  gone: boolean;
}

/** OBS用の表示境界。操作画面のShogiBoardは非表示設定の影響を受けない。 */
export function BoardStage({ snapshot, width, height, gone }: BoardStageProps) {
  if (!snapshot.view.visible) return null;

  // 背景も含めてOBSの領域全体を表示・非表示にする。
  const background = backgroundColor(snapshot.view);
  const boardSize = boardSizeToFit(snapshot.view, width, height);
  return (
    <div
      className={gone ? "stage stage--gone" : "stage"}
      style={background === null ? undefined : { background }}
    >
      <ShogiBoard
        board={snapshot.board}
        view={snapshot.view}
        boardSize={boardSize}
        selection={snapshot.selection}
        showSelection={snapshot.view.showSelection}
        paintBackground={false}
      />
    </div>
  );
}
