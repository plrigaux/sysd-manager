use adw::prelude::{AdwDialogExt, AlertDialogExt, AlertDialogExtManual};

use gettextrs::pgettext;
use gtk::prelude::WidgetExt;

pub(super) const PROCEED: &str = "proceed";

pub fn proxy_service_not_started(service_name: Option<&str>) -> adw::AlertDialog {
    //TODO tranlate
    let body = "Failed to perform action. The proxy service might be inactive.\nPlease install and start the following service";

    //TODO tranlate
    let header = "Operation Failed";

    let dialog = adw::AlertDialog::builder()
        .heading(header)
        .body(body)
        .can_close(true)
        .build();

    //TODO tranlate
    dialog.add_responses(&[("cancel", "_Cancel"), ("save", "_Save")]);

    if let Some(service_name) = service_name {
        let label = gtk::Label::builder()
            .label(service_name)
            .selectable(true)
            .build();

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

    //TODO tranlate
    dialog.add_responses(&[("close", &pgettext("flatpak", "_Close"))]);
    dialog
}
