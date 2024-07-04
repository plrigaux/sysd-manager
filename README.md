# SysD Manager

This application exists to allow user to manage their **systemd units** via a GUI. Not only are you able to make changes to the enablement and running status of each of the units, but you will also be able to view and modify their unit files, check the journal logs. 

## Features

Here are __Sysd Manager__ main features :
* Enable or disable a unit
* Actrivate or desactivate a unit
* View and modify unit file
* List of all running units, ordered by the time they took to initialize __systemd-analyze blame__


*__Note__ if you need a feature communicate with the author or contribute to the project. If you're stuck take a look at __systemctl__.*

## Requirement

Any Linux distribution that has adopted **systemd**.

## Screenshots

![Unit Files](screenshots/unit%20file.png)

![Unit Journal](screenshots/Journal.png)

![Analyze](screenshots/analyse%20blame.png)

## Credit
This project is inspired by the work of Guillaume Gomez https://github.com/GuillaumeGomez/systemd-manager/

## Contributing
Contact me on my [GitHub](https://github.com/plrigaux/sysd-manager) if you want to contribute to this project.

## Project Roadmap
For now new features are planned to be added, such as:
* File list browser
* Non-blocking calls
* Syntax highlighting 
* Always administrator mode
* Svec the window state
* Improve UX
    * Better layout
    * Follow Dark and Light syte switch

## Installation Instructions


### From your computer
For the moment:
* Download and install rust https://www.rust-lang.org/tools/install
* Install needed libraries (GTK4, ...)
  * Install GTK 4 and the build essentials. https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_linux.html
* Compile and run  ```cargo run```

### Intall on RHEL, Fedora, and CentOS based distributions
You can install the application from COPR

#### Add the repo
First, you need to have dnf-plugins-core installed
```
sudo dnf install dnf-plugins-core
```

Then you can enable the repo with the following command
```
sudo dnf copr enable plrigaux/sysd-manager
```
#### Install with dnf

Then you can simply install sysd-manager with the following command
```
sudo dnf install sysd-manager
```

### Generate a RPM localy
You can generate youe rpm localy with the help of the crate `cargo-generate-rpm`.

#### Install
```
cargo install cargo-generate-rpm
```

#### Usage
```
cargo build --release
strip -s target/release/sysd-manager
cargo generate-rpm
```

#### Install with dnf

Then you can install sysd-manager with the following command 

*Don't forget to ajust the the rpm file path*
```
sudo dnf localinstall target/generate-rpm/sysd-manager[version-release-arch].rpm
```

#### Setup 
```bash
cargo install cargo-generate-rpm
```
#### Usage
Run the following script. 

```bash
sh ./create_rpm
```

It will create a rpm file in the target/generate-rpm subdirectory.

### Flatpack
*For now Flatpack is too restrictive*

### APT
*Later*



[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)