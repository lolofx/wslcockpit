# WSLCockpit

**English** | [Français](./README.fr.md)

[![CI](https://github.com/lolofx/wslcockpit/actions/workflows/ci.yml/badge.svg)](https://github.com/lolofx/wslcockpit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

A small, native-feeling desktop app to manage your WSL2 distributions on Windows: see what is running, start and stop distros, edit `.wslconfig` / `wsl.conf` safely, and export, import, clone, convert or delete distributions without remembering `wsl.exe` flags.

Built with Tauri 2, Svelte and Rust. Portable single executable, no installer, no administrator rights, light and dark theme following Windows.

> **Status:** early development. The interface is currently in French only; English and French are planned (see the [roadmap](#roadmap)).

## Features

- **Overview**: state, WSL version, default distribution, virtual disk (VHDX) size, VM RAM and CPU, free disk space of running distros.
- **Lifecycle**: start, stop, restart, open a terminal (Windows Terminal if available, console otherwise), stop everything (`wsl --shutdown`).
- **Policies** per distribution: manual, start when the app opens, or keep alive.
- **Aliases**: local display names. WSL itself is never renamed.
- **Configuration forms** for `.wslconfig` (memory, processors, swap, networking mode, `vmIdleTimeout`) and each distro's `wsl.conf` (systemd, default user, automount). Invalid values block saving; comments and unknown keys are preserved; a timestamped backup is written next to the file before every change.
- **Distribution operations**: export, import, clone, delete (type-the-name confirmation), WSL1/WSL2 conversion, sparse VHDX mode, set default.
- **Tray icon**: closing the window hides it; *Quit* is in the tray menu. Optional start with Windows.
- **Update notice** when a newer GitHub release exists (silent when offline).

## Install

1. Download `WSLCockpit.exe` from the [releases](https://github.com/lolofx/wslcockpit/releases) page, or the `WSLCockpit-portable` artifact of a CI run.
2. Run it. No installation and no administrator rights are needed.

Windows SmartScreen may warn about an unrecognized app because the executable is not signed: choose *More info*, then *Run anyway*.

**Requirements**: Windows 10 or 11 with WSL installed (`wsl --install`) and WebView2 (included with Windows 11).

Settings (aliases, policies) live in `%APPDATA%\WSLCockpit`.

## Distribution operations

One operation at a time, with progress and cancellation. Paths are typed by hand for now (no file picker yet).

| Operation | WSL command | Safeguards |
|---|---|---|
| Export | `wsl --export` | never overwrites an existing file, `.tar` required; a partial archive is removed on failure or cancel |
| Import | `wsl --import --version 2` | unique name (letters, digits, `_ - .`), target folder must be absent or empty; a half-imported distro is unregistered on failure or cancel |
| Clone | export, then import | temporary archive next to the chosen folder, removed afterwards |
| Delete | `wsl --unregister` | exact name must be typed (checked again in the backend), export offered first; **irreversible** |
| Convert WSL1 / WSL2 | `wsl --set-version` | distro must be stopped; can take several minutes |
| Sparse mode | `wsl --manage --set-sparse` | distro must be stopped; disabled with the reason when WSL is too old (2.0.0 or later) |

Known limits: the current sparse state is not displayed; progress shows bytes written, not a percentage; there is no elevation, so VHDX compaction is not available yet.

## How it works

The app only drives `wsl.exe`, PowerShell and `reg.exe`, with arguments passed as arrays (never concatenated into a shell string). `wsl.exe` output (UTF-16, localized) is decoded and parsed by pure functions in `crates/wsl-core`, which are unit-tested on any OS. The Tauri shell in `src-tauri/` only wires those functions to the Svelte front end.

```
crates/wsl-core/   pure parsers, INI editor, validation, operations (tested on Linux)
src-tauri/         Tauri commands, tray, settings
src/               Svelte front end
```

## Development

```
npm install
npm test                                                   # cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
(cd src-tauri && cargo clippy -- -D warnings)              # src-tauri is outside the root workspace
npm run check                                              # svelte-check
npm run build                                              # front end build
```

The Windows build is produced by CI (`windows-build` job); the parsers and core logic can be developed and tested on Linux or macOS.

## Roadmap

Planned, in no particular order:

- Create a new distribution from the online list, with download size information.
- Check for WSL updates and install them.
- Multilingual interface (English and French first).
- Integrated terminal with panes; Windows Terminal split presets.
- VHDX compaction (needs elevation); in-app update of the portable exe.
- Quick per-distro menu in the tray.

## Contributing

Issues and pull requests are welcome. Before opening a PR, run the checks listed in [Development](#development); the CI runs the same ones. Please keep changes small and focused, and add unit tests for any parsing or validation logic in `crates/wsl-core`.

## License

MIT, see [LICENSE](./LICENSE).
