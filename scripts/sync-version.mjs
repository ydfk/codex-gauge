import { readFileSync, writeFileSync } from "node:fs";

const input = process.argv[2] || process.env.CODEX_GAUGE_VERSION;
if (!input) {
  console.error("Usage: pnpm version:sync <version-or-tag>");
  process.exit(1);
}

const version = input.replace(/^v/, "");
if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`Invalid SemVer: ${input}`);
  process.exit(1);
}

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
packageJson.version = version;
writeFileSync("package.json", `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriConfigPath = "src-tauri/tauri.conf.json";
const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, "utf8"));
tauriConfig.version = version;
writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

const cargoPath = "src-tauri/Cargo.toml";
const cargoToml = readFileSync(cargoPath, "utf8").replace(
  /^version = ".*"$/m,
  `version = "${version}"`,
);
writeFileSync(cargoPath, cargoToml);
