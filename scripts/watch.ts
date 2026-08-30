/**
 * 開発用。client の watch ビルドと Rust サーバーを同時に走らせる。
 *
 * debug ビルドの rust-embed は `client/static` を実行時に読むので、
 * ページを直しただけならサーバーの再起動は要らない。Rust を直したときは Ctrl+C して入れ直す。
 */

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

const children = [
  Bun.spawn(["bun", "run", "bin/build.ts", "--watch"], {
    cwd: join(projectRoot, "client"),
    stdout: "inherit",
    stderr: "inherit",
  }),
  Bun.spawn(
    ["cargo", "run", "--manifest-path", join(projectRoot, "app", "Cargo.toml")],
    {
      cwd: projectRoot,
      stdout: "inherit",
      stderr: "inherit",
      // watch 中はファイルを直すたびに立ち上げ直すので、そのたびにタブが増えないようにする。
      env: { ...process.env, STREAM_SHOUGI_BOARD_NO_OPEN: "1" },
    },
  ),
];

const stopAll = () => {
  for (const child of children) child.kill();
};

process.on("SIGINT", stopAll);
process.on("SIGTERM", stopAll);

// どちらかが落ちたら、もう一方も畳んで開発者に気づかせる。
await Promise.race(children.map((child) => child.exited));
stopAll();
