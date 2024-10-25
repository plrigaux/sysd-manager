use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::widget::preferences::data::PREFERENCES;
use crate::widget::preferences::PreferencesDialog;
use crate::widget::title_bar::menu;
use crate::widget::{self};
use log::info;

pub const APP_ID: &str = "io.github.plrigaux.sysd-manager";

pub fn launch() -> glib::ExitCode {
    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|app| {
        load_css();
        menu::on_startup(app)
    });
    app.connect_activate(build_ui);

    app.run()
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
    {
        /*         let search_button = title_bar_elements.search_button.clone();
        search_entry.connect_search_started(move |_| {
            search_button.set_active(true);
        }); */
    }
    {
        /*         let search_button = title_bar_elements.search_button.clone();
        search_entry.connect_stop_search(move |_| {
            search_button.set_active(false);
        }); */
    }

    let window = widget::app_window::AppWindow::new(application);

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
