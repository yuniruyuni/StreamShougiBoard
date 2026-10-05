/**
 * ビルドした exe を常駐先へ置き換える。
 *
 * 置き場所では `StreamShougiBoard.exe` という名前で置く。cargo が吐くのは
 * パッケージ名の `stream-shougi-board.exe` なので、ここで名前を合わせる。
 * release workflow も同じように配布名へ付け替えている。
 *
 * 常駐中の exe は Windows 側でロックされており、上書きすると壊れた exe が残りうる。
 * 既定ポートが応答するうちは何も書かずに終了する。
 */

import { copyFileSync, mkdirSync, renameSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const BUILT = "stream-shougi-board.exe";
const DEPLOYED = "StreamShougiBoard.exe";
const destination =
  process.env.STREAM_SHOUGI_DEPLOY_DIR ?? "/mnt/d/tools/StreamShougiBoard";
const source = join(
  projectRoot,
  "app/target/x86_64-pc-windows-gnu/release",
  BUILT,
);

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

let built: number;
try {
  built = statSync(source).size;
} catch {
  fail(
    `ビルド済み exe がありません: ${source}\n` +
      "先に bun run build:windows を実行してください。",
  );
}

// 埋め込みマニフェストが崩れた exe は「サイドバイサイド構成が正しくありません」で
// 起動すらできない。見た目では分からないので、配置前に実物の PE を検査する。
const manifest = Bun.spawnSync([
  "bun",
  join(projectRoot, "scripts/check-windows-manifest.ts"),
  source,
]);
if (manifest.exitCode !== 0)
  fail(
    `exe の検査に失敗しました。配置を中止します。\n${manifest.stderr.toString()}`,
  );

const running = await fetch("http://127.0.0.1:16874/board", {
  signal: AbortSignal.timeout(2000),
})
  .then(() => true)
  .catch(() => false);
if (running)
  fail(
    "StreamShougiBoard が常駐中です (127.0.0.1:16874 が応答)。\n" +
      "タスクトレイから終了してから実行してください。",
  );

mkdirSync(destination, { recursive: true });
const target = join(destination, DEPLOYED);
const staged = `${target}.new`;
const previous = `${target}.previous`;

// 同じディレクトリへ書いてから rename する。途中で失敗しても常駐先に半端な exe を残さない。
copyFileSync(source, staged);
try {
  rmSync(previous, { force: true });
  try {
    renameSync(target, previous);
  } catch {
    // 初回配置。戻す先が無いだけなので続行する。
  }
  renameSync(staged, target);
} catch (error) {
  rmSync(staged, { force: true });
  fail(
    `配置できませんでした: ${error}\n` +
      "exe が実行中だとロックされます。常駐していないか確かめてください。",
  );
}

console.log(
  `配置しました: ${target} (${built.toLocaleString()} bytes)\n` +
    `直前の exe: ${previous}`,
);
