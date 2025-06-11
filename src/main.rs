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

use std::env;

use adw::prelude::AdwApplicationExt;
use clap::{Parser, command};

use gettextrs::gettext;
use gio::glib::translate::FromGlib;
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
        data::{DbusLevel, KEY_PREF_PREFERRED_COLOR_SCHEME, PREFERENCES},
    },
};

const DOMAIN_NAME: &str = "sysd-manager";
fn main() -> glib::ExitCode {
    dotenv().ok();
    env_logger::init();

    info!("LANGUAGE {:?}", env::var("LANGUAGE"));
    let textdomain_dir = env::var("TEXTDOMAINDIR");
    info!("env TEXTDOMAINDIR {:?}", textdomain_dir);
    let locale_dir = if let Ok(domain_dir) = textdomain_dir {
        domain_dir
    } else {
        #[cfg(feature = "flatpak")]
        let domain_dir = "/app/share/locale".to_owned();

        #[cfg(not(feature = "flatpak"))]
        let domain_dir = "/usr/share/locale".to_owned();

        domain_dir
    };

    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");

    // Set up gettext translations
    let path =
        gettextrs::bindtextdomain(DOMAIN_NAME, locale_dir).expect("Unable to bind the text domain");
    info!("bindtextdomain path {:?}", path);

    match gettextrs::bind_textdomain_codeset(DOMAIN_NAME, "UTF-8") {
        Ok(v) => log::info!("bind_textdomain_codeset {:?}", v),
        Err(error) => log::error!("Unable to set the text domain encoding Error: {:?}", error),
    }

    // Specify the name of the .mo file to use.
    match gettextrs::textdomain(DOMAIN_NAME) {
        Ok(v) => log::info!("textdomain {:?}", String::from_utf8_lossy(&v)),
        Err(error) => log::error!("Unable to switch to the text domain Error: {:?}", error),
    }

    // Ask gettext for UTF-8 strings. THIS CRATE CAN'T HANDLE NON-UTF-8 DATA!
    if let Err(error) = gettextrs::bind_textdomain_codeset(DOMAIN_NAME, "UTF-8") {
        log::error!("bind_textdomain_codeset Error: {:?}", error);
    };

    info!("Program starting up");
    // Just a simple log that it's all ok. Need to set env RUST_LOG="info" to see it
    info!("{}", gettext("Program starting up"));

    let unit = handle_args();

    #[cfg(feature = "flatpak")]
    {
        info!("Flatpak version");
    }

    match gio::resources_register_include!("sysd-manager.gresource") {
        Ok(_) => (),
        Err(e) => warn!("Failed to register resources. Error: {:?}", e),
    }

    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|application| {
        let style_manager = application.style_manager();
        menu::on_startup(application);

        let settings = new_settings();
        let preferred_color_scheme_id = settings.get::<i32>(KEY_PREF_PREFERRED_COLOR_SCHEME);
        let preferred_color_scheme: adw::ColorScheme =
            unsafe { adw::ColorScheme::from_glib(preferred_color_scheme_id) };

        info!(
            "id {:?} color {:?}",
            preferred_color_scheme_id, preferred_color_scheme
        );
        style_manager.set_color_scheme(preferred_color_scheme);
        load_css(&style_manager);
    });

    app.connect_activate(move |application| {
        build_ui(application, unit.as_ref());
    });

    //to not transfer args to gtk4
    app.run_with_args::<String>(&[])
}

/// Load the CSS file and add it to the provider
fn load_css(style_manager: &adw::StyleManager) {
    let resource = css_resource_light_dark(style_manager.is_dark());

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

    let style_manager = application.style_manager();

    {
        let window = window.clone();

        let is_dark = style_manager.is_dark();
        info!("is dark {is_dark}");

        window.set_inter_message(&widget::InterPanelMessage::IsDark(is_dark));

        style_manager.connect_dark_notify(move |style_manager: &adw::StyleManager| {
            let is_dark = style_manager.is_dark();
            info!("is dark {is_dark}");

            let resource = css_resource_light_dark(is_dark);
            load_css_ress(resource);
            window.set_inter_message(&widget::InterPanelMessage::IsDark(is_dark));
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
