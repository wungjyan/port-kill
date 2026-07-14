# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Port Kill is a macOS desktop app for managing port processes, built with Tauri 2 + Vue 3 + TypeScript + Rust. It scans TCP LISTEN ports, displays process information, and allows users to terminate processes.

## Platform Constraints

**macOS-only**: The backend relies on hardcoded macOS system commands:
- `/usr/sbin/lsof` - for scanning ports
- `/bin/ps` - for process details (command, start time, cwd)
- `/bin/kill` - for terminating processes

Linux and Windows are not currently supported. Any backend changes must account for these platform-specific dependencies.

## Development Commands

Install dependencies:
```bash
pnpm install
```

Run in development mode (starts both Vite dev server and Tauri):
```bash
pnpm tauri dev
```

Frontend dev server only (no Tauri shell):
```bash
pnpm dev
```

Build for production:
```bash
pnpm build          # Build frontend (includes vue-tsc --noEmit type check)
pnpm tauri build    # Build Tauri app bundle
```

Sync version across all config files (writes `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`):
```bash
pnpm version:set <version>   # e.g. pnpm version:set 0.1.4 — runs scripts/bump-version.mjs
```

Cut a release (dates the CHANGELOG, syncs the version, commits, tags — push is opt-in):
```bash
pnpm release <version> --dry-run   # preview the extracted release notes, no changes
pnpm release <version>             # rewrite CHANGELOG + version sync + commit + tag (local)
pnpm release <version> --push      # also push branch + tag, triggering the GitHub Actions build
```

Rust checks (backend logic has unit tests in `src-tauri/src/lib.rs`):
```bash
cargo test   --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Architecture

### Tauri Commands (Rust → Frontend)

Three commands exposed via `#[tauri::command]`:

1. **`list_ports()`** - Returns `PortListResponse`
   - Executes `lsof -nP -iTCP -sTCP:LISTEN -F pcLnPTu` to get raw port data
   - Enriches each process with `ps` data (command, start time, cwd)
   - Groups by `pid:protocol:port` to aggregate multiple listening addresses
   - Returns sorted by `started_at_ts` (most recent first), then port, then process name

2. **`kill_process(pid, force, expected_port, expected_started_at_ts, expected_process_name, expected_user)`** - Returns `KillResult`
   - Revalidates process identity and ownership before sending a signal
   - Sends `-TERM` (default) or `-KILL` (force) signal
   - Polls for up to 1.2 seconds to verify process termination
   - Returns normalized error messages in Chinese

3. **`check_port_rebound(port: u16)`** - Returns rebound status
   - Called after `kill_process` to detect if a new process has re-claimed the port (e.g., hot-reload, supervisor restart)

### Data Flow

```
Rust Backend (lib.rs)
  ↓ lsof + ps commands
  ↓ parse & aggregate
  ↓ Tauri IPC
Frontend (Vue 3)
  ↓ PortKillWorkbench.vue (main container)
  ↓ PortToolbar.vue (search, sort, refresh controls)
  ↓ PortTable.vue (data table with actions)
  ↓ PortDetailPanel.vue (selected process details)
```

### Frontend Structure

- **App.vue**: Theme provider — reads/persists the user's theme preference, follows the system on first run, and wires the overrides into `NConfigProvider`
- **src/styles/naiveTheme.ts**: `darkThemeOverrides` / `lightThemeOverrides` — the actual Naive UI color/token overrides for both modes
- **PortKillWorkbench.vue**: Main container, manages state (port list, selected process, auto-refresh)
- **PortTable.vue**: Displays port data using Naive UI DataTable, handles sorting/filtering
- **PortToolbar.vue**: Search input, sort controls, refresh button, theme toggle
- **PortDetailPanel.vue**: Shows detailed info for selected process
- **types.ts**: TypeScript type definitions matching Rust structs
- **portHints.ts** / **portMeta.ts**: Port metadata for common development ports (Vite, Next.js, PostgreSQL, etc.)

### Key Technical Details

**Port aggregation**: Multiple listening addresses (e.g., IPv4 + IPv6) for the same `pid:protocol:port` are grouped into a single `PortProcess` with arrays for `hosts` and `ip_versions`.

**Process name derivation**: The Rust backend attempts to extract a clean process name from the full command by:
1. Splitting on ` --` and taking the first part
2. Taking the first whitespace-separated token
3. Extracting the basename from paths

**Start time parsing**: `ps -o lstart=` output (e.g., "Mon Jan 15 14:23:45 2024") is parsed into a sortable integer timestamp (`started_at_ts`) for sorting by recency.

**Theme system**: Theme overrides live in `src/styles/naiveTheme.ts` (`darkThemeOverrides` / `lightThemeOverrides`), with specific colors for DataTable, Input, and Card components. App.vue selects between them, persists the choice to `localStorage` (`port-kill.theme`), and defaults to the OS preference when unset.

**Auto-refresh**: Managed in PortKillWorkbench.vue. Users can toggle auto-refresh on/off and pick an interval in the settings panel; the default is 10 seconds (`DEFAULT_REFRESH_INTERVAL_MS = 10_000`), and both preferences are persisted to `localStorage`.

### When Modifying

**Backend (Rust)**: Preserve the aggregation key (`pid + protocol + port`), timeout controls on system commands, and Chinese error message style. Only scan `TCP LISTEN` — do not expand to UDP or established connections unless explicitly requested.

**Frontend**: Keep state management centralized in PortKillWorkbench.vue. The `src/types.ts` types must stay in sync with Rust return structures — if you change a Tauri command's return shape, update both sides. Rust structs use `#[serde(rename_all = "camelCase")]`, so snake_case fields (e.g. `started_at_ts`) arrive as camelCase (`startedAtTs`) on the frontend. Search matches across port, PID, process name, command, cwd, and listen address summaries.

### Verification

After changes, always type-check with `pnpm build`. For behavioral changes, manually verify in `pnpm tauri dev`: port list loads, search/sort works, kill flows complete, port rebound prompt appears, and both themes render correctly.

## Tauri Configuration

- **Window size**: 1180x760 (min: 960x640)
- **Dev server**: http://localhost:1420 (Vite default: 1420, not 5173)
- **Bundle identifier**: `me.wjian.portkill`
- **Hidden title bar**: macOS-style frameless window

## Dependencies

Frontend:
- Vue 3.5+ with TypeScript
- Naive UI 2.44+ (component library)
- @lucide/vue (icons — replaced the earlier custom icon components)
- Vite 6+ (build tool)

Backend:
- Tauri 2
- serde + serde_json (serialization)

## Reference Docs

- `docs/frontend-api.md` — full reference for the three Tauri commands: request params, return field shapes (camelCase), and which UI scenarios call each. Read this before changing a command's return shape or the frontend that consumes it.
- `docs/releasing.md` — release checklist: keep `CHANGELOG.md`'s `## Unreleased` section filled during development, then `pnpm release <version>` dates it, syncs the version, commits, and tags. Pre-release verification (build, `cargo test`, `cargo clippy`, `tauri build`) and the tag-triggered GitHub Actions flow (`.github/workflows/release.yml`) that builds Apple Silicon + Intel bundles. `release.mjs` fails early if `Unreleased` is empty or versions/tags disagree, so mismatches never reach CI.
