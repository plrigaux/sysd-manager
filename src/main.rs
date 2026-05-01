extern crate dotenv;
extern crate gtk;
extern crate systemd;
extern crate tracing;

mod analyze;
mod consts;
mod errors;
mod systemd_gui;
mod utils;
mod widget;

use crate::consts::ACTION_APP_CREATE_UNIT;
use crate::systemd_gui::set_is_dark;
use adw::prelude::AdwApplicationExt;
use base::enums::UnitDBusLevel;
use base::{RunMode, consts::APP_ID};
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use gettextrs::gettext;
use gio::glib::translate::FromGlib;
use gtk::{
    gdk,
    gio::{self},
    glib,
    prelude::*,
};
use std::env;
use systemd::data::UnitInfo;
use systemd_gui::new_settings;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use widget::{
    app_window::{AppWindow, menu},
    preferences::{
        PreferencesDialog,
        data::{DbusLevel, KEY_PREF_PREFERRED_COLOR_SCHEME, PREFERENCES},
    },
};

const DOMAIN_NAME: &str = "sysd-manager";
fn main() -> glib::ExitCode {
    #[cfg(all(feature = "flatpak", feature = "appimage"))]
    compile_error!("Features flatpack and appimage can't be combined");

    dotenv().ok();

    let timer = tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_owned());
    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_line_number(true)
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let (unit, command, level, run_mode, args) = handle_args();

    #[allow(clippy::single_match)]
    match command {
        Some(Command::Test { test }) => {
            info!("End test");

            systemd::runtime().block_on(async move {
                systemd::test(&test, level).await;
            });

            return gtk::glib::ExitCode::SUCCESS;
        }

        _ => {}
    }

    info!("LANGUAGE {:?}", env::var("LANGUAGE"));
    let textdomain_dir = env::var("TEXTDOMAINDIR");
    info!("env TEXTDOMAINDIR {textdomain_dir:?}");
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
    info!("bindtextdomain path {path:?}");

    match gettextrs::bind_textdomain_codeset(DOMAIN_NAME, "UTF-8") {
        Ok(v) => info!("bind_textdomain_codeset {v:?}"),
        Err(error) => error!("Unable to set the text domain encoding Error: {error:?}"),
    }

    // Specify the name of the .mo file to use.
    match gettextrs::textdomain(DOMAIN_NAME) {
        Ok(v) => info!("textdomain {:?}", String::from_utf8_lossy(&v)),
        Err(error) => error!("Unable to switch to the text domain Error: {error:?}"),
    }

    // Ask gettext for UTF-8 strings. THIS CRATE CAN'T HANDLE NON-UTF-8 DATA!
    if let Err(error) = gettextrs::bind_textdomain_codeset(DOMAIN_NAME, "UTF-8") {
        error!("bind_textdomain_codeset Error: {error:?}");
    };

    info!("Program starting up");
    // Just a simple log that it's all ok. Need to set env RUST_LOG="info" to see it
    info!("{}", gettext("Program starting up"));

    #[cfg(feature = "flatpak")]
    info!("Flatpak version");

    #[cfg(feature = "appimage")]
    info!("AppImage version");

    debug!("Run mode: {:?}", run_mode);

    //systemd::init(run_mode);

    if let Err(e) = gio::resources_register_include!("sysd-manager.gresource") {
        warn!("Failed to register resources. Error: {e:?}");
    }

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|application| {
        let style_manager = application.style_manager();
        menu::on_startup(application);

        let settings = systemd_gui::new_settings();

        let preferred_color_scheme_id = settings.get::<i32>(KEY_PREF_PREFERRED_COLOR_SCHEME);
        let preferred_color_scheme: adw::ColorScheme =
            unsafe { adw::ColorScheme::from_glib(preferred_color_scheme_id) };

        info!("id {preferred_color_scheme_id:?} color {preferred_color_scheme:?}");
        style_manager.set_color_scheme(preferred_color_scheme);
        load_css(&style_manager);
    });

    app.connect_activate(move |application| {
        build_ui(application, unit.as_ref(), args.create);

        //Start the Proxy after the app is loaded
        #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
        crate::systemd::runtime().spawn(async move {
            systemd::init_proxy_async(run_mode).await;
        });
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

fn build_ui(application: &adw::Application, unit: Option<&UnitInfo>, create: bool) {
    let window = AppWindow::new(application);

    let style_manager = application.style_manager();

    {
        let window = window.clone();

        let is_dark = style_manager.is_dark();
        info!("is dark {is_dark}");

        set_is_dark(is_dark);
        window.set_inter_message(&widget::InterPanelMessage::IsDark(is_dark));

        style_manager.connect_dark_notify(move |style_manager: &adw::StyleManager| {
            let is_dark = style_manager.is_dark();
            info!("is dark {is_dark}");

            let resource = css_resource_light_dark(is_dark);
            load_css_ress(resource);
            window.set_inter_message(&widget::InterPanelMessage::IsDark(is_dark));
            set_is_dark(is_dark);
        });
    }

    window.set_unit(unit);

    window.present();

    if PREFERENCES.is_app_first_connection() {
        info!("Is application first connection");

        let pdialog = PreferencesDialog::new(Some(&window));

        adw::prelude::AdwDialogExt::present(&pdialog, Some(&window));
    }

    if create {
        glib::spawn_future_local(async move {
            if let Err(err) =
                gtk::prelude::WidgetExt::activate_action(&window, ACTION_APP_CREATE_UNIT, None)
            {
                warn!("{:?}", err);
            };
        });
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

    /// Development mode (uses dev proxy service)
    #[arg(short, long, default_value_t = false)]
    dev: bool,

    /// Normal mode (uses normal proxy service)
    #[arg(short, long, default_value_t = false)]
    normal: bool,

    /// Open create unit Window
    #[arg(short, long, default_value_t = false)]
    create: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

/// Doc comment
#[derive(Subcommand, Debug, Clone, PartialEq)]
enum Command {
    /// Install Flatpak proxy files
    #[cfg(feature = "flatpak")]
    Install,

    /// Test some api call
    Test {
        #[arg( default_value_t = ("null").to_string())]
        test: String,
    },

    /// Run has proxy (used in flatpak distrubution)
    #[cfg(feature = "flatpak")]
    Proxy,
}

fn handle_args() -> (
    Option<UnitInfo>,
    Option<Command>,
    UnitDBusLevel,
    RunMode,
    Args,
) {
    let args = Args::parse();

    let run_mode = RunMode::from_flags(args.dev, args.normal);

    let current_level = PREFERENCES.dbus_level();

    debug!("Current level: {current_level:?}");
    let (app_level, unit_level) = match (args.system, args.user) {
        (true, _) => (DbusLevel::System, UnitDBusLevel::System),
        (false, true) => (DbusLevel::UserSession, UnitDBusLevel::UserSession),
        (false, false) => (current_level, UnitDBusLevel::System),
    };

    let settings = new_settings();
    PREFERENCES.set_and_save_dbus_level(app_level, &settings);

    let unit = if let Some(ref unit_name) = args.unit {
        match systemd::fetch_unit(unit_level, &unit_name) {
            Ok(unit) => Some(unit),
            Err(e) => {
                warn!("Cli unit: {e:?}");
                None
            }
        }
    } else {
        None
    };

    (unit, args.command.clone(), unit_level, run_mode, args)
}
