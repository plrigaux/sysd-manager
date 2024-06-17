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
* Improve UX
    * Better layout
    * Follow Dark and Light syte switch

## Installation Instructions

For the moment:
* Download and install rust 
* Install needed libraries (GTK4, ...)
* Compile code
* Copy the binary in your PATH

### DNF
*Soon*

### Flatpack
*Soon*

### APT
*Later*



[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)