import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { execFileSync } from "node:child_process";

import { normalizeVersion, semverPattern, setVersion } from "./bump-version.mjs";

// Automates a release: dates the CHANGELOG "Unreleased" section, syncs the
// version across config files, then commits and tags. Push is opt-in via --push
// so the tag (which triggers the GitHub Actions release build) is never sent
// without an explicit choice.
//
//   pnpm release <version> [--push] [--dry-run]

const args = process.argv.slice(2);
const flags = new Set(args.filter((arg) => arg.startsWith("--")));
const positional = args.filter((arg) => !arg.startsWith("--"));

const shouldPush = flags.has("--push");
const dryRun = flags.has("--dry-run");

const version = normalizeVersion(positional[0]);

if (!version) {
  console.error("Usage: pnpm release <version> [--push] [--dry-run]");
  process.exit(1);
}

if (!semverPattern.test(version)) {
  console.error(`Invalid version: ${version}. Expected a semver value like 0.1.4`);
  process.exit(1);
}

const tag = `v${version}`;
const rootDir = process.cwd();
const changelogPath = path.join(rootDir, "CHANGELOG.md");

function git(...gitArgs) {
  return execFileSync("git", gitArgs, { encoding: "utf8" }).trim();
}

function runGit(...gitArgs) {
  execFileSync("git", gitArgs, { stdio: "inherit" });
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

function today() {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

// 1. Must be inside a git repository.
try {
  git("rev-parse", "--is-inside-work-tree");
} catch {
  fail("Not inside a git repository.");
}

// 2. The release commit should contain only version + changelog changes, so the
//    working tree must be clean first. Skipped in dry-run so a preview is always
//    possible mid-work.
if (!dryRun) {
  const status = git("status", "--porcelain");
  if (status) {
    console.error("Working tree is not clean. Commit or stash changes first:");
    fail(status);
  }
}

// 3. Refuse to reuse an existing tag.
if (git("tag", "--list", tag)) {
  fail(`Tag ${tag} already exists.`);
}

// 4. Locate and validate the CHANGELOG "Unreleased" section.
const changelog = fs.readFileSync(changelogPath, "utf8");
const lines = changelog.split(/\r?\n/);

const headingOf = (line) => (line.startsWith("## ") ? line.slice(3).trim() : null);

let unreleasedIndex = -1;
for (let i = 0; i < lines.length; i += 1) {
  const heading = headingOf(lines[i]);
  if (heading && /^unreleased\b/i.test(heading)) {
    unreleasedIndex = i;
    break;
  }
}

if (unreleasedIndex === -1) {
  fail('No "## Unreleased" section found in CHANGELOG.md.');
}

let sectionEnd = lines.length;
for (let i = unreleasedIndex + 1; i < lines.length; i += 1) {
  if (lines[i].startsWith("## ")) {
    sectionEnd = i;
    break;
  }
}

// Guard against releasing a version that already has a section.
const versionAlreadyReleased = lines.some((line) => {
  const heading = headingOf(line);
  if (!heading) {
    return false;
  }
  return (
    heading === version ||
    heading.startsWith(`${version} `) ||
    heading === `[${version}]` ||
    heading.startsWith(`[${version}] `)
  );
});

if (versionAlreadyReleased) {
  fail(`CHANGELOG.md already contains a section for ${version}.`);
}

// The section must have real notes — blank lines and "###" subheadings alone
// do not count. This is the check that turns a forgotten changelog into an early,
// local failure instead of a late CI failure after the tag is pushed.
const sectionBody = lines
  .slice(unreleasedIndex + 1, sectionEnd)
  .filter((line) => {
    const text = line.trim();
    return text !== "" && !text.startsWith("#");
  });

if (sectionBody.length === 0) {
  fail(
    'The "## Unreleased" section is empty. Add release notes before releasing.',
  );
}

const releaseNotes = lines
  .slice(unreleasedIndex + 1, sectionEnd)
  .join("\n")
  .trim();

console.log(`Releasing ${tag} (${today()})`);
console.log("");
console.log("Release notes to be published:");
console.log("----------------------------------------");
console.log(releaseNotes);
console.log("----------------------------------------");
console.log("");

if (dryRun) {
  console.log("Dry run — no files changed, no commit, no tag.");
  process.exit(0);
}

// 5. Rewrite the CHANGELOG: keep a fresh empty "Unreleased" on top, and move the
//    current notes under a dated version heading.
const rewritten = [...lines];
rewritten.splice(
  unreleasedIndex,
  1,
  "## Unreleased",
  "",
  `## ${version} - ${today()}`,
);
fs.writeFileSync(changelogPath, rewritten.join("\n"));

// 6. Sync the version across package.json / tauri.conf.json / Cargo.toml.
setVersion(version);

// 7. Commit and tag.
runGit(
  "add",
  "CHANGELOG.md",
  "package.json",
  "src-tauri/tauri.conf.json",
  "src-tauri/Cargo.toml",
);
runGit("commit", "-m", `chore(release): prepare ${tag}`);
runGit("tag", "-a", tag, "-m", `Port Kill ${tag}`);

const branch = git("rev-parse", "--abbrev-ref", "HEAD");

if (shouldPush) {
  runGit("push", "origin", branch);
  runGit("push", "origin", tag);
  console.log("");
  console.log(
    `Pushed ${branch} and ${tag}. GitHub Actions will now build the release.`,
  );
} else {
  console.log("");
  console.log("Local release commit and tag created. To publish, run:");
  console.log(`  git push origin ${branch}`);
  console.log(`  git push origin ${tag}`);
}
