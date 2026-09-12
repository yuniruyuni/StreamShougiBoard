/**
 * 同じ版を 2 箇所以上に書いている所が 3 組あるので、ずれていないことを check のたびに確かめる。
 *
 * - アプリの版: `app/Cargo.toml` の値が exe へ埋め込まれ、リリースタグとの照合にも使われる。
 *   `package.json` は npm 側の慣例で置いているだけだが、食い違うと読む人が迷う。
 * - ツールチェイン: `rust-toolchain.toml` はワークフローの `toolchain:` 入力より優先される。
 *   ずれていると、CI は入力の版を入れておきながら、コンパイルには別の版を使う。
 * - Bun: `package.json` の `packageManager` とワークフローの `bun-version:`。
 */

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function read(...parts: string[]): string {
  return readFileSync(join(projectRoot, ...parts), "utf8");
}

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

/** 一致していなければ、どこがどうずれているかを出して落ちる。 */
function assertSame(
  label: string,
  expected: { where: string; value: string },
  found: { where: string; value: string }[],
  hint: string,
): void {
  const wrong = found.filter((item) => item.value !== expected.value);
  if (wrong.length === 0) return;
  fail(
    `${label} がずれています: ${expected.where}=${expected.value}\n` +
      wrong.map((item) => `  ${item.where}=${item.value}`).join("\n") +
      `\n${hint}`,
  );
}

const packageJson = JSON.parse(read("package.json")) as {
  version: string;
  packageManager: string;
};

// ── アプリの版 ──
const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(
  read("app/Cargo.toml"),
)?.[1];
if (cargoVersion === undefined)
  fail("app/Cargo.toml から version を読めません");

assertSame(
  "版",
  { where: "app/Cargo.toml", value: cargoVersion },
  [{ where: "package.json", value: packageJson.version }],
  "リリースタグと照合するのは app/Cargo.toml の方です。",
);

// ── ツールチェインと Bun ──
const toolchainChannel = /^channel\s*=\s*"([^"]+)"/m.exec(
  read("rust-toolchain.toml"),
)?.[1];
if (toolchainChannel === undefined) {
  fail("rust-toolchain.toml から channel を読めません");
}

const bunVersion = /^bun@(.+)$/.exec(packageJson.packageManager)?.[1];
if (bunVersion === undefined) {
  fail("package.json の packageManager から bun の版を読めません");
}

const workflowDir = ".github/workflows";
const toolchainInputs: { where: string; value: string }[] = [];
const bunInputs: { where: string; value: string }[] = [];

for (const name of readdirSync(join(projectRoot, workflowDir)).sort()) {
  if (!name.endsWith(".yml") && !name.endsWith(".yaml")) continue;
  const workflow = read(workflowDir, name);
  for (const match of workflow.matchAll(/^\s*toolchain:\s*(\S+)\s*$/gm)) {
    toolchainInputs.push({ where: `${name}`, value: match[1] ?? "" });
  }
  for (const match of workflow.matchAll(/^\s*bun-version:\s*(\S+)\s*$/gm)) {
    bunInputs.push({ where: `${name}`, value: match[1] ?? "" });
  }
}

assertSame(
  "Rust のツールチェイン",
  { where: "rust-toolchain.toml", value: toolchainChannel },
  toolchainInputs,
  "rust-toolchain.toml がワークフローの toolchain: より優先されるので、\n" +
    "ずれているとコンパイルに使われる版が入力と食い違います。",
);

assertSame(
  "Bun",
  { where: "package.json packageManager", value: bunVersion },
  bunInputs,
  "ワークフローの bun-version: と揃えてください。",
);

console.log(
  `version ${cargoVersion} / rust ${toolchainChannel} / bun ${bunVersion}`,
);
