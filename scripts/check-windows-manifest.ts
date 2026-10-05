/** Verify the RT_MANIFEST inside the actual PE, not just the build input. */
import { readFileSync } from "node:fs";

const path = process.argv[2];
if (!path)
  throw new Error("Usage: bun scripts/check-windows-manifest.ts <exe>");
const bytes = readFileSync(path);
if (bytes.toString("ascii", 0, 2) !== "MZ")
  throw new Error("Not a PE executable");
const pe = bytes.readUInt32LE(0x3c);
if (bytes.readUInt32LE(pe) !== 0x4550) throw new Error("Invalid PE signature");
const coff = pe + 4;
const optional = coff + 20;
const count = bytes.readUInt16LE(coff + 2);
const optionalSize = bytes.readUInt16LE(coff + 16);
const directories =
  optional + (bytes.readUInt16LE(optional) === 0x20b ? 112 : 96);
const sections = Array.from({ length: count }, (_, index) => {
  const at = optional + optionalSize + index * 40;
  return {
    address: bytes.readUInt32LE(at + 12),
    size: Math.max(bytes.readUInt32LE(at + 8), bytes.readUInt32LE(at + 16)),
    offset: bytes.readUInt32LE(at + 20),
  };
});
function offset(rva: number): number {
  const section = sections.find(
    (s) => rva >= s.address && rva < s.address + s.size,
  );
  if (!section) throw new Error(`Invalid resource RVA: ${rva}`);
  return section.offset + rva - section.address;
}
const root = offset(bytes.readUInt32LE(directories + 16));
const manifests: Buffer[] = [];
function walk(at: number, ids: number[]) {
  if (ids.length > 3) throw new Error("Invalid resource directory depth");
  const count = bytes.readUInt16LE(at + 12) + bytes.readUInt16LE(at + 14);
  for (let index = 0; index < count; index++) {
    const entry = at + 16 + index * 8;
    const id = bytes.readUInt32LE(entry);
    const target = bytes.readUInt32LE(entry + 4);
    if (target >>> 31) walk(root + (target & 0x7fffffff), [...ids, id]);
    else if (ids[0] === 24 && ids[1] === 1) {
      const data = root + target;
      const start = offset(bytes.readUInt32LE(data));
      const size = bytes.readUInt32LE(data + 4);
      manifests.push(bytes.subarray(start, start + size));
    }
  }
}
walk(root, []);
if (manifests.length === 0)
  throw new Error("Application RT_MANIFEST (24/1) is missing");
for (const manifest of manifests) {
  const xml = manifest.toString("utf8").replace(/^\uFEFF/, "");
  if (!xml.startsWith('<?xml version="1.0"')) {
    throw new Error(
      "Invalid manifest: XML declaration/BOM must start at byte zero (SxS startup failure)",
    );
  }
  if (
    xml.includes("\uFEFF") ||
    !xml.includes('<assembly xmlns="urn:schemas-microsoft-com:asm.v1"') ||
    !xml.includes("</assembly>")
  ) {
    throw new Error("Invalid manifest content");
  }
}
if (process.argv.includes("--xml")) process.stdout.write(manifests[0]);
else
  console.log(
    `PASS: ${path}: ${manifests.length} application manifest(s), valid BOM/XML declaration position`,
  );
