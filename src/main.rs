/// from https://github.com/GuillaumeGomez/systemd-manager
extern crate gtk;


mod systemd_gui;     // Contains all of the heavy GUI-related work
mod systemd {
    pub mod analyze; // Support for systemd-analyze
    pub mod dbus;    // The dbus backend for systemd
}
mod grid_cell;

fn main() {
    systemd_gui::launch();
}
