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

Build for production:
```bash
pnpm build          # Build frontend
pnpm tauri build    # Build Tauri app bundle
```

Type checking (frontend):
```bash
pnpm build          # Runs vue-tsc --noEmit before vite build
```

## Architecture

### Tauri Commands (Rust → Frontend)

Two main commands exposed via `#[tauri::command]`:

1. **`list_ports()`** - Returns `PortListResponse`
   - Executes `lsof -nP -iTCP -sTCP:LISTEN -F pcLnPTu` to get raw port data
   - Enriches each process with `ps` data (command, start time, cwd)
   - Groups by `pid:protocol:port` to aggregate multiple listening addresses
   - Returns sorted by `started_at_ts` (most recent first), then port, then process name

2. **`kill_process(pid: i32, force: Option<bool>)`** - Returns `KillResult`
   - Sends `-TERM` (default) or `-KILL` (force) signal
   - Polls for up to 1.2 seconds to verify process termination
   - Returns normalized error messages in Chinese

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

- **App.vue**: Theme provider with custom Naive UI overrides for dark/light modes
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

**Theme system**: Custom theme overrides are defined in App.vue for both dark and light modes, with specific colors for DataTable, Input, and Card components.

**Auto-refresh**: Default interval is 5 seconds, managed in PortKillWorkbench.vue.

## Tauri Configuration

- **Window size**: 1180x760 (min: 960x640)
- **Dev server**: http://localhost:1420 (Vite default: 1420, not 5173)
- **Bundle identifier**: `me.wjian.portkill`
- **Hidden title bar**: macOS-style frameless window

## Dependencies

Frontend:
- Vue 3.5+ with TypeScript
- Naive UI 2.44+ (component library)
- Vite 6+ (build tool)

Backend:
- Tauri 2
- serde + serde_json (serialization)
- tauri-plugin-opener (open URLs)
