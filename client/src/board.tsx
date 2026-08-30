/**
 * OBS Browser Source 用のページ。操作 UI を一切持たず、サーバーの snapshot を描くだけ。
 *
 * 切断しても即座には消さない。アプリの再起動や設定の反映で一瞬切れることがあり、
 * そのたびに盤が明滅すると配信に出せないため、猶予を過ぎてから初めて消す。
 */

import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { ShogiBoard } from "~/Board/component";
import { boardSizeToFit } from "~/Board/layout";
import { backgroundColor } from "~/Board/theme";
import { useSession } from "~/useSession";

/** これを超えて切断が続いたら、古い盤面を配信から引っ込める。 */
const DISCONNECT_GRACE_MS = 3_000;

/**
 * ブラウザソースの領域。盤の大きさはここから決めるので、設定としては持たない。
 * OBS でソースの大きさを変えると resize が飛んでくるので、そのまま追従する。
 */
function useViewportSize() {
  const [size, setSize] = useState(() => ({
    width: window.innerWidth,
    height: window.innerHeight,
  }));

  useEffect(() => {
    const onResize = () =>
      setSize({ width: window.innerWidth, height: window.innerHeight });
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return size;
}

function BoardPage() {
  const { snapshot, status } = useSession();
  const { width, height } = useViewportSize();
  const [gone, setGone] = useState(false);

  useEffect(() => {
    if (status === "connected") {
      setGone(false);
      return;
    }
    const timer = window.setTimeout(() => setGone(true), DISCONNECT_GRACE_MS);
    return () => window.clearTimeout(timer);
  }, [status]);

  if (snapshot === null) return null;

  // 地はブラウザソースの領域いっぱいに敷く。盤は領域に合わせて伸縮するので、
  // SVG の中を塗ると余った上下左右が塗り残しになる。
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

const root = document.getElementById("root");
if (root !== null) {
  // StrictMode は effect を二重に走らせて WebSocket を二重接続するので使わない。
  ReactDOM.createRoot(root).render(<BoardPage />);
}
