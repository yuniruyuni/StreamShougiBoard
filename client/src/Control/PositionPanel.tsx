/**
 * 局面の文字表記 (将棋は SFEN、囲碁は SGF) の表示・入力と、操作ページのローカルに置く保存局面。
 */

import { type ChangeEvent, useEffect, useState } from "react";
import type { Command, GameMode } from "~/protocol";
import {
  addSaved,
  formatSavedLabel,
  loadSaved,
  removeSaved,
  type SavedPosition,
  storeSaved,
} from "./storage";

/** SGF の棋譜ファイルとして受け付ける上限。サーバーのフレーム上限より十分小さく取る。 */
const MAX_SGF_FILE_BYTES = 200 * 1024;

interface Format {
  name: string;
  placeholder: string;
  load(text: string): Command;
}

const FORMATS: Record<GameMode, Format> = {
  shogi: {
    name: "SFEN",
    placeholder: "sfen を貼り付け",
    load: (sfen) => ({ type: "set_sfen", sfen }),
  },
  go: {
    name: "SGF",
    placeholder: "sgf を貼り付け (棋譜なら手順ごと読み込みます)",
    load: (sgf) => ({ type: "set_sgf", sgf }),
  },
};

export interface PositionPanelProps {
  mode: GameMode;
  /** サーバーが整えた現在局面の表記。 */
  text: string;
  send(command: Command): void;
}

export function PositionPanel({ mode, text, send }: PositionPanelProps) {
  const format = FORMATS[mode];
  const [input, setInput] = useState("");
  const [name, setName] = useState("");
  const [saved, setSaved] = useState<SavedPosition[]>([]);
  const [copied, setCopied] = useState(false);
  const [fileError, setFileError] = useState<string | null>(null);

  useEffect(() => {
    setSaved(loadSaved(mode));
    setInput("");
    setFileError(null);
  }, [mode]);

  const update = (entries: SavedPosition[]) => {
    setSaved(entries);
    storeSaved(mode, entries);
  };

  const copy = () => {
    void navigator.clipboard.writeText(text).then(
      () => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1_200);
      },
      () => setCopied(false),
    );
  };

  const save = () => {
    const label = name.trim() === "" ? "保存局面" : name.trim();
    update(addSaved(saved, { name: label, text, savedAt: Date.now() }));
    setName("");
  };

  const openFile = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    // 同じファイルを続けて選んでも change が飛ぶように、選択を空へ戻しておく。
    event.target.value = "";
    if (file === undefined) return;
    if (file.size > MAX_SGF_FILE_BYTES) {
      setFileError("ファイルが大きすぎます");
      return;
    }
    setFileError(null);
    void file.text().then(
      (content) => send(format.load(content)),
      () => setFileError("ファイルを読めませんでした"),
    );
  };

  return (
    <section className="panel">
      <h2 className="panel__title">{format.name}</h2>

      <p className="sfen">{text}</p>
      <button type="button" className="button" onClick={copy}>
        {copied ? "コピーしました" : `現在の ${format.name} をコピー`}
      </button>

      <div className="row">
        {mode === "go" ? (
          // SGF の棋譜は改行を含むので、1 行の入力欄では貼り付けた中身を確かめにくい。
          <textarea
            className="input input--multiline"
            placeholder={format.placeholder}
            rows={3}
            value={input}
            onChange={(event) => setInput(event.target.value)}
          />
        ) : (
          <input
            type="text"
            className="input"
            placeholder={format.placeholder}
            value={input}
            onChange={(event) => setInput(event.target.value)}
          />
        )}
        <button
          type="button"
          className="button"
          disabled={input.trim() === ""}
          onClick={() => send(format.load(input.trim()))}
        >
          読み込み
        </button>
      </div>

      {mode === "go" && (
        <div className="row">
          <label className="button">
            SGF ファイルを開く
            <input
              type="file"
              accept=".sgf,application/x-go-sgf,text/plain"
              className="visually-hidden"
              onChange={openFile}
            />
          </label>
          {fileError !== null && <span className="muted">{fileError}</span>}
        </div>
      )}

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
                title={entry.text}
                onClick={() => send(format.load(entry.text))}
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
