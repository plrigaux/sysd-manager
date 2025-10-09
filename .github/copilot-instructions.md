# Copilot Instructions for sysd-manager

## Project Overview
- **SysD Manager** is a Rust-based GUI application for managing systemd units, targeting less experienced users.
- The project is organized as a multi-crate workspace:
  - Main GUI app in `src/`
  - Supporting tools in `tiny_daemon/` and `sysd-manager-translating/`
  - Packaging and build scripts in `packaging/` and `scripts/`

## Architecture & Key Components
- **src/**: Main Rust codebase. Major modules:
  - `main.rs`: Application entry point, sets up GTK UI and core logic.
  - `systemd/`: Systemd interaction logic (unit control, journal, etc).
  - `widget/`: GTK widgets for UI (e.g., unit info, dependencies, properties selectors).
  - `analyze/`: Utilities for analyzing systemd units.
- **data/**: UI definitions (`.ui` files), icons, schemas, and resources.
- **packaging/**: Scripts and configs for AppImage, Flatpak, AUR, etc.
- **tiny_daemon/**: Minimal web server for testing systemd management.
- **sysd-manager-translating/**: Translation utilities and resources.

## Developer Workflows
- **Build**: Use `cargo build` (main app), or `cargo build --workspace` for all crates.
- **Run**: `cargo run` (main app). For Flatpak/AppImage, use scripts in `packaging/`.
- **Test**: `cargo test` (unit/integration tests). Some crates have their own tests.
- **Translations**: Update `.po` files in `po/`, use `sysd-manager-translating` for tooling.
- **Packaging**: Use scripts in `packaging/` for building distributables.

## Project Conventions
- **GTK UI**: All UI layouts are defined in `data/interfaces/*.ui` (Glade/Cambalache XML).
- **Icons/Resources**: Place icons in `data/icons/`, update `resources.gresource.xml` as needed.
- **Systemd Integration**: All systemd calls are wrapped in `src/systemd/` for testability and separation.
- **Testing**: Use Rust's built-in test framework. For widget/UI logic, see `widget/` submodules.
- **Error Handling**: Centralized in `src/errors.rs`.
- **Style**: Follows Rust 2021 idioms. UI style in `data/styles/`.

## Integration Points
- **Systemd**: Interacts via D-Bus and CLI, abstracted in `src/systemd/`.
- **GTK**: Uses `gtk-rs` for UI; all widgets/components in `widget/`.
- **Packaging**: Integrates with Flatpak, AppImage, AUR, etc. via `packaging/` scripts.

## Examples
- To add a new UI panel: create a `.ui` in `data/interfaces/`, implement logic in `src/widget/`, and register in `main.rs`.
- To add a new systemd operation: extend `src/systemd/`, expose via widget logic.

## References
- See `README.md` for user-facing features and screenshots.
- See `packaging/` for build/distribution scripts.
- See `po/` for translation workflow.

---
For questions, check the code comments, or see the main `README.md` for more context.
