use gtk::{gio, prelude::ActionMapExtManual};
use gtk::prelude::*;

fn build_popover_menu() -> gtk::PopoverMenu {
    let menu = gio::Menu::new();

    menu.append(Some("About"), Some("app.about"));
    menu.append(Some("_Quit"), Some("app.quit"));

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

pub fn on_startup(app: &gtk::Application) {
    let about = gio::ActionEntry::builder("about")
        .activate(|_, _, _| {
            let about = create_about();
            about.present();
        })
        .build();

    app.add_action_entries([about]);
}

fn create_about() -> gtk::AboutDialog  {

    let about = gtk::AboutDialog::builder()
    .authors(["Pierre-Luc Rigaux"])
    .name("About")
    .program_name("SysD manager")
    .modal(true)
    .version("0.0.1")
    .comments("This is comments")
    .build();

    about
}