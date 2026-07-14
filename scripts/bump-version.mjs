import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

export const semverPattern =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-.]+)?(?:\+[0-9A-Za-z-.]+)?$/;

export function normalizeVersion(input) {
  if (!input) {
    return "";
  }

  return input.replace(/^v/, "");
}

// Writes the given version into package.json, tauri.conf.json, Cargo.toml, and
// the crate's own entry in Cargo.lock. Throws on invalid input or an unparseable
// Cargo.toml. Returns the normalized version.
export function setVersion(rawVersion) {
  const nextVersion = normalizeVersion(rawVersion);

  if (!nextVersion) {
    throw new Error("A version argument is required.");
  }

  if (!semverPattern.test(nextVersion)) {
    throw new Error(
      `Invalid version: ${nextVersion}. Expected a semver value like 0.1.1 or 1.0.0-beta.1`,
    );
  }

  const rootDir = process.cwd();
  const packageJsonPath = path.join(rootDir, "package.json");
  const tauriConfigPath = path.join(rootDir, "src-tauri", "tauri.conf.json");
  const cargoTomlPath = path.join(rootDir, "src-tauri", "Cargo.toml");
  const cargoLockPath = path.join(rootDir, "src-tauri", "Cargo.lock");

  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  packageJson.version = nextVersion;
  fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

  const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"));
  tauriConfig.version = nextVersion;
  fs.writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

  const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
  const packageVersionPattern =
    /(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/;

  if (!packageVersionPattern.test(cargoToml)) {
    throw new Error("Failed to locate package version in src-tauri/Cargo.toml");
  }

  const updatedCargoToml = cargoToml.replace(
    packageVersionPattern,
    `$1${nextVersion}$3`,
  );

  if (updatedCargoToml !== cargoToml) {
    fs.writeFileSync(cargoTomlPath, updatedCargoToml);
  }

  // Keep the crate's own version in Cargo.lock in sync so the committed tree is
  // consistent. Cargo always writes `name` immediately before `version`, so we
  // target the port-kill package block specifically.
  if (fs.existsSync(cargoLockPath)) {
    const packageName = packageJson.name ?? "port-kill";
    const escapedName = packageName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const cargoLock = fs.readFileSync(cargoLockPath, "utf8");
    const lockVersionPattern = new RegExp(
      `(\\[\\[package\\]\\]\\nname = "${escapedName}"\\nversion = ")([^"]+)(")`,
    );

    if (!lockVersionPattern.test(cargoLock)) {
      throw new Error(
        `Failed to locate "${packageName}" package version in src-tauri/Cargo.lock`,
      );
    }

    const updatedCargoLock = cargoLock.replace(
      lockVersionPattern,
      `$1${nextVersion}$3`,
    );

    if (updatedCargoLock !== cargoLock) {
      fs.writeFileSync(cargoLockPath, updatedCargoLock);
    }
  }

  return nextVersion;
}

const isMain =
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMain) {
  if (!normalizeVersion(process.argv[2])) {
    console.error("Usage: pnpm version:set <version>");
    process.exit(1);
  }

  try {
    const version = setVersion(process.argv[2]);
    console.log(`Updated project version to ${version}`);
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
