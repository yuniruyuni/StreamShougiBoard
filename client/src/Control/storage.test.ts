import { describe, expect, test } from "bun:test";
import { addSaved, MAX_SAVED, parseSaved, removeSaved } from "./storage";

describe("parseSaved", () => {
  test("保存が無ければ空", () => {
    expect(parseSaved(null)).toEqual([]);
  });

  test.each([
    ["壊れた JSON", "{"],
    ["配列でない", '{"a":1}'],
  ])("%s は空として扱う", (_label, raw) => {
    expect(parseSaved(raw)).toEqual([]);
  });

  test("形の合わない要素だけを捨てる", () => {
    const raw = JSON.stringify([
      { id: "a", name: "序盤", sfen: "9/9/9/9/9/9/9/9/9 b - 1", savedAt: 1 },
      { id: "b", name: "壊れ" },
      null,
    ]);
    const parsed = parseSaved(raw);
    expect(parsed).toHaveLength(1);
    expect(parsed[0]?.name).toBe("序盤");
  });
});

describe("addSaved / removeSaved", () => {
  test("新しいものを先頭へ積み、上限で古いものを落とす", () => {
    let entries = addSaved([], { name: "1", sfen: "s1", savedAt: 1 });
    for (let i = 2; i <= MAX_SAVED + 5; i += 1) {
      entries = addSaved(entries, {
        name: String(i),
        sfen: `s${i}`,
        savedAt: i,
      });
    }
    expect(entries).toHaveLength(MAX_SAVED);
    expect(entries[0]?.name).toBe(String(MAX_SAVED + 5));
    expect(entries.some((entry) => entry.name === "1")).toBe(false);
  });

  test("id を指定して削除する", () => {
    const entries = addSaved([], { name: "a", sfen: "s", savedAt: 1 });
    const id = entries[0]?.id ?? "";
    expect(removeSaved(entries, id)).toEqual([]);
    expect(removeSaved(entries, "other")).toHaveLength(1);
  });
});
