use gtk::gio::ResourceLookupFlags;
use gtk::{gdk, gio, prelude::ActionMapExtManual};
use gtk::prelude::*;


use crate::analyze::build_analyze_window;
use crate::info;
use log::warn;

pub const APP_TITLE: &str = "SysD Manager";

fn build_popover_menu() -> gtk::PopoverMenu {
    let menu = gio::Menu::new();

    menu.append(Some("Analyze Blame"), Some("app.analyze_blame"));
    menu.append(Some("About"), Some("app.about"));
    menu.append(Some("Systemd Info"), Some("app.systemd_info"));

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
        .activate(|_, _, _| {
            let analyze_blame_window = build_analyze_window();
            analyze_blame_window.present();
        })
        .build();

    let systemd_info = gio::ActionEntry::builder("systemd_info")
        .activate(|_, _, _| {
            let analyze_blame_window = info::build_systemd_info();
            analyze_blame_window.present();
        })
        .build();

    app.add_action_entries([about, analyze_blame, systemd_info]);
}

fn create_about() -> gtk::AboutDialog {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    const CARGO_PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

    let authors: Vec<&str> = CARGO_PKG_AUTHORS.split(',').collect();

    let about = gtk::AboutDialog::builder()
        .authors(authors)
        .name("About")
        .program_name(APP_TITLE)
        .modal(true)
        .version(VERSION)
        .license_type(gtk::License::Gpl30)
        .comments(CARGO_PKG_DESCRIPTION)
        .website("https://github.com/plrigaux/sysd-manager")
        .build();

    //TODO create const for the path prefix
    match gio::functions::resources_lookup_data(
        "/org/tool/sysd/manager/org.tool.sysd-manager.svg",
        ResourceLookupFlags::NONE,
    ) {
        Ok(bytes) => {
            let logo = gdk::Texture::from_bytes(&bytes).expect("gtk-rs.svg to load");
            about.set_logo(Some(&logo));
        }
        Err(e) => warn!("Fail to load logo: {}", e),
    };

    about
}
