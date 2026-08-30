/**
 * 配布する exe に入っている第三者コンポーネントのライセンスページを組み立てる。
 *
 * 出力は `app/assets/third-party-licenses.generated.html` で、client のビルドが
 * `static/licenses.html` として取り込み、exe へ埋め込まれる。実行中は
 * `http://127.0.0.1:<port>/licenses` とタスクトレイから開ける。
 *
 * 掲載するのは「exe の中に実際に入っているもの」だけに絞る。
 *
 * - Cargo の実行時依存: 実行ファイル本体へ静的リンクされる
 * - client の実行時依存 (react とその依存): バンドルされて control.js / board.js に入る
 *
 * build-dependencies と dev-dependencies、npm の devDependencies はビルドにしか
 * 使わないので対象外。
 */

import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, "..");
const cargoAboutVersion = "0.9.1";
const htmlOutput = join(
  projectRoot,
  "app/assets/third-party-licenses.generated.html",
);

/** npm 側の検討済みライセンス。増やすときは本文を読んでから足す。 */
const allowedNpmLicenseIdentifiers = new Set([
  "0BSD",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "MIT",
  "MIT-0",
  "Unlicense",
  "Zlib",
]);

interface PackageJson {
  name?: string;
  version?: string;
  license?: string;
  author?: string | { name?: string };
  contributors?: Array<string | { name?: string }>;
  repository?: string | { url?: string };
  homepage?: string;
  dependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
}

interface CargoAboutOutput {
  crates: Array<{
    license: string;
    package: {
      authors: string[];
      license_file: string | null;
      manifest_path: string;
      name: string;
      license: string | null;
      repository: string | null;
      source: string | null;
      version: string;
    };
  }>;
}

interface LicenseDocument {
  hash: string;
  name: string;
  text: string;
}

interface Component {
  authors: string[];
  documents: LicenseDocument[];
  /** 配布物のどこに入っているか。読む人が「なぜ載っているか」を辿れるようにする。 */
  embeddedAs: string;
  ecosystem: "Cargo" | "npm";
  license: string;
  name: string;
  source: string;
  version: string;
}

interface IndexedDocument {
  componentNames: Set<string>;
  hash: string;
  names: Set<string>;
  text: string;
}

function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

