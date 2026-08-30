/**
 * 操作ページと盤面ページを `static/` へビルドする。
 *
 * 出力は Rust 側の rust-embed が拾う。release ビルドでは exe へ埋め込まれ、
 * debug ビルドでは実行時にこのディレクトリを読むので、watch 中は再起動なしで反映される。
 */

import { readFileSync, watch } from "node:fs";
import { mkdir, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const packageDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = join(packageDir, "src");
const outputDir = join(packageDir, "static");
/** scripts/generate-third-party-notices.ts が先に書き出しているページ。 */
const noticesPath = join(
  packageDir,
  "..",
  "app",
  "assets",
  "third-party-licenses.generated.html",
);

/**
 * ページへ埋める版。`app/Cargo.toml` と同じ値であることは
 * `bun run check:version` が確かめている。
 */
function appVersion(): string {
  const pkg = JSON.parse(
    readFileSync(join(packageDir, "..", "package.json"), "utf8"),
  ) as { version: string };
  return pkg.version;
}

/** ページ名 → エントリーポイント。HTML と CSS も同じ名前で対にする。 */
const PAGES = ["control", "board"] as const;

async function cleanOutput(): Promise<void> {
  await mkdir(outputDir, { recursive: true });
  for (const entry of await readdir(outputDir)) {
    if (entry === ".gitkeep") continue;
    await rm(join(outputDir, entry), { recursive: true, force: true });
  }
}

async function copyText(name: string): Promise<void> {
  const text = await Bun.file(join(sourceDir, name)).text();
  await writeFile(join(outputDir, name), text);
}

/**
 * ライセンスページは必須の入力として扱い、無ければ止める。
 * 黙って欠けたまま exe を作ると、配布物からライセンス表示だけが消えてしまう。
 */
async function copyNotices(): Promise<void> {
  const notices = Bun.file(noticesPath);
  if (!(await notices.exists())) {
    throw new Error(
      `${noticesPath} がありません。先に \`bun run generate:licenses\` を実行してください。`,
    );
  }
  await writeFile(join(outputDir, "licenses.html"), await notices.text());
}

async function bundlePages(): Promise<void> {
  const result = await Bun.build({
    entrypoints: PAGES.map((page) => join(sourceDir, `${page}.tsx`)),
    outdir: outputDir,
    target: "browser",
    format: "esm",
    minify: true,
    sourcemap: "none",
    define: {
      "process.env.NODE_ENV": JSON.stringify("production"),
      // サーバーが送ってくる版と突き合わせるために、焼いた版をページへ埋める。
      __APP_VERSION__: JSON.stringify(appVersion()),
    },
  });

  if (!result.success) {
    for (const log of result.logs) console.error(log);
    throw new Error("client bundle failed");
  }
}

async function build(): Promise<void> {
  await cleanOutput();
  await Promise.all([
    ...PAGES.flatMap((page) => [
      copyText(`${page}.html`),
      copyText(`${page}.css`),
    ]),
    copyNotices(),
  ]);
  await bundlePages();
}

async function buildQuietly(): Promise<void> {
  try {
    await build();
    console.log(`[client] built ${new Date().toLocaleTimeString()}`);
  } catch (error) {
    // watch 中はビルド失敗で終了せず、次の保存で直せるようにする。
    console.error("[client] build failed:", error);
  }
}

if (process.argv.includes("--watch")) {
  await buildQuietly();

  let pending: ReturnType<typeof setTimeout> | null = null;
  watch(sourceDir, { recursive: true }, () => {
    if (pending !== null) clearTimeout(pending);
    pending = setTimeout(() => {
      pending = null;
      void buildQuietly();
    }, 50);
  });
  console.log("[client] watching src/");
} else {
  await build();
}
