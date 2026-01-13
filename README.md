<div align="center">

![SysD Manager Icon](data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg "App Icon")

# SysD Manager

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://raw.githubusercontent.com/plrigaux/sysd-manager/refs/heads/main/LICENSE)
[![Love SysD Manager? Please consider donating to sustain our activities](https://img.shields.io/static/v1?label=Sponsor&message=%E2%9D%A4&logo=GitHub&color=%23fe8e86&style=flat)](https://github.com/sponsors/plrigaux)
[![Flathub](https://img.shields.io/flathub/v/io.github.plrigaux.sysd-manager?logo=flathub&logoColor=white&label=Flathub)](https://flathub.org/apps/io.github.plrigaux.sysd-manager)

A user-friendly GUI to manage systemd units

</div>

## 📋 Description

- Change the enablement and running status of systemd units
- View and modify unit files with syntax highlighting
- Check journal logs with syntax highlighting
- Explore unit dependencies visually
- And much more!

> **Disclaimer:** This application is intended for users with less experience with systemd rather than professional system administrators. If you consider yourself an administrator, please refer to `systemctl` and `journalctl` documentation.

## ✨ Features

| Feature                                                         | Category      | Status |
| --------------------------------------------------------------- | ------------- | ------ |
| Unit file browser with search and filtering                     | Functionality | ✅     |
| Browser add remove property                                     | Functionality | ✅     |
| Enable or disable a unit                                        | Functionality | ✅     |
| Enable a unit in runtime                                        | Functionality | ✅     |
| Edit unit file                                                  | Functionality | ✅     |
| Edit and manage unit drop-ins                                   | Functionality | ✅     |
| Activate or deactivate a unit                                   | Functionality | ✅     |
| View and modify unit file                                       | Functionality | ✅     |
| View and navigate unit's dependencies                           | Functionality | ✅     |
| Unit file syntax highlighting                                   | UX            | ✅     |
| Journal event syntax highlighting                               | UX            | ✅     |
| List of all running units (systemd-analyze blame)               | Functionality | ✅     |
| Dark and Light style switching                                  | UX            | ✅     |
| Select a unit at program opening via CLI                        | Functionality | ✅     |
| Clean unit like `systemctl clean`                               | Functionality | ✅     |
| Freeze & Thaw unit like `systemctl freeze` and `systemctl thaw` | Functionality | ✅     |
| Multi-language support                                          | UX            | ✅     |
| Real-time journal events update                                 | Functionality | ✅     |
| Retrieve list of boot IDs                                       | Functionality | ✅     |
| Ability to watch _systemd_ signals                              | UX            | ✅     |
| Filter units on loaded properties                               | UX            | ✅     |
| Browser contextual menu                                         | UX            | ✅     |
| Browser add remove property                                     | UX            | ✅     |

_Need a feature? Contact the author or contribute to the project! If you're stuck, take a look at `systemctl`._

## 📸 Screenshots

<div align="center">

### Unit Info (Dark)

![Unit Info Dark](screenshots/unit_info_dark.png)

### Unit Info (Light)

![Unit Info Light](screenshots/unit_info.png)

### Unit Dependencies

![Unit Dependencies](screenshots/dependencies_dark.png)

### Unit Files

![Unit Files](screenshots/unit_file_dark.png)

### Unit Journal

![Unit Journal](screenshots/journal_dark.png)

</div>

## 🔧 Installation

### Flathub

<a href="https://flathub.org/apps/io.github.plrigaux.sysd-manager"><img width="200" alt="Download on Flathub" src="https://flathub.org/api/badge?svg"/></a>

### Arch Linux

<a href="https://aur.archlinux.org/packages/sysd-manager"><img width="200" alt="Download on Flathub" src="https://aur.archlinux.org/static/css/archnavbar/aurlogo.png"/></a>

```bash
yay -S sysd-manager
```

### AppImage

<a href="https://github.com/plrigaux/sysd-manager/releases/latest"><img width="100" alt="Download latest Appimage release" src="https://docs.appimage.org/_images/appimage.svg"/></a>
__Currently Broken__

### Build from Source

1. Install prerequisites:
   - [Rust](https://www.rust-lang.org/tools/install)
   - GTK 4 and build essentials ([how-to](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_linux.html))
   - Libadwaita ([how-to](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html))
   - Systemd development library (`libsystemd-dev`)
   - GtkSourceView 5 development library

2. Clone and build:

```bash
git clone https://github.com/plrigaux/sysd-manager
cd sysd-manager/scripts
sh install.sh
```

3. Run:

```bash
sysd-manager
```

_For a clean removal, execute:_ `sh uninstall.sh`

## 🧪 Testing

You can safely test SysD Manager using **tiny_daemon**, a simple web server service included with the project:

```bash
cd packaging
python install_tiny
```

## 🛣️ Roadmap

Planned features:

- Save unit filters
- Adjust cell viewer according to property types (e.g. display uint as human time)
- Always administrator mode
- Continuous UX improvements

## 📝 Changelog

All notable changes are documented in the [CHANGELOG](CHANGELOG.md).

## 🤝 Contributing

Interested in contributing? Contact the project maintainer on [GitHub](https://github.com/plrigaux/sysd-manager).

## 🌐 Internationalization

**SysD Manager** can be displayed in different languages as long a translation has been provided.

<div align="center">
<a href="https://hosted.weblate.org/engage/sysd-manager/" target="_blank">
<img src="https://hosted.weblate.org/widget/sysd-manager/translation/multi-auto.svg" alt="Status da tradução" style="height:300px;" >
</a>
</div>

### Translators

Translations are generously hosted by [Weblate](https://weblate.org).
Please help translate **Sysd Manager** into more languages through the [**Sysd Manager** Hosted Weblate](https://hosted.weblate.org/engage/sysd-manager/).

_Information for developers to handle translations can be found [here](https://github.com/plrigaux/sysd-manager/wiki/Translation)._

## 💡 Credits

This project is inspired by the work of Guillaume Gomez: https://github.com/GuillaumeGomez/systemd-manager/