function sha256Text(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

function sha256File(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function normalizeText(value: string): string {
  const normalized = value
    .replaceAll("\r\n", "\n")
    .split("\n")
    .map((line) => line.replaceAll("\t", "    ").trimEnd())
    .join("\n")
    .trim();
  return `${normalized}\n`;
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function isLicenseNoticeName(name: string): boolean {
  return /^(licen[cs]e|copying|copyright|notice)(?:[._-].*)?$/i.test(name);
}

/** パッケージに同梱されたライセンス・告知ファイルを集める。同一本文はまとめる。 */
function licenseDocuments(
  packageDir: string,
  extraPath?: string,
): LicenseDocument[] {
  const paths = readdirSync(packageDir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && isLicenseNoticeName(entry.name))
    .map((entry) => join(packageDir, entry.name));
  if (extraPath !== undefined) {
    const resolved = resolve(packageDir, extraPath);
    if (existsSync(resolved)) paths.push(resolved);
  }

  const documents = new Map<string, LicenseDocument>();
  for (const path of paths.sort(compareText)) {
    const text = normalizeText(readFileSync(path, "utf8"));
    const hash = sha256Text(text);
    const existing = documents.get(hash);
    if (existing) {
      existing.name = [
        ...new Set(existing.name.split(" / ").concat(basename(path))),
      ]
        .sort(compareText)
        .join(" / ");
    } else {
      documents.set(hash, { hash, name: basename(path), text });
    }
  }
  return [...documents.values()].sort(
    (left, right) =>
      compareText(left.name, right.name) || compareText(left.hash, right.hash),
  );
}

function normalizeSourceUrl(
  repository: PackageJson["repository"],
  homepage: string | undefined,
  fallback: string,
): string {
  const raw =
    (typeof repository === "string" ? repository : repository?.url) ??
    homepage ??
    fallback;
  const normalized = raw
    .replace(/^git\+/, "")
    .replace(/^git:\/\/github\.com\//, "https://github.com/")
    .replace(/^ssh:\/\/git@github\.com\//, "https://github.com/")
    .replace(/^git@github\.com:/, "https://github.com/")
    .replace(/\.git(?:#.*)?$/, "");
  try {
    const url = new URL(normalized);
    if (url.protocol === "https:" || url.protocol === "http:") {
      return url.toString().replace(/\/$/, "");
    }
  } catch {
    // fallback は必ず https の URL なのでそのまま使う。
  }
  return fallback;
}

function validateNpmLicense(expression: string, component: string): void {
  if (
    expression.trim().length === 0 ||
    expression === "UNLICENSED" ||
    expression.startsWith("SEE LICENSE IN")
  ) {
    throw new Error(`${component} のライセンス表記を扱えません: ${expression}`);
  }
  for (const token of expression
    .replaceAll("(", " ")
    .replaceAll(")", " ")
    .split(/\s+/)) {
    if (token === "" || token === "AND" || token === "OR") continue;
    if (!allowedNpmLicenseIdentifiers.has(token)) {
      throw new Error(
        `${component} が検討済みの範囲外のライセンスを使っています: ${token}`,
      );
    }
  }
}

/**
 * cargo-about に、Windows 向けの実行時依存だけを列挙させる。
 * `--frozen` で動かすので、呼び出し前に `cargo fetch --locked` が済んでいる必要がある
 * (package.json の `prepare:cargo`)。ロックにない crate を勝手に足させないための指定。
 */
function cargoAboutOutput(): CargoAboutOutput {
  const version = Bun.spawnSync({
    cmd: ["cargo", "about", "--version"],
    cwd: projectRoot,
    stdout: "pipe",
    stderr: "pipe",
  });
  const installed = new TextDecoder().decode(version.stdout).trim();
  if (
    version.exitCode !== 0 ||
    installed !== `cargo-about ${cargoAboutVersion}`
  ) {
    throw new Error(
      `cargo-about ${cargoAboutVersion} が必要です。cargo install cargo-about --version ${cargoAboutVersion} --locked --features cli`,
    );
  }

  const directory = mkdtempSync(join(tmpdir(), "stream-shougi-board-about-"));
  const outputPath = join(directory, "licenses.json");
  try {
    const result = Bun.spawnSync({
      cmd: [
        "cargo",
        "about",
        "generate",
        "--format",
        "json",
        "--output-file",
        outputPath,
        "--manifest-path",
        join(projectRoot, "app/Cargo.toml"),
        "--config",
        join(projectRoot, "about.toml"),
        "--frozen",
        "--fail",
      ],
      cwd: projectRoot,
      stdout: "ignore",
      stderr: "pipe",
    });
    if (result.exitCode !== 0) {
      throw new Error(new TextDecoder().decode(result.stderr));
    }
    return readJson<CargoAboutOutput>(outputPath);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function collectCargoComponents(): Component[] {
  const components: Component[] = [];
  for (const crate of cargoAboutOutput().crates) {
    const pkg = crate.package;
    // source が null なのはこのリポジトリ自身のクレート。本体の LICENSE が覆う。
    if (pkg.source === null) continue;

    const name = `${pkg.name}@${pkg.version}`;
    const license = pkg.license ?? crate.license;
    if (!license)
      throw new Error(`${name} に Cargo のライセンス表記がありません`);

    const packageDir = dirname(realpathSync(pkg.manifest_path));
    const documents = licenseDocuments(
      packageDir,
      pkg.license_file ?? undefined,
    );
    if (documents.length === 0) {
      throw new Error(`${name} にライセンス本文が同梱されていません`);
    }

    components.push({
      authors: pkg.authors,
      documents,
      ecosystem: "Cargo",
      embeddedAs: "実行ファイル本体 (静的リンク)",
      license,
      name: pkg.name,
      source:
        pkg.repository ??
        `https://crates.io/crates/${encodeURIComponent(pkg.name)}/${encodeURIComponent(pkg.version)}`,
      version: pkg.version,
    });
  }
  return components;
}

function authorNames(metadata: PackageJson): string[] {
  return [metadata.author, ...(metadata.contributors ?? [])]
    .map((value) => (typeof value === "string" ? value : value?.name))
    .filter((value): value is string => Boolean(value));
}

function findNpmPackage(
  packageName: string,
  searchPaths: readonly string[],
): string | undefined {
  for (const nodeModules of searchPaths) {
    const candidate = join(nodeModules, ...packageName.split("/"));
    if (existsSync(join(candidate, "package.json")))
      return realpathSync(candidate);
  }
  return undefined;
}

/** client の実行時依存を推移的にたどる。ここで集めたものが JS バンドルへ入る。 */
function collectNpmComponents(): Component[] {
  const components = new Map<string, Component>();
  const rootSearchPaths = [
    join(projectRoot, "client/node_modules"),
    join(projectRoot, "node_modules"),
  ];

  const visit = (
    packageName: string,
    searchPaths: readonly string[],
    optional: boolean,
  ): void => {
    const packageDir = findNpmPackage(packageName, searchPaths);
    if (packageDir === undefined) {
      if (optional) return;
      throw new Error(`実行時依存を解決できません: ${packageName}`);
    }

    const metadata = readJson<PackageJson>(join(packageDir, "package.json"));
    if (!metadata.name || !metadata.version || !metadata.license) {
      throw new Error(
        `${packageDir} の package.json に name / version / license が揃っていません`,
      );
    }
    const key = `${metadata.name}@${metadata.version}`;
    if (components.has(key)) return;

    validateNpmLicense(metadata.license, key);
    const documents = licenseDocuments(packageDir);
    if (documents.length === 0) {
      throw new Error(`${key} にライセンス本文が同梱されていません`);
    }

    components.set(key, {
      authors: authorNames(metadata),
      documents,
      ecosystem: "npm",
      embeddedAs: "JS バンドル (control.js / board.js)",
      license: metadata.license,
      name: metadata.name,
      source: normalizeSourceUrl(
        metadata.repository,
        metadata.homepage,
        `https://www.npmjs.com/package/${encodeURIComponent(metadata.name)}/v/${encodeURIComponent(metadata.version)}`,
      ),
      version: metadata.version,
    });

    // Bun の隔離レイアウトでは、そのパッケージの依存が同じ階層に並ぶ。
    const nested = [
      join(packageDir, "node_modules"),
      dirname(packageDir),
      ...searchPaths,
    ];
    for (const dependency of Object.keys(metadata.dependencies ?? {}).sort(
      compareText,
    )) {
      visit(dependency, nested, false);
    }
    for (const dependency of Object.keys(
      metadata.optionalDependencies ?? {},
    ).sort(compareText)) {
      visit(dependency, nested, true);
    }
  };

  const client = readJson<PackageJson>(
    join(projectRoot, "client/package.json"),
  );
  for (const dependency of Object.keys(client.dependencies ?? {}).sort(
    compareText,
  )) {
    visit(dependency, rootSearchPaths, false);
  }
  return [...components.values()];
}

function indexDocuments(components: readonly Component[]): IndexedDocument[] {
  const documents = new Map<string, IndexedDocument>();
  for (const component of components) {
    const componentName = `${component.name} ${component.version}`;
    for (const document of component.documents) {
      const existing = documents.get(document.hash);
      if (existing) {
        existing.names.add(document.name);
        existing.componentNames.add(componentName);
      } else {
        documents.set(document.hash, {
          componentNames: new Set([componentName]),
          hash: document.hash,
          names: new Set([document.name]),
          text: document.text,
        });
      }
    }
  }
  return [...documents.values()].sort((left, right) =>
    compareText(left.hash, right.hash),
  );
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function renderHtml(
  components: readonly Component[],
  documents: readonly IndexedDocument[],
  lockHashes: { bun: string; cargo: string },
  projectLicense: string,
): string {
  const componentSections = components
    .map((component) => {
      const authors =
        component.authors.length > 0
          ? `    <p><strong>Authors:</strong> ${escapeHtml(component.authors.join(", "))}</p>\n`
          : "";
      const documentLinks = component.documents
        .map(
          (document) =>
            `<a href="#license-${document.hash}">${escapeHtml(document.name)}</a>`,
        )
        .join(" / ");
      return `<details class="component">
  <summary><span>${escapeHtml(component.name)} ${escapeHtml(component.version)}</span><small>${escapeHtml(component.ecosystem)} · ${escapeHtml(component.license)}</small></summary>
  <div class="component-body">
    <p><strong>含まれ方:</strong> ${escapeHtml(component.embeddedAs)}</p>
    <p><strong>Source:</strong> <a href="${escapeHtml(component.source)}" rel="noreferrer">${escapeHtml(component.source)}</a></p>
${authors}    <p><strong>License documents:</strong> ${documentLinks}</p>
  </div>
</details>`;
    })
    .join("\n");

  const documentSections = documents
    .map(
      (
        document,
      ) => `<section class="license-document" id="license-${document.hash}">
  <h3>${escapeHtml([...document.names].sort(compareText).join(" / "))}</h3>
  <p>Used by: ${escapeHtml([...document.componentNames].sort(compareText).join(", "))}</p>
  <pre>${escapeHtml(document.text)}</pre>
</section>`,
    )
    .join("\n");

  return `<!doctype html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <meta name="robots" content="noindex, nofollow">
  <title>StreamShougiBoard ライセンス</title>
  <style>
    :root { font-family: system-ui, sans-serif; line-height: 1.55; color-scheme: light dark; }
    body { max-width: 1080px; margin: 0 auto; padding: 2rem 1rem 5rem; }
    h1, h2, h3 { line-height: 1.25; }
    a { overflow-wrap: anywhere; }
    .meta { color: #777; font-size: .875rem; overflow-wrap: anywhere; }
    .component { border: 1px solid #8886; border-radius: .5rem; margin: .5rem 0; }
    .component summary { cursor: pointer; display: flex; gap: 1rem; justify-content: space-between; padding: .75rem 1rem; }
    .component summary small { color: #777; text-align: right; }
    .component-body { border-top: 1px solid #8886; padding: .25rem 1rem; }
    .license-document { border-top: 1px solid #8886; margin-top: 2rem; padding-top: 1rem; scroll-margin-top: 1rem; }
    pre { background: #8881; border-radius: .5rem; overflow: auto; padding: 1rem; white-space: pre-wrap; }
    @media (max-width: 640px) { .component summary { flex-direction: column; gap: .25rem; } .component summary small { text-align: left; } }
  </style>
</head>
<body>
  <main>
    <h1>StreamShougiBoard ライセンス</h1>
    <p>StreamShougiBoard 本体と、配布される Windows 実行ファイルに含まれる第三者コンポーネントのライセンス情報です。</p>
    <p class="meta">Cargo.lock SHA-256: ${lockHashes.cargo}<br>bun.lock SHA-256: ${lockHashes.bun}<br>Scanner: cargo-about ${cargoAboutVersion}</p>

    <h2>StreamShougiBoard</h2>
    <p>StreamShougiBoard は MIT License で提供されます。</p>
    <pre>${escapeHtml(projectLicense)}</pre>

    <h2>第三者コンポーネント (${components.length})</h2>
    ${componentSections}

    <h2>ライセンス本文</h2>
    ${documentSections}
  </main>
</body>
</html>
`;
}

const components = [
  ...collectCargoComponents(),
  ...collectNpmComponents(),
].sort(
  (left, right) =>
    compareText(left.ecosystem, right.ecosystem) ||
    compareText(left.name, right.name) ||
    compareText(left.version, right.version),
);

const html = renderHtml(
  components,
  indexDocuments(components),
  {
    bun: sha256File(join(projectRoot, "bun.lock")),
    cargo: sha256File(join(projectRoot, "app/Cargo.lock")),
  },
  normalizeText(readFileSync(join(projectRoot, "LICENSE"), "utf8")),
);

mkdirSync(dirname(htmlOutput), { recursive: true });
writeFileSync(htmlOutput, html);
console.log(`generated ${htmlOutput} (${components.length} components)`);
