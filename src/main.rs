/// from https://github.com/GuillaumeGomez/systemd-manager
extern crate gtk;
//#[macro_use]
extern crate env_logger;
extern crate log;

mod grid_cell;
mod systemd;
mod systemd_gui; // Contains all of the heavy GUI-related work
use gtk::gio;
use gtk::glib;
mod analyze;
mod menu;

use log::{info, warn};

extern crate dotenv;

use dotenv::dotenv;

fn main() -> glib::ExitCode {
    dotenv().ok();

    env_logger::init();

    info!("Program starting up");

    match gio::resources_register_include!("sysd-manager.gresource") {
        Ok(_) => (),
        Err(e) => warn!("Failed to register resources. Error: {:?}", e),
    }

    systemd_gui::launch()
}
