/** SFEN の表示・入力と、操作ページのローカルに置く保存局面。 */

import { useEffect, useState } from "react";
import type { Command } from "~/protocol";
import {
  addSaved,
  formatSavedLabel,
  loadSaved,
  removeSaved,
  type SavedPosition,
  storeSaved,
} from "./storage";

export interface SfenPanelProps {
  sfen: string;
  send(command: Command): void;
}

export function SfenPanel({ sfen, send }: SfenPanelProps) {
  const [input, setInput] = useState("");
  const [name, setName] = useState("");
  const [saved, setSaved] = useState<SavedPosition[]>([]);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    setSaved(loadSaved());
  }, []);

  const update = (entries: SavedPosition[]) => {
    setSaved(entries);
    storeSaved(entries);
  };

  const copy = () => {
    void navigator.clipboard.writeText(sfen).then(
      () => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1_200);
      },
      () => setCopied(false),
    );
  };

  const save = () => {
    const label = name.trim() === "" ? "保存局面" : name.trim();
    update(addSaved(saved, { name: label, sfen, savedAt: Date.now() }));
    setName("");
  };

  return (
    <section className="panel">
      <h2 className="panel__title">SFEN</h2>

      <p className="sfen">{sfen}</p>
      <button type="button" className="button" onClick={copy}>
        {copied ? "コピーしました" : "現在の SFEN をコピー"}
      </button>

      <div className="row">
        <input
          type="text"
          className="input"
          placeholder="sfen を貼り付け"
          value={input}
          onChange={(event) => setInput(event.target.value)}
        />
        <button
          type="button"
          className="button"
          disabled={input.trim() === ""}
          onClick={() => send({ type: "set_sfen", sfen: input.trim() })}
        >
          読み込み
        </button>
      </div>

      <h3 className="panel__subtitle">保存した局面</h3>
      <div className="row">
        <input
          type="text"
          className="input"
          placeholder="名前 (省略可)"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        <button type="button" className="button" onClick={save}>
          今の局面を保存
        </button>
      </div>

      {saved.length === 0 ? (
        <p className="muted">まだ保存していません。</p>
      ) : (
        <ul className="saved">
          {saved.map((entry) => (
            <li key={entry.id} className="saved__item">
              <button
                type="button"
                className="saved__load"
                title={entry.sfen}
                onClick={() => send({ type: "set_sfen", sfen: entry.sfen })}
              >
                {formatSavedLabel(entry)}
              </button>
              <button
                type="button"
                className="saved__delete"
                aria-label={`${entry.name} を削除`}
                onClick={() => update(removeSaved(saved, entry.id))}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
