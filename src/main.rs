extern crate dotenv;
extern crate env_logger;
extern crate gtk;
extern crate log;

mod analyze;
mod errors;
mod systemd;
mod systemd_gui;
mod widget;

use clap::{command, ArgAction, Parser};
use gtk::{gdk, gio, glib, prelude::*};

use log::{info, warn};

use dotenv::dotenv;
use systemd_gui::APP_ID;
use widget::{
    app_window::{menu, AppWindow},
    preferences::{data::PREFERENCES, PreferencesDialog},
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the unit
    #[arg()]
    unit: Option<String>,

    /// Specify the user session bus
    #[arg(short, long)]
    user: bool,

    /// Specify the system session bus (This is the implied default)
    #[arg(short, long)]
    system: bool,
}

fn main() -> glib::ExitCode {
    dotenv().ok();

    env_logger::init();

    let args = Args::parse();


    info!("Program starting up");

    #[cfg(feature = "flatpak")]
    info!("Flatpak version");

    match gio::resources_register_include!("sysd-manager.gresource") {
        Ok(_) => (),
        Err(e) => warn!("Failed to register resources. Error: {:?}", e),
    }

    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|app| {
        load_css();
        menu::on_startup(app)
    });
    app.connect_activate(build_ui);

    let empty: Vec<String> = vec![];
    app.run_with_args(&empty)
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/io/github/plrigaux/sysd-manager/style.css");

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(application: &adw::Application) {
    let window = AppWindow::new(application);

    {
        let window = window.clone();
        let system_manager = adw::StyleManager::default();
        window.set_dark(system_manager.is_dark());

        system_manager.connect_dark_notify(move |a: &adw::StyleManager| {
            let is_dark = a.is_dark();
            info!("is dark {is_dark}");
            window.set_dark(is_dark);
        });
    }

    window.present();

    if PREFERENCES.is_app_first_connection() {
        info!("Is application first connection");

        let pdialog = PreferencesDialog::new();

        adw::prelude::AdwDialogExt::present(&pdialog, Some(&window));
    }
}
