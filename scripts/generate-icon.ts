/**
 * exe に埋め込む ICO を生成する。
 *
 * 駒の形は client/src/Piece/component.tsx と同じ五角形なので、外部の画像素材を持ち込まずに
 * アプリの見た目と揃う。生成物は Git 管理し、通常のビルドでは再生成しない
 * (画像ライブラリをビルド依存に加えないため)。
 *
 *   bun run generate:icon
 */

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..", "app");
const SIZES = [16, 32, 48] as const;
/** 1 辺あたりの supersample 数。輪郭のギザつきを抑える。 */
const SAMPLES = 4;

type Rgba = readonly [number, number, number, number];

const WOOD: Rgba = [242, 220, 174, 255];
const EDGE: Rgba = [80, 52, 20, 255];

/** PieceGlyph の KOMA_PATH と同じ五角形 (0..1 の正規座標)。 */
const KOMA: readonly (readonly [number, number])[] = [
  [0.5, 0.035],
  [0.775, 0.165],
  [0.895, 0.965],
  [0.105, 0.965],
  [0.225, 0.165],
];

function scaleAboutCenter(
  points: readonly (readonly [number, number])[],
  factor: number,
): (readonly [number, number])[] {
  const cx = points.reduce((sum, [x]) => sum + x, 0) / points.length;
  const cy = points.reduce((sum, [, y]) => sum + y, 0) / points.length;
  return points.map(
    ([x, y]) => [cx + (x - cx) * factor, cy + (y - cy) * factor] as const,
  );
}

function isInside(
  points: readonly (readonly [number, number])[],
  x: number,
  y: number,
): boolean {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i, i += 1) {
    const a = points[i];
    const b = points[j];
    if (a === undefined || b === undefined) continue;
    const [ax, ay] = a;
    const [bx, by] = b;
    if (ay > y !== by > y && x < ((bx - ax) * (y - ay)) / (by - ay) + ax)
      inside = !inside;
  }
  return inside;
}

function blend(under: Rgba, over: Rgba, coverage: number): Rgba {
  const alpha = (over[3] / 255) * coverage;
  if (alpha <= 0) return under;
  const baseAlpha = under[3] / 255;
  const outAlpha = alpha + baseAlpha * (1 - alpha);
  if (outAlpha <= 0) return [0, 0, 0, 0];
  const mix = (o: number, u: number) =>
    Math.round((o * alpha + u * baseAlpha * (1 - alpha)) / outAlpha);
  return [
    mix(over[0], under[0]),
    mix(over[1], under[1]),
    mix(over[2], under[2]),
    Math.round(outAlpha * 255),
  ];
}

/** 外側を縁の色で、内側を木地で塗る。supersample した被覆率をそのまま alpha にする。 */
function renderKoma(size: number): Rgba[] {
  const outer = KOMA;
  const inner = scaleAboutCenter(KOMA, 0.78);
  const pixels: Rgba[] = [];

  for (let py = 0; py < size; py += 1) {
    for (let px = 0; px < size; px += 1) {
      let outerHits = 0;
      let innerHits = 0;
      for (let sy = 0; sy < SAMPLES; sy += 1) {
        for (let sx = 0; sx < SAMPLES; sx += 1) {
          const x = (px + (sx + 0.5) / SAMPLES) / size;
          const y = (py + (sy + 0.5) / SAMPLES) / size;
          if (isInside(outer, x, y)) outerHits += 1;
          if (isInside(inner, x, y)) innerHits += 1;
        }
      }
      const total = SAMPLES * SAMPLES;
      let pixel: Rgba = [0, 0, 0, 0];
      pixel = blend(pixel, EDGE, outerHits / total);
      pixel = blend(pixel, WOOD, innerHits / total);
      pixels.push(pixel);
    }
  }

  return pixels;
}

/** ICO 内の 1 画像。BITMAPINFOHEADER + BGRA (下から上) + AND マスク。 */
function encodeBitmap(size: number, pixels: Rgba[]): Uint8Array {
  const maskStride = Math.ceil(size / 32) * 4;
  const header = 40;
  const colorBytes = size * size * 4;
  const maskBytes = maskStride * size;
  const buffer = new Uint8Array(header + colorBytes + maskBytes);
  const view = new DataView(buffer.buffer);

  view.setUint32(0, 40, true);
  view.setInt32(4, size, true);
  // ICO では XOR 画像と AND マスクを積んだ高さを書く。
  view.setInt32(8, size * 2, true);
  view.setUint16(12, 1, true);
  view.setUint16(14, 32, true);
  view.setUint32(20, colorBytes + maskBytes, true);

  for (let y = 0; y < size; y += 1) {
    const sourceRow = size - 1 - y;
    for (let x = 0; x < size; x += 1) {
      const pixel = pixels[sourceRow * size + x] ?? ([0, 0, 0, 0] as const);
      const offset = header + (y * size + x) * 4;
      buffer[offset] = pixel[2];
      buffer[offset + 1] = pixel[1];
      buffer[offset + 2] = pixel[0];
      buffer[offset + 3] = pixel[3];
    }
  }

  // 32bpp では AND マスクを使わないが、構造として 0 埋めのものを置く。
  return buffer;
}

function encodeIco(images: { size: number; data: Uint8Array }[]): Uint8Array {
  const headerBytes = 6 + images.length * 16;
  const total = images.reduce(
    (sum, image) => sum + image.data.length,
    headerBytes,
  );
  const buffer = new Uint8Array(total);
  const view = new DataView(buffer.buffer);

  view.setUint16(0, 0, true);
  view.setUint16(2, 1, true);
  view.setUint16(4, images.length, true);

  let offset = headerBytes;
  images.forEach((image, index) => {
    const entry = 6 + index * 16;
    buffer[entry] = image.size === 256 ? 0 : image.size;
    buffer[entry + 1] = image.size === 256 ? 0 : image.size;
    view.setUint16(entry + 4, 1, true);
    view.setUint16(entry + 6, 32, true);
    view.setUint32(entry + 8, image.data.length, true);
    view.setUint32(entry + 12, offset, true);
    buffer.set(image.data, offset);
    offset += image.data.length;
  });

  return buffer;
}

const images = SIZES.map((size) => ({
  size,
  data: encodeBitmap(size, renderKoma(size)),
}));
const ico = encodeIco(images);

await mkdir(join(appDir, "assets"), { recursive: true });
await writeFile(join(appDir, "assets", "stream-shougi-board.ico"), ico);

console.log(
  `generated stream-shougi-board.ico (${ico.length} bytes, sizes: ${SIZES.join(", ")})`,
);
