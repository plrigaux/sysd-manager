# Copilot Instructions for sysd-manager

## Project Overview
- **SysD Manager** is a Rust-based GTK4 application for managing systemd units, targeting less experienced users
- The project is organized as a multi-crate workspace with specialized components:
  - Main GUI app in `src/`
  - Test web server in `tiny_daemon/`
  - Translation tools in `sysd-manager-translating/` and `transtools/`
  - Distribution packaging in `packaging/`

## Core Architecture
- **Main App (`src/`)**: Key components and their responsibilities:
  - `main.rs`: App entry point, GTK initialization, window setup
  - `systemd/`: D-Bus and CLI based systemd interaction (units, journal)
  - `widget/`: GTK4 widget implementations following composite template pattern
  - `analyze/`: Unit file parsing and validation utilities
  - `errors.rs`: Centralized error handling with user-friendly messages

## Integration Patterns
- **GTK Integration**:
  - UI layouts defined in `data/interfaces/*.ui` (XML)
  - Custom widgets implement `ObjectSubclass` trait pattern (see `src/widget/unit_properties_selector/`)
  - Resources (icons, styles) bundled via `data/resources/resources.gresource.xml`

- **Systemd Integration**: 
  - Primary D-Bus interface via `zbus` crate
  - Fallback to CLI commands when needed
  - Error handling adapts systemd errors to GUI-friendly messages
  - Unit operations abstracted in `systemd/` module

## Development Workflows
- **Build**: 
  - Development: `cargo build` or `cargo run` 
  - Full workspace: `cargo build --workspace`
  - Resource compilation happens in `build.rs`

- **Testing**:
  - Unit tests alongside code
  - `tiny_daemon/` provides mock systemd service for testing
  - Example patterns in `widget/` modules

- **UI Development**:
  - Edit `.ui` files in Glade/Cambalache
  - Follow composite template pattern for new widgets
  - Update `resources.gresource.xml` when adding resources

- **Localization**:
  - Source strings in code/UI files
  - Run `transtools` to update `.po` files in `po/`
  - Build generates `.mo` files via `build.rs`

## Common Patterns
- **Widget Creation**: 
  ```rust
  // 1. Define in data/interfaces/my_widget.ui
  // 2. Implement in src/widget/my_widget/
  pub struct MyWidget(ObjectSubclass<imp::MyWidgetImpl>);
  // 3. Register in main.rs
  ```

- **Error Handling**:
  ```rust
  // Convert external errors to SystemdErrors
  impl From<zbus::Error> for SystemdErrors {
      fn from(error: zbus::Error) -> Self {
          SystemdErrors::DBusError(error)
      }
  }
  ```

## Key References
- `README.md`: User-facing features and screenshots
- `data/interfaces/`: UI layout definitions 
- `src/widget/`: Widget implementation examples
- `packaging/`: Distribution-specific build scripts

For details on specific systems, check module-level documentation in source files.
