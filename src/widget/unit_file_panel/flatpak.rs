use adw::prelude::{AdwDialogExt, AlertDialogExt, AlertDialogExtManual};

use gettextrs::pgettext;
use gtk::prelude::{BoxExt, WidgetExt};

use crate::format2;
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
    let body = format!("You are about to clear the Drop-ins for unit <b>{unit_name}<b>");

    //TODO tranlate
    let header = "Warning!";

    let dialog = adw::AlertDialog::builder()
        .heading(header)
        .body(body)
        .can_close(true)
        .build();

    //TODO tranlate
    dialog.add_responses(&[("cancel", "_Cancel"), ("proceed", "_Proceed")]);

    dialog.set_response_appearance(PROCEED, adw::ResponseAppearance::Destructive);
    dialog.set_response_appearance("cancel", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    dialog
}

pub fn inner_msg(command_line: Option<String>, file_link: Option<String>) -> gtk::Box {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .build();

    let title = gtk::Label::builder()
        .css_classes(["title-1"])
        .label(
            //flatpak permision error dialog title
            pgettext("unit file", "Flatpak permission needed!"),
        )
        .build();

    content.append(&title);

    let description1 = gtk::Label::builder()
    .selectable(true)
    .label(
        //flatpak permision error dialog line 1
        pgettext("unit file", "To save this file content, it requires permission to talk to <b>org.freedesktop.Flatpak</b> D-Bus interface when the program is packaged as a Flatpak."))
    .use_markup(true)
    .wrap(true)
    .build();

    let description2 = gtk::Label::builder()
    .selectable(true)
    .label(
        //flatpak permision error dialog line 2
        pgettext("unit file", "<b>Option 1:</b> You can use <a href=\"https://flathub.org/apps/com.github.tchx84.Flatseal\">Flatseal</a>. Under Session Bus Talks add <b>org.freedesktop.Flatpak</b> and restart the program."))
    .use_markup(true)
    .margin_top(15)
    .wrap(true)
    .build();

    content.append(&description1);
    content.append(&description2);
    //let texture = gtk::gdk::Texture::from_resource("/io/github/plrigaux/sysd-manager/add_permission_dark.mp4");

    let picture = gtk::Video::for_resource(Some(
        "/io/github/plrigaux/sysd-manager/add_permission_dark.mp4",
    ));

    picture.set_autoplay(true);
    picture.set_loop(true);
    picture.set_height_request(272);
    picture.set_width_request(576);

    content.append(&picture);

    let lbl = if let Some(file_link) = file_link {
        format2!(
            //flatpak permision error dialog option 2
            pgettext(
                "unit file",
                "<b>Option 2:</b> Edit the <a href=\"file://{}\">file</a> through another editor."
            ),
            file_link
        )
    } else if let Some(cmd) = command_line {
        format2!(
            //flatpak permision error dialog option 3
            pgettext(
                "unit file",
                "<b>Option 3:</b> In your terminal, run the command: <u>{}</u>"
            ),
            cmd
        )
    } else {
        String::new()
    };

    let description2 = gtk::Label::builder()
        .selectable(true)
        .label(lbl)
        .use_markup(true)
        .wrap(true)
        .xalign(0.0)
        .build();

    content.append(&description2);

    content
}
