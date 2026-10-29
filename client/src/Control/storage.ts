/**
 * 局面の保存。操作ページのローカルにだけ残る localStorage を使う。
 * サーバーへは送らないので、配信中の局面が勝手に増えることはない。
 *
 * 将棋 (SFEN) と囲碁 (SGF) は互いに読めないので、別々の鍵へ分けて置く。
 */

import type { GameMode } from "~/protocol";

const STORAGE_KEYS: Record<GameMode, string> = {
  // 囲碁に対応する前からある鍵。中身はそのまま読めるよう、名前を変えない。
  shogi: "stream-shougi-board.saved-positions.v1",
  go: "stream-shougi-board.saved-go-positions.v1",
};
export const MAX_SAVED = 30;

export interface SavedPosition {
  id: string;
  name: string;
  /** 局面の文字表記。将棋なら SFEN、囲碁なら SGF。 */
  text: string;
  savedAt: number;
}

/** 囲碁に対応する前の保存は、局面を `sfen` に入れていた。 */
function toSavedPosition(value: unknown): SavedPosition | null {
  if (typeof value !== "object" || value === null) return null;
  const record = value as Record<string, unknown>;
  const text = record.text ?? record.sfen;
  if (
    typeof record.id !== "string" ||
    typeof record.name !== "string" ||
    typeof text !== "string" ||
    typeof record.savedAt !== "number"
  ) {
    return null;
  }
  return { id: record.id, name: record.name, text, savedAt: record.savedAt };
}

/** 壊れた保存内容で操作ページごと落ちないよう、読めない要素は黙って捨てる。 */
export function parseSaved(raw: string | null): SavedPosition[] {
  if (raw === null) return [];
  try {
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return [];
    return value
      .map(toSavedPosition)
      .filter((entry): entry is SavedPosition => entry !== null)
      .slice(0, MAX_SAVED);
  } catch {
    return [];
  }
}

export function addSaved(
  current: SavedPosition[],
  entry: Omit<SavedPosition, "id">,
): SavedPosition[] {
  const id = `${entry.savedAt}-${Math.random().toString(36).slice(2, 8)}`;
  return [{ ...entry, id }, ...current].slice(0, MAX_SAVED);
}

export function removeSaved(
  current: SavedPosition[],
  id: string,
): SavedPosition[] {
  return current.filter((entry) => entry.id !== id);
}

export function loadSaved(mode: GameMode): SavedPosition[] {
  try {
    return parseSaved(window.localStorage.getItem(STORAGE_KEYS[mode]));
  } catch {
    // プライベートウィンドウなどで localStorage 自体が使えない場合。
    return [];
  }
}

export function storeSaved(mode: GameMode, entries: SavedPosition[]): void {
  try {
    window.localStorage.setItem(STORAGE_KEYS[mode], JSON.stringify(entries));
  } catch {
    // 保存できなくても操作は続けられるので握りつぶす。
  }
}

export function formatSavedLabel(entry: SavedPosition): string {
  const at = new Date(entry.savedAt);
  const stamp = `${at.getMonth() + 1}/${at.getDate()} ${String(at.getHours()).padStart(2, "0")}:${String(at.getMinutes()).padStart(2, "0")}`;
  return `${entry.name} (${stamp})`;
}
