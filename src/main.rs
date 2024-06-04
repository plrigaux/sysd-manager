/// from https://github.com/GuillaumeGomez/systemd-manager
extern crate gtk;
//#[macro_use]
extern crate log;

mod systemd_gui;     // Contains all of the heavy GUI-related work
mod systemd;
mod grid_cell;
use gtk::glib;
mod menu;


fn main() -> glib::ExitCode {
    systemd_gui::launch()
}
