import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const nextVersion = normalizeVersion(process.argv[2]);

if (!nextVersion) {
  console.error("Usage: pnpm version:set <version>");
  process.exit(1);
}

const semverPattern =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-.]+)?(?:\+[0-9A-Za-z-.]+)?$/;

if (!semverPattern.test(nextVersion)) {
  console.error(`Invalid version: ${nextVersion}`);
  console.error("Expected a semver value like 0.1.1 or 1.0.0-beta.1");
  process.exit(1);
}

const rootDir = process.cwd();
const packageJsonPath = path.join(rootDir, "package.json");
const tauriConfigPath = path.join(rootDir, "src-tauri", "tauri.conf.json");
const cargoTomlPath = path.join(rootDir, "src-tauri", "Cargo.toml");

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
packageJson.version = nextVersion;
fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"));
tauriConfig.version = nextVersion;
fs.writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
const packageVersionPattern =
  /(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/;
const cargoVersionMatch = cargoToml.match(packageVersionPattern);

if (!cargoVersionMatch) {
  console.error("Failed to locate package version in src-tauri/Cargo.toml");
  process.exit(1);
}

const updatedCargoToml = cargoToml.replace(
  packageVersionPattern,
  `$1${nextVersion}$3`,
);

if (updatedCargoToml !== cargoToml) {
  fs.writeFileSync(cargoTomlPath, updatedCargoToml);
}

console.log(`Updated project version to ${nextVersion}`);

function normalizeVersion(input) {
  if (!input) {
    return "";
  }

  return input.replace(/^v/, "");
}
