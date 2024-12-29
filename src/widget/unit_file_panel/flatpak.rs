use adw::prelude::AdwDialogExt;
use gtk::prelude::{BoxExt, ButtonExt};

pub fn new(file_link: &str) -> adw::Dialog {
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
    .label(format!("To save this file content, it requires permission to talk to <b>org.freedesktop.Flatpak</b> D-Bus interface when the program is packaged as a Flatpak.

<b>Option 1:</b> You can use Flatseal. Under Session Bus Talks add <b>org.freedesktop.Flatpak</b> and restart the program.

<b>Option 2:</b> Edit the <a href=\"file://{}\">file</a> through another editor.", file_link))
    .use_markup(true)
    .wrap(true)
    .build();

    content.append(&description);

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
