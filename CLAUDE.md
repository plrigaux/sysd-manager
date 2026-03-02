# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SysD Manager is a Rust/GTK4 desktop application that provides a graphical interface for managing systemd units. It targets users who prefer a GUI over `systemctl`/`journalctl`. The app is distributed via Flathub, AUR, AppImage, and RPM.

## Build & Run Commands

```bash
# Development build and run (auto-detects dev mode via CARGO env var)
cargo run

# Run with specific unit selected at startup
cargo run -- NetworkManager.service

# Run targeting user session D-Bus
cargo run -- --user

# Release build
cargo build --release

# Build entire workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p sysd-manager-comcontroler

# Format code (uses rustfmt.toml - max_width=100, edition=2024)
cargo fmt

# Lint
cargo clippy --workspace

# Install to system (requires sudo, from scripts/ directory)
sh scripts/install.sh          # release build
sh scripts/install.sh debug    # debug build

# Update translation .pot file and .po files
cargo run -p transtools -- packfiles

# Install the tiny_daemon test service (safe sandbox for testing)
cd packaging && python install_tiny
```

## Workspace Structure

The project is a Cargo workspace with the main binary at the root and these member crates:

| Crate | Purpose |
|-------|---------|
| `sysd-manager-base` | Shared types, constants, enums, D-Bus proxy interfaces used by all crates |
| `sysd-manager-comcontroler` | All systemd communication — D-Bus calls, journal reading, unit operations |
| `sysd-manager-proxy` | Privileged D-Bus proxy daemon (`sysd-manager-proxy`) that runs as a system service for operations requiring elevated permissions |
| `sysd-manager-translating` | Build-time library for `.po`/`.mo` file generation |
| `transtools` | CLI tool to regenerate `.pot` and `.po` translation files |
| `sysd-manager-test-base` | Shared test utilities |
| `tiny_daemon` | Minimal web server used as a safe test systemd service |

## Main App Architecture (`src/`)

```
src/
  main.rs              # Entry point: GTK init, CLI args (clap), locale setup, resource registration
  systemd_gui.rs       # App-wide state (is_dark, GSettings)
  errors.rs            # GUI error display helpers
  consts.rs            # App-level constants
  analyze/             # Unit file parsing and blame/diff analysis
  utils/               # Font management, color palette, text view hyperlinks, syntax writer
  widget/              # All GTK4 composite widgets
    mod.rs             # InterPanelMessage enum — the primary inter-widget communication bus
    app_window/        # Main application window (imp.rs = subclass impl, mod.rs = public API)
    unit_list/         # Filterable/searchable list of systemd units
    unit_info/         # Unit properties display panel
    unit_file_panel/   # Unit file viewer/editor with GtkSourceView syntax highlighting
    journal/           # Real-time journal log viewer
    unit_dependencies_panel/  # Dependency tree visualization
    unit_control_panel/       # Start/stop/enable/disable buttons
    preferences/       # Preferences dialog and persistent settings (GSettings)
    ...
```

### Widget Pattern

All custom widgets follow the GTK4 composite template pattern:

1. **UI file**: `data/interfaces/<widget_name>.ui` (XML, editable in Cambalache)
2. **Subclass impl**: `src/widget/<widget_name>/imp.rs` — implements `ObjectSubclass`, `WidgetImpl`, etc.
3. **Public wrapper**: `src/widget/<widget_name>/mod.rs` — the GObject wrapper with public methods

Resources (UI files, CSS, icons) are registered via `data/resources/resources.gresource.xml` and compiled by `build.rs` using `glib-compile-resources`.

### Inter-Widget Communication

Widgets communicate via `InterPanelMessage` enum (defined in `src/widget/mod.rs`). The app window dispatches messages to all panels via `AppWindow::set_inter_message()`. This avoids direct coupling between sibling panels.

### Systemd Communication (`sysd-manager-comcontroler`)

The `comcontroler` crate is imported in `Cargo.toml` with the alias `systemd`. It abstracts all systemd interaction:

- **D-Bus (primary)**: Uses `zbus` crate with generated proxy structs in `src/sysdbus/`
- **Privilege escalation**: For system-level operations from a non-privileged GUI, calls are routed through `sysd-manager-proxy` (a separate D-Bus daemon). The `proxy_switcher` module decides whether to use the proxy or go direct.
- **`flatpak` feature**: When compiled with `--features flatpak`, the proxy communication path changes to use `flatpak-spawn` for host command execution.

### Proxy Architecture

The `sysd-manager-proxy` binary runs as a system D-Bus service (`io.github.plrigaux.SysDManager`). It:
- Exposes privileged systemd operations over session D-Bus
- Is started automatically by the main app via D-Bus activation
- In development mode (`cargo run`), connects to `io.github.plrigaux.SysDManagerDev` bus name

The `RunMode` enum (`sysd-manager-base/src/lib.rs`) controls dev vs. production proxy bus names. When run via `cargo`, dev mode is assumed automatically.

## Key Configuration Files

- `.env` — sets `RUST_LOG=info` and `TEXTDOMAINDIR=target/locale` for development runs
- `data/schemas/io.github.plrigaux.sysd-manager.gschema.xml` — GSettings schema (compiled by `build.rs` into `~/.local/share/glib-2.0/schemas/` during development)
- `data/resources/resources.gresource.xml` — lists all bundled resources
- `rustfmt.toml` — `max_width = 100`, edition 2024

## Localization

- Source strings use `gettextrs::gettext()` macro
- `.po` files live in `po/`, listed in `po/LINGUAS`
- `build.rs` compiles `.po` → `.mo` files into `target/locale/`
- `transtools` CLI regenerates the `.pot` template and updates `.po` files
- Development locale dir is `target/locale/` (set via `.env`)
- Translations managed on [Weblate](https://hosted.weblate.org/engage/sysd-manager/)

## Features Flags

- `default` — standard build, uses the `sysd-manager-proxy` for privilege escalation
- `flatpak` — Flatpak distribution build; changes file access paths, uses `flatpak-spawn` for host commands, bundles the proxy inside the Flatpak sandbox
