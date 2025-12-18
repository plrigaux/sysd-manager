use adw::prelude::*;
use base::consts::APP_ID;
use base::enums::UnitDBusLevel;
use gettextrs::gettext;

use glib::VariantTy;
use gtk::glib;
use gtk::{gio, prelude::ActionMapExtManual};
use log::{error, info, warn};

use crate::consts::ACTION_DAEMON_RELOAD_BUS;
use crate::format2;
use crate::{
    analyze::build_analyze_window,
    consts::ACTION_DAEMON_RELOAD,
    systemd,
    widget::{
        app_window::AppWindow,
        info_window,
        preferences::{
            PreferencesDialog,
            data::{DbusLevel, PREFERENCES},
        },
        signals_dialog::SignalsWindow,
    },
};

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

    let signals = gio::ActionEntry::builder("signals")
        .activate(|application: &adw::Application, _, _| {
            let Some(window) = application.active_window() else {
                warn!("No window");
                return;
            };

            let Some(app_window) = window.downcast_ref::<AppWindow>() else {
                warn!("No app window");
                return;
            };

            let signals_window = if let Some(signals_window) = app_window.signals_window() {
                signals_window
            } else {
                let signals_window = SignalsWindow::new(app_window);
                app_window.set_signal_window(Some(&signals_window));
                //signals_window.set_transient_for(Some(&window));
                signals_window
            };

            signals_window.present();
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
                //pdialog.present(Some(&win));
                //gtk::prelude::GtkWindowExt::present(&pdialog);
            } else {
                let pdialog = PreferencesDialog::new(None);
                pdialog.present(None::<&gtk::Widget>);
            }
        })
        .build();

    let daemon_reload_all_units: gio::ActionEntry<adw::Application> =
        gio::ActionEntry::builder(ACTION_DAEMON_RELOAD)
            .activate(|application: &adw::Application, simple_action, _variant| {
                let simple_action = simple_action.clone();
                let application = application.clone();

                let Some(app_win_op) = application
                    .active_window()
                    .and_downcast_ref::<AppWindow>()
                    .cloned()
                else {
                    error!("Not an AppWindow");
                    return;
                };

                let dbus_level = PREFERENCES.dbus_level();

                daemon_reload_with_dialog(simple_action, app_win_op, dbus_level);
            })
            .build();

    let daemon_reload_all_units_with_bus: gio::ActionEntry<adw::Application> =
        gio::ActionEntry::builder(ACTION_DAEMON_RELOAD_BUS)
            .activate(
                |application: &adw::Application, simple_action, variant: Option<&glib::Variant>| {
                    let Some(app_win_op) = application
                        .active_window()
                        .and_downcast_ref::<AppWindow>()
                        .cloned()
                    else {
                        error!("Not an AppWindow");
                        return;
                    };
                    let user_session = variant.and_then(|v| v.get::<bool>());
                    let dbus_level = match user_session {
                        Some(true) => DbusLevel::UserSession,
                        Some(false) => DbusLevel::System,
                        None => {
                            warn!("No user session specified, using default");
                            PREFERENCES.dbus_level()
                        }
                    };
                    let simple_action = simple_action.clone();
                    daemon_reload_with_dialog(simple_action, app_win_op, dbus_level);
                },
            )
            .parameter_type(Some(VariantTy::BOOLEAN))
            .build();

    app.set_accels_for_action("app.preferences", &["<Ctrl>comma"]);

    app.add_action_entries([
        about,
        analyze_blame,
        systemd_info,
        preferences,
        daemon_reload_all_units,
        signals,
        daemon_reload_all_units_with_bus,
    ]);
}

fn daemon_reload_with_dialog(
    simple_action: gio::SimpleAction,
    app_win_op: AppWindow,
    dbus_level: DbusLevel,
) {
    match dbus_level {
        DbusLevel::UserSession => {
            daemon_relaod(simple_action, app_win_op, UnitDBusLevel::UserSession)
        }
        DbusLevel::System => daemon_relaod(simple_action, app_win_op, UnitDBusLevel::System),
        DbusLevel::SystemAndSession => {
            const SYSTEM: &str = "system";
            const USER: &str = "user";
            const CLOSE: &str = "zclose";

            let body = "Need to determine the bus to use for daemon reload.\n\n\
                        Please select either <b>System</b> or <b>User\u{00A0}Session</b> bus.";

            let alert = adw::AlertDialog::builder()
                .heading("Select bus")
                .body_use_markup(true)
                .body(body)
                .close_response(CLOSE)
                .can_close(true)
                .build();

            alert.add_responses(&[
                (CLOSE, "_Cancel"),
                (USER, "_User Session"),
                (SYSTEM, "_System"),
            ]);

            alert.set_response_appearance(SYSTEM, adw::ResponseAppearance::Suggested);
            alert.set_response_appearance(USER, adw::ResponseAppearance::Suggested);

            {
                let win = app_win_op.clone();
                alert.connect_response(None, move |_dialog, response| {
                    info!("Response {response}");

                    match response {
                        SYSTEM => {
                            daemon_relaod(simple_action.clone(), win.clone(), UnitDBusLevel::System)
                        }
                        USER => daemon_relaod(
                            simple_action.clone(),
                            win.clone(),
                            UnitDBusLevel::UserSession,
                        ),
                        _ => {
                            warn!("Dialog response not handled: {response}");
                        }
                    }
                });
            }
            alert.present(Some(&app_win_op));
        }
    }
}

fn daemon_relaod(
    simple_action: gio::SimpleAction,
    app_win_op: AppWindow,
    dbus_level: UnitDBusLevel,
) {
    glib::spawn_future_local(async move {
        simple_action.set_enabled(false);

        let user_session = dbus_level.user_session();
        let res = gio::spawn_blocking(async move || systemd::daemon_reload(dbus_level).await)
            .await
            .expect("Task needs to finish successfully.");

        match res.await {
            //TODO ??? await
            Ok(_) => {
                info!("All units reloaded! User session {}", user_session);
                let instance_level = if user_session {
                    //instance level user
                    gettext("user")
                } else {
                    //instance level system
                    gettext("system")
                };

                let msg = format2!(
                    "Systemd manager configuration reloaded at <b>{}</b> level!",
                    instance_level
                );
                add_toast(&app_win_op, &msg);
            }
            Err(e) => {
                error!("Daemon Reload failed {e:?}");
                let msg = gettext("Daemon Reload failed!");
                add_toast(&app_win_op, &msg); //TODO make red
            }
        }
        simple_action.set_enabled(true);
    });
}

fn add_toast(app_window: &AppWindow, toast_msg: &str) {
    app_window.add_toast_message(toast_msg, true, None);
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
        .translator_credits(
            "John Peter Sa <johnppetersa@gmail.com>
Pierre-Luc Rigaux
Priit Jõerüüt <hwlate@joeruut.com>",
        )
        .build();

    #[cfg(feature = "flatpak")]
    about.set_version(&format!("{} (Flatpak)", VERSION));

    about.add_acknowledgement_section(
        //about dialogue
        Some(&gettext("Thank you for your support")),
        &["AsciiWolf", "Justin Searle", "Damglador"],
    );

    if let Some(rn_version) = RELEASE_NOTES_VERSION {
        about.set_release_notes_version(rn_version);
    }

    if let Some(release_notes) = RELEASE_NOTES {
        let mut release_notes = String::from(release_notes);
        release_notes.push_str("<p>_________________________</p><p>Full release notes:</p><p>https://github.com/plrigaux/sysd-manager/blob/main/CHANGELOG.md</p>");
        about.set_release_notes(&release_notes);
    }

    about
}
