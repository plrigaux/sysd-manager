use adw::prelude::AdwDialogExt;
use adw::prelude::AlertDialogExt;
use gtk::prelude::*;
use gtk::{gio, prelude::ActionMapExtManual};

use crate::analyze::build_analyze_window;
use crate::info;
use crate::systemd_gui::APP_ID;
use log::error;

use super::preferences;

pub const APP_TITLE: &str = "SysD Manager";

fn build_popover_menu() -> gtk::PopoverMenu {
    let menu = gio::Menu::new();

    menu.append(Some("Analyze Blame"), Some("app.analyze_blame"));
    menu.append(Some("About"), Some("app.about"));
    menu.append(Some("Systemd Info"), Some("app.systemd_info"));
    menu.append(Some("Preferences"), Some("app.preferences"));

    let unit_menu_popover = gtk::PopoverMenu::builder().menu_model(&menu).build();

    unit_menu_popover
}

pub fn build_menu() -> gtk::MenuButton {
    let popover = build_popover_menu();
    let menu_button = gtk::MenuButton::builder()
        .focusable(true)
        .receives_default(true)
        .icon_name("open-menu-symbolic")
        .halign(gtk::Align::End)
        .direction(gtk::ArrowType::Down)
        .popover(&popover)
        .build();

    menu_button
}

pub fn on_startup(app: &adw::Application) {
    let about = gio::ActionEntry::builder("about")
        .activate(|application: &adw::Application, _, _| {
            let about = create_about();
            if let Some(win) = application.active_window() {
                about.present(Some(&win));
            } else {
                about.present(None::<&gtk::Widget>);
            }
        })
        .build();

    let analyze_blame = gio::ActionEntry::builder("analyze_blame")
        .activate(|application: &adw::Application, _b, _c| {
            let wins = application.windows();
            match build_analyze_window() {
                Ok(analyze_blame_window) => {
                    if let Some(first_window) = wins.first() {
                        analyze_blame_window.set_transient_for(Some(first_window));
                        analyze_blame_window.set_modal(true);
                    }

                    analyze_blame_window.present();
                }
                Err(sd_error) => {
                    let resp = sd_error.gui_description();

                    if let Some(resp) = resp {
                        let alert = adw::AlertDialog::builder()
                            .heading("Unavailable")
                            .body_use_markup(true)
                            .body(resp)
                            .close_response("close")
                            .build();

                        alert.add_response("close", "Close");
                        let firt_window = wins.first();
                        alert.present(firt_window);
                    }
                }
            };
        })
        .build();

    let systemd_info = gio::ActionEntry::builder("systemd_info")
        .activate(|application: &adw::Application, _, _| {
            let systemd_info_window = info::build_systemd_info();

            if let Some(first_window) = application.windows().first() {
                systemd_info_window.set_transient_for(Some(first_window));
                systemd_info_window.set_modal(true);
            }

            systemd_info_window.present();
        })
        .build();

    let preferences: gio::ActionEntry<adw::Application> = gio::ActionEntry::builder("preferences")
        .activate(
            |application: &adw::Application, _, _| match preferences::build_preferences() {
                Ok(preferences_window) => {
                    if let Some(first_window) = application.windows().first() {
                        preferences_window.set_transient_for(Some(first_window));
                    }

                    preferences_window.present();
                }
                Err(e) => {
                    error! {"{:?}",e}
                }
            },
        )
        .build();

    app.add_action_entries([about, analyze_blame, systemd_info, preferences]);

    app.set_accels_for_action("app.preferences", &["<Ctrl>comma"]);
}

fn create_about() -> adw::AboutDialog {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    const CARGO_PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

    let authors: Vec<&str> = CARGO_PKG_AUTHORS.split(',').collect();

    let about = adw::AboutDialog::builder()
        .developers(authors)
        .name("About")
        .application_name(APP_TITLE)
        .application_icon(APP_ID)
        .version(VERSION)
        .license_type(gtk::License::Gpl30)
        .comments(CARGO_PKG_DESCRIPTION)
        .website("https://github.com/plrigaux/sysd-manager")
        .issue_url("https://github.com/plrigaux/sysd-manager/issues")
        .build();

    about
}
