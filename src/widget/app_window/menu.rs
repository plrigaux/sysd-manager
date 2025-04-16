use adw::prelude::AdwDialogExt;
use adw::prelude::AlertDialogExt;
use gtk::glib;
use gtk::prelude::*;
use gtk::{gio, prelude::ActionMapExtManual};
use log::error;
use log::info;

use crate::analyze::build_analyze_window;
use crate::systemd;
use crate::systemd_gui::APP_ID;
use crate::widget::app_window::AppWindow;
use crate::widget::info_window;
use crate::widget::preferences::PreferencesDialog;

pub const APP_TITLE: &str = "SysD Manager";

include!(concat!(env!("OUT_DIR"), "/release_notes.rs"));

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
            let systemd_info_window = info_window::InfoWindow::new(None);
            systemd_info_window.fill_systemd_info();

            if let Some(first_window) = application.windows().first() {
                systemd_info_window.set_transient_for(Some(first_window));
                systemd_info_window.set_modal(true);
            }

            systemd_info_window.present();
        })
        .build();

    let preferences: gio::ActionEntry<adw::Application> = gio::ActionEntry::builder("preferences")
        .activate(|application: &adw::Application, _, _| {
            if let Some(win) = application.active_window() {
                let app_window: Option<&AppWindow> = win.downcast_ref::<AppWindow>();

                let pdialog = PreferencesDialog::new(app_window);
                pdialog.present(Some(&win));
            } else {
                let pdialog = PreferencesDialog::new(None);
                pdialog.present(None::<&gtk::Widget>);
            }
        })
        .build();

    let reload_all_units: gio::ActionEntry<adw::Application> =
        gio::ActionEntry::builder("reload_all_units")
            .activate(|application: &adw::Application, simple_action, _variant| {
                let simple_action = simple_action.clone();
                let application = application.clone();

                glib::spawn_future_local(async move {
                    simple_action.set_enabled(false);

                    let res = gio::spawn_blocking(systemd::reload_all_units)
                        .await
                        .expect("Task needs to finish successfully.");

                    simple_action.set_enabled(true);

                    match res {
                        Ok(_) => {
                            info!("All units relaoded!");
                            add_toast(&application, "All units relaoded!");
                        }
                        Err(e) => {
                            error!("Roload failed {:?}", e);
                            add_toast(&application, "Reload failed!");
                        }
                    }
                });
            })
            .build();

    app.set_accels_for_action("app.preferences", &["<Ctrl>comma"]);

    app.add_action_entries([
        about,
        analyze_blame,
        systemd_info,
        preferences,
        reload_all_units,
    ]);
}

fn add_toast(application: &adw::Application, toast_msg: &str) {
    if let Some(win) = application.active_window() {
        let app_win_op: Option<&AppWindow> = win.downcast_ref::<AppWindow>();

        if let Some(app_win) = app_win_op {
            app_win.add_toast_message(toast_msg, false);
        }
    }
}

fn create_about() -> adw::AboutDialog {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    const CARGO_PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

    let authors: Vec<&str> = CARGO_PKG_AUTHORS.split(',').collect();

    let about = adw::AboutDialog::builder()
        .developers(authors.clone())
        .designers(authors)
        .name("About")
        .application_name(APP_TITLE)
        .application_icon(APP_ID)
        .version(VERSION)
        .license_type(gtk::License::Gpl30)
        .comments(CARGO_PKG_DESCRIPTION)
        .website("https://github.com/plrigaux/sysd-manager")
        .issue_url("https://github.com/plrigaux/sysd-manager/issues")
        .artists(["4nyNoob"])
        .build();

    about.add_acknowledgement_section(
        Some("Thank you for your support"),
        &["AsciiWolf", "Justin Searle"],
    );
    //<a target="_blank" href="https://icons8.com/icon/37724/anonymous-mask">Anonymous Mask</a> icon by <a target="_blank" href="https://icons8.com">Icons8</a>
    if let Some(rn_version) = RELEASE_NOTES_VERSION {
        about.set_release_notes_version(rn_version);
    }

    if let Some(release_notes) = RELEASE_NOTES {
        about.set_release_notes(release_notes);
    }

    about.add_legal_section("icons8", Some(r#"<a href="https://icons8.com/icon/37724/anonymous-mask">Anonymous Mask</a> icon by <a  href="https://icons8.com">Icons8</a>"#), gtk::License::Unknown, Some("license"));

    about
}
