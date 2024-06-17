use gtk::{glib, gio, gdk, prelude::ActionMapExtManual};
use gtk::{prelude::*, ListBox};
use rowitem::Metadata;

use crate::analyze::build_analyze_window;
use crate::systemd;
use log::error;

pub mod rowitem;

static LOGO_SVG: &[u8] = include_bytes!("../../resources/icons/hicolor/scalable/org.tool.sysd-manager.svg");
pub const APP_TITLE : &str = "SysD Manager";


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
            let analyze_blame_window = build_systemd_info();
            analyze_blame_window.present();
        })
        .build();

    app.add_action_entries([about, analyze_blame, systemd_info]);
}

fn create_about() -> gtk::AboutDialog {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const CARGO_PKG_AUTHORS : &str = env!("CARGO_PKG_AUTHORS");
    const CARGO_PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

    let authors: Vec<&str> = CARGO_PKG_AUTHORS.split(',').collect();

    let bytes = glib::Bytes::from_static(LOGO_SVG);
    let logo = gdk::Texture::from_bytes(&bytes).expect("gtk-rs.svg to load");

    let about = gtk::AboutDialog::builder()
        .authors(authors )
        .name("About")
        .program_name(APP_TITLE)
        .modal(true)
        .version(VERSION)
        .license_type(gtk::License::Gpl30)
        .comments(CARGO_PKG_DESCRIPTION)
        .logo(&logo)
        .website("https://github.com/plrigaux/sysd-manager")
        .build();

    about
}

fn build_systemd_info() -> gtk::Window {
    let systemd_info = build_systemd_info_data();

    let window = gtk::Window::builder()
        .title("Systemd Info Blame")
        .default_height(600)
        .default_width(600)
        .child(&systemd_info)
        .build();

    window
}

fn build_systemd_info_data() -> gtk::ScrolledWindow {

    let store = gio::ListStore::new::<rowitem::Metadata>();

    let Ok(map) = systemd::fetch_system_info() else {
        let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .focusable(true)
            .build();

        return unit_analyse_scrolled_window;
    };

    for (key, value) in map {
        store.append(&rowitem::Metadata::new(key, value));
    }

    let no_selection = gtk::SingleSelection::new(Some(store));

    let list_box = ListBox::builder().build();

    list_box.bind_model(Some(&no_selection), |object| {
        let meta = match object.downcast_ref::<Metadata>() {
            Some(any_objet) => any_objet,
            None => {
                error!("No linked object");
                let list_box_row = gtk::ListBoxRow::new();
                return list_box_row.upcast::<gtk::Widget>();
            }
        };

        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 15);

        const SIZE: usize = 30;

        let mut tmp = String::new();
        let mut long_text = false;
        let key_label = if meta.col1().chars().count() > SIZE {
            long_text = true;
            tmp.push_str(&meta.col1()[..(SIZE - 3)]);
            tmp.push_str("...");
            &tmp
        } else {
            tmp.push_str(meta.col1().as_str());
            &tmp
        };

        let l1 = gtk::Label::builder()
            .label(key_label)
            .width_chars(SIZE as i32)
            .xalign(0.0)
            .max_width_chars(30)
            .single_line_mode(true)
            .build();

        if long_text {
            l1.set_tooltip_text(Some(&meta.col1()));
        }

        let l2 = gtk::Label::new(Some(&meta.col2()));

        box_.append(&l1);
        box_.append(&l2);

        box_.upcast::<gtk::Widget>()
    });
    //list_box.set_mo

    let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&list_box)
        .build();

    unit_analyse_scrolled_window
}
