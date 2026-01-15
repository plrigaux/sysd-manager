<div align="center">

![SysD Manager Icon](data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg "App Icon")

# SysD Manager

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://raw.githubusercontent.com/plrigaux/sysd-manager/refs/heads/main/LICENSE)
[![Sponsor](https://img.shields.io/static/v1?label=Sponsor&message=Support&logo=GitHub&color=%23fe8e86&style=flat)](https://github.com/sponsors/plrigaux)
[![Flathub](https://img.shields.io/flathub/v/io.github.plrigaux.sysd-manager?logo=flathub&logoColor=white&label=Flathub)](https://flathub.org/apps/io.github.plrigaux.sysd-manager)

**A user-friendly graphical interface for managing systemd units**

[Features](#features) • [Installation](#installation) • [Screenshots](#screenshots) • [Contributing](#contributing)

</div>

---

## 📋 Overview

SysD Manager provides an intuitive graphical interface for managing systemd units, making system administration more accessible. The application offers comprehensive functionality for viewing, editing, and controlling systemd services, with features including:

- Enable, disable, start, and stop systemd units
- View and edit unit files with syntax highlighting
- Monitor journal logs in real-time
- Visualize unit dependencies
- Manage unit drop-ins and configurations

**Note:** This application is designed for users who prefer a graphical interface over command-line tools. System administrators may prefer using `systemctl` and `journalctl` directly.

---

## ✨ Features

### Core Functionality

| Feature | Status |
|---------|--------|
| Unit file browser with search and filtering | ✅ |
| Enable/disable units and runtime enablement | ✅ |
| Start, stop, and restart units | ✅ |
| Edit unit files and manage drop-ins | ✅ |
| View and navigate unit dependencies | ✅ |
| Clean, freeze, and thaw operations | ✅ |
| Real-time journal monitoring | ✅ |
| Boot ID retrieval and filtering | ✅ |

### User Experience

| Feature | Status |
|---------|--------|
| Syntax highlighting for unit files and journal logs | ✅ |
| Dark and light theme support | ✅ |
| Multi-language support | ✅ |
| Contextual menus and property management | ✅ |
| System signal monitoring | ✅ |
| CLI unit selection at startup | ✅ |

---

## 📸 Screenshots

<div align="center">

### Unit Information Panel (Dark Theme)

![Unit Info Dark](screenshots/unit_info_dark.png)

### Unit Information Panel (Light Theme)

![Unit Info Light](screenshots/unit_info.png)

### Dependency Visualization

![Unit Dependencies](screenshots/dependencies_dark.png)

### Unit File Editor

![Unit Files](screenshots/unit_file_dark.png)

### Journal Viewer

![Unit Journal](screenshots/journal_dark.png)

</div>

---

## 🔧 Installation

### Method 1: Flathub (Recommended)

The easiest way to install SysD Manager on any Linux distribution.

<a href="https://flathub.org/apps/io.github.plrigaux.sysd-manager">
  <img width="200" alt="Download on Flathub" src="https://flathub.org/api/badge?svg"/>
</a>

```bash
flatpak install flathub io.github.plrigaux.sysd-manager
```

---

### Method 2: Arch Linux (AUR)

For Arch Linux users, install directly from the Arch User Repository.

<a href="https://aur.archlinux.org/packages/sysd-manager">
  <img width="200" alt="AUR Package" src="https://aur.archlinux.org/static/css/archnavbar/aurlogo.png"/>
</a>

```bash
yay -S sysd-manager
```

Or using any other AUR helper:

```bash
paru -S sysd-manager
```

---

### Method 3: AppImage

Portable application that runs on most Linux distributions.

<a href="https://github.com/plrigaux/sysd-manager/releases/latest">
  <img width="100" alt="Download AppImage" src="https://docs.appimage.org/_images/appimage.svg"/>
</a>

> **Status:** AppImage builds are currently unavailable. Please use Flathub or build from source.

---

### Method 4: Build from Source

For developers and users who prefer building from source.

#### System Requirements

Ensure the following dependencies are installed on your system:

| Dependency | Package Name (Debian/Ubuntu) | Package Name (Arch) |
|------------|------------------------------|---------------------|
| Rust toolchain | `cargo rustc` | `rust` |
| GTK 4 development files | `libgtk-4-dev` | `gtk4` |
| Libadwaita development files | `libadwaita-1-dev` | `libadwaita` |
| Systemd development library | `libsystemd-dev` | `systemd-libs` |
| GtkSourceView 5 library | `libgtksourceview-5-dev` | `gtksourceview5` |
| Build essentials | `build-essential` | `base-devel` |

**Additional Resources:**
- [Rust installation guide](https://www.rust-lang.org/tools/install)
- [GTK 4 setup guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_linux.html)
- [Libadwaita setup guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html)

#### Build Steps

1. Clone the repository:

```bash
git clone https://github.com/plrigaux/sysd-manager
cd sysd-manager/scripts
```

2. Run the installation script:

```bash
sh install.sh
```

3. Launch the application:

```bash
sysd-manager
```

#### Uninstall

To remove the application:

```bash
cd sysd-manager/scripts
sh uninstall.sh
```

---

## Testing

Test SysD Manager safely using the included **tiny_daemon** web server service:

```bash
cd packaging
python install_tiny
```

---

## Roadmap

Planned enhancements include:

- Persistent unit filter configurations
- Type-aware property display (e.g., human-readable time formats)
- Administrator mode option
- Ongoing user experience improvements

For the complete list of changes, see the [CHANGELOG](CHANGELOG.md).

---

## Contributing

Contributions are welcome. Please contact the project maintainer on [GitHub](https://github.com/plrigaux/sysd-manager) or submit a pull request.

---

## Internationalization

SysD Manager supports multiple languages through community translations.

<div align="center">
<a href="https://hosted.weblate.org/engage/sysd-manager/" target="_blank">
<img src="https://hosted.weblate.org/widget/sysd-manager/translation/multi-auto.svg" alt="Translation Status" style="height:300px;">
</a>
</div>

Translation services are generously hosted by [Weblate](https://weblate.org). To contribute translations, visit the [SysD Manager Hosted Weblate](https://hosted.weblate.org/engage/sysd-manager/) project page.

For developers working with translations, refer to the [Translation Wiki](https://github.com/plrigaux/sysd-manager/wiki/Translation).

---

## Credits

This project is inspired by the work of Guillaume Gomez: https://github.com/GuillaumeGomez/systemd-manager/

---

## License

SysD Manager is licensed under the GNU General Public License v3.0. See [LICENSE](https://raw.githubusercontent.com/plrigaux/sysd-manager/refs/heads/main/LICENSE) for details.
