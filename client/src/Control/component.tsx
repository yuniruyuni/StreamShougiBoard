/**
 * 操作ページ。盤のクリックをコマンドとしてサーバーへ送り、返ってきた snapshot を描く。
 *
 * 盤そのものは盤面ページと同じ ShogiBoard / GoBoardView を同じ view で描くので、
 * ここに見えているものが、そのまま OBS に出ているものになる。
 * 違うのは大きさだけで、OBS 側はブラウザソースの領域に合わせて伸縮する。
 */

import { useCallback, useEffect, useRef } from "react";
import { type BoardHandlers, ShogiBoard } from "~/Board/component";
import { type GoBoardHandlers, GoBoardView } from "~/Board/GoBoard";
import type { ConnectionStatus } from "~/connection";
import { GO_SIZES } from "~/go";
import type { Command, GameMode, Snapshot } from "~/protocol";
import type { Side } from "~/shogi";
import { useSession } from "~/useSession";
import { PositionPanel } from "./PositionPanel";
import { ViewPanel } from "./ViewPanel";

/**
 * ダブルクリックの 1 打目を待つ時間。
 * 何も選んでいないときだけ遅らせるので、駒を動かす操作には遅延がかからない。
 */
const DOUBLE_CLICK_WINDOW_MS = 180;

/**
 * プレビューを描く大きさ。CSS で枠の幅まで縮むので、ここは解像度を決めるだけ。
 * OBS 側の大きさとは無関係で、比率だけが一致する。
 */
const PREVIEW_BOARD_SIZE = 560;

const MODES: { mode: GameMode; label: string }[] = [
  { mode: "shogi", label: "将棋" },
  { mode: "go", label: "囲碁" },
];

const PRESETS: Record<GameMode, { command: Command; label: string }[]> = {
  shogi: [
    { command: { type: "preset", name: "hirate" }, label: "盤面初期化" },
    { command: { type: "preset", name: "allPieces" }, label: "全駒" },
    { command: { type: "preset", name: "empty" }, label: "空の盤" },
  ],
  go: GO_SIZES.map((size) => ({
    command: { type: "go_new", size },
    label: `${size}路で初期化`,
  })),
};

const SIDE_LABEL: Record<GameMode, Record<Side, string>> = {
  shogi: { b: "先手", w: "後手" },
  go: { b: "黒番", w: "白番" },
};

const HINTS: Record<GameMode, string> = {
  shogi:
    "クリックで選択・移動、駒台へ送るときは駒を選んでから駒台をクリック。ダブルクリックで成／不成、右クリックで先後を反転。 ←→ で履歴、Esc で選択解除。",
  go: "クリックで手番の石を置き、囲んだ石は自動で取ります。Shift+クリックで黒だけ、Ctrl (Mac は ⌘)+クリックで白だけを手番を変えずに並べ、Alt+クリックか右クリックで石を消します。 ←→ で履歴。",
};

function StatusBadge({ status }: { status: ConnectionStatus }) {
  return (
    <span
      className={
        status === "connected" ? "badge badge--on" : "badge badge--off"
      }
    >
      {status === "connected" ? "接続中" : "切断"}
    </span>
  );
}

function ObsUrlRow() {
  const url = `${window.location.origin}/board`;
  return (
    <div className="obs-url">
      <span className="obs-url__label">OBS のブラウザソースに設定する URL</span>
      <code className="obs-url__value">{url}</code>
      <button
        type="button"
        className="button"
        onClick={() => void navigator.clipboard.writeText(url)}
      >
        コピー
      </button>
    </div>
  );
}

