/** OBS 側の見た目を変える操作。変更は set_view としてサーバーへ送り、盤面ページへ即座に届く。 */

import type { Command } from "~/protocol";
import {
  type BackgroundColor,
  type HandLayout,
  MAX_BACKGROUND_OPACITY,
  MAX_MARGIN,
  MIN_BACKGROUND_OPACITY,
  MIN_MARGIN,
  type ViewSettings,
} from "~/shogi";

const BACKGROUND_LABELS: { value: BackgroundColor; label: string }[] = [
  { value: "white", label: "白" },
  { value: "black", label: "黒" },
];

const LAYOUT_LABELS: { value: HandLayout; label: string }[] = [
  { value: "sides", label: "左右" },
  { value: "stacked", label: "上下" },
];

const TOGGLES: { key: keyof ViewSettings; label: string; hint: string }[] = [
  { key: "showLastMove", label: "直前の手", hint: "動いたマスを光らせる" },
  {
    key: "showSelection",
    label: "選択枠もOBSへ",
    hint: "手元の選択枠を配信にも出す",
  },
  {
    key: "showCoordinates",
    label: "筋・段の番号",
    hint: "9〜1 と 一〜九 を表示する",
  },
  { key: "flipped", label: "盤を反転", hint: "後手視点にする" },
  { key: "animate", label: "駒を滑らせる", hint: "移動を補間して描く" },
];

export interface ViewPanelProps {
  view: ViewSettings;
  send(command: Command): void;
}

export function ViewPanel({ view, send }: ViewPanelProps) {
  const patch = (partial: Partial<ViewSettings>) =>
    send({ type: "set_view", view: partial });

  return (
    <section className="panel">
      <h2 className="panel__title">OBS の見た目</h2>

      <div className="field">
        <span className="field__label">背景の色</span>
        <div className="segmented">
          {BACKGROUND_LABELS.map((option) => (
            <button
              key={option.value}
              type="button"
              className={
                view.backgroundColor === option.value
                  ? "segmented__item is-on"
                  : "segmented__item"
              }
              onClick={() => patch({ backgroundColor: option.value })}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      <label className="field" htmlFor="background-opacity">
        <span className="field__label">
          背景の濃さ{" "}
          <span className="field__value">{view.backgroundOpacity}%</span>
        </span>
        <input
          id="background-opacity"
          type="range"
          min={MIN_BACKGROUND_OPACITY}
          max={MAX_BACKGROUND_OPACITY}
          step={5}
          value={view.backgroundOpacity}
          onChange={(event) =>
            patch({ backgroundOpacity: Number(event.target.value) })
          }
        />
      </label>

      <div className="field">
        <span className="field__label">駒台</span>
        <div className="segmented">
          {LAYOUT_LABELS.map((option) => (
            <button
              key={option.value}
              type="button"
              className={
                view.handLayout === option.value
                  ? "segmented__item is-on"
                  : "segmented__item"
              }
              onClick={() => patch({ handLayout: option.value })}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      <label className="field" htmlFor="board-margin">
        <span className="field__label">
          外周の余白 <span className="field__value">{view.margin}px</span>
        </span>
        <input
          id="board-margin"
          type="range"
          min={MIN_MARGIN}
          max={MAX_MARGIN}
          step={2}
          value={view.margin}
          onChange={(event) => patch({ margin: Number(event.target.value) })}
        />
        <span className="field__note">
          盤の大きさは OBS
          のブラウザソースの領域いっぱいに広がる。ここで決まるのは領域の縁から空ける
          px
        </span>
      </label>

      <ul className="toggles">
        {TOGGLES.map((toggle) => (
          <li key={toggle.key}>
            <label className="toggle" title={toggle.hint}>
              <input
                type="checkbox"
                checked={view[toggle.key] === true}
                onChange={(event) =>
                  patch({ [toggle.key]: event.target.checked })
                }
              />
              <span>{toggle.label}</span>
            </label>
          </li>
        ))}
      </ul>
    </section>
  );
}
