use gtk::{gio, prelude::ActionMapExtManual};
use gtk::prelude::*;

use crate::analyze::build_analyze_window;

fn build_popover_menu() -> gtk::PopoverMenu {
    let menu = gio::Menu::new();

    menu.append(Some("Analyze Blame"), Some("app.analyze_blame"));
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

        let analyze_blame = gio::ActionEntry::builder("analyze_blame")
        .activate(|_ , _, _| {
            let analyze_blame_window = build_analyze_window();
            analyze_blame_window.present();
        })
        .build();

    app.add_action_entries([about, analyze_blame]);
}

fn create_about() -> gtk::AboutDialog  {

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let about = gtk::AboutDialog::builder()
    .authors(["Pierre-Luc Rigaux"])
    .name("About")
    .program_name("SysD manager")
    .modal(true)
    .version(VERSION)
    .comments("This is comments")
    .build();

    about
}