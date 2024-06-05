/// from https://github.com/GuillaumeGomez/systemd-manager
extern crate gtk;
//#[macro_use]
extern crate log;
extern crate env_logger;

mod systemd_gui;     // Contains all of the heavy GUI-related work
mod systemd;
mod grid_cell;
use gtk::glib;
mod menu;
mod analyze;

use log::info;

extern crate dotenv;

use dotenv::dotenv;

fn main() -> glib::ExitCode {
    dotenv().ok();

    env_logger::init();

    info!("Program starting up");

    systemd_gui::launch()
}
