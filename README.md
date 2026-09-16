# Port Kill

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Release](https://img.shields.io/github/v/release/wungjyan/port-kill)](https://github.com/wungjyan/port-kill/releases)
[![License](https://img.shields.io/github/license/wungjyan/port-kill)](./LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)](#platform-support)

Port Kill is a native macOS utility for finding processes that occupy TCP listening ports and terminating them with additional identity checks.

It is built for local development workflows where a server, watcher, or background service keeps a port busy and a raw `kill` command carries the risk of targeting a stale or unrelated PID.

Built with Tauri 2, Vue 3, TypeScript, Vite, and Rust.

## Why Port Kill?

Command-line tools such as `lsof`, `ps`, and `kill` are powerful, but resolving a port conflict usually requires combining their output manually. Port Kill presents the relevant process context in one place and revalidates the target immediately before sending a signal.

## Features

- Inspect TCP listening ports, process names, PIDs, commands, working directories, listening addresses, and start times.
- Search by port, PID, process name, command, working directory, or listening address.
- Sort by most recent start time, port, or process name.
- Distinguish loopback, specific-address, and all-interface listeners.
- Recognize common development ports used by tools such as Vite, Next.js, PostgreSQL, Redis, and MongoDB.
- Send `TERM` for a normal shutdown or `KILL` when a forceful shutdown is explicitly requested.
- Show other listening ports owned by the same process before termination.
- Detect when a watcher or service manager reopens a port after the original process exits.
- Refresh automatically at a configurable interval while preserving the last successful result after a scan failure.
- Follow the system appearance or use a persistent light/dark preference.

## Safety model

Process termination is treated as a destructive operation. Before sending a signal, the Rust backend rescans the selected port and checks the PID, owning user, process name, and start time against the item shown in the interface. This reduces the chance of acting on stale data or a reused PID.

Additional boundaries:

- Only processes owned by the current user can be terminated.
- Normal termination uses `TERM`; force termination uses the non-catchable `KILL` signal and requires an explicit action.
- System commands run with timeouts, and timed-out child processes are terminated and reaped.
- Port and process information is processed locally. Port Kill does not include telemetry or send scan results to a remote service.

These checks reduce risk but cannot eliminate every race condition. Review the process details before terminating it, especially when using force termination.

## Download

Download the latest build from [GitHub Releases](https://github.com/wungjyan/port-kill/releases):

- Apple Silicon Macs: `Port.Kill_<version>_aarch64.dmg`
- Intel Macs: `Port.Kill_<version>_x64.dmg`

Open the DMG and drag **Port Kill.app** into the Applications folder.

### macOS reports that the app is damaged

Current releases are not yet signed and notarized by Apple. A browser download may therefore receive a quarantine attribute that Gatekeeper cannot verify.

After confirming that the application came from this repository's GitHub Releases page, move it to Applications and run:

```bash
xattr -dr com.apple.quarantine "/Applications/Port Kill.app"
open "/Applications/Port Kill.app"
```

This removes the quarantine attribute from this application only. Do not run the command on copies obtained from an untrusted source.

## Platform support

Port Kill currently supports **macOS only**. The backend depends on these system commands and paths:

- `/usr/sbin/lsof`
- `/bin/ps`
- `/bin/kill`

It scans `TCP LISTEN` sockets only. UDP sockets and established TCP connections are intentionally outside the current scope.

## Development

### Requirements

- macOS
- Node.js 20 or later
- pnpm 9 or later
- Rust stable
- Xcode Command Line Tools

### Run locally

```bash
pnpm install
pnpm tauri dev
```

Use `pnpm dev` to work on the interface in a browser. Tauri backend commands are unavailable in browser-only mode.

### Validate changes

```bash
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

### Build the desktop application

```bash
pnpm tauri build
```

## Project structure

```text
src/                          Vue frontend
src/components/               Port list, toolbar, and process details
src-tauri/src/lib.rs          Port scanning, identity checks, and termination
src-tauri/tauri.conf.json     Desktop window, security, and bundle settings
docs/frontend-api.md          Frontend/backend data contract
docs/releasing.md             Maintainer release checklist
.github/workflows/release.yml Dual-architecture macOS release workflow
```

## Releases

Pushing a `v*` tag starts the GitHub Actions release workflow. It validates that the tag matches the versions in the JavaScript, Tauri, and Rust manifests, then builds release artifacts for Apple Silicon and Intel Macs.

See the [release checklist](./docs/releasing.md) and [changelog](./CHANGELOG.md) for details.

## Contributing

Bug reports and focused pull requests are welcome. For a substantial behavior change or new platform proposal, open an issue first so the scope and safety implications can be discussed.

Changes to the termination flow should preserve target revalidation, current-user restrictions, command timeouts, and clear user-facing errors.

## License

Port Kill is available under the [MIT License](./LICENSE).
