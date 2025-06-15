use adw::prelude::AdwDialogExt;
use gettextrs::pgettext;
use gtk::prelude::{BoxExt, ButtonExt, WidgetExt};

pub fn new(command_line: Option<String>, file_link: Option<String>) -> adw::Dialog {
    let content = inner_msg(command_line, file_link);

    let close_button = gtk::Button::builder()
        .label("Close")
        .margin_bottom(5)
        .margin_top(20)
        .halign(gtk::Align::Center)
        .build();

    content.append(&close_button);

    let dialog = adw::Dialog::builder().child(&content).build();
    {
        let dialog = dialog.clone();
        close_button.connect_clicked(move |_b| {
            dialog.close();
        });
    }
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
        .label("Flatpak permission needed!")
        .build();

    content.append(&title);

    let description = gtk::Label::builder()
    .selectable(true)
    .label(pgettext("unit file", "To save this file content, it requires permission to talk to <b>org.freedesktop.Flatpak</b> D-Bus interface when the program is packaged as a Flatpak.

<b>Option 1:</b> You can use <a href=\"https://flathub.org/apps/com.github.tchx84.Flatseal\">Flatseal</a>. Under Session Bus Talks add <b>org.freedesktop.Flatpak</b> and restart the program."))
    .use_markup(true)
    .wrap(true)
    .build();

    content.append(&description);
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
        format!(
            "<b>Option 2:</b> Edit the <a href=\"file://{}\">file</a> through another editor.",
            file_link
        )
    } else if let Some(cmd) = command_line {
        format!(
            "<b>Option 2:</b> In your terminal, run the command: <u>{}</u>",
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