function BoardPanel({
  snapshot,
  send,
}: {
  snapshot: Snapshot;
  send(command: Command): void;
}) {
  const { history, game } = snapshot;
  const { mode, board } = game;
  const canBack = history.index > 0;
  const canForward = history.index < history.length - 1;

  return (
    <section className="panel">
      <h2 className="panel__title">盤面</h2>

      <div className="field">
        <span className="field__label">ゲーム</span>
        <div className="segmented">
          {MODES.map((option) => (
            <button
              key={option.mode}
              type="button"
              className={
                mode === option.mode
                  ? "segmented__item is-on"
                  : "segmented__item"
              }
              onClick={() => send({ type: "set_mode", mode: option.mode })}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      <div className="row row--wrap">
        {PRESETS[mode].map((preset) => (
          <button
            key={preset.label}
            type="button"
            className="button"
            onClick={() => send(preset.command)}
          >
            {preset.label}
          </button>
        ))}
      </div>

      <div className="field">
        <span className="field__label">手番</span>
        <div className="segmented">
          {(["b", "w"] as const).map((side) => (
            <button
              key={side}
              type="button"
              className={
                board.turn === side
                  ? "segmented__item is-on"
                  : "segmented__item"
              }
              onClick={() => send({ type: "set_turn", side })}
            >
              {SIDE_LABEL[mode][side]}
            </button>
          ))}
        </div>
      </div>

      <div className="field">
        <span className="field__label">
          履歴{" "}
          <span className="field__value">
            {history.index + 1} / {history.length}
          </span>
        </span>
        <div className="row">
          <button
            type="button"
            className="button"
            disabled={!canBack}
            onClick={() =>
              send({ type: "history_go", index: history.index - 1 })
            }
          >
            ＜
          </button>
          <select
            className="input"
            value={history.index}
            onChange={(event) =>
              send({ type: "history_go", index: Number(event.target.value) })
            }
          >
            {Array.from({ length: history.length }, (_, index) => (
              // 履歴は位置そのものが識別子で、途中に挿入されることもない。
              // biome-ignore lint/suspicious/noArrayIndexKey: index は履歴の位置そのもの
              <option key={`history-${index}`} value={index}>
                {index === 0 ? "開始局面" : `${index} 手目`}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="button"
            disabled={!canForward}
            onClick={() =>
              send({ type: "history_go", index: history.index + 1 })
            }
          >
            ＞
          </button>
        </div>
      </div>
    </section>
  );
}

export function ControlPage() {
  const { snapshot, status, rejected, send } = useSession();
  const pendingClick = useRef<number | null>(null);

  const cancelPendingClick = useCallback(() => {
    if (pendingClick.current === null) return;
    window.clearTimeout(pendingClick.current);
    pendingClick.current = null;
  }, []);

  useEffect(() => cancelPendingClick, [cancelPendingClick]);

  const hasSelection =
    snapshot?.game.mode === "shogi" && snapshot.game.selection !== null;

  const handlers: BoardHandlers = {
    onSquare: (square) => {
      cancelPendingClick();
      if (hasSelection) {
        // 駒を持っている状態のクリックは移動なので、遅らせずに即座に送る。
        send({ type: "tap_square", square });
        return;
      }
      // 何も持っていないときのクリックは選択なので、ダブルクリックに化ける余地を残す。
      pendingClick.current = window.setTimeout(() => {
        pendingClick.current = null;
        send({ type: "tap_square", square });
      }, DOUBLE_CLICK_WINDOW_MS);
    },
    onHand: (side, pieceKind) => {
      cancelPendingClick();
      send({ type: "tap_hand", side, pieceKind });
    },
    onSquareDoubleClick: (square) => {
      cancelPendingClick();
      send({ type: "toggle_promote", square });
    },
    onSquareContextMenu: (square) => {
      cancelPendingClick();
      send({ type: "flip_piece", square });
    },
  };

  const goHandlers: GoBoardHandlers = {
    onPoint: (point, tool) => {
      switch (tool) {
        case "turn":
          send({ type: "go_place", point });
          return;
        case "black":
          send({ type: "go_place", point, color: "b" });
          return;
        case "white":
          send({ type: "go_place", point, color: "w" });
          return;
        case "erase":
          send({ type: "go_remove", point });
          return;
      }
    },
  };

  useEffect(() => {
    if (snapshot === null) return;
    const onKey = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement
      )
        return;
      if (event.key === "ArrowLeft" && snapshot.history.index > 0) {
        send({ type: "history_go", index: snapshot.history.index - 1 });
      } else if (
        event.key === "ArrowRight" &&
        snapshot.history.index < snapshot.history.length - 1
      ) {
        send({ type: "history_go", index: snapshot.history.index + 1 });
      } else if (event.key === "Escape") {
        send({ type: "clear_selection" });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [snapshot, send]);

  if (snapshot === null) {
    return (
      <div className="app app--waiting">
        <p>サーバーへ接続しています…</p>
      </div>
    );
  }

  const { game } = snapshot;
  const text = game.mode === "shogi" ? game.sfen : game.sgf;

  return (
    <div className="app">
      <header className="header">
        <h1 className="header__title">StreamShougiBoard</h1>
        <StatusBadge status={status} />
        <ObsUrlRow />
      </header>

      <main className="layout">
        <section className="preview">
          <div className="preview__canvas">
            {game.mode === "shogi" ? (
              <ShogiBoard
                board={game.board}
                view={snapshot.view}
                boardSize={PREVIEW_BOARD_SIZE}
                selection={game.selection}
                showSelection
                handlers={handlers}
              />
            ) : (
              <GoBoardView
                board={game.board}
                view={snapshot.view}
                boardSize={PREVIEW_BOARD_SIZE}
                handlers={goHandlers}
              />
            )}
          </div>
          <p className="hint">{HINTS[game.mode]}</p>
          <p className={rejected === null ? "reject reject--hidden" : "reject"}>
            {rejected ?? " "}
          </p>
        </section>

        <aside className="panels">
          <BoardPanel snapshot={snapshot} send={send} />
          <ViewPanel view={snapshot.view} mode={game.mode} send={send} />
          {/* モードごとに保存先も入力中の文字も別なので、切り替えたら作り直す。 */}
          <PositionPanel
            key={game.mode}
            mode={game.mode}
            text={text}
            send={send}
          />
        </aside>
      </main>

      <footer className="footer">
        <span>StreamShougiBoard は MIT License で提供されます。</span>
        <a
          className="footer__link"
          href="/licenses"
          target="_blank"
          rel="noreferrer"
        >
          第三者ライセンス
        </a>
      </footer>
    </div>
  );
}
