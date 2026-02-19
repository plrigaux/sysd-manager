use adw::prelude::{AdwDialogExt, AlertDialogExt, AlertDialogExtManual};

use gettextrs::pgettext;
use gtk::prelude::WidgetExt;
use tracing::{info, warn};

use crate::widget::app_window::AppWindow;

pub(super) const PROCEED: &str = "proceed";

pub fn proxy_service_not_started(
    service_name: Option<&str>,
    app_window: Option<&AppWindow>,
) -> adw::AlertDialog {
    let body = pgettext(
        "warning",
        "Failed to perform action. The proxy service might be inactive.\nPlease install and start the following service",
    );

    let header = pgettext("warning", "Operation Failed");

    let dialog = adw::AlertDialog::builder()
        .heading(header)
        .body(body)
        .can_close(true)
        .build();

    let cancel_label = pgettext("warning", "_Cancel");
    let save_label = pgettext("warning", "_Save");
    dialog.add_responses(&[("cancel", &cancel_label), ("save", &save_label)]);

    if let Some(service_name) = service_name {
        let label = gtk::LinkButton::builder()
            .label(service_name)
            .uri(format!("unit://{service_name}"))
            .build();

        if let Some(app_window) = app_window {
            let app_window = app_window.clone();
            label.connect_activate_link(move |button_link| {
                use base::enums::UnitDBusLevel;

                let uri = button_link.uri();
                info!("link uri: {}", uri);

                if !uri.starts_with("unit://") {
                    return glib::Propagation::Proceed;
                }

                let Some(unit_name) = uri.strip_prefix("unit://") else {
                    return glib::Propagation::Proceed;
                };

                let (unit_name, level) = match unit_name.split_once("?") {
                    Some((prefix, suffix)) => (prefix, UnitDBusLevel::from_short(suffix)),
                    None => (unit_name, UnitDBusLevel::System),
                };

                info!("open unit {:?} at level {}", unit_name, level.short());

                let unit = systemd::fetch_unit(level, unit_name)
                    .inspect_err(|e| warn!("Cli unit: {e:?}"))
                    .ok();

                app_window.set_unit(unit.as_ref());

                glib::Propagation::Stop
            });
        }

        dialog.set_extra_child(Some(&label));
    }

    dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("save"));
    dialog.set_close_response("cancel");

    dialog.set_margin_top(5);
    dialog.set_margin_start(5);
    dialog.set_can_close(true);

    dialog
}

pub fn revert_drop_in_alert(unit_name: &str) -> adw::AlertDialog {
    //TODO tranlate
    let body = format!("You are about to clear the Drop-ins for unit <b>{unit_name}</b>");

    //TODO tranlate
    let header = "Warning!";

    let dialog = adw::AlertDialog::builder()
        .heading(header)
        .body(body)
        .can_close(true)
        .body_use_markup(true)
        .build();

    //TODO tranlate
    dialog.add_responses(&[("cancel", "_Cancel"), ("proceed", "_Proceed")]);

    dialog.set_response_appearance(PROCEED, adw::ResponseAppearance::Destructive);
    dialog.set_response_appearance("cancel", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    dialog
}

pub fn flatpak_permision_alert() -> adw::AlertDialog {
    let body = pgettext(
        "flatpak",
        "You need to jailbreak your Flatpak application to be able to save files on the host system.\n\n\
                            Follow the <a href=\"https://github.com/plrigaux/sysd-manager/wiki/Flatpak\">link</a> to know how to aquire needed permission.",
    );

    let header = pgettext("flatpak", "Missing Flatpak Permission!");

    let dialog = adw::AlertDialog::builder()
        .heading(header)
        .body(body)
        .can_close(true)
        .body_use_markup(true)
        .close_response("close")
        .default_response("close")
        .build();

    //Button label
    dialog.add_responses(&[("close", &pgettext("flatpak", "_Close"))]);
    dialog
}
