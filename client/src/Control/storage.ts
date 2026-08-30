/**
 * 局面の保存。操作ページのローカルにだけ残る localStorage を使う。
 * サーバーへは送らないので、配信中の局面が勝手に増えることはない。
 */

const STORAGE_KEY = "stream-shougi-board.saved-positions.v1";
export const MAX_SAVED = 30;

export interface SavedPosition {
  id: string;
  name: string;
  sfen: string;
  savedAt: number;
}

function isSavedPosition(value: unknown): value is SavedPosition {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Record<string, unknown>;
  return (
    typeof record.id === "string" &&
    typeof record.name === "string" &&
    typeof record.sfen === "string" &&
    typeof record.savedAt === "number"
  );
}

/** 壊れた保存内容で操作ページごと落ちないよう、読めない要素は黙って捨てる。 */
export function parseSaved(raw: string | null): SavedPosition[] {
  if (raw === null) return [];
  try {
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return [];
    return value.filter(isSavedPosition).slice(0, MAX_SAVED);
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

export function loadSaved(): SavedPosition[] {
  try {
    return parseSaved(window.localStorage.getItem(STORAGE_KEY));
  } catch {
    // プライベートウィンドウなどで localStorage 自体が使えない場合。
    return [];
  }
}

export function storeSaved(entries: SavedPosition[]): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // 保存できなくても操作は続けられるので握りつぶす。
  }
}

export function formatSavedLabel(entry: SavedPosition): string {
  const at = new Date(entry.savedAt);
  const stamp = `${at.getMonth() + 1}/${at.getDate()} ${String(at.getHours()).padStart(2, "0")}:${String(at.getMinutes()).padStart(2, "0")}`;
  return `${entry.name} (${stamp})`;
}
