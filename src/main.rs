extern crate dotenv;
extern crate env_logger;
extern crate gtk;
extern crate log;

mod analyze;
mod consts;
mod errors;
mod systemd;
mod systemd_gui;
mod utils;
mod widget;

use clap::{Parser, command};
use gtk::{
    gdk,
    gio::{self},
    glib,
    prelude::*,
};

use log::{debug, info, warn};

use dotenv::dotenv;
use systemd::{data::UnitInfo, enums::UnitDBusLevel};
use systemd_gui::{APP_ID, new_settings};
use widget::{
    app_window::{AppWindow, menu},
    preferences::{
        PreferencesDialog,
        data::{DbusLevel, PREFERENCES},
    },
};

include!(concat!(env!("OUT_DIR"), "/release_notes.rs"));

fn main() -> glib::ExitCode {
    println!("NOTE_VERSION {:?}", RELEASE_NOTES_VERSION);
    println!("RELEASE_NOTE {:?}", RELEASE_NOTES);
    dotenv().ok();

    env_logger::init();

    //std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/user/1000/bus");
    info!("Program starting up");

    let unit = handle_args();

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

    app.connect_activate(move |app| {
        build_ui(app, unit.as_ref());
    });

    //to not transfer args to gtk4
    app.run_with_args::<String>(&[])
}

fn load_css() {
    // Load the CSS file and add it to the provider

    let system_manager = adw::StyleManager::default();

    let resource = css_resource_light_dark(system_manager.is_dark());

    load_css_ress(resource);
}

fn css_resource_light_dark(is_dark: bool) -> &'static str {
    if is_dark {
        "/io/github/plrigaux/sysd-manager/style_dark.css"
    } else {
        "/io/github/plrigaux/sysd-manager/style.css"
    }
}

fn load_css_ress(resource: &str) {
    // Load the CSS file and add it to the provider

    let provider = gtk::CssProvider::new();
    provider.load_from_resource(resource);

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(application: &adw::Application, unit: Option<&UnitInfo>) {
    let window = AppWindow::new(application);

    {
        let window = window.clone();
        let system_manager = adw::StyleManager::default();
        window.set_inter_action(&widget::InterPanelAction::IsDark(system_manager.is_dark()));

        system_manager.connect_dark_notify(move |style_manager: &adw::StyleManager| {
            let is_dark = style_manager.is_dark();
            info!("is dark {is_dark}");

            let resource = css_resource_light_dark(is_dark);
            load_css_ress(resource);
            window.set_inter_action(&widget::InterPanelAction::IsDark(is_dark));
        });
    }

    window.set_unit(unit);

    window.present();

    if PREFERENCES.is_app_first_connection() {
        info!("Is application first connection");

        let pdialog = PreferencesDialog::new(Some(&window));

        adw::prelude::AdwDialogExt::present(&pdialog, Some(&window));
    }
}

/// A GUI interface to manage systemd units
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

fn handle_args() -> Option<UnitInfo> {
    let args = Args::parse();

    let current_level = PREFERENCES.dbus_level();

    debug!("Current level: {:?}", current_level);
    let (app_level, unit_level) = match (args.system, args.user) {
        (true, _) => (DbusLevel::System, UnitDBusLevel::System),
        (false, true) => (DbusLevel::UserSession, UnitDBusLevel::UserSession),
        (false, false) => (current_level, UnitDBusLevel::System),
    };

    PREFERENCES.set_dbus_level(app_level);

    let current_level = PREFERENCES.dbus_level();
    debug!("Current level: {:?}", current_level);
    if current_level != app_level {
        let settings = new_settings();
        PREFERENCES.save_dbus_level(&settings);
    }

    let unit_name = args.unit?;

    match systemd::fetch_unit(unit_level, &unit_name) {
        Ok(unit) => Some(unit),
        Err(e) => {
            warn!("Cli unit: {:?}", e);
            None
        }
    }
}
