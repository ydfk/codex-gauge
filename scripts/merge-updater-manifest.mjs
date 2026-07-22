import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const [manifestPath, target, signaturePath, assetUrl] = process.argv.slice(2);

if (!manifestPath || !target || !signaturePath || !assetUrl) {
  throw new Error("Usage: merge-updater-manifest <manifest> <target> <signature> <asset-url>");
}

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const signature = (await readFile(signaturePath, "utf8")).trim();

manifest.platforms ??= {};
manifest.platforms[target] = { signature, url: assetUrl };

await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Added ${target} to ${path.basename(manifestPath)}.`);
