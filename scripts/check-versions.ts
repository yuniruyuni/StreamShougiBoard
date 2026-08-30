/**
 * 版の持ち場所が 2 つあるので、ずれていないことを check のたびに確かめる。
 *
 * `app/Cargo.toml` の版が exe へ埋め込まれ、リリースタグとの照合にも使われる。
 * `package.json` は npm 側の慣例で置いているだけだが、食い違うと 読む人が迷う。
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

const packageVersion = (
  JSON.parse(readFileSync(join(projectRoot, "package.json"), "utf8")) as {
    version: string;
  }
).version;

const cargoToml = readFileSync(join(projectRoot, "app/Cargo.toml"), "utf8");
const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargoToml)?.[1];

if (cargoVersion === undefined) {
  console.error("app/Cargo.toml から version を読めません");
  process.exit(1);
}

if (packageVersion !== cargoVersion) {
  console.error(
    `版がずれています: package.json=${packageVersion} app/Cargo.toml=${cargoVersion}\n` +
      "リリースタグと照合するのは app/Cargo.toml の方です。",
  );
  process.exit(1);
}

console.log(`version ${cargoVersion}`);
